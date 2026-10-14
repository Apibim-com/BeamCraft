#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
# SPDX-FileCopyrightText: 2026 Apibim
"""Independent oracle for the F07 fixture set.

This script shares no code with the Rust generator. It uses only the Python standard library:

* `hashlib` recomputes every SHA-256 digest and size recorded in the manifest from the bytes
  on disk, and the directory must hold exactly the manifest's files;
* the hand-written expectations must cover the same fixture IDs, support statuses and
  outcomes as the manifest;
* for every accepted fixture, volume, axis length, mass and axis-aligned bounds are
  recomputed from the fixture inputs by straightforward vector algebra and compared with the
  hand-derived values within the stated tolerances.

Run from any directory:  python tools/fixtures/verify_expectations.py
Options --fixtures DIR and --expectations FILE select other inputs (used for mutation checks).
Exit status 0 means every check passed; 1 means at least one failed.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
FIXTURES = ROOT / "tests" / "fixtures" / "f07"
EXPECTATIONS = ROOT / "tests" / "fixtures" / "expectations" / "f07.json"
MAX_BYTES = 1024 * 1024

failures: list[str] = []
checked = {"files": 0, "digests": 0, "quantities": 0, "cases": 0}


def fail(message: str) -> None:
    failures.append(message)


def read(path: Path) -> bytes:
    if path.stat().st_size > MAX_BYTES:
        raise ValueError(f"{path} is larger than {MAX_BYTES} bytes")
    return path.read_bytes()


def strict_json(data: bytes):
    def no_duplicates(pairs):
        names = [name for name, _ in pairs]
        if len(names) != len(set(names)):
            raise ValueError("duplicate member name")
        return dict(pairs)

    def no_constants(token):
        raise ValueError(f"non-finite token {token}")

    return json.loads(
        data.decode("utf-8"), object_pairs_hook=no_duplicates, parse_constant=no_constants
    )


def safe_relative(path: str) -> bool:
    parts = path.split("/")
    allowed = set("abcdefghijklmnopqrstuvwxyz0123456789._-")
    return all(p not in ("", ".", "..") and set(p) <= allowed for p in parts)


# ---- Vector algebra --------------------------------------------------------------------------


def sub(a, b):
    return [a[0] - b[0], a[1] - b[1], a[2] - b[2]]


def add(a, b):
    return [a[0] + b[0], a[1] + b[1], a[2] + b[2]]


def scale(a, k):
    return [a[0] * k, a[1] * k, a[2] * k]


def dot(a, b):
    return a[0] * b[0] + a[1] * b[1] + a[2] * b[2]


def cross(a, b):
    return [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]


def norm(a):
    return math.sqrt(dot(a, a))


def unit(a):
    return scale(a, 1.0 / norm(a))


def rotate(p, axis, degrees):
    """Rodrigues' rotation of p about the unit vector of `axis`, right-hand rule."""
    k = unit(axis)
    theta = math.radians(degrees)
    c, s = math.cos(theta), math.sin(theta)
    return add(add(scale(p, c), scale(cross(k, p), s)), scale(k, dot(k, p) * (1.0 - c)))


def bounds(points):
    return (
        [min(p[i] for p in points) for i in range(3)],
        [max(p[i] for p in points) for i in range(3)],
    )


# ---- Body quantities -------------------------------------------------------------------------


def plate(obj):
    sx, sy, sz = obj["size"]
    placement = obj["placement"]
    corners = [[x, y, z] for x in (0.0, sx) for y in (0.0, sy) for z in (0.0, sz)]
    world = [
        add(rotate(p, placement["axis"], placement["angle"]), placement["origin"])
        for p in corners
    ]
    return {"volume_mm3": sx * sy * sz, "bounds": bounds(world), "axis_length_mm": None}


