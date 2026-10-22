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


def check_standard_metadata(stage, prim, name, mesh, points, counts, indices, check):
    """U09: model kinds, display name, authored extent and flat normals, as plain USD."""
    world = Usd.ModelAPI(stage.GetPrimAtPath("/World"))
    steel = Usd.ModelAPI(stage.GetPrimAtPath("/World/Steel"))
    element = Usd.ModelAPI(prim)
    geometry = Usd.ModelAPI(mesh.GetPrim())
    check(world.GetKind() == "assembly", "/World kind is not assembly")
    check(steel.GetKind() == "group", "/World/Steel kind is not group")
    check(element.GetKind() == "component", "element kind is not component")
    check(geometry.GetKind() == "subcomponent", "Geometry kind is not subcomponent")
    check(prim.IsModel() and prim.IsComponent(), "element is not in the model hierarchy")
    check(prim.GetDisplayName() == name, "displayName differs from bim:name")
    extent = mesh.GetExtentAttr().Get()
    computed = UsdGeom.PointBased.ComputeExtent(points)
    check(
        extent is not None and len(extent) == 2
        and all(abs(a - b) <= 1e-3 for e, c in zip(extent, computed) for a, b in zip(e, c)),
        "authored extent %r differs from the computed bounds %r" % (extent, computed),
    )
    normals = mesh.GetNormalsAttr().Get()
    check(mesh.GetNormalsInterpolation() == UsdGeom.Tokens.uniform, "normals are not per face")
    check(normals is not None and len(normals) == len(counts), "normals count differs from faces")
    if normals is not None and len(normals) == len(counts):
        cursor = 0
        for normal, count in zip(normals, counts):
            face = [Gf.Vec3d(points[i]) for i in indices[cursor:cursor + count]]
            cursor += count
            newell = Gf.Cross(face[1] - face[0], face[2] - face[0])
            if newell.GetLength() > 0.0:
                check(Gf.Dot(Gf.Vec3d(normal), newell.GetNormalized()) > 0.999, "a face normal is not outward")


