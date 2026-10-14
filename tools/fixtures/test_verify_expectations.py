#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
# SPDX-FileCopyrightText: 2026 Apibim
"""Mutation tests for verify_expectations.py: the oracle must pass on the committed fixtures
and fail on each deliberate corruption.

Run:  python tools/fixtures/test_verify_expectations.py
"""

from __future__ import annotations

import json
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
ORACLE = HERE / "verify_expectations.py"
FIXTURES = ROOT / "tests" / "fixtures" / "f07"
EXPECTATIONS = ROOT / "tests" / "fixtures" / "expectations" / "f07.json"


class OracleTest(unittest.TestCase):
    def setUp(self):
        self.tmp = Path(tempfile.mkdtemp(prefix="beamcraft-f07-oracle-"))
        self.fixtures = self.tmp / "f07"
        shutil.copytree(FIXTURES, self.fixtures)
        self.expectations = self.tmp / "f07.json"
        shutil.copyfile(EXPECTATIONS, self.expectations)

    def tearDown(self):
        shutil.rmtree(self.tmp, ignore_errors=True)

    def run_oracle(self):
        return subprocess.run(
            [sys.executable, str(ORACLE), "--fixtures", str(self.fixtures),
             "--expectations", str(self.expectations)],
            capture_output=True, text=True, check=False,
        )

    def edit_expectations(self, edit):
        data = json.loads(self.expectations.read_text(encoding="utf-8"))
        edit({c["id"]: c for c in data["cases"]})
        self.expectations.write_text(json.dumps(data), encoding="utf-8")

    def assert_fails(self, *needles):
        result = self.run_oracle()
        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
        for needle in needles:
            self.assertIn(needle, result.stdout)

    def test_committed_fixtures_pass(self):
        result = self.run_oracle()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertTrue(result.stdout.startswith("OK: 33 files, 33 SHA-256 digests"))

    def test_detects_a_tampered_byte(self):
        path = self.fixtures / "valid" / "plate-axis-aligned.json"
        path.write_bytes(path.read_bytes().replace(b"[640, 410, 15]", b"[640, 410, 16]"))
        self.assert_fails("valid/plate-axis-aligned.json: sha256", "volume_mm3")

    def test_detects_an_unlisted_file(self):
        (self.fixtures / "valid" / "extra.json").write_text("{}", encoding="utf-8")
        self.assert_fails("unexpected file valid/extra.json")

    def test_detects_a_wrong_volume(self):
        self.edit_expectations(
            lambda c: c["f07.member.along-x"]["objects"][0].update(volume_mm3=131040001))
        self.assert_fails("f07.member.along-x/member-1.volume_mm3")

    def test_detects_a_wrong_oblique_bound(self):
        def edit(cases):
            cases["f07.member.diagonal-5-12-13"]["objects"][0]["bounds_mm"]["max"][0] += 1e-3
        self.edit_expectations(edit)
        self.assert_fails("f07.member.diagonal-5-12-13/member-1.bounds.max.x")

    def test_unknown_mass_must_stay_null(self):
        self.edit_expectations(
            lambda c: c["f07.plate.rotated-z-30"]["objects"][0].update(mass_kg=0))
        self.assert_fails("unknown must stay null")

    def test_detects_a_wrong_rejection_code(self):
        self.edit_expectations(
            lambda c: c["f07.invalid.member-zero-length"].update(error="non_positive_dimension"))
        self.assert_fails("f07.invalid.member-zero-length: error")

    def test_detects_a_missing_expectation(self):
        data = json.loads(self.expectations.read_text(encoding="utf-8"))
        data["cases"] = [c for c in data["cases"] if c["id"] != "f07.copy.two-plates"]
        self.expectations.write_text(json.dumps(data), encoding="utf-8")
        self.assert_fails("expectation IDs differ from manifest IDs")


if __name__ == "__main__":
    unittest.main(verbosity=2)
