#!/usr/bin/env python3
"""Open one issue when a BeamCraft Actions job waits in the runner queue too long."""

from __future__ import annotations

import json
import os
import sys
from datetime import datetime, timedelta, timezone
from urllib.error import HTTPError
from urllib.parse import quote, urlencode
from urllib.request import Request, urlopen


def fail(message: str) -> "NoReturn":
    print(message, file=sys.stderr)
    raise SystemExit(1)


def main() -> None:
    token = os.environ.get("GITHUB_TOKEN", "")
    repository = os.environ.get("GITHUB_REPOSITORY", "")
    api_url = os.environ.get("GITHUB_API_URL", "https://api.github.com").rstrip("/")
    try:
        threshold = int(os.environ.get("QUEUE_THRESHOLD_MINUTES", "15"))
    except ValueError:
        fail("QUEUE_THRESHOLD_MINUTES must be an integer")
    if not token or repository.count("/") != 1 or threshold < 1:
        fail("GITHUB_TOKEN, GITHUB_REPOSITORY and a positive threshold are required")

    owner, name = repository.split("/", maxsplit=1)
    repo_path = f"/repos/{quote(owner, safe='')}/{quote(name, safe='')}"

    def api(method: str, path: str, payload: dict | None = None) -> object:
        data = None if payload is None else json.dumps(payload).encode("utf-8")
        request = Request(
            f"{api_url}{path}",
            data=data,
            method=method,
            headers={
                "Accept": "application/vnd.github+json",
                "Authorization": f"Bearer {token}",
                "X-GitHub-Api-Version": "2022-11-28",
                "Content-Type": "application/json",
            },
        )
        try:
            with urlopen(request, timeout=20) as response:
                raw = response.read(2_000_001)
                if len(raw) > 2_000_000:
                    fail("GitHub API response exceeded the 2 MB limit")
                return None if not raw else json.loads(raw)
        except HTTPError as error:
            detail = error.read(4096).decode("utf-8", errors="replace")
            fail(f"GitHub API returned HTTP {error.code}: {detail}")

    def pages(path: str) -> list[dict]:
        items: list[dict] = []
        for page in range(1, 6):
            separator = "&" if "?" in path else "?"
            result = api("GET", f"{path}{separator}{urlencode({'per_page': 100, 'page': page})}")
            if not isinstance(result, list):
                fail("GitHub API returned an unexpected list response")
            items.extend(item for item in result if isinstance(item, dict))
            if len(result) < 100:
                return items
        return items

    now = datetime.now(timezone.utc)
    cutoff = now - timedelta(minutes=threshold)
    queued_runs = pages(f"{repo_path}/actions/runs?status=queued")
    stale: list[dict] = []
    for run in queued_runs:
        run_id = run.get("id")
        if not isinstance(run_id, int):
            continue
        jobs = pages(f"{repo_path}/actions/runs/{run_id}/jobs")
        for job in jobs:
            if job.get("status") != "queued" or not isinstance(job.get("id"), int):
                continue
            created = job.get("created_at")
            if not isinstance(created, str):
                continue
            try:
                queued_at = datetime.fromisoformat(created.replace("Z", "+00:00"))
            except ValueError:
                continue
            if queued_at.tzinfo is None:
                continue
            if cutoff - queued_at < timedelta(0):
                continue
            stale.append(
                {
                    "job": str(job.get("name", "unnamed job"))[:200],
                    "run_id": run_id,
                    "ref": str(run.get("head_branch", "unknown ref"))[:200],
                    "queued_at": queued_at.isoformat(),
                    "run_url": str(run.get("html_url", ""))[:500],
                }
            )

    if not stale:
        print("No queued job exceeded the threshold.")
        return

    open_alerts = pages(f"{repo_path}/issues?state=open&labels=ci-queue")
    if any("pull_request" not in item for item in open_alerts):
        print("An open ci-queue issue already exists; skipping duplicate alert.")
        return

    labels = pages(f"{repo_path}/labels")
    if not any(label.get("name") == "ci-queue" for label in labels):
        api(
            "POST",
            f"{repo_path}/labels",
            {"name": "ci-queue", "color": "d93f0b", "description": "CI jobs waiting for hosted runners"},
        )

    refs = sorted({item["ref"] for item in stale})
    title_ref = refs[0] if len(refs) == 1 else f"{refs[0]} (+{len(refs) - 1} refs)"
    lines = [
        f"The following CI jobs have remained queued for at least {threshold} minutes.",
        "",
        "| Ref | Job | Run | Queued since (UTC) |",
        "|---|---|---:|---|",
    ]
    for item in stale:
        lines.append(
            f"| `{item['ref']}` | `{item['job']}` | [#{item['run_id']}]({item['run_url']}) | `{item['queued_at']}` |"
        )
    lines.extend(["", "Close this issue after the queue drains. The watcher will then alert on a later incident."])
    api(
        "POST",
        f"{repo_path}/issues",
        {"title": f"CI runner queue: {title_ref}"[:256], "body": "\n".join(lines), "labels": ["ci-queue"]},
    )
    print(f"Opened one ci-queue issue for {len(stale)} queued job(s).")


if __name__ == "__main__":
    main()
