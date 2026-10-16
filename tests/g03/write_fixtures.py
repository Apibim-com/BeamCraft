# SPDX-License-Identifier: MPL-2.0
# SPDX-FileCopyrightText: 2026 Apibim
"""Reproduce G03 original inputs and rational references; never invoke a kernel.

Run from any directory with Python 3.11+. --check compares committed bytes without
writing. The explicit cases declare outcomes, independently of admission code.
Fractions describe the exact submitted binary64 values, not decimal intentions.
"""

from __future__ import annotations

import argparse
from dataclasses import dataclass
from fractions import Fraction
import hashlib
import math
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2] / "docs/fixtures/g03"


@dataclass(frozen=True)
class Case:
    """An original parameter description and explicitly declared support/outcome."""

    name: str
    dimensions: tuple[float, float, float]
    support: str = "supported"
    diagnostics: str = "-"
    origin: tuple[float, float, float] = (0.0, 0.0, 0.0)
    plan: str = "G03"


def cases() -> list[Case]:
    """Choose synthetic inputs by arithmetic boundaries, without production code."""
    floor_up = math.nextafter(0.001, math.inf)
    floor_down = math.nextafter(0.001, 0.0)
    cap_up = math.nextafter(1_000_000.0, math.inf)
    scaled = 500_000 / 4_194_304  # 3-4-5 triangle; exact dyadic floor.
    valid = [
        ("unit", (1, 1, 1)),
        ("unequal", (23, 47, 3)),
        ("dyadic", (0.125, 0.25, 0.0625)),
        ("decimal", (0.1, 0.2, 0.3)),
        ("all-minimum", (0.001, 0.001, 0.001)),
        ("width-minimum", (0.001, 1, 0.01)),
        ("length-minimum", (1, 0.001, 0.01)),
        ("thickness-minimum", (1, 1, 0.001)),
        ("absolute-next-up", (1, 1, floor_up)),
        ("small-flat", (3, 4, 0.001)),
        ("absolute-dominates", (2048, 2048, 0.001)),
        ("scaled-minimum", (3072, 4096, 5 / 4096)),
        ("scaled-next-up", (3072, 4096, math.nextafter(5 / 4096, math.inf))),
        ("large-scaled-minimum", (300_000, 400_000, scaled)),
        ("large-scaled-next-up", (300_000, 400_000, math.nextafter(scaled, math.inf))),
        ("width-cap", (1_000_000, 1, 1)),
        ("length-cap", (1, 1_000_000, 1)),
        ("thickness-cap", (1, 1, 1_000_000)),
        ("all-caps", (1_000_000, 1_000_000, 1_000_000)),
        ("thickness-excluded-from-scale", (0.001, 0.001, 1_000_000)),
        ("anisotropic", (8192, 0.002, 1)),
        ("small-dyadic", (0.03125, 0.015625, 0.0078125)),
    ]
    result = [Case(name, tuple(map(float, dims))) for name, dims in valid]
    for axis, field in enumerate(("width_mm", "length_mm", "thickness_mm")):
        for name, value, stage, code in (
            ("zero", 0.0, "Ranges", "GEO_NOT_POSITIVE"),
            ("negative", -float(axis + 1), "Ranges", "GEO_NOT_POSITIVE"),
            ("below-absolute", floor_down, "Ranges", "GEO_BELOW_MINIMUM"),
            ("above-cap", cap_up, "Caps", "GEO_ABOVE_LIMIT"),
            ("nan", math.nan, "Finiteness", "GEO_NON_FINITE"),
        ):
            dims = [1.0, 1.0, 1.0]
            dims[axis] = value
            result.append(Case(f"{field[:-3]}-{name}", tuple(dims), "invalid",
                               f"{stage}:{field}:{code}"))
    invalid = [
        ("signed-zero", (1, 1, -0.0), "Ranges:thickness_mm:GEO_NOT_POSITIVE"),
        ("subnormal", (1, 1, float.fromhex("0x0.0000000000001p-1022")),
         "Ranges:thickness_mm:GEO_BELOW_MINIMUM"),
        ("huge-negative", (-float.fromhex("0x1.fffffffffffffp+1023"), 1, 1),
         "Ranges:width_mm:GEO_NOT_POSITIVE"),
        ("huge-positive", (1, float.fromhex("0x1.fffffffffffffp+1023"), 1),
         "Caps:length_mm:GEO_ABOVE_LIMIT"),
        ("positive-infinity", (math.inf, 1, 1), "Finiteness:width_mm:GEO_NON_FINITE"),
        ("negative-infinity", (1, -math.inf, 1), "Finiteness:length_mm:GEO_NON_FINITE"),
        ("scaled-next-down", (3072, 4096, math.nextafter(5 / 4096, 0)),
         "Ranges:thickness_mm:GEO_BELOW_MINIMUM"),
        ("large-scaled-next-down", (300_000, 400_000, math.nextafter(scaled, 0)),
         "Ranges:thickness_mm:GEO_BELOW_MINIMUM"),
        ("precedence-finiteness", (0, cap_up, math.nan),
         "Finiteness:thickness_mm:GEO_NON_FINITE"),
        ("precedence-caps", (0, cap_up, -1), "Caps:length_mm:GEO_ABOVE_LIMIT"),
        ("aggregate-ranges", (0, 0.0009, -1),
         "Ranges:width_mm:GEO_NOT_POSITIVE,Ranges:length_mm:GEO_BELOW_MINIMUM,"
         "Ranges:thickness_mm:GEO_NOT_POSITIVE"),
    ]
    result.extend(Case(name, tuple(map(float, dims)), "invalid", diagnostic)
                  for name, dims, diagnostic in invalid)
    for name, dims, origin in (
        ("large-positive", (3, 4, 0.001), (1e9, 1e9, 1e9)),
        ("large-negative", (3, 4, 0.001), (-1e9, -1e9, -1e9)),
        ("large-mixed", (3, 4, 0.001), (1e9, -1e9, 0)),
        ("large-minimum-cube", (0.001, 0.001, 0.001), (1e9, 1e9, 1e9)),
    ):
        result.append(Case(name, tuple(map(float, dims)), "unverified",
                           origin=origin, plan="G05/G09"))
    for name, plan in (("through-hole", "A04/G09"), ("cavity", "A02/G05/G09"),
                       ("thin-wall", "A02/G05/G09"), ("nonrectangular", "A02/G09")):
        result.append(Case(name, (23.0, 47.0, 3.0), "unsupported", plan=plan))
    return result