def member(obj):
    start, end = obj["start"], obj["end"]
    width, height = obj["profile"]["width"], obj["profile"]["height"]
    axis = sub(end, start)
    length = norm(axis)
    x = unit(axis)
    up = obj["up"]
    z = unit(sub(up, scale(x, dot(up, x))))  # height direction: up made orthogonal to the axis
    y = cross(z, x)  # width direction, so that (x, y, z) is right-handed
    corners = [
        add(add(base, scale(y, sy * width / 2.0)), scale(z, sz * height / 2.0))
        for base in (start, end)
        for sy in (-1.0, 1.0)
        for sz in (-1.0, 1.0)
    ]
    return {"volume_mm3": width * height * length, "bounds": bounds(corners), "axis_length_mm": length}


def mass(obj, volume):
    density = obj.get("density")
    return None if density is None else density * volume * 1e-9


# ---- Comparisons -----------------------------------------------------------------------------


def close_abs(where, actual, expected, tol):
    checked["quantities"] += 1
    if not math.isfinite(actual) or abs(actual - expected) > tol:
        fail(f"{where}: computed {actual!r}, expected {expected!r} (abs tol {tol})")


def close_rel(where, actual, expected, tol):
    checked["quantities"] += 1
    if expected is None or actual is None:
        if expected is not actual:
            fail(f"{where}: computed {actual!r}, expected {expected!r} (unknown must stay null)")
        return
    if not math.isfinite(actual) or abs(actual - expected) > tol * max(abs(expected), 1e-300):
        fail(f"{where}: computed {actual!r}, expected {expected!r} (rel tol {tol})")


def close_bounds(where, actual, expected, tol):
    lo, hi = actual
    for i, axis in enumerate("xyz"):
        close_abs(f"{where}.min.{axis}", lo[i], expected["min"][i], tol)
        close_abs(f"{where}.max.{axis}", hi[i], expected["max"][i], tol)


def check_accepted(case, fixture, tol):
    cid = case["id"]
    if fixture.get("id") != cid:
        fail(f"{cid}: fixture id is {fixture.get('id')!r}")
    objects = {o["key"]: o for o in fixture["objects"]}
    expected_keys = [o["key"] for o in case["objects"]]
    if sorted(expected_keys) != sorted(objects):
        fail(f"{cid}: expectation keys {expected_keys} differ from fixture keys {list(objects)}")
        return
    results = {}
    for exp in case["objects"]:
        obj = objects[exp["key"]]
        got = plate(obj) if obj["kind"] == "plate" else member(obj)
        got["mass_kg"] = mass(obj, got["volume_mm3"])
        results[exp["key"]] = got
        where = f"{cid}/{exp['key']}"
        close_rel(f"{where}.volume_mm3", got["volume_mm3"], exp["volume_mm3"], tol["volume_rel"])
        close_rel(f"{where}.mass_kg", got["mass_kg"], exp["mass_kg"], tol["mass_rel"])
        close_bounds(f"{where}.bounds", got["bounds"], exp["bounds_mm"], tol["length_and_bounds_abs_mm"])
        if "axis_length_mm" in exp:
            close_abs(f"{where}.axis_length_mm", got["axis_length_mm"], exp["axis_length_mm"],
                      tol["length_and_bounds_abs_mm"])
        elif got["axis_length_mm"] is not None:
            fail(f"{where}: member without an axis_length_mm expectation")

    total = case["total"]
    volumes = [r["volume_mm3"] for r in results.values()]
    masses = [r["mass_kg"] for r in results.values()]
    total_mass = None if any(m is None for m in masses) else math.fsum(masses)
    close_rel(f"{cid}/total.volume_mm3", math.fsum(volumes), total["volume_mm3"], tol["volume_rel"])
    close_rel(f"{cid}/total.mass_kg", total_mass, total["mass_kg"], tol["mass_rel"])
    union = bounds([p for r in results.values() for p in r["bounds"]])
    close_bounds(f"{cid}/total.bounds", union, total["bounds_mm"], tol["length_and_bounds_abs_mm"])

    if "separation" in case:
        sep = case["separation"]
        a, b = (results[k]["bounds"] for k in sep["between"])
        i = "xyz".index(sep["axis"])
        gap = b[0][i] - a[1][i]
        close_abs(f"{cid}/separation.gap_mm", gap, sep["gap_mm"], tol["length_and_bounds_abs_mm"])


