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


def inventory():
    command = ["cargo", "metadata", "--offline", "--format-version", "1"]
    result = subprocess.run(command, cwd=ROOT, capture_output=True, check=True)
    metadata = json.loads(result.stdout)
    locked = tomllib.loads((ROOT / "Cargo.lock").read_text(encoding="utf-8"))
    checksums = {(p["name"], p["version"]): p.get("checksum") for p in locked["package"]}
    nodes = {n["id"]: n for n in metadata["resolve"]["nodes"]}
    packages = []
    for package in metadata["packages"]:
        directory = Path(package["manifest_path"]).parent
        notices = []
        # The local spike is not a registry dependency; avoid hashing its build tree.
        candidates = [] if package["source"] is None else directory.rglob("*")
        for path in candidates:
            if path.is_file() and any(term in path.name.lower() for term in ["license", "licence", "copying", "notice"]):
                notices.append({"path": path.relative_to(directory).as_posix(), "sha256": digest(path)})
        packages.append({
            "name": package["name"], "version": package["version"],
            "license": package["license"], "rust_version": package["rust_version"],
            "source": package["source"], "repository": package["repository"],
            "checksum": checksums[(package["name"], package["version"])],
            "features": nodes[package["id"]]["features"], "notice_files": notices,
        })
    assert {(p["name"], p["version"]) for p in packages} == set(checksums)
    usd = next(p for p in metadata["packages"] if p["name"] == "openusd")
    directory = Path(usd["manifest_path"]).parent
    files = [{"path": p.relative_to(directory).as_posix(), "sha256": digest(p), "bytes": p.stat().st_size}
             for p in sorted(directory.rglob("*")) if p.is_file()]
    (ROOT / "inventory.json").write_text(json.dumps({
        "command": command, "packages": packages, "openusd_files": files,
        "openusd_declared_features": usd["features"],
        "finding": "README licence section assigns src/usd/core_schemas.rs to TOST-1.0; MIT manifest is incomplete for bundled files. No policy exception inferred.",
    }, indent=2) + "\n", encoding="utf-8")
    tree = subprocess.run(["cargo", "tree", "--offline", "--edges", "all"], cwd=ROOT, capture_output=True, check=True)
    (ROOT / "dependency-tree.txt").write_bytes(tree.stdout)
    print(f"inventory: {len(packages)} locked packages, all matched; {len(files)} openusd files hashed; optional mmap/serde disabled")


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


def corpus(smoke):
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
        # Corrupt each byte of a small original binary produced by this run, if supplied.
        binaries = list((ARTIFACTS).glob("test-*-roundtrip/model.usdc"))
        if binaries:
            binary = binaries[-1].read_bytes()
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
    results = []
    for index, (name, data) in enumerate(corpus(smoke)):
        assert len(data) <= 65536
        path = directory / (name + (".usdc" if "usdc" in name else ".usda"))
        path.write_bytes(data)
        result = bounded([str(binary), "parse", str(path)], gate=True)
        result.update({"case": name, "bytes": len(data), "sha256": digest(path)})
        results.append(result)
        if (index + 1) % 200 == 0:
            print(f"hostile progress: {index+1}", flush=True)
    output = ROOT / ("hostile-smoke-results.json" if smoke else "hostile-results.json")
    output.write_text(json.dumps(results, indent=2) + "\n", encoding="utf-8")
    counts = {status: sum(r["status"] == status for r in results) for status in sorted({r["status"] for r in results})}
    print(f"hostile: {len(results)} cases; {counts}; max seconds={max(r['elapsed_seconds'] for r in results):.3f}; max commit={max(r['peak_job_commit_bytes'] for r in results)}")
    # Recorded parser failures are findings; supervisor failure raises and is a failed test.


def performance(binary):
    results = []
    for count in [1000, 10000]:
        for repeat in range(3):
            destination = ARTIFACTS / f"sanity-{count}-{repeat}"
            result = bounded([str(binary), "sanity", str(count), str(destination)], gate=True, seconds=60)
            result.update({"elements": count, "repeat": repeat})
            results.append(result)
            print(result["stdout"] or result["status"], flush=True)
    (ROOT / "performance-results.json").write_text(json.dumps({
        "os": platform.platform(), "machine": platform.machine(), "processor": platform.processor(),
        "method": "release build; 3 fresh child processes per size; fixture generation before timed open; warm OS cache; compose/traverse forces width reads; save separate; startup included only in supervisor elapsed", "results": results,
    }, indent=2) + "\n", encoding="utf-8")


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
