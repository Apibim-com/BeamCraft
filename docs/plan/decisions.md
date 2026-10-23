# Owner decisions

Decisions that shape the plan, newest first. Each names its date and, where one exists, the
document that records it in detail. A later decision overrides an earlier one where they differ.
Open questions are at the end.

## 2026-10-22

- **Section projects and reproducible coordination: foundation approved.** Distinguish an
  engineering module (`steel`) from a responsible project section (`KM`, `KMD`) and its optional
  zones. Sections can share one project store or publish independent projects under one common
  standard. A coordination revision pins the exact source revisions; a new section publication
  does not update coordination automatically. This extends the project-organization decision of
  2026-10-20 from independent zone projects to independent section projects as well. The draft
  specifications are [F12](../contracts/f12-project-coordination-v0.1.md) and its
  [F10 0.2 amendment](../contracts/f10-domain-vocabulary-v0.2.md). Their detailed clauses are for
  review; approved direction is not a claim of implementation or frozen contract acceptance.
- **One branch and one PR for this foundation.** F12 and the associated vocabulary amendment,
  architecture alignment and plan references form one coherent documentation task. Do not split
  this agreed package into multiple PRs. Implementation tasks remain separate from this contract
  work; no implementation status or MVP priority changes through this approval.
- **One opening assignment.** The initiating engineer creates one shared assignment for an
  opening; the responsible host-section engineer decides whether it is acceptable. The physical
  opening is separately authored in the host model and linked to the assignment. Acceptance is
  distinct from implementation, and applies to the reviewed request/host versions. The reserved
  F10 mechanism is not an implemented workflow.
- **Stage 4 approved: reference-first design and assignment files.** Services engineers take a
  pinned structural project as a read-only reference and model pipes/ducts without cutting holes.
  They collect required openings and send a file containing individual assignments; structural
  engineers decide per item and author accepted openings in their own model. Decisions and
  implementation evidence return in a response file. Retransmission creates no duplicates.
  Acceptance of a 150 mm proposal never approves its 200 mm successor and never overwrites it;
  retain the old decision with the exact proposal/host versions it reviewed. A changed structural
  basis likewise requires revalidation. The [F12 stage 4 companion](../contracts/f12-opening-assignments-v0.1.md)
  specifies this workflow in the same branch and PR. Detailed encoding and executable acceptance
  remain open; this approval adds neither a general project-review system nor runtime features.
- **Existing foundations retained.** USD remains the model; source element identity, immutable
  revisions, typed commands and operation-based merge remain authoritative. The frozen F04/F06/
  F10 0.1 documents and their existing schemas stay unchanged. C03–C06 remain after the MVP;
  approval/review tooling, remote hosting and additional engineering modules are not added to
  the current implementation scope.

## 2026-10-21

- **MVP scope widened.** The first version is a full manual steel-modelling tool with views,
  display control, attributes and drawings, not only a frame with a bill of materials. Still
  a free single-user desktop product, still no automation: no parametric connections, macros or
  automatic detailing. The scope of 2026-10-18 and 2026-10-20 is replaced by the list below.
  1. **Modelling:** columns, beams, polybeams, curved beams, plates, bent plates, lofted plates and
     other plates of complex contour; fits, cuts, chamfers, split and combine; the full section
     placement (plane, depth, rotation, offsets); part classes.
  2. **Connection objects and assemblies:** bolt groups with holes in all connected parts, welds,
     assemblies, nested assemblies, explode an assembly, numbering.
  3. **Attributes and schedules:** user-defined attributes on any element; schedules built from
     them. Items (elements with foreign or own mesh geometry) are after the MVP (see below).
  4. **Model tools:** copy, move, rotate, mirror, arrays; grids; construction points, lines and
     circles; numeric input and direction lock.
  5. **Views and display:** named 3D and plane views, filters, cut plane, five render modes on
     Ctrl+1 to Ctrl+5, colour by filter or class (saved representation sets and a temporary
     overlay that never changes the model), the bottom bar of selection switches, the snap panel,
     the orthographic and perspective switch.
  6. **Drawings and drafting, all manual:** the user places views on a sheet; a drawing view shows
     what the chosen model view and its filters show (a view of the columns shows the columns).
     Dimensions, marks and leaders are placed by hand; the view title is configurable. Dimensions
     and marks stay bound to model features. No automatic views, dimensioning or mark placement.
     Drafting tools, PDF and print.
  Order of work (milestones, each its own set of PRs): M1 modelling and the UI shell (selection
  bar, snap panel, projection switch, render modes); M2 model tools, views, filters, cut plane,
  colour by filter, classes; M3 attributes and schedules; M4 drawings; M5 the alpha scenario
  and packaging over the whole scope (Q01, Q02). Gates K9 to K11 are added; K8 (drawings) is no
  longer deferred. The task cards are in [`tasks.md`](tasks.md).
