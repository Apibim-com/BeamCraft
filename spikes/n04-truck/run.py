"""Original offline supervisor and independent scalar mesh audit (Python stdlib only).

Each invocation has a 30 second wall deadline. No truck object survives a failed call.
Raw meshes stay under ignored target; reviewed evidence contains hashes and numbers.
"""

import argparse
from collections import Counter, defaultdict
import hashlib
import json
import math
import os
from pathlib import Path
import re
import struct
import subprocess
import sys
import time
from oracle import expected

ROOT = Path(__file__).resolve().parent
sys.stdout.reconfigure(encoding="utf-8")
OUT = ROOT / "target/evidence"
TMP = ROOT / "target/probes"
source = (ROOT / "src/lib.rs").read_text(encoding="utf-8")
names = re.findall(r'"([a-z0-9-]+)"', source.split("pub const CASES:", 1)[1].split("];", 1)[0])
header = re.search(r'pub const HEADER: &str = "(.*)";', source)[1].replace("\\t", "\t").split("\t")


def scalar_audit(data, nv, nt):
    """No truck math, topology helper, returned volume or bounds is trusted here."""
    assert len(data) == 24 * (nv + nt)
    positions = list(struct.iter_unpack("<ddd", data[:24*nv]))
    triangles = list(struct.iter_unpack("<QQQ", data[24*nv:]))
    anchor = positions[0]
    dets = []
    edges = defaultdict(list)
    geom_edges = defaultdict(list)
    aliases = {}
    gids = []
    for point in positions:
        gids.append(aliases.setdefault(point, len(aliases)))
    used = set()
    for tri in triangles:
        assert all(i < nv for i in tri)
        used.update(tri)
        a, b, c = [tuple(x-y for x,y in zip(positions[i],anchor)) for i in tri]
        dets.append((a[0]*(b[1]*c[2]-b[2]*c[1]) + a[1]*(b[2]*c[0]-b[0]*c[2]) + a[2]*(b[0]*c[1]-b[1]*c[0])) / 6)
        for target, ids in ((edges,tri),(geom_edges,[gids[i] for i in tri])):
            for x,y in zip(ids, (*ids[1:],ids[0])):
                target[min(x,y),max(x,y)].append((x,y))

    def closed(table):
        return bool(table) and all(len(v)==2 and v[0] == v[1][::-1] and v[0][0] != v[0][1] for v in table.values())

    return {"volume": math.fsum(dets), "raw_closed": closed(edges),
        "position_closed": closed(geom_edges), "raw_chi": nv-len(edges)+nt,
        "position_chi": len(aliases)-len(geom_edges)+nt, "unused_positions": nv-len(used),
        "raw_boundary_or_nonmanifold_edges": sum(len(v)!=2 for v in edges.values()),
        "exact_position_boundary_or_nonmanifold_edges": sum(len(v)!=2 for v in geom_edges.values()),
        "min": [min(p[d] for p in positions) for d in range(3)],
        "max": [max(p[d] for p in positions) for d in range(3)]}


