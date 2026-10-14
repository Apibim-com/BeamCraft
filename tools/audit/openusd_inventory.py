# SPDX-License-Identifier: MPL-2.0
# SPDX-FileCopyrightText: 2026 Apibim

"""Inspect a hash-pinned wheel offline, without extracting or importing it."""

from __future__ import annotations

import argparse
import email
import hashlib
import json
import lzma
import re
import stat
import sys
import zipfile
import zlib
from pathlib import Path, PurePosixPath

MAX_ARCHIVE_BYTES = 128 * 1024 * 1024
MAX_MEMBER_BYTES = 256 * 1024 * 1024
MAX_TOTAL_BYTES = 512 * 1024 * 1024
MAX_TEXT_BYTES = 2 * 1024 * 1024
MAX_MEMBERS = 4096


def inspect_wheel(path: Path, expected_sha256: str) -> dict[str, object]:
    """Return static inventory after verifying the artifact and archive limits.

    This does not certify licences, native linkage, signatures or runtime behavior.
    Raises OSError, ValueError or BadZipFile for an unreadable or invalid input.
    """
    if not re.fullmatch(r"[0-9a-f]{64}", expected_sha256):
        raise ValueError("expected SHA-256 must contain 64 lowercase hex digits")
    digest = hashlib.sha256()
    compressed_size = 0
    with path.open("rb") as source:
        while block := source.read(1024 * 1024):
            compressed_size += len(block)
            if compressed_size > MAX_ARCHIVE_BYTES:
                raise ValueError("archive exceeds compressed size limit")
            digest.update(block)
        if digest.hexdigest() != expected_sha256:
            raise ValueError("archive SHA-256 mismatch")
        source.seek(0)
        with zipfile.ZipFile(source) as archive:
            members = archive.infolist()
            if len(members) > MAX_MEMBERS:
                raise ValueError("archive exceeds member count limit")
            names: set[str] = set()
            total_size = 0
            for member in members:
                name = member.filename
                parts = PurePosixPath(name).parts
                if (
                    not parts
                    or member.orig_filename != name
                    or len(name) > 1024
                    or "\\" in name
                    or ":" in name
                    or PurePosixPath(name).is_absolute()
                    or ".." in parts
                    or name in names
                    or stat.S_ISLNK(member.external_attr >> 16)
                    or member.flag_bits & 1
                ):
                    raise ValueError("unsafe or duplicate archive member")
                names.add(name)
                total_size += member.file_size
                if member.file_size > MAX_MEMBER_BYTES or total_size > MAX_TOTAL_BYTES:
                    raise ValueError("archive exceeds uncompressed size limit")

            def read_text(name: str) -> str:
                """Read bounded UTF-8 evidence without executing package content."""
                if archive.getinfo(name).file_size > MAX_TEXT_BYTES:
                    raise ValueError("metadata or licence exceeds text size limit")
                return archive.read(name).decode("utf-8")

            metadata_names = sorted(n for n in names if n.endswith(".dist-info/METADATA"))
            if len(metadata_names) != 1:
                raise ValueError("expected exactly one distribution METADATA")
            metadata_name = metadata_names[0]
            metadata = email.message_from_string(read_text(metadata_name))
            if metadata.get("Name") != "usd-core" or metadata.get("Version") != "26.8":
                raise ValueError("expected usd-core version 26.8")
            wheel_name = metadata_name.removesuffix("METADATA") + "WHEEL"
            wheel_metadata = email.message_from_string(read_text(wheel_name))
            files = sorted(m.filename for m in members if not m.is_dir())
            licence_files = [
                n for n in files if any(term in n.lower() for term in ("license", "notice", "copying"))
            ]
            licences = []
            for name in licence_files:
                text = read_text(name)
                licences.append({
                    "path": name,
                    "sha256": hashlib.sha256(archive.read(name)).hexdigest(),
                    "sections": re.findall(r"={5,}\s*\n([^\n]+)\n={5,}", text),
                })

            return {
                "schema_version": 1,
                "artifact": path.name,
                "sha256": digest.hexdigest(),
                "compressed_bytes": compressed_size,
                "uncompressed_bytes": total_size,
                "entry_count": len(members),
                "metadata": {
                    field: metadata.get_all(field, [])
                    for field in ("Name", "Version", "License", "License-Expression",
                                  "Requires-Python", "Requires-Dist", "License-File")
                },
                "wheel_tags": wheel_metadata.get_all("Tag", []),
                "licence_evidence": licences,
                "native_members": [
                    n for n in files if n.endswith((".dll", ".pyd", ".dylib", ".so", ".so.2"))
                ],
                "schema_members": [
                    n for n in files if n.endswith(("plugInfo.json", "generatedSchema.usda"))
                ],
                "visual_asset_members": [
                    n for n in files if n.lower().endswith((".ttf", ".otf", ".png", ".jpg", ".svg"))
                ],
                "tool_members": [
                    n for n in files if any(term in n for term in
                                           ("usdchecker", "usdGenSchema", "usdview", "entry_points"))
                ],
                "members": [
                    {"path": m.filename, "bytes": m.file_size, "compressed_bytes": m.compress_size}
                    for m in sorted(members, key=lambda m: m.filename)
                ],
                "production_approved": False,
            }


def main() -> int:
    """Inspect one pinned artifact and emit a machine-readable report."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("wheel", type=Path)
    parser.add_argument("--sha256", required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    try:
        result = inspect_wheel(args.wheel, args.sha256)
        args.output.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    except (OSError, ValueError, KeyError, RuntimeError, EOFError,
            zipfile.BadZipFile, zipfile.LargeZipFile, zlib.error, lzma.LZMAError) as error:
        print(f"audit error: {error}", file=sys.stderr)
        return 1
    print(f"Verified {result['artifact']}: {result['entry_count']} entries; static audit only")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