- **Still after the MVP:** items (I01 to I03: the item contract and reading GLB and IFC geometry), IFC export (E01, E02), multi-client rebase and conflicts (C03–C06),
  reference measurements (R02–R03), DSTV NC and DXF output, parametric connections and macros,
  concrete and reinforcement, analysis.
- **Items after the MVP.** Items (I01), and with them reading GLB (I02) and IFC geometry (I03),
  are deferred whole. When they come back, only the geometry of a product becomes an item, no IFC
  model is converted into native elements, and files are untrusted input (`AGENTS.md` §4).
- **Own palette and shortcuts data.** The class colour palette, render-mode set and shortcut
  map are our own design and user-remappable presets (environment data). The Ctrl+1 to Ctrl+5
  habit is kept as a usability choice; no colour table, label or text of another product is
  copied (`AGENTS.md` §1–2). The owner's private functional notes stay outside the repository.
- **Explode means dissolve.** Exploding an assembly deletes the assembly and leaves its parts as
  loose, unlinked elements; geometry, placement and attributes of the parts do not change. It is not
  a display-only exploded view. Exploding acts on one level: the sub-assemblies of an exploded
  assembly stay assemblies. It is undoable; numbering becomes stale.
- **Curved geometry stays faceted.** Bent, lofted and curved parts are meshes built to a stated
  chord deviation, like holes (amendment A-2, D-2); no analytical curved-surface kernel for the
  MVP. The deviation is reported with the result.

### Defaults fixed for this scope

- Plates: only the kinds listed above (contours with arcs, chamfers and rounded corners, bent and
  lofted plates); nothing further in the MVP.
- Drawing views default to scale 1:100; the scale is a property of each view and can be changed.
  Further sheet defaults (sizes, title-block fields) are settled in task D05 and are presets the
  user can change.

## 2026-10-20

- **K0 passed.** Every foundation task is done or retired by decision: F04, F06 and F10 frozen at
  0.1.0, F08 a living document checked by CI, F01, F07 and F11 done, F03, F05 and F09 retired, and
  the F02 requirements map kept outside the repository.
- **IFC export after the MVP.** IFC needs independent validation and has many details of its own;
  the MVP takes the model out as a bill of materials, and the model itself is ordinary USD. The E01
  contract stays a draft and the experimental `export-ifc` stays in the code; their acceptance moves
  after the MVP. This changes the MVP scope of 2026-10-18.
- **KM to KMD.** Steel detailing is built from steel design: KMD starts as a copy of KM elements
  linked by `bim:builtFrom` to the KM element and revision; KM is read-only for the detailer, and a
  KM change marks affected KMD elements for checking without rewriting them (task R06).
- **Project organization.** Layers are split by discipline and optional zone, named
  `<code>[_<zone>]_<discipline>.usda` with the discipline codes `Coord`, `Grids`, `KM`, `KMD`, `KR`, `AR`,
  `AI`, `GP`, `EOM`, `VK`, `OV`, `SS`, `TH`, each present only when the project has that discipline;
  no program version in names. The coordination layer is
  the root and carries the project code, name, organization and zones. A team may instead keep one
  project per zone and coordinate them from a project on top. Levels are elements in the `Grids`
  layer, not layers of their own. Recorded in the architecture note; tasks U19 to U21.