def one(profile, mode, threads, repeat, name, deadline_seconds=30):
    prefix = TMP / f"{profile}-{mode}-{threads}-{repeat}-{name}"
    executable = ROOT / "target" / profile / "beamcraft-n04-truck-spike.exe"
    started = time.monotonic()
    env = dict(os.environ, RAYON_NUM_THREADS=str(threads), RUST_BACKTRACE="0", CARGO_NET_OFFLINE="true")
    record = dict(profile=profile, mode=mode, threads=threads, repeat=repeat, case=name,
        deadline_seconds=deadline_seconds)
    try:
        result = subprocess.run([str(executable), name, mode, str(prefix)], capture_output=True,
            encoding="utf-8", errors="replace", env=env, timeout=deadline_seconds)
        record.update(exit=result.returncode, stdout=result.stdout.strip(), stderr=result.stderr.strip())
        fields = result.stdout.strip().split("\t")
        if result.returncode:
            record["status"] = "PANIC" if "panicked at" in result.stderr else "FAIL"
        elif len(fields) == len(header):
            record.update(status=fields[1], metrics=dict(zip(header,fields)))
            reference = expected(name)["analytic_volume_mm3"]
            assert abs(float(record["metrics"]["expected_volume"])-reference) <= 1e-10*reference
            data = prefix.with_suffix(".mesh").read_bytes()
            tri = prefix.with_suffix(".tri").read_bytes()
            record["mesh_sha256"] = hashlib.sha256(data).hexdigest()
            record["triangle_sha256"] = hashlib.sha256(tri).hexdigest()
            record["audit"] = scalar_audit(data,int(record["metrics"]["mesh_v"]),int(record["metrics"]["triangles"]))
            # A second hash ignores triangle order, but preserves each triangle's winding.
            canonical = []
            for chunk in struct.iter_unpack("<9d", tri):
                vertices = [tuple(0.0 if x == 0 else x for x in chunk[i:i+3]) for i in (0,3,6)]
                rotations = [vertices[i:]+vertices[:i] for i in range(3)]
                canonical.append(min(rotations))
            record["sorted_triangle_sha256"] = hashlib.sha256(repr(sorted(canonical)).encode()).hexdigest()
            for key in ("volume","raw_closed","position_closed","position_chi"):
                actual = record["audit"][key]
                rust = record["metrics"][{"volume":"mesh_volume"}.get(key,key)]
                if isinstance(actual,bool):
                    assert str(actual).lower() == rust
                elif key == "volume":
                    assert abs(actual-float(rust)) <= 1e-12*max(1,abs(actual))
                else:
                    assert actual == int(rust)
        else:
            record["status"] = fields[1] if len(fields)>1 else "FAIL"
    except subprocess.TimeoutExpired as error:
        stderr = error.stderr or b""
        if isinstance(stderr, bytes):
            stderr = stderr.decode("utf-8",errors="replace")
        record.update(status="FAIL", error=f"{deadline_seconds} second supervisor timeout", timeout=True,stderr=stderr)
    record["seconds"] = round(time.monotonic()-started,6)
    return record


def main() -> None:
    """Run a bounded schedule and checkpoint each profile to its own file."""
    parser = argparse.ArgumentParser(description=__doc__)
    group = parser.add_mutually_exclusive_group()
    group.add_argument("--debug-only", action="store_true")
    group.add_argument("--release-only", action="store_true")
    parser.add_argument("--quick", action="store_true")
    parser.add_argument("--release", action="store_true", help="release quick pass")
    parser.add_argument("cases", nargs="*")
    args = parser.parse_args()
    if any(case not in names for case in args.cases):
        parser.error("unknown case; select a fixed name from src/lib.rs CASES")
    cases = args.cases or names
    quick_profile = "release" if args.release else "debug"
    profiles = ("debug",) if args.debug_only else ("release",) if args.release_only else ("debug", "release")
    schedule = [(quick_profile,"caught",1,0)] if args.quick else [
        (profile,mode,threads,repeat)
        for profile in profiles
        for mode,threads,repeat in [("caught",1,0),("caught",1,1),("caught",1,2),("caught",4,0),("direct",1,0)]
    ]
    TMP.mkdir(parents=True, exist_ok=True)
    OUT.mkdir(parents=True, exist_ok=True)
    records = []
    paths = {profile: TMP / f"quick-{profile}.json" if args.quick
             else OUT / f"results-{profile}.json"
             for profile in ((quick_profile,) if args.quick else profiles)}
    # Clear every selected profile before starting so interruption cannot reuse old runs.
    for path in paths.values():
        path.write_text("[]\n", encoding="utf-8", newline="\n")
    for profile,mode,threads,repeat in schedule:
        for case in cases:
            record = one(profile,mode,threads,repeat,case)
            records.append(record)
            paths[profile].write_text(
                json.dumps([r for r in records if r["profile"] == profile], indent=2)+"\n",
                encoding="utf-8", newline="\n")
            print(profile,mode,threads,repeat,case,record["status"],record["seconds"],flush=True)
    print(dict(Counter(r["status"] for r in records)))


if __name__ == "__main__":
    main()
