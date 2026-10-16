"""Insert measured evidence into the original report and verify deliverable encoding."""

from collections import Counter
import hashlib
import json
from pathlib import Path
import re
import sys

from summarize import summarize

ROOT = Path(__file__).resolve().parent
REPO = ROOT.parent.parent
OUT = ROOT / "evidence"
REPORT = REPO / "docs/design/n04-truck-evaluation-v0.1.md"
sys.stdout.reconfigure(encoding="utf-8")
summarize()
summary = json.loads((OUT / "summary.json").read_text(encoding="utf-8"))
records = sum((json.loads((OUT / f"results-{p}.json").read_text(encoding="utf-8")) for p in ("debug","release")),[])
assert summary["records"] == 440, "all 44 cases × 2 profiles × 5 schedules must finish"
assert len(summary["cases"]) == 44
assert all(sum(c[p]["counts"].values())==5 for c in summary["cases"] for p in ("debug","release"))
text = REPORT.read_text(encoding="utf-8")


def replace(label, value):
    global text
    pattern = rf"(<!-- N04-{label}-BEGIN -->).*?(<!-- N04-{label}-END -->)"
    text, n = re.subn(pattern, lambda m: m[1]+"\n"+value.strip()+"\n"+m[2],text,flags=re.S)
    assert n==1


totals = []
for p in ("debug","release"):
    selected = [r for r in records if r["profile"]==p]
    direct = [r for r in selected if r["mode"]=="direct"]
    caught = [r for r in selected if r["mode"]=="caught"]
    totals.append(f"{p}: {dict(Counter(r['status'] for r in selected))}; "
        f"timeouts={sum(r.get('timeout',False) for r in selected)}; "
        f"direct panics={sum(r['status']=='PANIC' for r in direct)}; "
        f"caught panics={sum(r['status']=='PANIC' for r in caught)}")
failure_types = Counter()
for r in records:
    if r["status"] == "FAIL":
        reason = "process timeout" if r.get("timeout") else "boolean None" if "boolean None" in r.get("stdout","") else "audit cap" if "audit mesh cap" in r.get("stdout","") else "other returned error"
        failure_types[reason] += 1
replace("CASES", "**440 final invocations, 44 cases.**\n\n"+"\n\n".join(totals)+
    "\n\nFAIL breakdown: "+str(dict(failure_types))+". See raw records for exact stages/reasons.\n\n"+(OUT / "case-results.md").read_text(encoding="utf-8"))
supplement = json.loads((OUT / "supplement.json").read_text(encoding="utf-8"))
assert len(supplement)==2
extra = ["**Two supplementary calls, separate 120-second deadlines; 442 calls total.** These test whether the shorter matrix budget hid a complete result; they supply no repeatability claim.",
    "", "| Profile/case | Result | Analytic mm3 | Returned diagnostic mm3 / relative error | B-rep V/E/F; chi | Mesh V/T; missing surfaces |", "|---|---|---:|---|---|---|"]
for r in supplement:
    m = r.get("metrics",{})
    ref = next(c["reference"] for c in summary["cases"] if c["reference"]["case"]==r["case"])
    values = f"{m['mesh_volume']} / {m['rel_error']}" if m else r.get("error",r.get("stdout","no mesh"))
    brep = f"{m['brep_v']}/{m['brep_e']}/{m['brep_f']}; {m['brep_chi']}" if m else "—"
    mesh = f"{m['mesh_v']}/{m['triangles']}; {m['missing_surfaces']}" if m else "—"
    extra.append(f"| {r['profile']} / {r['case']} | {r['status']} | {ref['analytic_volume_mm3']:.12g} | {values} | {brep} | {mesh} |")
replace("SUPPLEMENT","\n".join(extra))
byte_totals = []
for p in ("debug","release"):
    produced = [c[p] for c in summary["cases"] if not c[p]["no_triangle_output"]]
    byte_totals.append(f"{p}: {sum(c['three_triangle_bytes_identical'] for c in produced)}/{len(produced)} output-producing cases have identical triangle bytes across three runs; "
        f"{sum(c['thread_change_triangle_bytes_identical'] for c in produced)}/{len(produced)} match the changed thread setting.")
replace("DETERMINISM","\n\n".join(byte_totals)+"\n\n"+(OUT / "determinism.md").read_text(encoding="utf-8"))
checks = json.loads((OUT / "checks.json").read_text(encoding="utf-8"))
table = ["| Directory | Command | Actual result |","|---|---|---|"]
for c in checks:
    result = c.get("qualification",c["status"]+f" (exit {c['exit']})")
    table.append(f"| {c['directory']} | `{c['command']}` | {result} |")
replace("CHECKS","\n".join(table))
REPORT.write_text(text,encoding="utf-8",newline="\n")
pr = ROOT / "PR.md"
body = pr.read_text(encoding="utf-8")
counter_table = ["| Profile | PASS | FAIL | PANIC | WRONG | Timeouts (subset of FAIL) |",
    "|---|---:|---:|---:|---:|---:|"]
for profile in ("debug","release"):
    selected = [r for r in records if r["profile"]==profile]
    counts = Counter(r["status"] for r in selected)
    counter_table.append(f"| {profile} | {counts['PASS']} | {counts['FAIL']} | {counts['PANIC']} | {counts['WRONG']} | {sum(r.get('timeout',False) for r in selected)} |")
body = re.sub(r"(<!-- N04-PR-RESULTS-BEGIN -->).*?(<!-- N04-PR-RESULTS-END -->)",
    lambda m: m[1]+"\n"+"\n".join(counter_table)+"\n"+m[2],body,flags=re.S)
pr.write_text(body,encoding="utf-8",newline="\n")
paths = [p for p in ROOT.rglob("*") if p.is_file() and "target" not in p.relative_to(ROOT).parts
         and "__pycache__" not in p.relative_to(ROOT).parts and p != OUT / "verification.json"] + [REPORT]
hashes = {}
for p in paths:
    data = p.read_bytes()
    data.decode("utf-8",errors="strict")
    assert not data.startswith(b"\xef\xbb\xbf"),p
    if b"\r" in data:
        data = data.replace(b"\r\n",b"\n")
        assert b"\r" not in data,p
        p.write_bytes(data)
    hashes[str(p.relative_to(REPO)).replace("\\","/")] = hashlib.sha256(data).hexdigest()
verification = {"utf8_no_bom_lf":True,"completed_invocations":len(records),
    "g03_rational_references":summary["g03_references_verified"],
    "binary_sha256":{p:hashlib.sha256((ROOT/f"target/{p}/beamcraft-n04-truck-spike.exe").read_bytes()).hexdigest() for p in ("debug","release")},
    "files_sha256":hashes}
verification["failure_types"] = dict(failure_types)
verification["raw_f06_closure_failures"] = sum(r.get("metrics",{}).get("raw_closed")=="false" for r in records)
verification["emitted_meshes"] = sum("metrics" in r for r in records)
verification["supplement_invocations"] = len(supplement)
verification["supplement_outcomes"] = dict(Counter(r["status"] for r in supplement))
(OUT / "verification.json").write_text(json.dumps(verification,indent=2)+"\n",encoding="utf-8",newline="\n")
print("Report updated; UTF-8 without BOM / LF verified.")
