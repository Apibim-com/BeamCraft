"""Synthetic corruption controls for the independent measurement harness."""

import struct
import unittest

from oracle import expected, verify_g03
from run import scalar_audit


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


if __name__ == "__main__":
    unittest.main()
