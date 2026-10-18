# SPDX-License-Identifier: MPL-2.0
# SPDX-FileCopyrightText: 2026 Apibim
# Contributor-original test oracle. It uses only public pxr API and no BeamCraft code.
"""Reads a BeamCraft revision with a reference wrapper using the OpenUSD reference implementation.

Usage: reference_oracle.py ROOT_LAYER_PATH EXPECTATION_JSON

The directory of ROOT_LAYER_PATH holds the layers named as in the revision manifest, the foreign
layer among them under external/. The script checks, with pxr only, that the foreign model
composes below the owned wrapper, that the wrapper's placement and units correction place the
foreign points where the closed form says, that no owned layer holds an opinion below the arc
holder, and that the foreign layer still declares what it always declared. It exits 0 and prints
one JSON report only if every check passes.
"""
import json
import sys

from pxr import Gf, Sdf, Usd, UsdGeom, UsdUtils


def main():
    root_path, expect_path = sys.argv[1], sys.argv[2]
    expect = json.load(open(expect_path))
    failures = []

    def check(condition, message):
        if not condition:
            failures.append(message)

    stage = Usd.Stage.Open(root_path)
    check(stage is not None, "pxr could not open the root layer")
    if stage is None:
        print("\n".join(failures), file=sys.stderr)
        return 1
    report = {"usd_version": ".".join(str(v) for v in Usd.GetVersion())}

    check(abs(UsdGeom.GetStageMetersPerUnit(stage) - 0.001) < 1e-12, "stage is not millimetres")
    check(UsdGeom.GetStageUpAxis(stage) == "Z", "stage is not Z up")

    wrapper_path = "/World/Steel/E_" + expect["id"].replace("-", "")
    holder_path = wrapper_path + "/Correction/Source"
    wrapper = stage.GetPrimAtPath(wrapper_path)
    check(wrapper.IsValid(), "missing wrapper " + wrapper_path)
    check(wrapper.GetAttribute("bim:elementKind").Get() == "reference", "wrapper kind differs")
    check(wrapper.GetAttribute("bim:id").Get() == expect["id"], "bim:id differs")
    check(wrapper.GetDisplayName() == expect["name"], "displayName differs")
    check(
        wrapper.GetAttribute("bim:reference:sourceDigest").Get() == expect["digest"],
        "the pinned digest differs",
    )
    holder = stage.GetPrimAtPath(holder_path)
    check(holder.IsValid(), "missing arc holder")
    check(holder.GetTypeName() == "Xform", "the foreign root type did not show through")

    # The arc holder composes exactly two specs: our arc and the foreign root prim.
    stack = holder.GetPrimStack()
    layers = [spec.layer.identifier for spec in stack]
    check(len(stack) == 2, "arc holder has %d specs" % len(stack))
    foreign_specs = [s for s in stack if "external" in s.layer.identifier.replace("\\", "/")]
    check(len(foreign_specs) == 1, "no foreign spec in the stack: %r" % layers)

    # The foreign layer is untouched and still declares its own units and up axis.
    foreign_layer = foreign_specs[0].layer if foreign_specs else None
    if foreign_layer is not None:
        check(foreign_layer.pseudoRoot.GetInfo("metersPerUnit") == 1.0, "foreign units changed")
        check(foreign_layer.pseudoRoot.GetInfo("upAxis") == "Y", "foreign up axis changed")
        check(foreign_layer.defaultPrim == "Site", "foreign default prim changed")

    # No owned layer holds an opinion at or below the arc holder, apart from the arc itself.
    for layer in stage.GetLayerStack(includeSessionLayers=False):
        if foreign_layer is not None and layer == foreign_layer:
            continue
        spec = layer.GetPrimAtPath(holder_path)
        if spec is not None:
            check(not spec.properties, "owned layer %s has properties on the arc holder" % layer.identifier)
            check(not spec.nameChildren, "owned layer %s has children below the arc holder" % layer.identifier)
            check(not spec.typeName, "owned layer %s sets a type on the arc holder" % layer.identifier)
            check(spec.kind == "", "owned layer %s sets a kind on the arc holder" % layer.identifier)

    # The note the source declares is read through the wrapper.
    note = stage.GetPrimAtPath(holder_path + "/Block").GetAttribute("note").Get()
    check(note == "declared by the source", "foreign attribute differs: %r" % (note,))

    # Placement and correction place the foreign points where the closed form says.
    body = stage.GetPrimAtPath(holder_path + "/Block/Body")
    check(body.IsValid(), "missing foreign mesh")
    points = list(UsdGeom.Mesh(body).GetPointsAttr().Get())
    xf = UsdGeom.Xformable(body).ComputeLocalToWorldTransform(Usd.TimeCode.Default())
    world = [xf.Transform(Gf.Vec3d(p)) for p in points]
    expected = expect["world_points"]
    check(len(world) == len(expected), "point count differs")
    worst = 0.0
    for got, want in zip(world, expected):
        delta = max(abs(got[i] - want[i]) for i in range(3))
        worst = max(worst, delta)
    check(worst <= 1e-6, "world points differ by %g" % worst)
    report["worst_point_error_mm"] = worst

    cache = UsdGeom.BBoxCache(Usd.TimeCode.Default(), [UsdGeom.Tokens.default_])
    box = cache.ComputeWorldBound(wrapper).ComputeAlignedRange()
    lo, hi = box.GetMin(), box.GetMax()
    check(
        all(abs(lo[i] - expect["world_min"][i]) <= 1e-3 for i in range(3))
        and all(abs(hi[i] - expect["world_max"][i]) <= 1e-3 for i in range(3)),
        "world bound differs: %r %r" % (tuple(lo), tuple(hi)),
    )
    report["world_bound_mm"] = [list(lo), list(hi)]

    checker = UsdUtils.ComplianceChecker(
        arkit=False, skipARKitRootLayerCheck=True, rootPackageOnly=False,
        skipVariants=False, verbose=False)
    checker.CheckCompliance(root_path)
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
