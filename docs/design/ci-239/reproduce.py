# SPDX-License-Identifier: MPL-2.0
# SPDX-FileCopyrightText: 2026 Apibim
"""Reproduce the #239 baseline from the adjacent, source-backed CSV exports."""

import csv
import datetime as dt
import json
import math
import statistics
from collections import Counter, defaultdict
from pathlib import Path


def timestamp(value):
    return dt.datetime.fromisoformat(value.replace("Z", "+00:00"))


def percentile95(values):
    return sorted(values)[math.ceil(len(values) * 0.95) - 1]


def main():
    root = Path(__file__).resolve().parent
    with (root / "runs.csv").open(newline="") as source:
        runs = list(csv.DictReader(source))
    with (root / "jobs.csv").open(newline="") as source:
        jobs = list(csv.DictReader(source))
    by_run = defaultdict(list)
    by_name = defaultdict(list)
    job_ids = set()
    for job in jobs:
        if job["job_id"] in job_ids:
            raise ValueError("duplicate job id")
        job_ids.add(job["job_id"])
        created, started, completed = map(timestamp, (job["created_at"], job["started_at"], job["completed_at"]))
        if not created <= started <= completed:
            raise ValueError("job timestamp order")
        queue = (started - created).total_seconds()
        wall = (completed - started).total_seconds()
        if queue != float(job["queue_s"]) or wall != float(job["wall_s"]):
            raise ValueError("job duration disagrees with timestamps")
        by_run[job["run_id"]].append(job)
        by_name[job["name"]].append(wall)
    run_ids = {run["run_id"] for run in runs}
    if len(run_ids) != len(runs) or run_ids != set(by_run):
        raise ValueError("run/job identity mismatch")
    walls, waiting, execution = [], 0, 0
    last_jobs = Counter()
    expected_names = None
    for run in runs:
        children = by_run[run["run_id"]]
        names = {job["name"] for job in children}
        if len(names) != len(children) or (expected_names is not None and names != expected_names):
            raise ValueError("job matrix changed within baseline")
        expected_names = names
        final = max(children, key=lambda job: timestamp(job["completed_at"]))
        wall = (timestamp(final["completed_at"]) - timestamp(run["created_at"])).total_seconds()
        queue = sum(float(job["queue_s"]) for job in children)
        work = sum(float(job["wall_s"]) for job in children)
        if (wall, queue, work) != (float(run["wall_s"]), float(run["queue_s"]), float(run["execution_s"])):
            raise ValueError("run summary disagrees with raw job timestamps")
        walls.append(wall)
        waiting += queue
        execution += work
        last_jobs[final["name"]] += 1
    created = [timestamp(run["created_at"]) for run in runs]
    window = (max(created) - min(created)).total_seconds() / 86400
    result = {
        "runs": len(runs), "jobs": len(jobs), "window_days": round(window, 4),
        "median_run_wall_s": statistics.median(walls), "p95_run_wall_s": percentile95(walls),
        "queue_share_of_summed_job_time": waiting / (waiting + execution),
        "numeric_target_met": statistics.median(walls) <= 180 and percentile95(walls) <= 300,
        "week_window_present": window >= 7,
        "last_job_counts": dict(last_jobs),
        "jobs_by_median_wall": sorted([
            {"name": name, "samples": len(values), "median_s": statistics.median(values), "p95_s": percentile95(values)}
            for name, values in by_name.items()
        ], key=lambda row: row["median_s"], reverse=True),
    }
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
