"""Unit tests for the scheduled CI queue alert's queue detection."""

import unittest
from datetime import datetime, timezone
from urllib.parse import parse_qs, urlparse

from tools.ci_queue_alert import find_stale_jobs


class FindStaleJobsTests(unittest.TestCase):
    def test_checks_queued_and_in_progress_runs_at_threshold(self):
        now = datetime(2026, 10, 26, 12, 0, tzinfo=timezone.utc)
        runs = {
            "queued": [
                {"id": 101, "head_branch": "queued-ref", "html_url": "https://github.com/o/runs/101"}
            ],
            "in_progress": [
                {"id": 202, "head_branch": "active-ref", "html_url": "https://github.com/o/runs/202"}
            ],
        }
        jobs = {
            101: [
                {"id": 1, "name": "exact threshold", "status": "queued",
                 "created_at": "2026-10-26T11:45:00Z"},
                {"id": 2, "name": "recent", "status": "queued",
                 "created_at": "2026-10-26T11:46:00Z"},
                {"id": 3, "name": "already running", "status": "in_progress",
                 "created_at": "2026-10-26T11:00:00Z"},
            ],
            202: [
                {"id": 4, "name": "queued behind dependency", "status": "queued",
                 "created_at": "2026-10-26T11:44:59Z"},
            ],
        }
        requested_statuses = []

        def pages(path):
            if "/actions/runs?" in path:
                status = parse_qs(urlparse(path).query)["status"][0]
                requested_statuses.append(status)
                return runs[status]
            if path.endswith("/actions/runs/101/jobs"):
                return jobs[101]
            if path.endswith("/actions/runs/202/jobs"):
                return jobs[202]
            self.fail(f"Unexpected API path: {path}")

        stale = find_stale_jobs(pages, "/repos/o/r", 15, now)

        self.assertEqual(requested_statuses, ["queued", "in_progress"])
        self.assertEqual([item["run_id"] for item in stale], [101, 202])
        self.assertEqual([item["job"] for item in stale],
                         ["exact threshold", "queued behind dependency"])

    def test_malformed_queued_job_timestamp_fails_loudly(self):
        now = datetime(2026, 10, 26, 12, 0, tzinfo=timezone.utc)

        def pages(path):
            if "/actions/runs?" in path:
                return [{"id": 303}]
            if path.endswith("/actions/runs/303/jobs"):
                return [{"id": 5, "status": "queued", "created_at": "not-a-timestamp"}]
            self.fail(f"Unexpected API path: {path}")

        with self.assertRaises(SystemExit):
            find_stale_jobs(pages, "/repos/o/r", 15, now)


if __name__ == "__main__":
    unittest.main()
