# Roadmap

**Stage:** pre-alpha. Last updated 2026-10-20 (`master` at `d456670`).

This file is the short summary of the [implementation plan](docs/plan/README.md): the stage, the
acceptance gates, the MVP scope and the next work. Every task ID (F04, U06, A03, ...) is in
[`docs/plan/tasks.md`](docs/plan/tasks.md) and the owner decisions are in
[`docs/plan/decisions.md`](docs/plan/decisions.md). It is updated when a gate or the scope changes,
not for every merge. What each merged
package does is in its note under [`docs/design/`](docs/design/) and in
[`docs/architecture.md`](docs/architecture.md); the history is git and the pull requests. The
progress log kept here until 2026-10-18 is preserved in
[`docs/history/progress-log-2026-10.md`](docs/history/progress-log-2026-10.md).

## Where the product is

The CLI authors plates (rectangular and contour, with holes), straight members with rolled and
custom sections from definitions and libraries, one plane cut per element, assemblies, welds, bolt
groups and levels into verified USD revisions. It moves, copies, deletes and edits elements, keeps
personal workspaces with undo/redo and publishes them, compacts history, previews and approves
numbering, and exports IFC4 and a CSV bill of materials. The native window shows a revision,
picks and snaps, creates plates and members, moves, copies and deletes them, edits their
properties and publishes the workspace.

Every merged slice is experimental: "slices merged" below means the code is on `master`, not
that the gate's acceptance is claimed.

## Acceptance gates

| Gate | Acceptance result | Packages | Status |
|------|-------------------|----------|--------|
| K0 | Verified base, requirements, clean-room boundaries, contracts and original fixtures | F01–F11 | in progress; F04 and F06 frozen 0.1.0 (F06 amendment A-2 applied), F05, F08 and F10 drafts |
| K1 | CLI creates a plate and atomically saves a complete revision readable by ordinary OpenUSD | U01–U07, U09, U14, G01–G02, P01 | slices merged; acceptance open |
| K2 | Straight beam/column, quantities, independent copy, personal workspace and local undo | P02–P08, A07 | slices merged (rolled, custom and library sections); acceptance open |
| K3 | Foreign USD references remain read-only; supported measurements are separate results | R01–R03, G08 | R01 merged; R02–R03 deferred |
| K4 | Two clients safely publish, rebase, detect conflicts, retry and compensate accepted changes | C01–C06 | C01–C02 merged; C03–C06 deferred |
| K5 | Original native UI uses engine commands and saves the same result as the CLI | V01–V06 | V01–V04, V05.1–V05.2 and V06.1 merged; acceptance open |
| K6 | Supported assemblies, cuts, holes, bolts, welds, approved numbering and IFC/BOM export | A01–A09, E01–E03 | slices merged for all; A03 cuts and A04 holes still use element attributes instead of the A-2 operation records; E01 contract draft; acceptance open |
| K7 | Complete alpha scenario and native packaging rehearsal | Q01–Q02 | Q02 packaging merged, release rehearsal not run; Q01 open |
| K8 | First associative drawing projection, dimension and published document version | D01–D03 | deferred |

## MVP scope (owner decision 2026-10-18)

A free single-user desktop MVP: an engineer models a simple steel frame in the native UI and takes
it out as IFC and a bill of materials. Package boundaries follow the implementation plan and each
package keeps its own PR.

1. **Real sections:** rolled and custom section definitions with kernel extrusion and libraries
   (A07), contour plates (A02). Merged. Profile data stays original or from public standards
   (`AGENTS.md` §1.2); no commercial catalogue is copied.
2. **Editing in the UI:** create, select, move, copy and delete (V05), properties and publication
   (V06.1). Merged as slices.
3. **Detailing basics:** plane cuts (A03), holes (A04), bolts (A05). Merged as slices; cuts and
   holes still need the A-2 operation-record form.
4. **Output:** numbering (A08/A09), IFC export (E02 on the E01 contract) and BOM (E03). Merged as
   slices.
5. **Packaging:** the alpha scenario (Q01) and native installers for Windows, Linux and macOS
   (Q02). Q02 merged; Q01 open.

Deferred until after the MVP: multi-client rebase, conflicts, retry and compensation (C03–C06),
drawings (D01–D03) and reference measurements (R02–R03). Which parts later stay open source and
which become a commercial product is decided after the MVP; until then everything stays MPL-2.0.

## Next work

In this order; the task cards are in [`docs/plan/tasks.md`](docs/plan/tasks.md).

1. **A03/A04 to the contract:** `CutByPlane` and `AddHole` as owned operation records with the
   role-based FeatureIds (F06 amendment A-2), replacing the `steel:cut:*` and hole attributes, with
   migration of stores written before.
2. **UI without the CLI:** open and create a project, choose sections from a library, numbering and
   export actions, and a loading indicator in the native window.
3. **Q01 alpha scenario** end to end, and a first manual run of the release workflow to rehearse
   the Q02 installers.
4. **Gate acceptance:** turn the merged slices of K1, K2, K5 and K6 into accepted gates.

Later work expands supported operations, licensed data packs, drawings and DSTV NC export. IFC
import and foreign-to-native conversion require a new scope decision. BeamCraft is desktop-only on
Windows, Linux and macOS; only `beamcraft-geom`, `beamcraft-model` and `beamcraft-kernel` keep the
WASM compatibility requirement as a layering discipline.
