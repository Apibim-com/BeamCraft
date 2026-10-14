# SPDX-License-Identifier: MPL-2.0
# SPDX-FileCopyrightText: 2026 Apibim

"""Synthetic archive and CLI checks; no third-party fixtures or network."""

import hashlib
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
