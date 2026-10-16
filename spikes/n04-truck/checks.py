"""Run required offline checks with all build outputs inside the allowed spike."""

import hashlib
import json
import os
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parent
REPO = ROOT.parent.parent
OUT = ROOT / "evidence"
OUT.mkdir(exist_ok=True)
env = dict(os.environ, CARGO_NET_OFFLINE="true", CARGO_TARGET_DIR=str(ROOT / "target/root-checks"))
commands = [
    ("root", ["cargo","fmt","--all","--","--check"]),
    ("root", ["cargo","clippy","--workspace","--all-targets","--","-D","warnings"]),
    ("root", ["cargo","test","--workspace"]),
    ("root", ["cargo","check","--manifest-path","spikes/n04-truck/Cargo.toml","--target","wasm32-unknown-unknown"]),
    ("spike", ["cargo","fmt","--all","--","--check"]),
    ("spike", ["cargo","clippy","--workspace","--all-targets","--","-D","warnings"]),
    ("spike", ["cargo","test","--workspace"]),
    ("spike", ["cargo","test","--workspace","--release"]),
    ("spike", ["python","test_harness.py"]),
]
records = []
before = hashlib.sha256((REPO / "Cargo.lock").read_bytes()).hexdigest()
for i,(location,command) in enumerate(commands,1):
    use_env = env if location == "root" and i != 4 else dict(os.environ,CARGO_NET_OFFLINE="true")
    result = subprocess.run(command,cwd=REPO if location == "root" else ROOT,
        env=use_env,capture_output=True,encoding="utf-8",errors="replace")
    (OUT / f"check-{i}.txt").write_text(result.stdout+result.stderr,encoding="utf-8",newline="\n")
    record = dict(directory=location, command=" ".join(command), exit=result.returncode,
        status="PASS" if result.returncode == 0 else "FAIL", log=f"check-{i}.txt")
    if i == 4:
        record["qualification"] = "not run: wasm32-unknown-unknown target absent; invocation fails E0463"
    records.append(record)
    (OUT / "checks.json").write_text(json.dumps(records,indent=2)+"\n",encoding="utf-8",newline="\n")
    print(record,flush=True)
after = hashlib.sha256((REPO / "Cargo.lock").read_bytes()).hexdigest()
assert before == after
(OUT / "root-lock-sha256.txt").write_text(before+"\n",encoding="utf-8",newline="\n")
