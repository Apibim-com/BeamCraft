# Owner decisions

Decisions that shape the plan, newest first. Each names its date and, where one exists, the
document that records it in detail. A later decision overrides an earlier one where they differ.
Open questions are at the end.

## 2026-10-20

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
  the layer engine runs in a separate process (OD-1); holes and cuts are our own implementation
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
