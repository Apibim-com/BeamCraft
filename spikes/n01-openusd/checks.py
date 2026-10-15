"""Run the requested N01 checks offline and retain exact results. MPL-2.0."""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import time

SPIKE = Path(__file__).resolve().parent
REPOSITORY = SPIKE.parent.parent
EXPECTED_LOCK = "7b15824fc67c90a837ad19af9d28e12dea2802797e90f541945ddb8e984497eb"

commands = [
    (SPIKE, ["cargo", "--offline", "fmt", "--all", "--", "--check"]),
    (SPIKE, ["cargo", "clippy", "--offline", "--all-targets", "--", "-D", "warnings"]),
    (SPIKE, ["cargo", "test", "--offline"]),
    (REPOSITORY, ["python", "-c", "import json; json.load(open('docs/design/layer-engine-evidence-plan-v0.1.json', encoding='utf-8')); print('json ok')"]),
    (REPOSITORY, ["cargo", "--offline", "fmt", "--all", "--", "--check"]),
    (REPOSITORY, ["cargo", "clippy", "--workspace", "--all-targets", "--offline", "--", "-D", "warnings"]),
    (REPOSITORY, ["cargo", "test", "--workspace", "--offline"]),
]

if __name__ == "__main__":
    before = hashlib.sha256((REPOSITORY / "Cargo.lock").read_bytes()).hexdigest()
    results = []
    for cwd, command in commands:
        environment = os.environ.copy()
        if cwd == REPOSITORY:
            environment["CARGO_TARGET_DIR"] = str(SPIKE / "target" / "repository")
        start = time.perf_counter()
        result = subprocess.run(command, cwd=cwd, env=environment, capture_output=True, timeout=180)
        item = {"cwd": "spike" if cwd == SPIKE else "repository", "command": command,
                "exit": result.returncode, "seconds": time.perf_counter() - start,
                "stdout": result.stdout.decode("utf-8", errors="replace"),
                "stderr": result.stderr.decode("utf-8", errors="replace")}
        results.append(item)
        print(item["cwd"], " ".join(command), "exit", result.returncode, flush=True)
    after = hashlib.sha256((REPOSITORY / "Cargo.lock").read_bytes()).hexdigest()
    assert before == after == EXPECTED_LOCK, "root lock integrity failure"
    (SPIKE / "artifacts" / "check-results.json").write_text(json.dumps({
        "root_lock_sha256_before": before, "root_lock_sha256_after": after,
        "results": results,
    }, indent=2) + "\n", encoding="utf-8")
    print("root Cargo.lock unchanged:", after)
    raise SystemExit(0 if all(r["exit"] == 0 for r in results) else 1)
