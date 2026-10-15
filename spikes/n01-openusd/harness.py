"""Original N01 offline inventory and bounded process experiments; MPL-2.0.

Python is a development supervisor only. USD operations run in the Rust binary.
"""
import argparse
import ctypes
from ctypes import wintypes as w
import hashlib
import json
import os
from pathlib import Path
import platform
import random
import subprocess
import sys
import time
import tomllib

ROOT = Path(__file__).resolve().parent
ARTIFACTS = ROOT / "artifacts"
CAP = 512 * 1024 * 1024


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def write_json(path, value):
    """Write JSON with LF endings on every platform."""
    path.write_text(json.dumps(value, indent=2) + "\n", encoding="utf-8", newline="\n")


def inventory():
    """Write a compact package/licence list; raw cargo metadata is not retained."""
    command = ["cargo", "metadata", "--offline", "--format-version", "1"]
    result = subprocess.run(command, cwd=ROOT, capture_output=True, check=True)
    metadata = json.loads(result.stdout)
    locked = tomllib.loads((ROOT / "Cargo.lock").read_text(encoding="utf-8"))
    checksums = {(p["name"], p["version"]) for p in locked["package"]}
    packages = [{"name": p["name"], "version": p["version"], "license": p["license"],
                 "rust_version": p["rust_version"]}
                for p in sorted(metadata["packages"], key=lambda p: (p["name"], p["version"]))]
    assert {(p["name"], p["version"]) for p in packages} == checksums
    usd = next(p for p in metadata["packages"] if p["name"] == "openusd")
    write_json(ROOT / "inventory.json", {
        "regenerate": "python harness.py inventory (from spikes/n01-openusd, cargo cache populated)",
        "packages": packages,
        "openusd_declared_features": sorted(usd["features"]),
        "finding": "openusd README: src/usd/core_schemas.rs is generated from OpenUSD schemas and covered by TOST-1.0; the MIT manifest field does not cover it. No policy exception is inferred.",
    })
    print(f"inventory: {len(packages)} locked packages, all matched; optional mmap/serde disabled")


class BasicLimits(ctypes.Structure):
    _fields_ = [("process_time", ctypes.c_int64), ("job_time", ctypes.c_int64),
                ("flags", w.DWORD), ("min_ws", ctypes.c_size_t), ("max_ws", ctypes.c_size_t),
                ("active", w.DWORD), ("affinity", ctypes.c_size_t),
                ("priority", w.DWORD), ("scheduling", w.DWORD)]


class IoCounters(ctypes.Structure):
    _fields_ = [(name, ctypes.c_uint64) for name in ("read_ops", "write_ops", "other_ops", "read_bytes", "write_bytes", "other_bytes")]


class ExtendedLimits(ctypes.Structure):
    _fields_ = [("basic", BasicLimits), ("io", IoCounters),
                ("process_memory", ctypes.c_size_t), ("job_memory", ctypes.c_size_t),
                ("peak_process_memory", ctypes.c_size_t), ("peak_job_memory", ctypes.c_size_t)]


class ProcessMemory(ctypes.Structure):
    _fields_ = [("size", w.DWORD), ("faults", w.DWORD),
                ("peak_ws", ctypes.c_size_t), ("ws", ctypes.c_size_t),
                ("peak_paged", ctypes.c_size_t), ("paged", ctypes.c_size_t),
                ("peak_nonpaged", ctypes.c_size_t), ("nonpaged", ctypes.c_size_t),
                ("pagefile", ctypes.c_size_t), ("peak_pagefile", ctypes.c_size_t)]


def windows_api():
    if os.name != "nt":
        raise RuntimeError("This harness requires Windows Job memory enforcement; do not run uncapped")
    api = ctypes.WinDLL("kernel32", use_last_error=True)
    api.CreateJobObjectW.argtypes = [ctypes.c_void_p, w.LPCWSTR]
    api.CreateJobObjectW.restype = w.HANDLE
    api.SetInformationJobObject.argtypes = [w.HANDLE, ctypes.c_int, ctypes.c_void_p, w.DWORD]
    api.SetInformationJobObject.restype = w.BOOL
    api.AssignProcessToJobObject.argtypes = [w.HANDLE, w.HANDLE]
    api.AssignProcessToJobObject.restype = w.BOOL
    api.QueryInformationJobObject.argtypes = [w.HANDLE, ctypes.c_int, ctypes.c_void_p, w.DWORD, ctypes.c_void_p]
    api.QueryInformationJobObject.restype = w.BOOL
    api.CloseHandle.argtypes = [w.HANDLE]
    api.CloseHandle.restype = w.BOOL
    memory_api = ctypes.WinDLL("psapi", use_last_error=True)
    memory_api.GetProcessMemoryInfo.argtypes = [w.HANDLE, ctypes.c_void_p, w.DWORD]
    memory_api.GetProcessMemoryInfo.restype = w.BOOL
    return api, memory_api


