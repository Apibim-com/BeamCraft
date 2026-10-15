"""Run the required checks offline; record exact command, exit status and output."""
from pathlib import Path
import hashlib
import json
import subprocess

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "spikes/n02-ifc-lite"
commands = [
    (OUT, ["cargo", "--offline", "fmt", "--all", "--", "--check"]),
    (OUT, ["cargo", "clippy", "--offline", "--all-targets", "--", "-D", "warnings"]),
    (OUT, ["cargo", "test", "--offline"]),
    (ROOT, ["cargo", "--offline", "fmt", "--all", "--", "--check"]),
    (ROOT, ["cargo", "clippy", "--workspace", "--all-targets", "--offline", "--", "-D", "warnings"]),
    (ROOT, ["cargo", "test", "--workspace", "--offline"]),
]
results = []
for index, (cwd, command) in enumerate(commands, 1):
    result = subprocess.run(command, cwd=cwd, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=240)
    text = result.stdout.decode("utf-8", errors="replace").replace("\r\n", "\n")
    log = f"check-{index}.txt"
    (OUT / log).write_text(text, encoding="utf-8", newline="\n")
    row = dict(cwd=str(cwd.relative_to(ROOT)), command=command, exit=result.returncode, log=log)
    results.append(row)
    print(json.dumps(row), flush=True)
print("ROOT_LOCK_SHA256", hashlib.sha256((ROOT / "Cargo.lock").read_bytes()).hexdigest(), flush=True)
(OUT / "checks.json").write_text(json.dumps(results, indent=2)+"\n", encoding="utf-8", newline="\n")
