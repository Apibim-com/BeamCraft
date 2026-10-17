# SPDX-License-Identifier: MPL-2.0
# SPDX-FileCopyrightText: 2026 Apibim
# Contributor-original test oracle. It uses only public pxr API and no BeamCraft code.
"""Reads a BeamCraft revision with the OpenUSD reference implementation and checks it.

Usage: pxr_oracle.py ROOT_LAYER_PATH EXPECTATION_JSON

The directory of ROOT_LAYER_PATH holds the layers named as in the revision manifest
(root.usda, authored.usda, derived.usda). EXPECTATION_JSON is a list of objects with the
closed-form numbers of each plate. The script exits 0 and prints one JSON report only if every
check passes; otherwise it prints the failures to stderr and exits 1.
"""
import json
import sys

from pxr import Gf, Usd, UsdGeom, UsdUtils


def signed_volume(points, counts, indices):
    """Divergence-theorem volume of a polygon mesh from the points pxr returned."""
    total = 0.0
    cursor = 0
    for count in counts:
        face = [Gf.Vec3d(points[i]) for i in indices[cursor:cursor + count]]
        cursor += count
        for k in range(1, count - 1):
            total += Gf.Dot(face[0], Gf.Cross(face[k], face[k + 1])) / 6.0
    return total


def close(actual, expected, rel=1e-6):
    return abs(actual - expected) <= rel * abs(expected)


def main():
    root, expectation = sys.argv[1], sys.argv[2]
    with open(expectation, encoding="utf-8") as handle:
        plates = json.load(handle)
    failures = []

    def check(condition, message):
        if not condition:
            failures.append(message)

    stage = Usd.Stage.Open(root)
    check(stage is not None, "stage did not open")
    if stage is None:
        print("\n".join(failures), file=sys.stderr)
        return 1
    check(UsdGeom.GetStageMetersPerUnit(stage) == 0.001, "metersPerUnit is not 0.001")
    check(UsdGeom.GetStageUpAxis(stage) == UsdGeom.Tokens.z, "upAxis is not Z")
    check(stage.GetDefaultPrim().GetPath().pathString == "/World", "defaultPrim is not /World")
    layers = [layer.identifier for layer in stage.GetLayerStack() if not layer.anonymous]
    check(len(layers) == 3, "expected root + authored + derived, got %r" % layers)

    report = {"usd_version": ".".join(str(v) for v in Usd.GetVersion()), "plates": []}
    cache = UsdGeom.BBoxCache(Usd.TimeCode.Default(), [UsdGeom.Tokens.default_])
    for plate in plates:
        path = "/World/Steel/E_" + plate["id"].replace("-", "")
        prim = stage.GetPrimAtPath(path)
        check(prim.IsValid(), "missing prim " + path)
        if not prim.IsValid():
            continue
        check(prim.GetTypeName() == "Xform", "element is not an Xform")
        check(prim.GetAttribute("bim:id").Get() == plate["id"], "bim:id is not the full UUID")
        check(prim.GetAttribute("bim:name").Get() == plate["name"], "bim:name differs")
        check(prim.GetAttribute("bim:elementKind").Get() == "plate", "bim:elementKind differs")
        check(prim.GetAttribute("bim:geometryMode").Get() == "parametric", "geometryMode differs")
        check(prim.GetAttribute("bim:schemaVersion").Get() == "0.1.0", "schemaVersion differs")
        check(prim.GetAttribute("steel:plate:thicknessMm").Get() == plate["thickness"], "thickness differs")
        contour = [tuple(p) for p in prim.GetAttribute("steel:plate:contourPointsMm").Get()]
        w, l = plate["width"], plate["length"]
        check(contour == [(0.0, 0.0, 0.0), (w, 0.0, 0.0), (w, l, 0.0), (0.0, l, 0.0)], "contour differs")

        mesh = UsdGeom.Mesh(stage.GetPrimAtPath(path + "/Geometry"))
        check(bool(mesh), "Geometry is not a UsdGeom.Mesh")
        points = mesh.GetPointsAttr().Get()
        counts = mesh.GetFaceVertexCountsAttr().Get()
        indices = mesh.GetFaceVertexIndicesAttr().Get()
        check(len(points) == 8 and len(counts) == 12 and len(indices) == 36, "mesh counts differ")
        check(mesh.GetOrientationAttr().Get() == UsdGeom.Tokens.rightHanded, "orientation differs")
        check(mesh.GetSubdivisionSchemeAttr().Get() == UsdGeom.Tokens.none, "subdivision differs")
        volume = signed_volume(points, counts, indices)
        expected = w * l * plate["thickness"]
        check(volume > 0.0, "mesh is not outward oriented (negative volume)")
        check(close(volume, expected), "pxr-read volume %r != closed form %r" % (volume, expected))
        check(close(volume, plate["cli_volume_mm3"]), "pxr-read volume differs from the CLI output")

        # Placement as a pxr reader composes it.
        transform = UsdGeom.Xformable(prim).ComputeLocalToWorldTransform(Usd.TimeCode.Default())
        tx, ty, tz = plate["translation"]
        check(tuple(transform.ExtractTranslation()) == (tx, ty, tz), "composed translation differs")
        world = cache.ComputeWorldBound(prim).ComputeAlignedRange()
        lo, hi = world.GetMin(), world.GetMax()
        check(
            all(abs(a - b) <= 1e-3 for a, b in zip(lo, (tx, ty, tz)))
            and all(abs(a - b) <= 1e-3 for a, b in zip(hi, (tx + w, ty + l, tz + plate["thickness"]))),
            "pxr world bound differs: %r %r" % (tuple(lo), tuple(hi)),
        )
        report["plates"].append({
            "id": plate["id"],
            "volume_mm3": volume,
            "extent_authored": mesh.GetExtentAttr().HasAuthoredValue(),
        })

    checker = UsdUtils.ComplianceChecker(
        arkit=False, skipARKitRootLayerCheck=True, rootPackageOnly=False,
        skipVariants=False, verbose=False)
    checker.CheckCompliance(root)
    report["compliance"] = {
        "checker": "UsdUtils.ComplianceChecker (default rules, ARKit off)",
        "errors": list(checker.GetErrors()),
        "failed_checks": list(checker.GetFailedChecks()),
        "warnings": list(checker.GetWarnings()),
    }
    if failures:
        print("\n".join(failures), file=sys.stderr)
        print(json.dumps(report), file=sys.stdout)
        return 1
    print(json.dumps(report))
    return 0


if __name__ == "__main__":
    sys.exit(main())
