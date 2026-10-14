# SPDX-License-Identifier: MPL-2.0
# SPDX-FileCopyrightText: 2026 Apibim

"""Synthetic archive and CLI checks; no third-party fixtures or network."""

import hashlib
import os
import struct
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch
import zipfile
from pathlib import Path

import openusd_inventory as audit


class InventoryTests(unittest.TestCase):
    """Check audit integrity failures and a real command-line round trip."""

    def setUp(self) -> None:
        """Create isolated, contributor-original inputs."""
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self.wheel = self.root / "synthetic.whl"

    def make_wheel(self, extra: str | None = None, version: str = "26.8") -> str:
        """Write a tiny synthetic wheel; return its actual digest."""
        with zipfile.ZipFile(self.wheel, "w") as archive:
            archive.writestr(
                "usd_core-26.8.dist-info/METADATA",
                f"Name: usd-core\nVersion: {version}\nRequires-Python: >=3.9, <3.15\n",
            )
            archive.writestr("usd_core-26.8.dist-info/WHEEL", "Tag: cp312-none-win_amd64\n")
            if extra is not None:
                # Windows ZipInfo normalizes backslashes. Preserve the raw test path.
                info = zipfile.ZipInfo("synthetic-input")
                info.filename = extra
                archive.writestr(info, "synthetic input")
        return hashlib.sha256(self.wheel.read_bytes()).hexdigest()

    def test_cli_reports_inventory_without_extracting_or_importing(self) -> None:
        """An executable-looking input stays untouched package data."""
        digest = self.make_wheel("pxr/__init__.py")
        output = self.root / "report.json"
        result = subprocess.run(
            [sys.executable, str(Path(audit.__file__)), str(self.wheel),
             "--sha256", digest, "--output", str(output)],
            capture_output=True, text=True, timeout=10, check=False,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn('"production_approved": false', output.read_text("utf-8"))
        self.assertFalse((self.root / "pxr").exists())

    def test_wrong_digest_is_rejected(self) -> None:
        """A tampered artifact cannot produce a successful inventory."""
        self.make_wheel()
        with self.assertRaisesRegex(ValueError, "SHA-256 mismatch"):
            audit.inspect_wheel(self.wheel, "0" * 64)

    def test_actual_directory_count_rejected_before_zipinfo_allocation(self) -> None:
        """A forged advertised count cannot hide excessive directory records."""
        self.make_wheel("pxr/extra.txt")
        data = bytearray(self.wheel.read_bytes())
        end = data.rfind(b"PK\x05\x06")
        self.assertGreaterEqual(end, 0)
        struct.pack_into("<HH", data, end + 8, 1, 1)
        self.wheel.write_bytes(data)
        digest = hashlib.sha256(data).hexdigest()
        with (
            patch.object(audit, "MAX_MEMBERS", 2),
            patch.object(audit.zipfile, "ZipFile") as parser,
        ):
            with self.assertRaisesRegex(ValueError, "actual member count limit"):
                audit.inspect_wheel(self.wheel, digest)
            parser.assert_not_called()

    def test_directory_byte_limit_precedes_zipinfo_allocation(self) -> None:
        """Directory bytes are rejected before constructing the ZIP parser."""
        digest = self.make_wheel()
        with (
            patch.object(audit, "MAX_CENTRAL_DIRECTORY_BYTES", 1),
            patch.object(audit.zipfile, "ZipFile") as parser,
        ):
            with self.assertRaisesRegex(ValueError, "central directory byte limit"):
                audit.inspect_wheel(self.wheel, digest)
            parser.assert_not_called()

    def test_understated_deflated_size_is_rejected_with_bounded_read(self) -> None:
        """A hash-valid deflated member with a forged size cannot be read wholesale."""
        payload = b"x" * 4096
        with zipfile.ZipFile(self.wheel, "w", compression=zipfile.ZIP_DEFLATED) as archive:
            archive.writestr("text.txt", payload)
        data = bytearray(self.wheel.read_bytes())
        central = data.find(b"PK\x01\x02")
        self.assertGreaterEqual(central, 0)
        struct.pack_into("<I", data, 22, 1)
        struct.pack_into("<I", data, central + 24, 1)
        self.wheel.write_bytes(data)
        with zipfile.ZipFile(self.wheel) as archive:
            stream = archive.open("text.txt")
            with (
                patch.object(audit, "MAX_TEXT_BYTES", 32),
                patch.object(archive, "open", return_value=stream),
                patch.object(stream, "read", wraps=stream.read) as reader,
            ):
                with self.assertRaises(zipfile.BadZipFile):
                    audit.read_text_bytes(archive, "text.txt")
                reader.assert_called_once_with(33)

    def test_bzip2_and_lzma_text_rejected_before_decompressor(self) -> None:
        """Unsupported text methods never open a decompression stream."""
        for compression in (zipfile.ZIP_BZIP2, zipfile.ZIP_LZMA):
            with self.subTest(compression=compression):
                with zipfile.ZipFile(self.wheel, "w", compression=compression) as archive:
                    archive.writestr("text.txt", "original synthetic text")
                with (
                    zipfile.ZipFile(self.wheel) as archive,
                    patch.object(archive, "open") as opener,
                ):
                    with self.assertRaisesRegex(ValueError, "unsupported text compression"):
                        audit.read_text_bytes(archive, "text.txt")
                    opener.assert_not_called()

    def test_cli_rejects_input_output_aliases_without_modifying_wheel(self) -> None:
        """Direct, hard-linked and supported symlink aliases preserve artifact bytes."""
        digest = self.make_wheel()
        original = self.wheel.read_bytes()
        hard_link = self.root / "hard-link.whl"
        os.link(self.wheel, hard_link)
        aliases = [self.wheel, hard_link]
        symbolic_link = self.root / "symbolic-link.whl"
        try:
            symbolic_link.symlink_to(self.wheel)
        except OSError:
            # Windows may require a privilege; direct and hard-link cases still run.
            pass
        else:
            aliases.append(symbolic_link)
        for output in aliases:
            with self.subTest(output=output.name):
                result = subprocess.run(
                    [sys.executable, str(Path(audit.__file__)), str(self.wheel),
                     "--sha256", digest, "--output", str(output)],
                    capture_output=True, text=True, timeout=10, check=False,
                )
                self.assertEqual(result.returncode, 1)
                self.assertIn("output must not refer to the input wheel", result.stderr)
                self.assertEqual(self.wheel.read_bytes(), original)

    def test_unsafe_paths_are_rejected(self) -> None:
        """Traversal, absolute and Windows drive paths cannot enter inventory."""
        for name in ("../escape", "/escape", "C:/escape", "pxr\\escape"):
            with self.subTest(name=name):
                digest = self.make_wheel(name)
                with self.assertRaisesRegex(ValueError, "unsafe"):
                    audit.inspect_wheel(self.wheel, digest)

    def test_member_size_limit_is_enforced(self) -> None:
        """Large entries are rejected before reading metadata content."""
        digest = self.make_wheel("pxr/large.txt")
        with patch.object(audit, "MAX_MEMBER_BYTES", 8):
            with self.assertRaisesRegex(ValueError, "uncompressed size limit"):
                audit.inspect_wheel(self.wheel, digest)

    def test_unexpected_package_version_is_rejected(self) -> None:
        """Version metadata cannot silently change the chosen candidate."""
        digest = self.make_wheel(version="0.0")
        with self.assertRaisesRegex(ValueError, "expected usd-core"):
            audit.inspect_wheel(self.wheel, digest)

    def test_corrupt_zip_has_a_cli_diagnostic(self) -> None:
        """A hash-valid non-ZIP file fails without a Python traceback."""
        self.wheel.write_bytes(b"synthetic corrupt zip")
        digest = hashlib.sha256(self.wheel.read_bytes()).hexdigest()
        result = subprocess.run(
            [sys.executable, str(Path(audit.__file__)), str(self.wheel),
             "--sha256", digest, "--output", str(self.root / "report.json")],
            capture_output=True, text=True, timeout=10, check=False,
        )
        self.assertEqual(result.returncode, 1)
        self.assertIn("audit error:", result.stderr)
        self.assertNotIn("Traceback", result.stderr)

    def test_unsupported_compression_has_a_cli_diagnostic(self) -> None:
        """Unsupported member compression returns a bounded error, not a traceback."""
        self.make_wheel()
        data = bytearray(self.wheel.read_bytes())
        # Original synthetic ZIP: change the first member's compression method.
        struct.pack_into("<H", data, 8, 99)
        central_header = data.find(b"PK\x01\x02")
        self.assertGreaterEqual(central_header, 0)
        struct.pack_into("<H", data, central_header + 10, 99)
        self.wheel.write_bytes(data)
        digest = hashlib.sha256(data).hexdigest()
        result = subprocess.run(
            [sys.executable, str(Path(audit.__file__)), str(self.wheel),
             "--sha256", digest, "--output", str(self.root / "report.json")],
            capture_output=True, text=True, timeout=10, check=False,
        )
        self.assertEqual(result.returncode, 1)
        self.assertIn("audit error:", result.stderr)
        self.assertNotIn("Traceback", result.stderr)


if __name__ == "__main__":
    unittest.main()
