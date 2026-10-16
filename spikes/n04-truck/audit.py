"""Offline Cargo inventory; read manifests, archives and licence files, never source/tests."""

import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import tomllib

ROOT = Path(__file__).resolve().parent
OUT = ROOT / "evidence"
OUT.mkdir(exist_ok=True)


def write(name, value):
    (OUT / name).write_text(value, encoding="utf-8", newline="\n")


def cargo(*args):
    result = subprocess.run(
        ["cargo", *args, "--offline", "--manifest-path", str(ROOT / "Cargo.toml")],
        capture_output=True, encoding="utf-8", check=True,
    )
    return result.stdout


metadata = json.loads(cargo("metadata", "--format-version", "1"))
lock = tomllib.loads((ROOT / "Cargo.lock").read_text(encoding="utf-8"))
locked = {(p["name"], p["version"]): p for p in lock["package"]}
policy = tomllib.loads((ROOT.parent.parent / "deny.toml").read_text(encoding="utf-8"))
allowed = set(policy["licenses"]["allow"])


def permits(expression):
    # Preserve the original declaration; legacy '/' is considered dual licensing
    # only for this declared-metadata screen, pending cargo-deny verification.
    tokens = re.findall(r"\(|\)|\bOR\b|\bAND\b|[A-Za-z0-9.+-]+(?:\s+WITH\s+[A-Za-z0-9.+-]+)?", expression.replace("/", " OR "))
    position = 0

    def atom():
        nonlocal position
        item = tokens[position]
        position += 1
        if item == "(":
            result = alternative()
            assert tokens[position] == ")"
            position += 1
            return result
        return item in allowed

    def conjunction():
        nonlocal position
        result = atom()
        while position < len(tokens) and tokens[position] == "AND":
            position += 1
            value = atom()
            result = result and value
        return result

    def alternative():
        nonlocal position
        result = conjunction()
        while position < len(tokens) and tokens[position] == "OR":
            position += 1
            value = conjunction()
            result = result or value
        return result

    result = alternative()
    assert position == len(tokens)
    return result


rows = []
for package in sorted(metadata["packages"], key=lambda p: (p["name"], p["version"])):
    folder = Path(package["manifest_path"]).parent
    record = {key: package.get(key) for key in (
        "name", "version", "license", "license_file", "repository", "source", "rust_version",
    )}
    record["checksum"] = locked[(package["name"], package["version"])].get("checksum")
    licences = []
    # Only licence/notice texts and packaging metadata are inspected.
    for candidate in sorted(folder.glob("*")):
        if candidate.is_file() and candidate.name.upper().startswith(("LICENSE", "LICENCE", "COPYING", "NOTICE")):
            data = candidate.read_bytes()
            text = data.decode("utf-8", errors="replace")
            licences.append({"file": candidate.name, "sha256": hashlib.sha256(data).hexdigest(),
                "apache_2_text": "Apache License" in text and "Version 2.0" in text,
                "mit_text": "Permission is hereby granted, free of charge" in text})
    record["licence_files"] = licences
    if package["name"] == "lzma-sys":
        notice = folder / "xz-5.2/COPYING"
        record["bundled_xz_licensing_summary"] = {
            "file":"xz-5.2/COPYING", "sha256":hashlib.sha256(notice.read_bytes()).hexdigest(),
            "liblzma":"public domain per bundled publisher summary; individual C files not audited",
            "other_bundled_material":"GPL-2.0-or-later/GPL-3.0-or-later build/scripts; possible LGPL getopt; not admitted for redistribution",
        }
    expression = package.get("license") or "UNKNOWN"
    identifiers = set(re.findall(r"[A-Za-z0-9.+-]+(?:\s+WITH\s+[A-Za-z0-9.+-]+)?", expression)) - {"AND", "OR"}
    record["identifiers_outside_allowlist"] = sorted(identifiers - allowed)
    record["declared_policy_satisfiable"] = permits(expression)
    record["private_ignored"] = package.get("publish") == []
    if package["name"] in ("truck-modeling", "truck-meshalgo", "truck-shapeops"):
        record["available_features"] = package["features"]
        record["vcs_metadata"] = json.loads((folder / ".cargo_vcs_info.json").read_text(encoding="utf-8")) if (folder / ".cargo_vcs_info.json").exists() else None
    record["manifest_sha256"] = hashlib.sha256(Path(package["manifest_path"]).read_bytes()).hexdigest()
    record["archive_verified"] = None
    if record["checksum"]:
        # Cargo registry src/<registry>/<package> -> registry/cache/<registry>/<package>.crate
        archive = folder.parent.parent.parent / "cache" / folder.parent.name / (folder.name + ".crate")
        record["archive_verified"] = archive.exists() and hashlib.sha256(archive.read_bytes()).hexdigest() == record["checksum"]
    rows.append(record)
write("inventory.json", json.dumps(rows, indent=2) + "\n")
write("tree.txt", cargo("tree"))
write("features.txt", cargo("tree", "-e", "features"))
write("resolution.json", json.dumps(metadata["resolve"], indent=2) + "\n")
table = ["| Package | Version | Declared SPDX | Allowlist screen | Lock checksum | Licence files (SHA-256 in inventory.json) |",
         "|---|---|---|---|---|---|"]
for r in rows:
    status = "allowed choice" if r["declared_policy_satisfiable"] else "private ignored" if r["private_ignored"] else "not allowed"
    table.append(f"| {r['name']} | {r['version']} | `{r['license']}` | {status} | {r['checksum'] or 'local'} | "
                 + (", ".join(x["file"] for x in r["licence_files"]) or "none packaged") + " |")
write("inventory.md", "\n".join(table) + "\n")
print(json.dumps({"packages": len(rows), "archives_verified": sum(r["archive_verified"] is True for r in rows),
    "outside": [(r["name"], r["version"], r["identifiers_outside_allowlist"]) for r in rows if r["identifiers_outside_allowlist"]]}, indent=2))