def bounded(command, gate=False, seconds=5):
    api, memory_api = windows_api()
    job = api.CreateJobObjectW(None, None)
    if not job:
        raise ctypes.WinError(ctypes.get_last_error())
    process = None
    try:
        limits = ExtendedLimits()
        # PROCESS_MEMORY, JOB_MEMORY, KILL_ON_JOB_CLOSE: hard commit cap, not just sampling.
        limits.basic.flags = 0x100 | 0x200 | 0x2000
        limits.process_memory = CAP
        limits.job_memory = CAP
        if not api.SetInformationJobObject(job, 9, ctypes.byref(limits), ctypes.sizeof(limits)):
            raise ctypes.WinError(ctypes.get_last_error())
        start = time.perf_counter()
        process = subprocess.Popen(command, cwd=ROOT, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        if not api.AssignProcessToJobObject(job, w.HANDLE(int(process._handle))):
            raise ctypes.WinError(ctypes.get_last_error())
        timeout = False
        try:
            stdout, stderr = process.communicate(b"G" if gate else b"", timeout=seconds)
        except subprocess.TimeoutExpired:
            timeout = True
            process.kill()
            stdout, stderr = process.communicate(timeout=2)
        elapsed = time.perf_counter() - start
        peak = ProcessMemory()
        peak.size = ctypes.sizeof(peak)
        peak_ok = memory_api.GetProcessMemoryInfo(w.HANDLE(int(process._handle)), ctypes.byref(peak), peak.size)
        queried = ExtendedLimits()
        if not api.QueryInformationJobObject(job, 9, ctypes.byref(queried), ctypes.sizeof(queried), None):
            raise ctypes.WinError(ctypes.get_last_error())
        out = stdout.decode("utf-8", errors="replace")[:2000].strip()
        err = stderr.decode("utf-8", errors="replace")[:4000].strip()
        status = ("TIMEOUT" if timeout else "PANIC" if "PANIC" in out else
                  "CRASH" if process.returncode != 0 else "REJECTED" if out.startswith("REJECTED") else "ACCEPTED")
        return {"status": status, "exit": process.returncode, "elapsed_seconds": elapsed,
                "peak_working_set_bytes": int(peak.peak_ws) if peak_ok else None,
                "peak_job_commit_bytes": int(queried.peak_job_memory),
                "stdout": out, "stderr": err, "cap_bytes": CAP}
    finally:
        if process is not None and process.poll() is None:
            process.kill()
            process.communicate(timeout=2)
        api.CloseHandle(job)


def seed_binary(binary):
    """Generate the deterministic USDC seed for the binary mutations; fail if it is missing."""
    directory = ARTIFACTS / "hostile-seed"
    seed = directory / "model.usdc"
    seed.unlink(missing_ok=True)
    subprocess.run([str(binary), "seed", str(directory)], cwd=ROOT, check=True, timeout=30, stdin=subprocess.DEVNULL)
    if not seed.is_file():
        raise SystemExit(f"binary seed was not generated: {seed}")
    return seed


def corpus(smoke, seed=None):
    base = b'#usda 1.0\ndef "A" { custom double x = 1.25\n}\n'
    cases = [("truncated", base[:-2]), ("malformed", b'#usda 1.0\ndef "A" { double x = [ }'),
             ("invalid-utf8", b'#usda 1.0\n\xff'),
             ("huge-integer", b'#usda 1.0\ndef "A" { int[] a = [18446744073709551615]\n}'),
             ("huge-exponent", b'#usda 1.0\ndef "A" { double x = 1e999999999\n}'),
             ("corrupt-usdc", b'PXR-USDC' + bytes([255]) * 80)]
    for depth in [32, 33, 64]:
        cases.append((f"depth-{depth}", b'#usda 1.0\n' + b'def "N" {\n' * depth + b'}\n' * depth))
    cases.append(("huge-face-count", b'#usda 1.0\ndef Mesh "A" { int[] faceVertexCounts = [2147483647]\n}'))
    if not smoke:
        cases.extend((f"truncate-{i:03}", base[:i]) for i in range(len(base)))
        rng = random.Random(0x4E3031)
        tokens = [b'{', b'}', b'[', b']', b'(', b')', b'"', b'@', b'0', b'99999999999999999999', b'def ', b'\xff']
        for number in range(1000):
            cases.append((f"seeded-{number:04}", b'#usda 1.0\n' + b''.join(rng.choice(tokens) for _ in range(rng.randrange(1, 25)))))
        # Corrupt each byte of the small deterministic binary generated by `seed_binary`.
        binary = seed.read_bytes()
        headerword = bytearray(binary)
        for position in range(16, 24):
            headerword[position] = 255
        cases.append(("usdc-huge-headerword", bytes(headerword)))
        for index in range(min(len(binary), 128)):
            changed = bytearray(binary)
            changed[index] ^= 255
            cases.append((f"usdc-corrupt-{index:03}", bytes(changed)))
    return cases


def hostile(binary, smoke):
    directory = ARTIFACTS / ("hostile-smoke" if smoke else "hostile")
    directory.mkdir(parents=True, exist_ok=True)
    seed = None if smoke else seed_binary(binary)
    results = []
    for index, (name, data) in enumerate(corpus(smoke, seed)):
        assert len(data) <= 65536
        path = directory / (name + (".usdc" if "usdc" in name else ".usda"))
        path.write_bytes(data)
        result = bounded([str(binary), "parse", str(path)], gate=True)
        result.update({"case": name, "bytes": len(data), "sha256": digest(path)})
        results.append(result)
        if (index + 1) % 200 == 0:
            print(f"hostile progress: {index+1}", flush=True)
    # Bulk per-case logs stay in the ignored artifacts directory; only a summary is committed.
    write_json(directory / "results.json", results)
    classes = {}
    for r in results:
        name = r["case"].rstrip("0123456789-")
        entry = classes.setdefault(name, {"cases": 0, "statuses": {}})
        entry["cases"] += 1
        entry["statuses"][r["status"]] = entry["statuses"].get(r["status"], 0) + 1
    accepted = [r["case"] for r in results if r["status"] == "ACCEPTED"]
    summary = {
        "regenerate": "cargo build --release, then python harness.py hostile" + (" --smoke" if smoke else "")
        + ("" if smoke else " (the harness generates its own deterministic USDC seed with the binary's seed command)"),
        "cases": len(results),
        "statuses": {s: sum(r["status"] == s for r in results) for s in sorted({r["status"] for r in results})},
        "classes": classes,
        "accepted_cases": accepted,
        "max_seconds": max(r["elapsed_seconds"] for r in results),
        "max_peak_job_commit_bytes": max(r["peak_job_commit_bytes"] for r in results),
        "max_peak_working_set_bytes": max(r["peak_working_set_bytes"] or 0 for r in results),
        "cap_bytes": CAP, "timeout_seconds": 5,
    }
    if not smoke:
        write_json(ROOT / "hostile-summary.json", summary)
    print(f"hostile: {summary['cases']} cases; {summary['statuses']}; max seconds={summary['max_seconds']:.3f}; max commit={summary['max_peak_job_commit_bytes']}")
    failed = [r["case"] for r in results if r["status"] not in ("ACCEPTED", "REJECTED")]
    if failed:
        # The results are already written above; a parser failure must not pass as success.
        print(f"hostile FAILED: {len(failed)} cases neither ACCEPTED nor REJECTED: {failed[:20]}", file=sys.stderr)
        sys.exit(1)


def performance(binary):
    results = []
    for count in [1000, 10000]:
        for repeat in range(3):
            destination = ARTIFACTS / f"sanity-{count}-{repeat}"
            result = bounded([str(binary), "sanity", str(count), str(destination)], gate=True, seconds=60)
            result.update({"elements": count, "repeat": repeat})
            results.append(result)
            print(result["stdout"] or result["status"], flush=True)
    write_json(ROOT / "performance-results.json", {
        "os": platform.platform(), "machine": platform.machine(), "processor": platform.processor(),
        "method": "release build; 3 fresh child processes per size; fixture generation before timed open; warm OS cache; compose/traverse forces width reads; save separate; startup included only in supervisor elapsed", "results": results,
    })


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("mode", choices=["inventory", "hostile", "performance"])
    parser.add_argument("--binary", type=Path, default=ROOT / "target" / "release" / "beamcraft-n01-openusd-spike.exe")
    parser.add_argument("--smoke", action="store_true")
    args = parser.parse_args()
    ARTIFACTS.mkdir(exist_ok=True)
    if args.mode == "inventory": inventory()
    elif args.mode == "hostile": hostile(args.binary.resolve(), args.smoke)
    else: performance(args.binary.resolve())
