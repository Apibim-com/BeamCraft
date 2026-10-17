# A04 pilot: plate through holes

Date: 2026-10-17. Status: **experimental pilot; A04 acceptance is NOT claimed.**
Branch `feat/a04-plate-holes`, stacked on `feat/p03-straight-member` (PR #64), which is stacked on
`feat/p02-definitions` (PR #63). Merge order: #63, #64, then this PR.

Plan: the G09 kernel hole is the headless API (`through_hole`); its command and end-to-end gate
belong to A04. This slice authors holes together with the plate (`create-plate`), because there is
no edit command yet (P04) and no operation-record vocabulary in the frozen F04 contract.

## What is implemented

```text
beamcraft create-plate ... [--hole-rect "X,Y,W,H;..."] [--hole-round "X,Y,D;..."] [--hole-segments N]
  CLI generators --> clockwise polygons in plate-local XY (the only stored form)
  model::CreatePlate.holes_mm (caps) --> engine: kernel Body::from_plate + one through_hole per hole
  --> U02 authored plate with steel:plate:holePointsMm / steel:plate:holeVertexCounts + derived mesh
  --> host commit; the verifier re-cuts every plate and compares the committed mesh
```

| Area | What exists |
|---|---|
| `beamcraft_model::command` | `CreatePlate.holes_mm`: at most 64 holes of 3 to 1024 points, finite, within the coordinate cap; part of the content fingerprint (so other holes under one `op_id` are `OP_ID_REUSE`). |
| `beamcraft_engine` | `apply_holes` (one kernel cutter per hole, in order, overshoot 1 mm past both faces), `SolidMesh` (the certified mesh of any body), `PlateBody.{solid, hole_count}`, volume algorithm `kernel-g09-through-hole-volume/1;tolerance-policy-3`. `SceneElement` now takes a `SceneMesh`, so the view snapshot accepts any sealed body. |
| `beamcraft_layer_engine::authored` | `Shape::Plate.holes`, schema descriptor amended (new schema digest) with two optional plate attributes: a flat `double3[]` (z = 0) and an `int[]` of vertex counts; validation of both on write and on every load. |
| `beamcraft_cli` | `--hole-rect`, `--hole-round`, `--hole-segments`; `plate_json` reports `hole_count` and `holes_mm`; the plate decoder reads the attributes (inconsistent ones are `INTEGRITY_ERROR`). |

## Decisions (owner review needed)

1. **Contract deviation, recorded.** F04 section 4 says the initial create has no cuts, holes or
   welds and that engineering operations are explicit owned operation records. This pilot stores
   the hole polygons as two plate attributes instead of operation records, authored with the plate,
   and amends the U02 schema descriptor (the schema digest changes; stores written before this
   change are not readable by the new schema). A logical `AddHole`/`UpdateElement` command and
   operation records need an F04 amendment and remain A04/P04 work.
2. **A "round" hole is a polygon.** The CLI generates a regular N-gon (default 32) inscribed in the
   circle; the model stores only the polygon, its volume is exact for that polygon, and the kernel's
   curve approximations (flagged `approximated`, which F06 0.1.0 does not cover) are not used. The
   vertices use the platform sin and cos at the input edge and are authored as the doubles they
   round to; the cross-target reproducibility question stays open for the same reason as for roll.
3. **Volume is an enclosure midpoint.** The kernel certifies the closed shell and gives an
   enclosure of the exact signed-volume sum whose width is rounding only; the quantity is its
   midpoint with status `current`. The mass uses it like any other volume.
4. **The kernel decides geometry.** Strictly inside the outline, clear of other holes, clockwise,
   simple: a refusal is `INVALID_ARGUMENT` for a bad polygon (the kernel's own code otherwise) and
   nothing is authored.

## Not done (not claimed)

* No `AddHole`/edit/remove command, no operation records, no hole features in the FeatureId table
  (the wall ids are the kernel's provisional `cut:f.h<ordinal>.s<k>`), no stored quantities.
* Holes only in the origin rectangle plate; no member holes, bolts, slots as curves or plane cuts.
* Placement rotation is still `UNSUPPORTED_GEOMETRY`; the layer engine runs in the CLI process.

## Evidence

* `cargo test -p beamcraft-cli --test holes` (real binary, 5 tests): exact volume
  `(1000*500 - 100*100 - 50*50)*10 = 4,875,000 mm3` and mass 38.26875 kg at 7850 kg/m3, a 64-gon with
  the polygon-area volume and clockwise signed area, saved layer text with the polygon arrays, plate
  with a hole next to a beam in one snapshot, every rejection code, replay and `OP_ID_REUSE`.
* Engine tests: divergence-theorem volume of the returned mesh, refusals (counter-clockwise,
  outside, crossing, touching, overlapping). Layer-engine test: authored arrays, validator and
  invalid input.
* pxr oracle (local, opt-in, usd-core 25.11): reads the hole arrays and the cut mesh, derives the
  volume from its own mesh and matches the closed form (rectangle plus 32-gon), and
  `ComplianceChecker` reports nothing.

## Roadmap entry (proposed, not applied)

- 2026-10-17: A04 pilot let `create-plate` author through holes (`--hole-rect`, `--hole-round`) as
  clockwise polygons cut by the G09 kernel, stored as two plate attributes under an amended U02
  schema (a recorded deviation from the F04 operation-record rule). Exact polygon volume and mass,
  verified at publish and reopen. No hole command, edit or feature ids. A04 is not complete.
