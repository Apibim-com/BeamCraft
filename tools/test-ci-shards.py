#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
# SPDX-FileCopyrightText: 2026 Apibim
"""Tests for tools/ci-shards.py, run as CI runs it: a separate process per call. These check
the plan and the filtersets as text; that the filtersets select every test exactly once is
checked against cargo-nextest by `ci-ok` on every CI run."""

import os
import re
import subprocess
import sys
import tempfile
import unittest

SCRIPT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "ci-shards.py")
REPO = os.path.join(os.path.dirname(SCRIPT), "..")


def junit(cases):
    rows = "".join(
        f'<testsuite name="{b}"><testcase name="{n}" classname="{b}" time="{t}"/></testsuite>'
        for b, n, t in cases
    )
    return f"<testsuites>{rows}</testsuites>"


CASES = (
    # One big binary split by module, with one heavy test; small binaries stay whole.
    [("beamcraft-cli::cli", f"suite{i}::case{j}", 1.5) for i in range(8) for j in range(4)]
    + [("beamcraft-cli::cli", "suite0::slow", 20.0)]
    + [("beamcraft-cli::cli", "every_suite_file_is_a_module_of_this_binary", 0.1)]
    + [("beamcraft-geom", f"tolerance::tests::t{j}", 0.2) for j in range(5)]
    + [("beamcraft-cli::bin/beamcraft", "report::tests::code", 0.1)]
)


class ShardsCase(unittest.TestCase):
    def setUp(self):
        self.dir = tempfile.TemporaryDirectory()
        self.addCleanup(self.dir.cleanup)
        self.report = os.path.join(self.dir.name, "junit.xml")
        with open(self.report, "w", encoding="utf-8") as handle:
            handle.write(junit(CASES))
        self.out = os.path.join(self.dir.name, "plan")

    def run_script(self, *args):
        return subprocess.run(
            [sys.executable, SCRIPT, *args], capture_output=True, text=True, check=False
        )

    def plan(self, shards=3, threads=2, split="10"):
        result = self.run_script(
            "plan", "--shards", str(shards), "--threads", str(threads), "--split", split,
            "--out", self.out, self.report,
        )
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        return result.stdout

    def filters(self, shards):
        out = []
        for k in range(1, shards + 1):
            result = self.run_script("filter", "--dir", self.out, "--shard", str(k), "--shards", str(shards))
            self.assertEqual(result.returncode, 0, result.stderr)
            out.append(result.stdout.strip())
        return out

    def test_the_plan_writes_all_shards_but_the_last(self):
        self.plan(shards=3)
        self.assertEqual(sorted(os.listdir(self.out)), ["1.txt", "2.txt"])

    def test_the_last_shard_is_the_complement_of_the_others(self):
        self.plan(shards=3)
        first, second, last = self.filters(3)
        self.assertEqual(last, f"not (({first}) or ({second}))")

    def test_a_heavy_test_is_its_own_group_and_left_out_of_its_module(self):
        self.plan(shards=3)
        text = " ".join(self.filters(3)[:2]) + " " + self.filters(3)[2]
        self.assertIn("(binary_id(=beamcraft-cli::cli) and test(=suite0::slow))", text)
        self.assertIn("test(/^suite0::/) and not (test(=suite0::slow))", text)

    def test_a_small_binary_stays_whole_and_a_large_one_splits_by_module(self):
        self.plan(shards=3)
        text = " ".join(self.filters(3))
        self.assertNotIn("test(/^tolerance::/)", text)
        self.assertRegex(text, r"binary_id\(=beamcraft-cli::cli\) and test\(/\^suite\d::/\)")

    def test_group_filters_name_exactly_their_tests(self):
        import importlib.util

        sys.dont_write_bytecode = True
        spec = importlib.util.spec_from_file_location("ci_shards", SCRIPT)
        shards = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(shards)
        self.assertEqual(
            shards.group_filter(("b::x", "module", ""), set()),
            "(binary_id(=b::x) and not test(/::/))",
        )
        self.assertEqual(shards.group_filter(("b::x", "binary", ""), set()), "binary_id(=b::x)")
        self.assertEqual(
            shards.group_filter(("b::x", "binary", ""), {"slow"}),
            "(binary_id(=b::x) and not (test(=slow)))",
        )
        self.assertEqual(
            shards.group_filter(("b::x", "=", "m::slow"), set()),
            "(binary_id(=b::x) and test(=m::slow))",
        )

    def test_the_heaviest_group_does_not_share_a_shard_with_more_than_it_needs(self):
        out = self.plan(shards=3, threads=1)
        times = [float(t) for t in re.findall(r"predicted ([0-9.]+) s", out)]
        self.assertEqual(len(times), 3)
        self.assertLessEqual(max(times) - min(times), 20.0)

    def test_a_test_in_several_reports_takes_its_mean_duration(self):
        # 10 s in one run and 4 s in the other is 7 s: below --heavy 8, so not a group of its own.
        def report(path, seconds):
            cases = [(b, n, seconds if n == "suite1::case0" else t) for b, n, t in CASES]
            with open(path, "w", encoding="utf-8") as handle:
                handle.write(junit(cases))
        other = os.path.join(self.dir.name, "other.xml")
        report(self.report, 10.0)
        report(other, 4.0)
        for reports, heavy in (([self.report, other], False), ([self.report], True)):
            result = self.run_script(
                "plan", "--shards", "3", "--threads", "2", "--split", "10",
                "--out", self.out, *reports,
            )
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            named = "test(=suite1::case0)" in " ".join(self.filters(3))
            self.assertEqual(named, heavy, reports)

    def test_a_shard_count_that_does_not_match_the_plan_fails(self):
        self.plan(shards=3)
        result = self.run_script("filter", "--dir", self.out, "--shard", "1", "--shards", "4")
        self.assertEqual(result.returncode, 2)
        self.assertIn("need exactly", result.stderr)

    def test_a_shard_outside_the_range_fails(self):
        self.plan(shards=3)
        result = self.run_script("filter", "--dir", self.out, "--shard", "4", "--shards", "3")
        self.assertEqual(result.returncode, 2)

    def test_the_committed_macos_plan_matches_the_workflow(self):
        directory = os.path.join(REPO, ".config", "ci-shards", "macos")
        with open(os.path.join(REPO, ".github", "workflows", "ci.yml"), encoding="utf-8") as handle:
            workflow = handle.read()
        shards = re.search(r"--dir \.config/ci-shards/macos --shard \$\{\{ matrix\.shard \}\} --shards (\d+)", workflow)
        self.assertIsNotNone(shards, "the macOS shard step no longer reads the committed plan")
        count = int(shards.group(1))
        matrix = re.search(r"test-macos:.*?shard: \[([0-9, ]+)\]", workflow, re.S)
        self.assertEqual([int(x) for x in matrix.group(1).split(",")], list(range(1, count + 1)))
        for k in range(1, count + 1):
            result = self.run_script("filter", "--dir", directory, "--shard", str(k), "--shards", str(count))
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertLess(len(result.stdout), 16_000)


if __name__ == "__main__":
    unittest.main()
