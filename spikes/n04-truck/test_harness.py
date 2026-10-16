"""Synthetic corruption controls for the independent measurement harness."""

import contextlib
import io
import json
from pathlib import Path
import struct
import tempfile
import unittest
from unittest.mock import patch

from oracle import expected, verify_g03
import run
from run import scalar_audit
from summarize import compare_hashes, comparisons, tables


class MeshAuditControls(unittest.TestCase):
    def mesh(self, faces):
        positions = [(0.,0.,0.),(1.,0.,0.),(0.,1.,0.),(0.,0.,1.)]
        return b"".join(struct.pack("<ddd",*p) for p in positions) + b"".join(struct.pack("<QQQ",*f) for f in faces)

    def test_tetrahedron_and_corruptions(self):
        # First-octant coordinate tetrahedron: V=1*1*1/6; outward faces.
        faces = [(0,2,1),(0,1,3),(0,3,2),(1,2,3)]
        good = scalar_audit(self.mesh(faces),4,4)
        self.assertAlmostEqual(good["volume"],1/6)
        self.assertTrue(good["raw_closed"])
        self.assertEqual(good["raw_chi"],2)
        for mutation in (faces[:-1],faces+[faces[0]],[faces[0][::-1],*faces[1:]]):
            bad = scalar_audit(self.mesh(mutation),4,len(mutation))
            self.assertFalse(bad["raw_closed"])
        with self.assertRaises(AssertionError):
            scalar_audit(self.mesh([*faces[:-1],(1,2,99)]),4,4)

    def test_fixture_references_and_independent_operation_formulas(self):
        self.assertEqual(verify_g03(),6)
        self.assertEqual(expected("plate-cut-vertex")["analytic_volume_mm3"],1000/6)
        self.assertEqual(expected("pattern")["expected_chi"],-6)
        self.assertEqual(expected("crossing-hole")["expected_chi"],2)
        self.assertAlmostEqual(expected("thin-wall")["analytic_volume_mm3"],3.59996)
        self.assertFalse(expected("scaled-below")["f06_dimension_admissible"])


class ReviewRegressions(unittest.TestCase):
    """Use synthetic records only; these tests never invoke truck geometry."""

    def test_default_schedule_writes_separate_profiles(self) -> None:
        def fake_one(profile, mode, threads, repeat, name):
            return dict(profile=profile, mode=mode, threads=threads, repeat=repeat,
                        case=name, status="PASS", seconds=0)

        with tempfile.TemporaryDirectory(dir=run.ROOT / "target") as directory:
            output = Path(directory)
            # Stale files must be cleared even if a run stops before release starts.
            (output / "results-release.json").write_text(
                '[{"stale": true}]\n', encoding="utf-8", newline="\n")
            with patch.object(run, "OUT", output), patch.object(run, "TMP", output), \
                    patch.object(run, "one", side_effect=fake_one), \
                    patch("sys.argv", ["run.py"]), contextlib.redirect_stdout(io.StringIO()):
                run.main()
            for profile in ("debug", "release"):
                records = json.loads((output / f"results-{profile}.json").read_text(encoding="utf-8"))
                self.assertEqual(len(records), 5 * len(run.names))
                self.assertEqual({r["profile"] for r in records}, {profile})
            self.assertFalse((output / "results.json").exists())
            with patch.object(run, "OUT", output), patch.object(run, "TMP", output), \
                    patch.object(run, "one", side_effect=RuntimeError("interrupted")), \
                    patch("sys.argv", ["run.py"]), contextlib.redirect_stdout(io.StringIO()):
                with self.assertRaises(RuntimeError):
                    run.main()
            self.assertEqual(json.loads((output / "results-release.json").read_text(encoding="utf-8")), [])

    def test_unavailable_hashes_are_unmeasured(self) -> None:
        repeats = [("caught", 1, n) for n in range(3)]
        records = [dict(case="contour", profile="debug", mode=mode, threads=threads,
                        repeat=repeat, status="PASS", triangle_sha256="same")
                   for mode, threads, repeat in repeats]
        self.assertTrue(compare_hashes(records, repeats, "triangle_sha256"))
        self.assertIsNone(comparisons(records)["thread triangles"])
        self.assertIsNone(compare_hashes(records[:-1], repeats, "triangle_sha256"))
        records[-1].pop("triangle_sha256")
        records[-1].update(status="FAIL", timeout=True)
        self.assertIsNone(compare_hashes(records, repeats, "triangle_sha256"))
        records[-1]["triangle_sha256"] = "different"
        self.assertFalse(compare_hashes(records, repeats, "triangle_sha256"))
        report = tables(records)
        self.assertIn("| debug | thread triangles | 0/0 | 0 | 1 |", report)
        self.assertIn("| debug | repeat triangles | 0/1 | 1 | 0 |", report)


if __name__ == "__main__":
    unittest.main()