def main() -> int:
    global FIXTURES
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--fixtures", type=Path, default=FIXTURES, help="fixture directory")
    parser.add_argument("--expectations", type=Path, default=EXPECTATIONS, help="expectations file")
    args = parser.parse_args()
    FIXTURES = args.fixtures.resolve()
    manifest = strict_json(read(FIXTURES / "manifest.json"))
    expectations = strict_json(read(args.expectations))

    for key, wanted in [("format", "beamcraft.fixture-manifest/1"), ("fixture_set", "f07")]:
        if manifest.get(key) != wanted:
            fail(f"manifest {key} is {manifest.get(key)!r}")
    if manifest.get("fixture_set_version") != expectations.get("fixture_set_version"):
        fail("manifest and expectations disagree on fixture_set_version")
    if manifest["provenance"]["licence"] != "MPL-2.0":
        fail("manifest licence is not MPL-2.0")

    # Hashes, sizes and directory contents.
    entries = manifest["fixtures"]
    listed = {"manifest.json"}
    ids = set()
    for entry in entries:
        path, cid = entry["path"], entry["id"]
        if cid in ids:
            fail(f"duplicate fixture id {cid}")
        ids.add(cid)
        for key in ("author", "origin", "licence", "expectation"):
            if not entry.get(key):
                fail(f"{cid}: manifest entry lacks {key}")
        if entry.get("licence") != "MPL-2.0":
            fail(f"{cid}: licence is {entry.get('licence')!r}")
        if not safe_relative(path):
            fail(f"{cid}: unsafe path {path!r}")
            continue
        listed.add(path)
        file = FIXTURES / path
        if not file.is_file():
            fail(f"{cid}: missing file {path}")
            continue
        data = read(file)
        checked["files"] += 1
        digest = hashlib.sha256(data).hexdigest()
        checked["digests"] += 1
        if len(entry["sha256"]) != 64 or digest != entry["sha256"]:
            fail(f"{path}: sha256 {digest} != manifest {entry['sha256']}")
        if len(data) != entry["bytes"]:
            fail(f"{path}: {len(data)} bytes != manifest {entry['bytes']}")
    on_disk = {p.relative_to(FIXTURES).as_posix() for p in FIXTURES.rglob("*") if p.is_file()}
    for extra in sorted(on_disk - listed):
        fail(f"unexpected file {extra}")

    # Expectations coverage and outcomes.
    by_id = {e["id"]: e for e in entries}
    cases = expectations["cases"]
    case_ids = [c["id"] for c in cases]
    if len(case_ids) != len(set(case_ids)):
        fail("duplicate IDs in expectations")
    if set(case_ids) != ids:
        fail(f"expectation IDs differ from manifest IDs: {sorted(set(case_ids) ^ ids)}")
    tol = expectations["tolerances"]
    for case in cases:
        entry = by_id.get(case["id"])
        if entry is None:
            continue
        checked["cases"] += 1
        cid = case["id"]
        if case["support"] != entry["support"]:
            fail(f"{cid}: support {case['support']} != manifest {entry['support']}")
        if case["outcome"] != entry["expected_outcome"]:
            fail(f"{cid}: outcome {case['outcome']} != manifest {entry['expected_outcome']}")
        accepts = case["support"] in ("supported", "unverified")
        if accepts != (case["outcome"] == "accept"):
            fail(f"{cid}: support {case['support']} conflicts with outcome {case['outcome']}")
        if case["outcome"] == "reject":
            if case.get("error") != entry.get("expected_error"):
                fail(f"{cid}: error {case.get('error')} != manifest {entry.get('expected_error')}")
            continue
        try:
            fixture = strict_json(read(FIXTURES / entry["path"]))
        except (ValueError, UnicodeDecodeError) as e:
            fail(f"{cid}: accepted fixture does not parse: {e}")
            continue
        check_accepted(case, fixture, tol)

    for message in failures:
        print(f"FAIL {message}")
    status = "FAILED" if failures else "OK"
    print(
        f"{status}: {checked['files']} files, {checked['digests']} SHA-256 digests, "
        f"{checked['cases']} expectation cases, {checked['quantities']} analytical quantities"
        + (f", {len(failures)} failures" if failures else "")
    )
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
