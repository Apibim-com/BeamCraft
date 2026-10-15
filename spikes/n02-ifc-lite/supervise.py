"""Original Windows supervisor: install hard job memory limits before opening the input gate."""
import ctypes
from ctypes import wintypes as w
import json
import hashlib
from pathlib import Path
import subprocess
import time

class Basic(ctypes.Structure):
    _fields_ = [("process_time", ctypes.c_int64), ("job_time", ctypes.c_int64), ("flags", w.DWORD), ("minimum", ctypes.c_size_t), ("maximum", ctypes.c_size_t), ("active", w.DWORD), ("affinity", ctypes.c_size_t), ("priority", w.DWORD), ("scheduling", w.DWORD)]
class Io(ctypes.Structure):
    _fields_ = [(name, ctypes.c_uint64) for name in ("read_ops", "write_ops", "other_ops", "read_bytes", "write_bytes", "other_bytes")]
class Extended(ctypes.Structure):
    _fields_ = [("basic", Basic), ("io", Io), ("process_memory", ctypes.c_size_t), ("job_memory", ctypes.c_size_t), ("peak_process", ctypes.c_size_t), ("peak_job", ctypes.c_size_t)]

k = ctypes.WinDLL("kernel32", use_last_error=True)
k.CreateJobObjectW.argtypes = [ctypes.c_void_p, w.LPCWSTR]
k.CreateJobObjectW.restype = w.HANDLE
k.SetInformationJobObject.argtypes = [w.HANDLE, ctypes.c_int, ctypes.c_void_p, w.DWORD]
k.SetInformationJobObject.restype = w.BOOL
k.AssignProcessToJobObject.argtypes = [w.HANDLE, w.HANDLE]
k.AssignProcessToJobObject.restype = w.BOOL
k.QueryInformationJobObject.argtypes = [w.HANDLE, ctypes.c_int, ctypes.c_void_p, w.DWORD, ctypes.c_void_p]
k.QueryInformationJobObject.restype = w.BOOL
k.CloseHandle.argtypes = [w.HANDLE]
k.CloseHandle.restype = w.BOOL

def checked(value):
    if not value:
        raise ctypes.WinError(ctypes.get_last_error())
    return value

def execute(mode, timeout=5):
    job = checked(k.CreateJobObjectW(None, None))
    child = None
    started = time.perf_counter()
    try:
        limits = Extended()
        limits.basic.flags = 0x100 | 0x200 | 0x2000 | 0x400
        limits.process_memory = 256 * 1024 * 1024
        limits.job_memory = 512 * 1024 * 1024
        checked(k.SetInformationJobObject(job, 9, ctypes.byref(limits), ctypes.sizeof(limits)))
        child = subprocess.Popen([str(EXE), mode], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        checked(k.AssignProcessToJobObject(job, w.HANDLE(int(child._handle))))
        timed_out = False
        try:
            stdout, stderr = child.communicate(input=b"G", timeout=timeout)
        except subprocess.TimeoutExpired:
            timed_out = True
            child.kill()
            stdout, stderr = child.communicate(timeout=2)
        peak = Extended()
        checked(k.QueryInformationJobObject(job, 9, ctypes.byref(peak), ctypes.sizeof(peak), None))
        return dict(mode=mode, exit=child.returncode, timeout=timed_out, seconds=time.perf_counter()-started, peak_committed_bytes=peak.peak_process, stdout=(stdout.hex() if mode == "dump" else stdout.decode("utf-8", errors="replace")), stdout_sha256=hashlib.sha256(stdout).hexdigest(), stderr=stderr.decode("utf-8", errors="replace"))
    finally:
        if child is not None and child.poll() is None:
            child.kill()
            child.wait(timeout=2)
        checked(k.CloseHandle(job))

ROOT = Path(__file__).resolve().parent
EXE = ROOT / "target/release/beamcraft-n02-ifc-lite-spike.exe"
if __name__ == "__main__":
    results = []
    for mode in ["suite", "clip", "boolean", "bench", "zero", "negative", "zero-width", "negative-width", "huge", "nan", "infinity", "nan-contour", "infinite-contour", "collinear", "self-intersecting", "empty", "suite", "boolean", "dump", "dump"]:
        result = execute(mode)
        results.append(result)
        print(json.dumps({key: value for key, value in result.items() if key != "stdout"}), flush=True)
    print("dump_bytes_identical", results[-1]["stdout"] == results[-2]["stdout"], flush=True)
    for result in results:
        if result["mode"] == "dump":
            result["stdout_bytes"] = len(result["stdout"]) // 2
            del result["stdout"]
    (ROOT / "results.json").write_text(json.dumps(results, indent=2)+"\n", encoding="utf-8", newline="\n")
