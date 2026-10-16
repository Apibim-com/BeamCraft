"""Print the report numbers from raw records; unavailable comparisons stay unmeasured.

This small post-processor is retained to reproduce the report tables without
re-running geometry. It writes no evidence or report files.
"""

import argparse
from collections import Counter
import json
from pathlib import Path
from typing import Any

from oracle import expected, verify_g03

ROOT = Path(__file__).resolve().parent
Record = dict[str, Any]


def compare_hashes(runs: list[Record], schedules: list[tuple[str, int, int]],
                   key: str) -> bool | None:
    """Compare only complete schedules with hashes; None means unmeasured."""
    hashes = []
    for schedule in schedules:
        matches = [r for r in runs
                   if (r["mode"], r["threads"], r["repeat"]) == schedule]
        if len(matches) != 1 or not matches[0].get(key):
            return None
        hashes.append(matches[0][key])
    return len(set(hashes)) == 1


def comparisons(runs: list[Record]) -> dict[str, bool | None]:
    """Return independent repeat and thread comparisons for one case/profile."""
    repeats = [("caught", 1, repeat) for repeat in range(3)]
    threads = [("caught", 1, 0), ("caught", 4, 0)]
    return {
        "repeat mesh": compare_hashes(runs, repeats, "mesh_sha256"),
        "repeat triangles": compare_hashes(runs, repeats, "triangle_sha256"),
        "thread triangles": compare_hashes(runs, threads, "triangle_sha256"),
    }


def tables(records: list[Record]) -> str:
    """Derive case counts, baseline quantities and measured-only denominators."""
    names = list(dict.fromkeys(r["case"] for r in records))
    rows = ["| Case | Expected mm3 | Debug observed / 5 | Release observed / 5 | Release volume / relative error | B-rep V/E/F; chi | Mesh V/T; missing faces |",
            "|---|---:|---|---|---|---|---|"]
    totals = {(profile, label): Counter() for profile in ("debug", "release")
              for label in ("repeat mesh", "repeat triangles", "thread triangles")}
    for name in names:
        ref = expected(name)["analytic_volume_mm3"]
        profiles = {}
        for profile in ("debug", "release"):
            runs = [r for r in records if r["case"] == name and r["profile"] == profile]
            profiles[profile] = runs
            for label, result in comparisons(runs).items():
                totals[profile, label][result] += 1
            for run in runs:
                if "metrics" in run:
                    assert abs(float(run["metrics"]["expected_volume"]) - ref) <= 1e-10 * ref
        baseline = next((r for r in profiles["release"]
                         if (r["mode"], r["threads"], r["repeat"]) == ("caught", 1, 0)), {})
        metrics = baseline.get("metrics", {})
        counts = []
        for runs in profiles.values():
            counts.append(", ".join(f"{status} x {count}" for status, count
                                    in sorted(Counter(r["status"] for r in runs).items()))
                          or "unmeasured")
        volume = (f"{float(metrics['mesh_volume']):.12g} / {float(metrics['rel_error']):.5g}"
                  if metrics else "no mesh")
        brep = (f"{metrics['brep_v']}/{metrics['brep_e']}/{metrics['brep_f']}; {metrics['brep_chi']}"
                if metrics else "-")
        mesh = (f"{metrics['mesh_v']}/{metrics['triangles']}; {metrics['missing_surfaces']}"
                if metrics else "-")
        rows.append(f"| {name} | {ref:.12g} | {counts[0]} | {counts[1]} | {volume} | {brep} | {mesh} |")
    rows.extend(["", "| Profile | Comparison | Identical / measured | Mismatched | Unmeasured |",
                 "|---|---|---:|---:|---:|"])
    for (profile, label), counts in totals.items():
        measured = counts[True] + counts[False]
        rows.append(f"| {profile} | {label} | {counts[True]}/{measured} | {counts[False]} | {counts[None]} |")
    return "\n".join(rows) + "\n"


def main() -> None:
    """Read per-profile raw outputs from the ignored directory or an archive."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input-dir", type=Path, default=ROOT / "target/evidence")
    args = parser.parse_args()
    records = []
    for profile in ("debug", "release"):
        path = args.input_dir / f"results-{profile}.json"
        if path.exists():
            records.extend(json.loads(path.read_text(encoding="utf-8")))
    if not records:
        parser.error("no raw records found")
    print(f"Observed {len(records)} calls; {dict(Counter(r['status'] for r in records))}; "
          f"G03 references {verify_g03()}")
    print(tables(records), end="")


if __name__ == "__main__":
    main()
