# SPDX-License-Identifier: MPL-2.0
"""Offline manifest/feature/font audit. No full metadata dump is retained."""
import collections
import json
from pathlib import Path
import re
import subprocess
import tomllib

HERE = Path(__file__).resolve().parent
metadata = json.loads(subprocess.check_output(
    ["cargo", "metadata", "--offline", "--locked", "--format-version", "1"], cwd=HERE
))
policy = tomllib.loads((HERE / "../../deny.toml").read_text(encoding="utf-8"))
allowed = set(policy["licenses"]["allow"])
groups = collections.defaultdict(list)
for package in metadata["packages"]:
    expression = package["license"] or "UNKNOWN"
    identifiers = set(re.findall(r"[A-Za-z][A-Za-z0-9.+-]*", expression))
    identifiers -= {"AND", "OR", "WITH", "LLVM-exception"}
    if identifiers - allowed:
        groups[expression].append(package["name"] + " " + package["version"])
print("Resolved packages (all platform dependencies):", len(metadata["packages"]))
print("Expressions containing identifiers outside global deny allowlist:")
for expression, packages in sorted(groups.items()):
    print(expression + ": " + ", ".join(packages))
by_id = {p["id"]: p for p in metadata["packages"]}
for node in metadata["resolve"]["nodes"]:
    name = by_id[node["id"]]["name"]
    if name in {"egui", "eframe", "epaint", "wgpu", "craft-ui", "craft-quit"}:
        print("Features", name, ", ".join(node["features"]))
forbidden = {"epaint_default_fonts", "craft-print", "craft-pdf", "craft-cms", "openusd"}
found = sorted(p["name"] for p in metadata["packages"] if p["name"] in forbidden)
fonts = [str(f) for p in metadata["packages"]
         for f in Path(p["manifest_path"]).parent.rglob("*")
         if f.is_file() and f.suffix.lower() in {".ttf", ".otf"}]
print("Forbidden packages:", found)
print("Font files in resolved source directories:", fonts)
if found or fonts:
    raise SystemExit("Unexpected font or forbidden dependency: audit failed")
# This inventory is not a full SPDX solver, licence-text audit, or cargo-deny replacement.
