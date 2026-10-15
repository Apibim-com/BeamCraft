# SPDX-License-Identifier: MPL-2.0
# SPDX-FileCopyrightText: 2026 Apibim

"""Inspect a hash-pinned wheel offline, without extracting or importing it."""

from __future__ import annotations

import argparse
import email
import hashlib
import json
import lzma
import os
import re
import stat
import struct
import sys
import tempfile
import zipfile
import zlib
from pathlib import Path, PurePosixPath
from typing import BinaryIO

MAX_ARCHIVE_BYTES = 128 * 1024 * 1024
MAX_MEMBER_BYTES = 256 * 1024 * 1024
MAX_TOTAL_BYTES = 512 * 1024 * 1024
MAX_TEXT_BYTES = 2 * 1024 * 1024
MAX_MEMBERS = 4096
MAX_CENTRAL_DIRECTORY_BYTES = 4 * 1024 * 1024
END_RECORD = struct.Struct("<4s4H2IH")
CENTRAL_HEADER_BYTES = 46


def preflight_directory(source: BinaryIO, archive_size: int) -> None:
    """Bound classic ZIP directory bytes and actual records before ZipInfo allocation.

    Layout follows PKWARE APPNOTE sections 4.3.12 and 4.3.16. ZIP64 containers,
    multiple disks, trailing data and prefixed archives are outside this audit's scope.
    """
    tail_size = min(archive_size, END_RECORD.size + 65535)
    source.seek(archive_size - tail_size)
    tail = source.read(tail_size)
    end_index = tail.rfind(b"PK\x05\x06")
    if end_index < 0 or end_index + END_RECORD.size > len(tail):
        raise ValueError("missing or truncated ZIP end record")
    _, disk, directory_disk, disk_entries, entries, size, offset, comment = (
        END_RECORD.unpack_from(tail, end_index)
    )
    end_offset = archive_size - tail_size + end_index
    if disk or directory_disk or disk_entries != entries:
        raise ValueError("multiple-disk ZIP archives are unsupported")
    if entries == 65535 or size == 0xFFFFFFFF or offset == 0xFFFFFFFF:
        raise ValueError("ZIP64 containers are unsupported")
    if end_offset >= 20:
        source.seek(end_offset - 20)
        if source.read(4) == b"PK\x06\x07":
            raise ValueError("ZIP64 containers are unsupported")
    if end_index + END_RECORD.size + comment != len(tail) or offset + size != end_offset:
        raise ValueError("invalid ZIP central directory bounds")
    if size > MAX_CENTRAL_DIRECTORY_BYTES:
        raise ValueError("archive exceeds central directory byte limit")
    if entries > MAX_MEMBERS:
        raise ValueError("archive exceeds member count limit")
    source.seek(offset)
    consumed = 0
    actual_entries = 0
    while consumed < size:
        if size - consumed < CENTRAL_HEADER_BYTES:
            raise ValueError("truncated central directory header")
        header = source.read(CENTRAL_HEADER_BYTES)
        if len(header) != CENTRAL_HEADER_BYTES or not header.startswith(b"PK\x01\x02"):
            raise ValueError("invalid central directory header")
        name_bytes, extra_bytes, comment_bytes = struct.unpack_from("<3H", header, 28)
        record_bytes = CENTRAL_HEADER_BYTES + name_bytes + extra_bytes + comment_bytes
        if record_bytes > size - consumed:
            raise ValueError("central directory record exceeds bounds")
        actual_entries += 1
        if actual_entries > MAX_MEMBERS:
            raise ValueError("archive exceeds actual member count limit")
        source.seek(record_bytes - CENTRAL_HEADER_BYTES, os.SEEK_CUR)
        consumed += record_bytes
    if actual_entries != entries:
        raise ValueError("central directory member count mismatch")
    source.seek(0)


def read_text_bytes(archive: zipfile.ZipFile, name: str) -> bytes:
    """Read capped stored/deflated text; other decompressors are unsupported."""
    member = archive.getinfo(name)
    if member.compress_type not in (zipfile.ZIP_STORED, zipfile.ZIP_DEFLATED):
        raise ValueError("unsupported text compression; require stored or deflated")
    if member.file_size > MAX_TEXT_BYTES:
        raise ValueError("metadata or licence exceeds text size limit")
    with archive.open(member) as stream:
        data = stream.read(MAX_TEXT_BYTES + 1)
    if len(data) > MAX_TEXT_BYTES:
        raise ValueError("decompressed text exceeds text size limit")
    if len(data) != member.file_size:
        raise ValueError("decompressed text size mismatch")
    return data


def validate_output_path(wheel: Path, output: Path) -> None:
    """Reject input aliases, including existing symlinks and hard links."""
    if output.resolve() == wheel.resolve() or (output.exists() and output.samefile(wheel)):
        raise ValueError("output must not refer to the input wheel")


def write_inventory(wheel: Path, output: Path, result: dict[str, object]) -> None:
    """Publish through a temporary file without following an output symlink on write."""
    validate_output_path(wheel, output)
    temporary: Path | None = None
    try:
        with tempfile.NamedTemporaryFile(
            mode="w", encoding="utf-8", dir=output.parent, delete=False,
        ) as stream:
            temporary = Path(stream.name)
            json.dump(result, stream, indent=2)
            stream.write("\n")
        validate_output_path(wheel, output)
        os.replace(temporary, output)
    finally:
        if temporary is not None:
            temporary.unlink(missing_ok=True)


def inspect_wheel(path: Path, expected_sha256: str) -> dict[str, object]:
    """Return static inventory after verifying the artifact and archive limits.

    This does not certify licences, native linkage, signatures or runtime behavior.
    Raises OSError, ValueError or BadZipFile for an unreadable or invalid input.
    """
    if not re.fullmatch(r"[0-9a-f]{64}", expected_sha256):
        raise ValueError("expected SHA-256 must contain 64 lowercase hex digits")
    digest = hashlib.sha256()
    compressed_size = 0
    # Parse a private snapshot so concurrent input changes cannot bypass preflight.
    with path.open("rb") as input_stream, tempfile.SpooledTemporaryFile(
        max_size=1024 * 1024, mode="w+b",
    ) as source:
        while block := input_stream.read(1024 * 1024):
            compressed_size += len(block)
            if compressed_size > MAX_ARCHIVE_BYTES:
                raise ValueError("archive exceeds compressed size limit")
            digest.update(block)
            source.write(block)
        if digest.hexdigest() != expected_sha256:
            raise ValueError("archive SHA-256 mismatch")
        preflight_directory(source, compressed_size)
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
                return read_text_bytes(archive, name).decode("utf-8")

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
                data = read_text_bytes(archive, name)
                text = data.decode("utf-8")
                licences.append({
                    "path": name,
                    "sha256": hashlib.sha256(data).hexdigest(),
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
        validate_output_path(args.wheel, args.output)
        result = inspect_wheel(args.wheel, args.sha256)
        write_inventory(args.wheel, args.output, result)
    except (OSError, ValueError, KeyError, RuntimeError, EOFError,
            zipfile.BadZipFile, zipfile.LargeZipFile, zlib.error, lzma.LZMAError) as error:
        print(f"audit error: {error}", file=sys.stderr)
        return 1
    print(f"Verified {result['artifact']}: {result['entry_count']} entries; static audit only")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