- **No separate USD worker process.** USD is read and written in the main process by the pure-Rust
  layer engine. The worker existed to isolate the C++ SDK, whose crashes cannot be caught; with
  `openusd` a bad file is an ordinary error, own project files are verified by SHA-256 before
  parsing, and foreign files are read under size and depth budgets. The worker binary, its host
  supervision and the F05 protocol are removed (task U17, [issue #146](https://github.com/Apibim-com/BeamCraft/issues/146)). This replaces OD-1.
- **Prim naming rule** (F04 amendment A-3; [architecture](../architecture.md#prim-hierarchy-and-naming-rule)):
  paths hold only category and identity (`/World/Steel/Parts/PRT_<uuid>`, `/World/Levels/LVL_<uuid>`,
  `/World/Definitions/Profiles/PRF_<sha256>`, ...); disciplines are second-level folders; names,
  assemblies, levels and marks are attributes and relationships. Compatibility is broken on
  purpose: stores written before task U16 are not migrated while the project is pre-alpha.
- **Assemblies are relationships, not parents.** A stable path keeps every link to a part valid
  when it changes assembly, and a part belongs to several groupings at once. Splitting storage into
  more files (per assembly, zone or element) is a separate performance decision for U10/U15 and
  changes no path.
- **No clean-room process.** BeamCraft is written from scratch with its own logic and nothing is
  reverse-engineered, so the F09 process (implementer admission, exposure records, isolated kernel
  agents) is retired and its documents removed. The asset and provenance rules of `AGENTS.md` §1–2 stay:
  nothing from Tekla or Trimble, no decompiling, no copied documentation, tables or GPL code.
- **The plan lives in the repository.** The implementation plan, task list and decisions move
  from the owner's local document into `docs/plan/` (English, no observation notes of commercial
  products). This reverses the 2026-10-14 decision to keep detailed plans outside the repository.
- **No progress log.** History is git, the pull requests and the design notes; `ROADMAP.md` keeps
  only the stage, gates, MVP and next work and changes in docs-only PRs. A single log file would
  conflict in every PR. Release notes are generated from merged PRs at tag time.
- **Review threshold for contributor PRs.** A contributor PR goes back to its author only for a
  blocking problem: failing tests, a merge conflict, broken store or schema compatibility, or a
  contract violation. Wording and stale references are fixed by us in a follow-up PR.
- **A03 cuts merged as element attributes.** PR #113 (one plane cut per element, authored at
  creation, `steel:cut:*`) is merged although amendment A-2 calls for `CutByPlane` operation
  records; moving to the records is our debt under A03.
- **A09: stored numbering of PR #103** is the implementation; the competing PR #110 (manual marks
  `number-assign`, `bim:mark`) is closed. Manual mark editing may come later on top of #103.

## 2026-10-19

**F06 amendment A-2** (recorded in the [F06 contract](../contracts/f06-geometry-contract-v0.1.md) and
[`g08-f06-a2-notes.md`](../design/g08-f06-a2-notes.md)):

| # | Decision |
|---|----------|
| D-1 | The contract covers holes, plane cuts and contour extrusion with polygonal cavities; true curved surfaces are unsupported |
| D-2 | A round hole is a regular N-gon; N grows with the diameter (chord deviation ≤ 0.05 mm, at least 16, at most 256, a multiple of 4) |
| D-3 | A certified closed mesh reports the midpoint of its enclosing volume interval as `current`, with the interval width |
| D-4 | FeatureIds `extrusion` and `cut`; tokens `GEO_OP_UNSUPPORTED` and `GEO_OP_NO_EFFECT`; index addressing is forbidden |
| D-5 | No project limit on holes; the present capacity is a certification performance defect (task G10); until G10 an honest "kernel capacity exceeded" refusal, nothing published |
| D-6 | F06-NEG-39 and NEG-08 are superseded (A02.1 and A-1) |
| D-7 | A wrongly built element (for example a reversed contour) is kept as degenerate without derived data; once fixed, geometry and quantities are computed (task P-Degenerate) |

## 2026-10-18

- **MVP scope.** A free single-user desktop MVP: model a simple steel frame in the native UI and
  take it out as IFC and a BOM. Order: real sections (A07, A02) → editing in the UI (V05, V06.1)
  → cuts, holes and bolts (A03–A05) → numbering, IFC and BOM (A08, A09, E02, E03) → alpha scenario
  and installers (Q01, Q02). Deferred until after the MVP: C03–C06, D01–D03, R02–R03. Which parts
  stay open source is decided after the MVP; until then everything is MPL-2.0.
- **A09 without C05** for the MVP: numbering approval is sequential and single-user.
- **Mass.** Mass = section area × length × density of the project material. The area comes from
  the catalogue when a profile is inserted from a library, otherwise the kernel computes it with
  the exact arc formula and the mass is marked computed (`area_source`: `catalogue` | `computed`).
  A tabulated mass per metre is a reference field only. Full values are stored; rounding happens
  only on output (0.1 kg by default). Check case: section 30B1 of GOST 27772-2015, 6 m, 7850 kg/m³
  gives 192.2 kg from the catalogue area (40.8 cm²) and 192.17 kg from the exact arcs.
- **Declared versus computed area (A07 D3).** A difference above 1 percent is a warning
  (`declared_area_warning`), never a refusal: catalogues use simplified radii.
- **History compaction (U14).** Undo past the compaction boundary is impossible and shown in the
  dry-run report; an old `op_id` is no longer recognised once its journal entry is rotated (entries
  referenced by kept revisions are not rotated); the head keeps its UUID and number; a real
  compaction needs `--backup DIR` or an explicit `--no-backup`.
- **Retention policy (proposal, not yet in code).** An undo window of the last 50 operations,
  configurable; milestones kept by compaction (manual, and automatic before numbering approval,
  IFC/BOM export and publication); no eternal history by default; U15 planned next to U14.

## 2026-10-17

- **USD features adopted from studying a public Omniverse sample** (observation only; the sample's
  bytes, names, geometry and materials never enter the repository): `UsdPreviewSurface` appearance
  with `displayColor` fallback, separate from the engineering material (U08); model `kind`,
  `displayName`, `extent` (U09); payloads for heavy geometry (U10); instancing of shared geometry
  (U11); units and axis correction on our reference wrapper, never on foreign prims (R01); saved
  views, review notes and measurements as our own objects bound to `bim:id` and revision (W01–W03);
  keeping foreign root-layer data that other tools write (W04). Not adopted: hash prim names,
  units baked into matrices, MDL-only materials, network asset paths, numbers with units in
  strings, overrides on foreign or instance-proxy prims.
- **Design options** are named delta layers over a base revision (an unpublished named changeset),
  not variantSets in the authored model. Switching is a view setting; accepting publishes the
  layer as an ordinary changeset; rejecting deletes it. Numbering, BOM, IFC and drawings use the
  accepted revision by default. VariantSets may be generated in a viewing package only (N07).
- **Discipline views** are view state (saved views), not model data. Own element representations
  use the standard `purpose` (Full `render`, Simplified `proxy`, Axis `guide`); engineering reads
  Full only.
- **Element data for schedules.** Three levels: authored inputs, immutable definitions by digest,
  derived quantities computed in the same transaction as the edit with an input signature. A BOM
  reads stored quantities from the snapshot and never recomputes them; totals are queries, not
  model data.

## 2026-10-16

- **USD layer engine: variant C** (F11): pure-Rust `openusd` 0.8.x behind a thin BeamCraft
  wrapper; the official SDK only as an explicit fallback and test oracle.
- **Desktop only** on Windows, Linux and macOS; only `geom`, `model` and `kernel` keep the WASM
  requirement.
- **F04 and F06 frozen** at 0.1.0.
- **OD-1 to OD-4** ([`usd-process-integration-v0.1.md`](../design/usd-process-integration-v0.1.md)):
  the layer engine runs in a separate process (OD-1, replaced on 2026-10-20); holes and cuts are our own implementation
  without third-party kernels (OD-2, scope widened by A-2 D-1); member roll uses `f64::sin`/`cos`
  (OD-3); spike evidence policy: the repository holds the report, the decision and a minimal
  harness, bulky raw output stays local (OD-4).

## 2026-10-15

- **Own kernel.** IFC-lite, Manifold, truck and OCCT are not production backends and are not
  copied or ported; the kernel is ours.
- **MSRV 1.96** (required by `openusd`).
- **OpenUSD-derived TOST-1.0 material** in the `openusd` crate stays under its upstream licence;
  BeamCraft's own code stays MPL-2.0 (`AGENTS.md` §1.2).
- **Development order**: USD core and the steel domain schema locally → BeamCraft for one
  engineer → versioning by delta layers without real time → IFC export before assemblies → a live
  server only for real multi-team need → a second discipline module only after the schema is
  proven on steel.

## 2026-10-14

- **MPL-2.0** for the project's own code.
- **TOST-1.0** allowed only for official OpenUSD and its SDK resources.
- **One focused PR per task**, branches `<type>/<description>` from the verified `origin/master`.

## Open questions

| # | Question | Blocks |
|---|----------|--------|
| 1 | Independent IFC validation tool (E01 Q-8) | E02 acceptance |
| 2 | macOS policy without project hardware | Q02 acceptance |
| 3 | IFC import scope (E04) | E04 |
| 4 | Live channel and permission model (C07, C08) | C07, C08 |
| 5 | Where saved views and review notes live: workspace only or a shared review layer | W01, W02 |
| 6 | Variant sets in our own layers (N07) | N07 |
| 7 | Local use of the third-party USD sample under its licence | N08 |
| 8 | Design options private or visible to everyone | O01 |
| 9 | What a rejected design option keeps besides its journal entry | O01 |
| 10 | Enabling options with overlapping footprints at once, marked as conflicting | O01 |
| 11 | Retention defaults of the history policy (window size, automatic milestones) | U14 follow-up, U15 |
