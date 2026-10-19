# A02/A02.1 contour plates

Date: 2026-10-19 (fork slice and upstream A02.1, merged 2026-10-20). Status: **experimental
vertical slice; A02 acceptance is NOT claimed.** The fork's A02 (arbitrary closed outline) and
upstream's A02.1 (any simple contour in the plane `z = 0`) described the same feature; the sync
kept the fork's engine shape and took upstream's CLI flag name, numbering outline comparison and
test coverage (see the sync notes in the PR).

Plan row: A02 / contour plates, MVP scope item 1 (with A07). Result: a plate whose outline is an
arbitrary closed contour instead of the origin rectangle only. Acceptance: API/CLI end to end;
no editing of an outline after creation; exact polygon quantities.

## What is implemented

```text
beamcraft create-plate --store ROOT (--width MM --length MM | --contour X,Y;X,Y;...) --thickness MM ...
  origin rectangle --> the unchanged G02 evaluation (existing meshes and digests stay valid)
  --contour --> kernel G05 outline admission + extrusion along local +Z by the thickness
             --> G06 certification of one closed shell (Body::from_extrusion)
             --> A04 through holes cut by G09 exactly as on the rectangle
  --> U02 authored plate (steel:plate:contourPointsMm, unchanged schema) + derived mesh
  --> host commit; the verifier re-evaluates every plate and compares the committed mesh
```

| Contour | Result |
|---|---|
| The origin rectangle `(0,0,0) (w,0,0) (w,l,0) (0,l,0)`, at any cyclic start | The G02 box, unchanged: same mesh bytes, digest, face ids and quantity label as in P01. |
| Any other simple counter-clockwise polygon with every `z = 0` | G05 extrusion along `+Z` by the thickness. |
| Clockwise, self-crossing, touching itself, doubling back, too few points | `INVALID_ARGUMENT` (the kernel decides with exact predicates, F04 P-1). Nothing is reordered, merged or repaired. |
| Any `z` other than `0`, or a `z` that varies | `UNSUPPORTED_GEOMETRY`, as before. |
| A placement rotation other than the identity | `UNSUPPORTED_GEOMETRY`, as before (the view contract carries translations). |

| Area | What exists |
|---|---|
| `beamcraft_kernel` | Nothing new: the slice reuses the A07.1 outline handling — G05 `extrude` (winding, simplicity, floors, caps), the G06 `Body::from_extrusion` certification and the G09 `through_hole` cutters. |
| `beamcraft_engine` | `evaluate_plate` no longer refuses a non-rectangle contour with `UNSUPPORTED_GEOMETRY`: it builds the contour plate. `SolidMesh::from_extrusion` seals a G05 extrusion by its own exact blob and digest, and `PlateBody::is_origin_rectangle` tells the two paths apart. New algorithm label `kernel-g05-contour-plate-volume/1;tolerance-policy-3` for a plain contour plate; a holed one keeps the G09 label. |
| `beamcraft_engine::numbering` | `PartFact.outline`: the outline of a contour plate relative to its bounding-box minimum, compared point by point within the tolerance. Without it two plates with the same bounding box and thickness but different outlines would share a number. |
| `beamcraft_cli` | `create-plate --contour "X,Y;X,Y;..."` (counter-clockwise, element-local millimetres; `--contour` and `--width/--length` are mutually exclusive ways of one outline). `plate-inspect` and `create-plate` report `contour_points_mm` and `shape` (`rectangle` or `contour`). `define-profile --family custom` keeps its own `--outer`, which is a profile ring, not a plate contour. |

## Decisions

- **No new kernel surface.** The outline handling added for section profiles (A07.1) already does
  everything a contour plate needs: the G05 admission decides winding (outer counter-clockwise),
  simplicity, backtracking, narrow features and the area floors; the extrusion is exact for the
  polygon; G06 certifies the shell. The engine composes these three pieces; the kernel is
  unchanged, so the section and plate paths cannot drift apart. The stored mesh is the
  extrusion's own exact blob and digest, and the G05 refusals keep the kernel's diagnosis
  (clockwise winding, self-intersection, area floor, thickness floor), each naming the field it
  refused.
- **Every stored contour body is certified.** The engine admits the extrusion with G06
  (`Body::from_extrusion`) even when no holes follow, so a contour that cannot form one sealed
  solid is refused at authoring time with the kernel's own code, not left to the publish-time
  verifier.
- **The origin rectangle keeps its G02 evaluation, at any cyclic start.** The closure of a
  contour is implicit, so the cycle is rotated onto the origin corner before classification: the
  same closed rectangle keeps its G02 quantities and mesh digest whatever corner it starts at.
  A translated rectangle stays on the contour path, because the mesh must match the authored
  outline and the placement is the vehicle for translations. Existing stored models re-verify
  with byte-identical meshes and digests. A clockwise outline (a clockwise mirror rectangle
  included) is refused as `INVALID_ARGUMENT` by the G05 winding rule, like the old dedicated
  refusal.
- **The contour stays in the plate body frame.** A contour that touches any nonzero `z` is
  valid F04 input in its own body frame, but this slice does not evaluate it:
  `UNSUPPORTED_GEOMETRY`, the code the first slice used, so no valid input gains a new refusal
  class. A translation belongs to the placement; F04's exact planarity tolerance is the G06
  policy, not an argument check here.
