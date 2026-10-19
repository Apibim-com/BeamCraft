# A02 first slice: contour plates

Date: 2026-10-19. Status: **experimental vertical slice; A02 acceptance is NOT claimed.**
Branch `feat/a02-contour-plates`, based on `origin/master` `11a2139` (A07.2 merged).

Plan row: A02 / contour plates, MVP scope item 1 (with A07). Result: a plate whose outline is an
arbitrary closed contour instead of the origin rectangle only. Acceptance: API/CLI end to end;
no editing of an outline after creation; exact polygon quantities.

## What is implemented

```text
beamcraft create-plate --store ROOT (--width MM --length MM | --outer X,Y;X,Y;...) --thickness MM ...
  origin rectangle --> the unchanged G02 evaluation (existing meshes and digests stay valid)
  --outer contour --> kernel G05 outline admission + extrusion along local +Z by the thickness
                  --> G06 certification of one closed shell (Body::from_extrusion)
                  --> A04 through holes cut by G09 exactly as on the rectangle
  --> U02 authored plate (steel:plate:contourPointsMm, unchanged schema) + derived mesh
  --> host commit; the verifier re-evaluates every plate and compares the committed mesh
```

| Area | What exists |
|---|---|
| `beamcraft_kernel` | Nothing new: the slice reuses the A07.1 outline handling — G05 `extrude` (winding, simplicity, floors, caps), the G06 `Body::from_extrusion` certification and the G09 `through_hole` cutters. |
| `beamcraft_engine` | `evaluate_plate` no longer refuses a non-rectangle contour with `UNSUPPORTED_GEOMETRY`: it builds the contour plate. `SolidMesh::from_body` seals any certified body. New algorithm label `kernel-g05-contour-plate-volume/1;tolerance-policy-3` for a plain contour plate; a holed one keeps the G09 label. |
| `beamcraft_cli` | `create-plate --outer "X,Y;X,Y;..."` (counter-clockwise, element-local millimetres, the same token and parser as `define-profile --family custom --outer`); `--outer` and `--width/--length` are mutually exclusive ways of one outline. |

## Decisions

- **No new kernel surface.** The outline handling added for section profiles (A07.1) already does
  everything a contour plate needs: the G05 admission decides winding (outer counter-clockwise),
  simplicity, backtracking, narrow features and the area floors; the extrusion is exact for the
  polygon; G06 certifies the shell. The engine composes these three pieces; the kernel is
  unchanged, so the section and plate paths cannot drift apart.
- **The origin rectangle keeps its G02 evaluation.** Classification of the exact origin
  rectangle is unchanged, so existing stored models re-verify with byte-identical meshes and
  digests. Every other closed outline, including a clockwise mirror rectangle, goes through the
  G05 admission (which refuses clockwise winding as `INVALID_ARGUMENT`, like the old dedicated
  refusal did).
- **The contour stays in the plate body frame.** Contour points with a nonzero `z` are refused
  as `INVALID_ARGUMENT`: the plate-local frame is the XY plane and a translation belongs to the
  placement, never to the outline. (The former F04-POS-18 shape, a rectangle at constant
  `z = 30`, is therefore refused as invalid input instead of `unsupported`; the former
  F04-POS-20 pentagon is now evaluated.)
- **Volume is exact for the polygon.** A plain contour plate reports the analytic
  `net_area * thickness` of the G05 extrusion (the shoelace area of the stored outline), under
  the new algorithm label. With holes it is the midpoint of the G06 enclosure of the certified
  cut result, as on the rectangle. As with the sections, a curve is only ever a polygon the
  author entered; nothing is approximated here that was not already a polygon.
- **`width`/`length` in reports are the bounding box.** For a contour plate the reported
  `dimensions_mm` of `plate`/`plate-inspect` are the bounding box of the outline; the rectangle
  reports its sides as before.
- **Storage needs no schema change.** The plate already authors its full contour as
  `steel:plate:contourPointsMm`; holes, quantity records, footprints, replay (`op_id`), copy and
  delete work unchanged, because they read the authored inputs and re-evaluate them.
- **The F04 conformance harness moved with the behaviour.** The former P01 deviations
  F04-POS-19/20/25/26 (accepted by F04) now pass, and F04-NEG-29/30/38 return F04's
  `INVALID_ARGUMENT`. The declared deviations are F04-POS-18 and F04-POS-28 (a contour at a
  constant nonzero `z`, which F04 accepts in its own frame and A02 refuses as
  `INVALID_ARGUMENT`).

## Not done (not claimed)

* No outline editing command (`update-element` changes name, material, thickness, placement and
  level, not the contour); no fillets, chamfers or curves on a plate outline; no cavity contours
  on a plate (holes are cut through, not modelled as inner loops).
* A02 acceptance is not claimed: rotated placements stay `UNSUPPORTED_GEOMETRY`, and the
  IFC/BOM path does not distinguish contour plates yet (E02/E03).
* The slice is experimental, like the other merged slices; no gate acceptance is claimed.

## Evidence

* Engine tests (`crates/engine/src/command.rs`): a right triangle evaluates with the exact
  volume `600,000 mm3` and the 8-triangle prism mesh; the former pentagon case evaluates; a
  clockwise outline, a bowtie, off-plane points, a two-point contour and 1025 points are refused
  with their F04 codes; a contour plate takes a through hole with the exact volume `575,000 mm3`
  and refuses a hole outside the outline; the rectangle tests (volumes, codes, hostile contours)
  pass unchanged.
* The F04 conformance harness (`apps/beamcraft-cli/tests/f04_cases.rs`) drives every decidable
  `CreatePlate` case through the real admission: 35 cases now conform (the former deviations
  POS-19/20/25/26 and NEG-29/30/38 included) and 2 are declared (POS-18, POS-28).
* End-to-end tests (`apps/beamcraft-cli/tests/contours.rs`, real binary): `create-plate
  --outer` publishes a revision whose plate lists back with the exact closed-form volume and
  mass at 7850 kg/m3, reopens verified in fresh processes with the same mesh digest, appears in
  the view snapshot with its bounding box, survives `update-element --thickness` re-evaluation
  and `copy-element`, and every refusal (clockwise, self-intersecting, two points, off-plane,
  `--outer` together with `--width`, malformed input) writes no revision; a retry replays and
  another contour under the same `op_id` is `OP_ID_REUSE`.

## Roadmap entry (proposed, not applied)

- 2026-10-19: A02 first slice let `create-plate` define a plate by an arbitrary closed outline
  (`--outer X,Y;X,Y;...`) beside the origin rectangle, reusing the A07.1 kernel outline handling
  (G05 admission and extrusion, G06 certification, G09 holes) with exact polygon volumes;
  experimental slice, no outline editing and no A02 acceptance claim.
