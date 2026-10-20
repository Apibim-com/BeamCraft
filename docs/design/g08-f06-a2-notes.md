# G08-docs: F06 amendment A-2 and the matching F04 text

Date: 2026-10-19. Status: **documentation only; no Rust, Cargo or runtime change.** Branch
`docs/g08-f06-amendment-a2`, base `origin/master` `e2d5df2a71c667dd8a0b92e00a340781aa10b50d`.

This note records what the owner decided on 2026-10-19 (D-1 to D-7), where each decision now sits in
the contracts, what is still a proposal, and the task cards that implement it. It answers finding F3
and precondition 1 of the [G07 acceptance record](g07-kernel-first-scope-acceptance.md) and sets up
precondition 3 (capacity). Behaviour comes only from the repository's own contracts and design
notes and public specifications; nothing from any commercial product is used.

## 1. What changed

| File | Change |
|---|---|
| `docs/contracts/geometry-contract-v0.1.md` (F06) | Amendment A-2 in the errata register (section 12), owner decisions table, amendments log, version decision; new subsection "Operations on a plate or extrusion body" in section 2; three tokens in section 3; FeatureId kinds `extrusion` and `cut` in section 5; volume rule in section 8; edits to the scope list, plate input, stage 4, status text, section 4 (one sentence), section 6, section 7, section 9, section 10 and section 11 |
| `docs/contracts/geometry-contract-cases-v0.1.json` | `F06-NEG-08` and `F06-NEG-39` carry a `superseded` marker; top-level `status` says A-2 is applied. No case added, removed or renumbered; counts stay 112, 59, 53 |
| `docs/contracts/model-and-commands-v0.1.md` (F04) | Operation records (section 4) replacing the A04 attribute deviation; `degenerate` element state (section 4, proposal); commands `AddHole`, `UpdateHole`, `RemoveHole`, `CutByPlane` and notes (section 7); footprints and the failure mapping incl. kernel capacity (section 8); section 3 contour support; header, sections 5, 10, 11 and the log (section 12) |
| `docs/design/g07-kernel-first-scope-acceptance.md` | Dated "Update 2026-10-19" notes in the traceability rows of NEG-39 and NEG-08 and in the state of findings F2 and F3 |
| `docs/design/a04-plate-holes.md` | One dated update paragraph after the decisions |
| `docs/design/g08-f06-a2-notes.md` | This note |