- **Volume is exact for the polygon.** A plain contour plate reports the analytic
  `net_area * thickness` of the G05 extrusion (the shoelace area of the stored outline), under
  the new algorithm label. With holes it is the midpoint of the G06 enclosure of the certified
  cut result, as on the rectangle. As with the sections, a curve is only ever a polygon the
  author entered; nothing is approximated here that was not already a polygon.
- **Reports name what they describe.** `plate` and `plate-inspect` carry a `shape` kind
  (`rectangle` or `contour`); a rectangle reports `dimensions_mm`, a contour plate reports
  `bounding_box_mm`, so `width * length * thickness` is never mistaken for the size of an
  arbitrary outline.
- **Storage needs no schema change.** The plate already authors its full contour as
  `steel:plate:contourPointsMm`; holes, quantity records, footprints, replay (`op_id`), copy and
  delete work unchanged, because they read the authored inputs and re-evaluate them.
- **The F04 conformance harness moved with the behaviour.** The former P01 deviations
  F04-POS-19/20/25/26 (accepted by F04) now pass, and F04-NEG-30 returns F04's
  `INVALID_ARGUMENT`. The declared deviations keep the first slice's codes: F04-POS-18/28 and
  F04-NEG-29/38 stay `UNSUPPORTED_GEOMETRY` (off-plane contours are not evaluated, and the
  exact planarity refusal needs the G06 tolerance policy).

## Not done (not claimed)

* No outline editing command (`update-element` changes name, material, thickness, placement and
  level, not the contour); no fillets, chamfers or curves on a plate outline; no cavity contours
  on a plate (holes are cut through, not modelled as inner loops).
* No feature ids for a contour plate: face, edge and vertex ids (V04 `fid1`) exist for box
  bodies only, so `pick` and `snap` list a contour plate as skipped, like a holed plate; defining
  their topology is a later task (the V04 note names A02 for it).
* Numbering treats two outlines as identical only when their vertex lists match point by point
  in the same order from the same start; the same shape written from another corner is a
  separate group. This errs towards not merging.
* A02 acceptance is not claimed: rotated placements stay `UNSUPPORTED_GEOMETRY`, and the BOM
  path does not distinguish contour plates yet (E03; E02 exports them as `IfcPlate`).
* The slice is experimental, like the other merged slices; no gate acceptance is claimed.

## Evidence

* Engine tests (`crates/engine/src/command.rs`): a right triangle evaluates with the exact
  volume `600,000 mm3` and the 8-triangle prism mesh; the former pentagon case evaluates; the
  origin rectangle is recognised at every cyclic start (same G02 label and mesh digest) while a
  translated rectangle stays on the contour path; a clockwise outline, a bowtie, off-plane
  points, a two-point contour and 1025 points are refused with their codes; a contour plate
  takes a through hole with the exact volume `575,000 mm3` and refuses a hole outside the
  outline; the rectangle tests (volumes, codes, hostile contours) pass unchanged.
* Engine numbering tests (`cargo test -p beamcraft-engine numbering`): equal bounding boxes with
  different outlines are not one group.
* The F04 conformance harness (`apps/beamcraft-cli/tests/f04_cases.rs`) drives every decidable
  `CreatePlate` case through the real admission: 33 cases now conform (the former deviations
  POS-19/20/25/26 and NEG-30 included) and 4 are declared (POS-18/28, NEG-29/38).
* End-to-end tests (`apps/beamcraft-cli/tests/contours.rs` and the A02.1 suite
  `apps/beamcraft-cli/tests/contour_plates.rs`, real binary): `create-plate --contour` publishes
  a revision whose plate lists back with the exact closed-form volume and mass at 7850 kg/m3,
  the `contour` shape kind and its `bounding_box_mm`, reopens verified in fresh processes with
  the same mesh digest, survives `update-element --thickness` re-evaluation, `copy-element` and
  `delete-element`, is numbered by outline (two identical L-shapes share a number, the mirrored
  L and the same-bounding-box rectangle do not), exports to IFC as `IfcPlate` with an
  `IfcTriangulatedFaceSet`, and every refusal (clockwise, self-crossing, touching, collinear,
  two points, off-plane, `--contour` together with `--width`, malformed input, a thickness below
  the floor) writes no revision; a trailing `;` is accepted; a retry replays and another contour
  under the same `op_id` is `OP_ID_REUSE`.

## Provenance

The contour handling is the G05 extrusion (ear clipping with exact predicates, described in
`crates/kernel/src/profile`) and G09 hole operation; this slice adds no algorithm. The L-shaped,
mirrored-L, triangular and pentagonal contours in the tests are invented for these tests and
follow no standard or catalogue.

## Roadmap entry (proposed, not applied)

- 2026-10-19: A02/A02.1 let `create-plate` define a plate by any simple closed counter-clockwise
  contour (`--contour X,Y;X,Y;...`) beside the origin rectangle, reusing the A07.1 kernel outline
  handling (G05 admission and extrusion, G06 certification, G09 holes) with exact polygon
  volumes; numbering compares outlines, so equal bounding boxes no longer merge; experimental
  slice, no outline editing and no A02 acceptance claim.