def check_member(stage, member, check, report):
    """A beam or column: authored inputs, pinned profile, closed-form volume, bounds, mass."""
    path = "/World/Steel/Parts/PRT_" + member["id"].replace("-", "")
    prim = stage.GetPrimAtPath(path)
    check(prim.IsValid(), "missing prim " + path)
    if not prim.IsValid():
        return
    check(prim.GetTypeName() == "Xform", "member is not an Xform")
    check(prim.GetAttribute("bim:id").Get() == member["id"], "bim:id is not the full UUID")
    check(prim.GetAttribute("bim:name").Get() == member["name"], "bim:name differs")
    check(prim.GetAttribute("bim:elementKind").Get() == member["kind"], "bim:elementKind differs")
    check(prim.GetAttribute("bim:geometryMode").Get() == "parametric", "geometryMode differs")
    axis = [tuple(p) for p in prim.GetAttribute("bim:axisPointsMm").Get()]
    check(axis == [tuple(p) for p in member["axis"]], "axis differs: %r" % axis)
    roll = prim.GetAttribute("steel:member:rollRad")
    if member["roll"] is not None:
        check(roll.IsValid() and roll.Get() == member["roll"], "roll differs")
    profile = prim.GetRelationship("bim:profile")
    targets = [t.pathString for t in profile.GetTargets()] if profile else []
    profile_path = "/World/Definitions/Profiles/PRF_" + member["profile"]
    check(targets == [profile_path], "profile target differs: %r" % targets)
    definition = stage.GetPrimAtPath(profile_path)
    check(definition.IsValid(), "missing profile definition prim")
    if definition.IsValid():
        check(definition.GetAttribute("bim:definitionKind").Get() == "rectangle", "profile kind differs")
        check(definition.GetAttribute("bim:units").Get() == "mm", "profile units differ")
        check(definition.GetAttribute("steel:profile:widthMm").Get() == member["width"], "profile width differs")
        check(definition.GetAttribute("steel:profile:heightMm").Get() == member["height"], "profile height differs")
    mesh = UsdGeom.Mesh(stage.GetPrimAtPath(path + "/Geometry"))
    check(bool(mesh), "Geometry is not a UsdGeom.Mesh")
    points = mesh.GetPointsAttr().Get()
    counts = mesh.GetFaceVertexCountsAttr().Get()
    indices = mesh.GetFaceVertexIndicesAttr().Get()
    check(len(points) == 8 and len(counts) == 12 and len(indices) == 36, "mesh counts differ")
    check(mesh.GetOrientationAttr().Get() == UsdGeom.Tokens.rightHanded, "orientation differs")
    check(mesh.GetSubdivisionSchemeAttr().Get() == UsdGeom.Tokens.none, "subdivision differs")
    check_standard_metadata(stage, prim, member["name"], mesh, points, counts, indices, check)
    volume = signed_volume(points, counts, indices)
    check(volume > 0.0, "mesh is not outward oriented (negative volume)")
    check(close(volume, member["volume_mm3"]), "pxr-read volume %r != closed form %r" % (volume, member["volume_mm3"]))
    check(close(volume, member["cli_volume_mm3"]), "pxr-read volume differs from the CLI output")
    lo = [min(p[i] for p in points) for i in range(3)]
    hi = [max(p[i] for p in points) for i in range(3)]
    check(
        all(abs(a - b) <= 1e-3 for a, b in zip(lo, member["bounds_min"]))
        and all(abs(a - b) <= 1e-3 for a, b in zip(hi, member["bounds_max"])),
        "pxr bounds differ: %r %r" % (lo, hi),
    )
    if member["density"] is not None:
        mass = (volume * member["density"]) / 1e9
        check(close(mass, member["cli_mass_kg"]), "pxr-derived mass %r != CLI mass %r" % (mass, member["cli_mass_kg"]))
    report["plates"].append({"id": member["id"], "volume_mm3": volume, "kind": member["kind"]})


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
        if plate.get("kind") in ("beam", "column"):
            check_member(stage, plate, check, report)
            continue
        path = "/World/Steel/Parts/PRT_" + plate["id"].replace("-", "")
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
        expected_contour = plate.get("contour", [(0.0, 0.0, 0.0), (w, 0.0, 0.0), (w, l, 0.0), (0.0, l, 0.0)])
        check(contour == [tuple(p) for p in expected_contour], "contour differs")

        if plate.get("unsupported"):
            check(not stage.GetPrimAtPath(path + "/Geometry").IsValid(), "unsupported plate has geometry")
            check(not any(p.GetTypeName() == "Mesh" for p in Usd.PrimRange(prim)), "unsupported plate has a mesh descendant")
            for attr in prim.GetAttributes():
                check(not attr.GetName().startswith(("bim:shape", "bim:volume", "bim:mass", "bim:quantity")), "unsupported plate has derived values")
            report["plates"].append({"id": plate["id"], "geometry_status": "unsupported"})
            continue

        mesh = UsdGeom.Mesh(stage.GetPrimAtPath(path + "/Geometry"))
        check(bool(mesh), "Geometry is not a UsdGeom.Mesh")
        points = mesh.GetPointsAttr().Get()
        if "vertices" in plate:
            check([tuple(p) for p in points] == [tuple(p) for p in plate["vertices"]], "framed mesh vertex order differs")
        counts = mesh.GetFaceVertexCountsAttr().Get()
        indices = mesh.GetFaceVertexIndicesAttr().Get()
        holes = plate.get("hole_counts")
        if holes is None:
            check(len(points) == 8 and len(counts) == 12 and len(indices) == 36, "mesh counts differ")
        else:
            # A plate with through holes: the authored polygons are plain USD arrays, the mesh is
            # a larger closed triangle mesh and the volume is the closed form of the polygons.
            hole_counts = list(prim.GetAttribute("steel:plate:holeVertexCounts").Get())
            hole_points = prim.GetAttribute("steel:plate:holePointsMm").Get()
            check(hole_counts == holes, "hole vertex counts differ: %r" % hole_counts)
            check(len(hole_points) == sum(holes), "hole point count differs")
            check(all(tuple(p)[2] == 0.0 for p in hole_points), "a hole point is not in the XY plane")
            check(all(c == 3 for c in counts) and len(indices) == 3 * len(counts), "mesh is not a triangle mesh")
            check(len(counts) > 12, "a plate with holes has more than 12 triangles")
        check(mesh.GetOrientationAttr().Get() == UsdGeom.Tokens.rightHanded, "orientation differs")
        check(mesh.GetSubdivisionSchemeAttr().Get() == UsdGeom.Tokens.none, "subdivision differs")
        check_standard_metadata(stage, prim, plate["name"], mesh, points, counts, indices, check)
        volume = signed_volume(points, counts, indices)
        expected = plate.get("expected_volume_mm3", w * l * plate["thickness"])
        check(volume > 0.0, "mesh is not outward oriented (negative volume)")
        check(close(volume, expected), "pxr-read volume %r != closed form %r" % (volume, expected))
        check(close(volume, plate["cli_volume_mm3"]), "pxr-read volume differs from the CLI output")
        # The stored quantity record beside the mesh is a plain USD attribute set.
        stored_volume = mesh.GetPrim().GetAttribute("bim:volumeMm3")
        check(stored_volume.IsValid() and close(stored_volume.Get(), plate["cli_volume_mm3"]), "stored volume differs")
        check(mesh.GetPrim().GetAttribute("bim:volumeStatus").Get() == "current", "stored volume status differs")
        check(len(mesh.GetPrim().GetAttribute("bim:shapeSignature").Get()) == 64, "shape signature is not a sha256")
        if plate.get("cli_mass_kg") is not None:
            stored_mass = mesh.GetPrim().GetAttribute("bim:massKg")
            check(stored_mass.IsValid() and close(stored_mass.Get(), plate["cli_mass_kg"]), "stored mass differs")
        elif "cli_mass_kg" in plate:
            check(not mesh.GetPrim().GetAttribute("bim:massKg").IsValid(), "unknown mass has a stored value")

        # Material: one pinned immutable definition, or no target at all (never a guessed default).
        relationship = prim.GetRelationship("bim:material")
        targets = [t.pathString for t in relationship.GetTargets()] if relationship else []
        material = plate.get("material")
        if material is None:
            check(targets == [], "plate without a material has targets %r" % targets)
        else:
            definition_path = "/World/Definitions/Materials/MAT_" + material["digest"]
            check(targets == [definition_path], "material target differs: %r" % targets)
            definition = stage.GetPrimAtPath(definition_path)
            check(definition.IsValid(), "missing definition prim " + definition_path)
            if definition.IsValid():
                check(definition.GetAttribute("bim:units").Get() == "mm", "definition units differ")
                check(definition.GetAttribute("bim:schemaVersion").Get() == "0.1.0", "definition version differs")
                check(definition.GetAttribute("bim:definitionKind").Get() == "material", "definition kind differs")
                check(definition.GetAttribute("bim:contentDigest").Get() == material["digest"], "definition digest label differs")
                density = definition.GetAttribute("bim:densityKgM3")
                if material["density"] is None:
                    check(not density.IsValid() or density.Get() is None, "unknown density has a value")
                    check(plate["cli_mass_kg"] is None, "CLI reports a mass for an unknown density")
                else:
                    check(density.IsValid() and density.Get() == material["density"], "density differs")
                    # Mass from the pxr-read volume (mm3) and density, in the order of the F06 contract.
                    mass = (volume * density.Get()) / 1e9
                    check(close(mass, plate["cli_mass_kg"]), "pxr-derived mass %r != CLI mass %r" % (mass, plate["cli_mass_kg"]))

        # Placement as a pxr reader composes it.
        transform = UsdGeom.Xformable(prim).ComputeLocalToWorldTransform(Usd.TimeCode.Default())
        tx, ty, tz = plate["translation"]
        check(tuple(transform.ExtractTranslation()) == (tx, ty, tz), "composed translation differs")
        world = cache.ComputeWorldBound(prim).ComputeAlignedRange()
        lo, hi = world.GetMin(), world.GetMax()
        bounds = plate.get("local_bounds", [[0, 0, 0], [w, l, plate["thickness"]]])
        check(
            all(abs(a - (b + t)) <= 1e-3 for a, b, t in zip(lo, bounds[0], (tx, ty, tz)))
            and all(abs(a - (b + t)) <= 1e-3 for a, b, t in zip(hi, bounds[1], (tx, ty, tz))),
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