**Version decision.** The F06 and F04 version strings stay `0.1.0`. A-2 is additive, an erratum under
F04 section 11 like A-1: no box input, byte, key preimage, id or code changes, and `algorithm` 3,
`tolerance_policy` 3, `fid1` and `mesh_layout` 1 stay. The one changed decision is the status of a
valid non-rectangle polygon (was `unsupported`, now evaluated); an unsupported result carries no key,
mesh or quantity, so nothing stored is invalidated. A strict reading of F06 section 2 ("a decision
for some admitted input raises `algorithm`") could require `algorithm` 4. **Open for the owner:**
confirm "no bump".

## 2. Decision map

| ID | Decision (owner, 2026-10-19) | Edited sections |
|---|---|---|
| D-1 | F06 covers holes, plane cuts and extrusion of a contour with polygonal cavities; true curved surfaces stay unsupported with typed results | F06 s1 (exclusions, assumption), s2 "Operations", s8 (`unsupported` definition); F06 s12 A-2 row |
| D-2 | Round hole = regular inscribed N-gon; chord deviation <= 0.05 mm from the radius; N >= 16, <= 256, multiple of 4; no `Approximated` status | F06 s2 "Operations" (round holes); F04 s4 (generator parameters, polygon stored); A04 note update |
| D-3 | Certified closed shell: volume = midpoint of the enclosure of the exact signed-volume sum, `current`, width reported; polygonal holes: exact polygon volume | F06 s8 (table, status rules, "Volume of a certified closed shell", closing paragraph), s3 (viewport sentence), s2 table; F04 s5 |
| D-4 | `extrusion` and `cut` FeatureId kinds stable and role-based; `GEO_OP_UNSUPPORTED`, `GEO_OP_NO_EFFECT` accepted with F04 mapping | F06 s5 (kinds, rule 9), s3 (table, status), s9 (`LookupFeature`), s7 (matrix row, shape-key note); F04 s8 (mapping) |
| D-5 | No designed hole limit; measured capacity is a certification performance defect (G10); command layer checks capacity first and refuses with a distinct "kernel capacity exceeded" result, publishing nothing | F06 s2 "Operations" (capacity), s3 (`GEO_KERNEL_CAPACITY`), s10 (row); F04 s4 (capacity before the kernel), s8 (mapping); G07 F2 state |
| D-6 | `F06-NEG-39` and `F06-NEG-08` superseded by A02.1 and A-1 | F06 s11 (note and two table rows), s12; case file markers; G07 rows |
| D-7 | Invalid user-authored part kept as degenerate element without derived data; computed on repair | F04 s4 "Element validity state" (proposal), s5, s7, s8 (footprint); F06 s3 and s8 cross-references |

## 3. Proposals and open details for owner review

Marked "proposal for owner review" in the text; nothing here changes an existing refusal.

1. **`GEO_KERNEL_CAPACITY` -> `LIMIT_EXCEEDED`** with field token `kernel_capacity`. A new GEO token, no
   new F04 code. The alternative is `UNSUPPORTED_GEOMETRY` (what the kernel gives a budget refusal
   today). It must never be `INVALID_ARGUMENT`.
2. **F06 mesh caps versus "no designed limit".** The F06 section 6 caps (4,096 vertices, 8,192
   triangles, 1,024 faces) are hard validator caps, so a very large hole count is still bounded by
   them (`GEO_BUFFER_LIMIT` / `GEO_TOPOLOGY_LIMIT`). Whether to raise those draft caps is open.
3. **Round holes needing N > 256** (diameter above about 1328 mm) are refused `GEO_OP_UNSUPPORTED`
   (`hole.radius`) because no `Approximated` status exists. The alternative, accepting N = 256 with a
   larger deviation, breaks the 0.05 mm rule.
4. **Version bump.** See the version decision above.
5. **Ids.** Local ids stay within the 64-byte qualified form; an `extrusion` cavity edge id with large
   indices can exceed it (66 bytes in the worst case) and the build must then be refused
   (`GEO_TOPOLOGY_LIMIT`). The kernel ordinal limit 999 is an implementation limit. Edges and
   vertices created by operations have no ids in 0.1.x. Shape-key bytes for bodies with operations
   are left to A03/A04 as an erratum.
6. **Quantities of operation bodies.** Only volume (and mass from it) is defined; surface area and
   bounds stay box-only.
7. **Plate frame.** A02.1 evaluates a z = 0 polygon as authored, without the F04 plate-frame
   derivation; F06-POS-48 and POS-49 (rectangle from another corner) keep their frame expectation.
   A-2 does not change the frame rules; the divergence stays recorded in `a02-contour-plates.md`.
8. **Holes in beams.** The owner's G10 acceptance names beam flanges and webs. The kernel's through
   hole needs a prism along the cutter direction (G07 S6), so a flange or web hole needs new kernel
   work. F04 refuses it with `UNSUPPORTED_GEOMETRY` until then.
9. **D-7 details** (F04 s4): `on_invalid_shape` with default `reject`; step-by-step repair of a
   degenerate element; which defects qualify (fewer than 3 points is open); operations on a
   degenerate element (`DEPENDENCY_UNAVAILABLE`); publication of a revision containing one
   (proposal: refuse, code to choose); numbering, IFC and BOM exclusion; `operation_id` as a new
   identity of ElementId form; generator parameters kept beside the authoritative polygon.
10. **Stores written by the A04 pilot** (`steel:plate:holePointsMm` and `holeVertexCounts`) need a
    reviewed migration (F04 s4); owned by the A04 acceptance task.

## 4. Edits recorded but not made

* `docs/contracts/model-command-cases-v0.1.json`: `F04-POS-20` still expects `geometry_status:
  unsupported` for the pentagon and cites `F06-NEG-39`. The file is read by
  `apps/beamcraft-cli/tests/f04_cases.rs`, which declares deviations, so changing the expectation
  belongs with a code change. Needed edit: mark the geometry expectation as evaluated for a z = 0
  polygon, and add cases for the A-2 commands and the degenerate state (A04 acceptance, P-Degenerate).
* `docs/design/kernel-validation-plan-v0.1.json`: row `G04-V07` lists `F06-NEG-08` and `G04-V36`
  lists it as its only reference. Needed edit: mark `F06-NEG-08` superseded in both and replace the
  V36 reference by the A-2 operation cases once A03 writes them. The plan is stale in general (G07
  finding F6); left unchanged.
* New F06 acceptance cases for A-2 (round-hole N, volume enclosure width, `extrusion`/`cut` ids,
  `GEO_OP_*`, `GEO_KERNEL_CAPACITY`) are written by A03, A04 acceptance and G10.

## 5. Task cards

### G10: kernel certification speed-up (owner: new task, depends on this amendment)

Why: G07 finding F2. Certification dominates the cost (about 0.08 to 0.14 ms per triangle) and its
default budgets end it at a few thousand triangles; a plate takes 16 holes of 16 edges, 9 of 32 or 5
of 64, and the CLI two holes of 32 edges. This is a performance defect, not a design limit (D-5).

Scope: make certification work near-linearly with a spatial index so that candidate pairs and exact
fallbacks stop growing quadratically; revise `OpLimits` and `Limits` and the capacity pre-check by
measurement; re-measure the layer-engine stage budget that limits CLI authoring.

Acceptance:

1. Tens of holes on a plate (for example 50 holes of 32 edges, and a larger set up to the F06
   mesh caps) are accepted end to end through kernel and CLI.
2. Tens of holes on a beam flange and web, once the kernel can cut them (open detail 8 above; G10
   states its dependency).
3. Measurements before and after for the G07 workloads (`g07_bench`), on the same machine class, in
   the PR; the speed-up is shown, not assumed.
4. `OpLimits`, `Limits` and the pre-check revised by those measurements; the numbers live in code
   and are labelled "implementation limit"; none is frozen in a contract.
5. No change of results: every existing sealed digest, volume and status is bit-identical (the
   G05/G06/G07 pinned-digest tests pass unchanged); the exact predicates and the certificate are not
   weakened; failures stay typed.
6. Never-crash rules hold; a regression test for any fixed defect.

### P-Degenerate: degenerate elements (implements D-7 after the owner reviews F04 s4)

Scope: the `validity` state; `on_invalid_shape` on `CreatePlate`, `CreateStraightMember` and
`UpdateElement`; storage of a degenerate element without derived data; repair path that evaluates
geometry and records volume and quantities as for a new element; `InspectElement` and list/snapshot
output; exclusion from numbering, IFC export and BOM with counts and `unknown` quantities; undo and
redo; the publication rule.

Acceptance: end-to-end CLI tests that (a) a clockwise or self-intersecting contour is kept with
`unknown` quantities and no mesh, (b) editing the points makes it valid and gives the same volume,
mass, keys and FeatureIds as creating the repaired part directly, (c) undo returns to the degenerate
state without stale data, (d) numbering, IFC and BOM exclude it with a report, (e) the default
`reject` behaviour and every existing refusal test are unchanged, (f) no store written before the
change is altered.

### A03: plane cuts (headless first)

Engine API and CLI for `CutByPlane` (and update/remove, F04 section 7 notes, item c) on a plate or extrusion, with
the capacity pre-check, the `GEO_OP_*` mapping, the `cut:f.p<n>` ids, the volume enclosure rule and
the shape-key erratum for bodies with operations; F06 cases for tangent, coplanar, missing and
multi-loop planes; end-to-end test that every refusal leaves the store unchanged.

### A04 acceptance and `AddHole`

`AddHole`, `UpdateHole`, `RemoveHole` on plates with operation records replacing the pilot
attributes (with the reviewed migration); round and rectangular generators with the D-2 N-gon rule;
exact polygon volume with the enclosure cross-check; `cut:f.h<n>.s<k>` ids and the 64-byte bound;
capacity pre-check and `GEO_KERNEL_CAPACITY` result before G10; ordinals never reused; replay from the
base shape; undo of each command as one unit; F04 and F06 cases added; the A04 pilot's evidence
re-run unchanged for the supported inputs.

## 6. Verification

`cargo fmt --all -- --check`, a JSON validity and unique-case-id check of the edited case file, and
`cargo test --workspace` (the F04 case file is read by `f04_cases.rs`; it was not edited). Results
are in the pull request.
