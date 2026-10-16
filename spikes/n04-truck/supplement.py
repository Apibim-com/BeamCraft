"""Longer-budget probes distinguish incomplete measurements from the 30 s matrix."""

import json
from pathlib import Path
from run import one

ROOT = Path(__file__).resolve().parent
records = []
for profile,name in (("release","pattern"),("debug","hole")):
    # Separate repeat ID prevents overwriting primary raw-byte evidence.
    record = one(profile,"caught",1,99,name,120)
    records.append(record)
    (ROOT / "evidence/supplement.json").write_text(json.dumps(records,indent=2)+"\n",encoding="utf-8",newline="\n")
    print(profile,name,record["status"],record["seconds"],flush=True)