def rational(value: Fraction) -> str:
    """Encode an exact reference with no rounded decimal intermediate."""
    return f"{value.numerator}/{value.denominator}"


def artifacts() -> dict[str, bytes]:
    """Create fixture bytes, a hashed manifest and separate cited expectations."""
    files: dict[str, bytes] = {}
    manifest = ["# beamcraft.g03-manifest/1; set=1; contributor-original; 2026-10-16; MPL-2.0",
                "# id\tpath\tbytes\tsha256\tsupport\tplan\tauthor\tlicence"]
    expected = ["# beamcraft.g03-expectations/1; exact rational binary64-input references",
                "# id\toutcome\tdiagnostics\tvolume_mm3\tbounds_min\tbounds_max\t"
                "V,E,F,T,meshE,shells,euler\tsources"]
    for case in cases():
        fixture_id = f"g03.plate.{case.name}"
        path = f"parameters/{case.name}.tsv"
        mode = "placeholder" if case.support == "unsupported" else "rectangle"
        content = ("# beamcraft.g03-parameters/1; mm; no rotation; tokens nan/inf are f64 probes\n"
                   f"{fixture_id}\t{mode}\t"
                   + "\t".join(repr(v) for v in (*case.dimensions, *case.origin)) + "\n")
        data = content.encode("utf-8")
        files[path] = data
        manifest.append("\t".join((fixture_id, path, str(len(data)),
                                   hashlib.sha256(data).hexdigest(), case.support,
                                   case.plan, "BeamCraft contributors", "MPL-2.0")))
        values = ["-", "-", "-", "-"]
        outcome = "not_run"
        sources = "F06:1,2,4,10"
        if case.support in ("supported", "unverified"):
            a, b, c = map(Fraction.from_float, case.dimensions)
            origin = list(map(Fraction.from_float, case.origin))
            values = [rational(a * b * c), ",".join(map(rational, origin)),
                      ",".join(rational(o + d) for o, d in zip(origin, (a, b, c))),
                      "8,12,6,12,18,1,2"]
            outcome = "ok" if case.support == "supported" else "not_run"
            sources = "F06:2,4,6,7,10;MATH:box-product,cartesian-bounds,euler"
        elif case.support == "invalid":
            outcome = ("limit_exceeded" if case.diagnostics.startswith("Caps:")
                       else "invalid_input")
            sources = "F06:2,3,4"
        expected.append("\t".join((fixture_id, outcome, case.diagnostics, *values, sources)))
    files["manifest.tsv"] = ("\n".join(manifest) + "\n").encode("utf-8")
    files["expectations.tsv"] = ("\n".join(expected) + "\n").encode("utf-8")
    return files


def main() -> None:
    """Write reproducible assets, or fail if any committed asset differs."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    files = artifacts()
    present = {p.relative_to(ROOT).as_posix() for p in ROOT.rglob("*.tsv")}
    extra = present - files.keys()
    if extra:
        raise ValueError(f"Unlisted TSV files: {sorted(extra)}")
    for name, data in files.items():
        path = ROOT / name
        if args.check:
            if path.read_bytes() != data:
                raise ValueError(f"Fixture/reference mismatch: {name}")
        else:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(data)
    print(f"G03: {len(cases())} original descriptions; {len(files)} files verified")


if __name__ == "__main__":
    main()
