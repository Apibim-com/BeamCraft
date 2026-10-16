"""Record local tools and fixed graph features without network or source inspection."""

import json
import os
from pathlib import Path
import platform
import subprocess

ROOT = Path(__file__).resolve().parent
records = {"platform":platform.platform(), "python":platform.python_version(),
    "offline":True,"target_dir":str(ROOT / "target"),
    "rustflags":os.environ.get("RUSTFLAGS"),"encoded_rustflags":os.environ.get("CARGO_ENCODED_RUSTFLAGS")}
for name in ("Cargo.toml","Cargo.lock"):
    path = ROOT / name
    value = path.read_text(encoding="utf-8")
    path.write_text(value,encoding="utf-8",newline="\n")
for name,command in {
    "rustc":["rustc","-Vv"],"cargo":["cargo","--version"],"clippy":["cargo","clippy","--version"],
    "installed_targets":["rustup","target","list","--installed"],"cargo_deny":["cargo","deny","--version"],
}.items():
    result = subprocess.run(command,capture_output=True,encoding="utf-8",errors="replace",
        env=dict(os.environ,CARGO_NET_OFFLINE="true"))
    records[name] = {"command":command,"exit":result.returncode,"stdout":result.stdout.strip(),"stderr":result.stderr.strip()}
(ROOT / "evidence/environment.json").write_text(json.dumps(records,indent=2)+"\n",encoding="utf-8",newline="\n")
print(records["cargo"]["stdout"],records["clippy"]["stdout"])
