"""Produce review tables from completed isolated runs; recheck independent references."""

from collections import Counter
import hashlib
import json
from pathlib import Path
import sys

from oracle import expected, verify_g03

ROOT = Path(__file__).resolve().parent
OUT = ROOT / "evidence"
sys.stdout.reconfigure(encoding="utf-8")


def write(name, value):
    (OUT / name).write_text(value,encoding="utf-8",newline="\n")


def summarize():
    records = []
    for profile in ("debug","release"):
        path = OUT / f"results-{profile}.json"
        if path.exists():
            records.extend(json.loads(path.read_text(encoding="utf-8")))
    if not records:
        return
    names = list(dict.fromkeys(r["case"] for r in records))
    rows = []
    for name in names:
        ref = expected(name)
        for r in (r for r in records if r["case"] == name and "metrics" in r):
            reported = float(r["metrics"]["expected_volume"])
            assert abs(reported-ref["analytic_volume_mm3"]) <= 1e-10*ref["analytic_volume_mm3"]
        row = dict(reference=ref)
        for profile in ("debug","release"):
            runs = [r for r in records if r["case"]==name and r["profile"]==profile]
            baseline = next((r for r in runs if r["mode"]=="caught" and r["threads"]==1 and r["repeat"]==0),None)
            repeats = [r for r in runs if r["mode"]=="caught" and r["threads"]==1]
            changed = [r for r in runs if r["mode"]=="caught" and r["threads"]==4]
            hashes = [r.get("triangle_sha256") for r in repeats]
            meshhashes = [r.get("mesh_sha256") for r in repeats]
            all_with_tri = [r for r in runs if "triangle_sha256" in r]
            row[profile] = {"counts":dict(Counter(r["status"] for r in runs)),
                "timeouts":sum(r.get("timeout",False) for r in runs),
                "baseline":baseline,
                "three_triangle_bytes_identical":len(hashes)==3 and None not in hashes and len(set(hashes))==1,
                "three_mesh_bytes_identical":len(meshhashes)==3 and None not in meshhashes and len(set(meshhashes))==1,
                "thread_change_triangle_bytes_identical": bool(baseline and baseline.get("triangle_sha256") and changed and baseline.get("triangle_sha256")==changed[0].get("triangle_sha256")),
                "sorted_geometry_variants":len({r["sorted_triangle_sha256"] for r in all_with_tri}),
                "no_triangle_output":not all_with_tri}
        rows.append(row)
    write("summary.json",json.dumps({"g03_references_verified":verify_g03(),"records":len(records),
        "outcomes":dict(Counter(r["status"] for r in records)),"cases":rows},indent=2)+"\n")
    write("expectations.json",json.dumps([r["reference"] for r in rows],indent=2)+"\n")
    table = ["| Case | Analytic mm3 | Debug outcomes (5 runs) | Release outcomes (5 runs) | Release baseline volume / relative error | B-rep V/E/F; chi; closed | Mesh V/T; missing faces |",
        "|---|---:|---|---|---|---|---|"]
    determinants = ["| Case | Debug 3 / threads 1→4 | Release 3 / threads 1→4 | Sorted triangle geometry variants (debug / release) |",
        "|---|---|---|---|"]
    for row in rows:
        name = row["reference"]["case"]
        d,r = row["debug"],row["release"]
        b = r["baseline"] or d["baseline"]
        m = b.get("metrics",{}) if b else {}
        outcomes = lambda profile: ", ".join(f"{key}×{value}" for key,value in profile["counts"].items()) or "pending"
        vol = f"{float(m['mesh_volume']):.12g} / {float(m['rel_error']):.5g}" if m else "no mesh"
        brep = f"{m['brep_v']}/{m['brep_e']}/{m['brep_f']}; {m['brep_chi']}; {m['brep_closed']}" if m else "—"
        mesh = f"{m['mesh_v']}/{m['triangles']}; {m['missing_surfaces']}" if m else "—"
        table.append(f"| {name} | {row['reference']['analytic_volume_mm3']:.12g} | {outcomes(d)} | {outcomes(r)} | {vol} | {brep} | {mesh} |")
        def deterministic(p):
            if p["no_triangle_output"]:
                return "not applicable: no triangles"
            return ("PASS" if p["three_triangle_bytes_identical"] else "FAIL")+" / "+("PASS" if p["thread_change_triangle_bytes_identical"] else "FAIL")
        determinants.append(f"| {name} | {deterministic(d)} | {deterministic(r)} | {d['sorted_geometry_variants']} / {r['sorted_geometry_variants']} |")
    write("case-results.md","\n".join(table)+"\n")
    write("determinism.md","\n".join(determinants)+"\n")
    print(len(records),dict(Counter(r["status"] for r in records)),"G03",verify_g03())


if __name__ == "__main__":
    summarize()
