# P01 first end-to-end plate

Date: 2026-10-16. Status: **experimental first vertical slice; K1 is NOT claimed complete.**
Branch `feat/p01-first-plate`, stacked on `feat/u07-journal-recovery` (PR #60), which is stacked on
`feat/u06-revision-commit` (PR #57). Merge order: #57, #60, then this PR.

Plan row: P01 / M, depends on U03, U04, U07, G02. Result: CreatePlate -> USD authoring -> derived ->
commit -> open. Acceptance: API/CLI E2E saves 2 independent IDs; pxr (any ordinary OpenUSD reader)
reads geometry/attrs without BeamCraft; volume is independently verified.

## What is implemented

```text
beamcraft create-plate --store ROOT --width W --length L --thickness T ...
  model::command::CreatePlate  --engine::command/transaction/footprint-->  authored state, kernel body,
                                                                           volume, footprint, fingerprint
  --layer engine (U02 Author)-->  authored.usda + derived.usda (standard Xform/Mesh, no plugin)
  --host (U06/U07)-->  layer blobs, intent, verifier (U02 check, U03 index, kernel re-evaluation),
                       root switch (hard link), committed record
beamcraft plate-inspect --store ROOT
  verified head revision --fresh U01 reader--> authored plates --engine--> kernel body, snapshot
```

| Area | What exists |
|---|---|
| `beamcraft_model::id` | `Uuid7` plus `ElementId`, `ProjectId`, `WorkspaceId`, `OpId`, `RevisionId`: full 128-bit, canonical lowercase, version 7 and RFC variant validated, `prim_path()` (`/World/Steel/E_<32 hex>`). `UuidV7Generator` (RFC 9562 section 5.7/6.2: 12-bit counter in `rand_a`, 62 random bits, logical clock): strictly increasing in one millisecond, never shortened; the clock and ten entropy bytes are injected so the module stays WASM-clean. |
| `beamcraft_model::limits`, `command` | F04 section 3 caps (re-exporting the geom constants); `CreatePlate`, `Placement`, label/count/finiteness/cap/placement admission with stable codes, typed `CommandError`, argument `content_fingerprint`. |
| `beamcraft_engine::{command, footprint, transaction}` | `create_plate`, `evaluate_plate`, `origin_rectangle_contour`; Create footprint (reads ID absence, writes all authored groups and derived), conflict test and the check that the layer engine writes nothing outside it; `admit_create_plate` decides Apply / Replay / `OP_ID_REUSE`. |
| `beamcraft_engine::query::committed` | Committed authored plates -> kernel re-evaluation -> `SceneElement`s -> V02 `ViewSnapshot` (so `ListElements`/mesh blobs come from a committed revision). `ElementId` moved onto `model::id`; V02 tests unchanged and green. |
| `beamcraft_host::ids` | `IdSource`: system clock + per-process `RandomState` entropy (no new dependency, uniqueness not secrecy). |
| `beamcraft_cli` | New library target `plate` (the wiring) and commands `create-plate`, `plate-inspect`. |

## Decisions

1. **Where the wiring lives.** The module table gives the engine no edge to the layer engine and
   the host none either (owner decisions of 2026-10-16). Only the application links all three, so
   the adapter is `beamcraft_cli::plate`. It computes no geometry. A library target was added to
   the CLI package so tests can inject a commit hook and kill a real process.
2. **Layer engine in-process.** The owner decision places it in a separate process; the frozen U04
   profile has no authoring/verify message. P01 therefore runs U02/U01/U03 in the CLI process.
   There is no process containment (LEW-03 remains open); this is documented and not a hostile-
   parser boundary.
3. **CreatePlate and CommitWorkspace are fused.** No workspace exists (P06). One implicit personal
   workspace per store: project/workspace ids come from the first revision, the generation is the
   head's plus one. The F04 envelope (`base_revision`, `expected_generation`, actor, version) is not
   implemented; the CLI takes flags instead of JSON.
4. **Replay.** The host's U07 ledger proves an op committed; the engine then compares the request
   with the element of that id in the head revision (reconstructed fingerprint). That is sound only
   while nothing edits or deletes elements (true in P01). P04/P08 must replace it with the stored
   canonical request (F05 section 6, `model::canonical`, planned).
5. **Verifier at publish.** The host's `ContentVerifier` is implemented in the CLI with the real
   engine: U02 structural validation, U03 `verify_index`, then every plate in the layers is
   re-evaluated by the kernel and its committed mesh must equal the kernel mesh (bit-exact float32
   conversion). A hand-edited layer cannot be published.
6. **Shape key.** `bim:shapeKey` carries the BCMESH01 mesh digest. The F06 section 7 key is not
   implemented (`kernel::keys` is planned); no signature is claimed.

## Gaps against frozen F04/F06 (not changed; amendments or tasks needed)

| Gap | Behaviour in P01 | Needed |
|---|---|---|
| F04 section 3 P-1: any planar contour is accepted; a four-point rectangle is evaluated in its body frame | Only the origin rectangle is evaluated. Other contours that pass the structural stages are `UNSUPPORTED_GEOMETRY` and **rejected** (nothing authored). F04-POS-18, 19, 20, 25, 26, 28 expect `accepted_draft`. | A02/G04: body-frame adapter and contour admission with exact rational planarity/winding/simplicity; schema/status to record `unsupported` geometry. |
| Planarity and simplicity as `INVALID_ARGUMENT` | F04-NEG-29, -30, -38 return `UNSUPPORTED_GEOMETRY`. | Same task. |
| Rotated placement | Accepted by validation; `UNSUPPORTED_GEOMETRY` for create (the view contract is translation-only). | V02/G04 transform support. |
| F04 section 5: published revisions include quantities | Volume is returned by the command and recomputed by `plate-inspect`; **not stored in USD**: the pinned U02 schema has no quantity attributes and the F06 keys are absent. | U02 schema amendment plus P04 signatures. |
| Mesh `extent` | The U02 Mesh has `points`, topology, `orientation`, `subdivisionScheme`, `bim:shapeKey` and no `extent`. Readers compute it. | U02 schema amendment. |
| Mesh precision | USD `point3f` is float32; the kernel blob is float64. The committed mesh equals the float32 cast; volumes agree within 1e-6 for the first-scope sizes. | Decide the F06/U02 precision policy (or double-precision points) before large coordinates matter. |
| F04 section 6 envelope, `OpId` canonical bytes, `CommitWorkspace` | Not implemented (see decision 3). | P06, F05 `canonical`. |
| F04 section 8 validation order | Implemented: arguments, replay, ID uniqueness, footprint, engineering validation; trusted context, workspace, base and generation preconditions are only partly covered by the host commit (base). | P06. |

## Evidence

* **Two independent IDs, saved, reopened in fresh processes** (`apps/beamcraft-cli/tests/plate.rs`):
  two real-binary `create-plate` calls give distinct complete UUIDv7 ids, sequences 1 and 2,
  generations 1 and 2, one project, and a fresh `plate-inspect` and `revision-open` list both.
* **Volume**: closed form `width*length*thickness` in the tests (1000x500x10 = 5,000,000 mm3 =
  0.005 m3; 800x300x12.5), and a divergence-theorem volume of the saved mesh computed by the test.
* **Independent reader**: a hand-written USDA text parser in the test (knows only USDA syntax and
  the UsdGeom attribute names) reads the saved layers straight from the store files: stage
  metadata (`metersPerUnit`, `upAxis`, `defaultPrim`, `subLayers`), `bim:*`/`steel:*` attributes,
  Xform placement, mesh `points`/`faceVertexCounts`/`faceVertexIndices`, closed 2-manifold,
  positive signed volume, bounds. A fresh process then reads the same bytes with the pure-Rust
  `openusd` 0.8.0 backend (the layer engine).
* **pxr / usdchecker: NOT run.** `python -c "import pxr"` fails and no `usdchecker`/`usdview` is on
  PATH; no package was installed or downloaded. Nothing here claims pxr or usdchecker
  compatibility; the Rust backend and the hand-written reader are not the OpenUSD reference
  implementation. The K1 phrase "readable by ordinary OpenUSD" is therefore **not demonstrated**.
* **Failure injection with real process death**: a child re-executes the test binary and
  `abort()`s inside the commit hook at each of the seven U06/U07 commit points, with and without a
  base revision. After each, a fresh `plate-inspect` sees one complete revision (old before the
  switch, new from `Switched`), `revision-outcome` claims no more than is proven, two recoveries
  succeed, and a retry of the same op id creates exactly one plate (replay if the root had switched).
* **Concurrency**: four parallel `create-plate` processes publish only acknowledged plates; losers
  get typed `BASE_REVISION_MISMATCH`.
* **F04 cases** (`apps/beamcraft-cli/tests/f04_cases.rs`, the frozen case file decoded with the
  bounded protocol decoder): 42 `CreatePlate` cases. 28 match the expected code/outcome:
  F04-POS-01, 04, 08, 09, 11, 12, 14, 17, 27 and F04-NEG-01, 02, 03, 04, 05, 06, 10, 19, 22, 23, 24,
  25, 26, 27, 28, 31, 32, 34, 35. 9 are declared deviations (above): POS-18, 19, 20, 25, 26, 28 and
  NEG-29, 30, 38. 5 are not run: NEG-07, 08, 09, 17, 18 (envelope, workspace, actor). Cases of other
  commands are not run.

## Not done / still open for K1

K1 asks that the CLI creates a plate and atomically saves a complete revision readable by ordinary
OpenUSD. Demonstrated here: CLI create, atomic complete revision (U06/U07, fault-injected), reopen
in fresh processes, independently hand-parsed USDA. Not demonstrated: pxr/usdchecker reading,
cross-platform bytes (Windows only), layer-engine process containment, `CommitWorkspace`/workspace,
cargo-deny and wasm32 (not installed here). K0 and K1 stay open.

## Provenance

Original code and tests, MPL-2.0. F04 sections 2, 3, 4, 5, 6, 8; F06 sections 2, 4, 6; UUID layout
from RFC 9562 sections 5.7 and 6.2; volume from the divergence theorem; USDA syntax from the public
OpenUSD documentation of the text format. No Tekla/Trimble material, no new dependency, no new
asset (`ATTRIBUTION.md` unchanged).
