# Roadmap

**Stage:** pre-alpha, foundation. Last updated 2026-10-14.

The repository still contains a scaffold: points, beam-axis length and CLI version reporting.
The gates below describe planned acceptance results, not implemented features.

| Gate | Acceptance result | Packages | Status |
|------|-------------------|----------|--------|
| K0 | Verified base, requirements, clean-room boundaries, contracts and original fixtures | F01–F09 | in progress |
| K1 | CLI creates a plate and atomically saves a complete revision readable by ordinary OpenUSD | U01–U07, G01–G02, P01 | planned |
| K2 | Straight beam/column, quantities, independent copy, personal workspace and local undo | P02–P08 | planned |
| K3 | Foreign USD references remain read-only; supported measurements are separate results | R01–R03, G08 | planned |
| K4 | Two clients safely publish, rebase, detect conflicts, retry and compensate accepted changes | C01–C06 | planned |
| K5 | Original native UI uses engine commands and saves the same result as the CLI | V01–V06 | planned |
| K6 | Supported assemblies, cuts, holes, bolts, welds, approved numbering and IFC/BOM export | A01–A09, E01–E03 | planned |
| K7 | Complete alpha scenario and native packaging rehearsal | Q01–Q02 | planned |
| K8 | First associative drawing projection, dimension and published document version | D01–D03 | planned |

## Next work

F01 records the MPL baseline and aligns these documents with the accepted USD architecture.
F02 (requirements), F03 (OpenUSD distribution audit) and F07 (original fixtures) can proceed
independently. F09 establishes clean-room admission after F02. F04 then fixes the model and command
contract; F05/F06 define worker and geometry contracts before F08 changes module boundaries.

The [F04 model/command draft](docs/contracts/model-and-commands-v0.1.md) and its original
acceptance descriptions are available for review. F02 currently remains a local analysis artifact;
F09 admission and contract freeze are still outstanding. No runtime command is implemented by F04.

The first demonstration starts with a rectangular plate and a supported straight member.
Published USD revisions include their computed geometry. Copying, editing and reopening must
preserve the original element and produce independently verified measurements.
Each user-facing feature requires an engine API, CLI command and end-to-end test.

The OpenUSD package audit is a separate integration gate. It does not block independent
mathematical requirements or fixture work. No USD runtime or third-party kernel is integrated yet.
Full alpha readiness requires requirement coverage; the first demonstration covers only a subset.

Later work expands supported operations, licensed data packs, drawings and DSTV NC export.
IFC import and foreign-to-native conversion require a new scope decision. Browser hosting is a
later project; portable domain crates alone do not establish a working web application.

## Progress log

- 2026-10-14: F05 drafted the private USD-worker protocol contract 0.1.0 on base
  `478f330e15819b574c62227bc23009729adb4fdc`, consuming F04 0.1.0 sections 2, 3, 6, 7 and 8.
  It fixes framing, attachments, handshake, correlation, deadlines, cancellation, restart
  limits, transport error codes and the canonical replay profile, with 49 original acceptance
  descriptions (12 positive, 37 negative). The contract is not frozen, no worker is
  implemented and K0 is not passed.

- 2026-10-14: F07 added original synthetic fixtures on base
  `b34205281ab8a1f95052c4dc7d84f0995633e179`. The new std-only dev tool `tools/fixtures`
  (`beamcraft-fixtures`) provides a generator API, a CLI and E2E tests. `tests/fixtures/f07`
  holds 33 fixtures: 10 accepted (one unverified), 2 unsupported and 21 invalid. A manifest
  records full SHA-256 digests. Separately, `tests/fixtures/expectations/f07.json` holds
  hand-derived expectations, derived in `docs/fixtures/expectations.md`, and a Python oracle
  checks them independently. CI verifies the committed bytes. This is test data only: it adds
  no geometry, USD or copy feature and does not pass K0.

- 2026-10-14: F09 added the clean-room admission policy and templates and an admitted
  first-scope package (rectangular plate, straight member, identities, units, placement,
  declared failures) under `docs/clean-room/`. Base `ebb5b928dda3d36d6b6570549c3f745bb1883ff7`.
  Admission covers that limited scope only; broader requirements stay pending and the F04
  contract remains a review draft. Exposure admission is separate and no implementer is declared
  eligible. Windows fmt, Clippy, three tests, docs and CLI checks passed; local WASM and
  cargo-deny checks remain unverified because the target and tool are absent; K0 is not passed.

- 2026-10-14: synchronized the branch-naming policy PR with master after F04 integration;
  retained both attribution rows and all task progress entries when resolving conflicts.

- 2026-10-14: recorded task-type branch prefixes in AGENTS.md and contributor instructions;
  new branches use feat/bug/docs/chore/refactor/test rather than agent/provider names.

- 2026-10-14: synchronized the F04 review branch with master after the OpenUSD policy merge;
  resolved the progress-log conflict while retaining both F04 and licence-decision entries.

- 2026-10-14: drafted F04 model/command version 0.1.0, including full identities, explicit units,
  bounded inputs, typed mutations, revision/generation preconditions, semantic footprints and
  atomic errors. Added original logical acceptance descriptions; F09 and freeze remain open.

- 2026-10-14: owner approved TOST-1.0 only for official OpenUSD and its SDK resources.
  Recorded the scoped exception in AGENTS.md; kept the global Cargo licence allowlist unchanged.
  Provenance, notices, native closure and runtime checks remain prerequisites for integration.

- 2026-10-14: F01 aligned the roadmap, architecture and development documentation with the
  accepted USD direction. Implementation base: `c6a1e73aea9c41bf88db96281040043471b80d0a`
  (MPL-2.0); local `master` still pointed to the initial scaffold at audit time.
  Windows fmt, Clippy, three tests, docs and CLI version checks passed. Local WASM and cargo-deny
  checks remain unverified because the target and tool are absent; K0 is not passed.

- 2026-10-14: switched the current source and documentation to MPL-2.0; updated Cargo metadata,
  source notices, contribution rules and release packaging. Third-party licence policy remains
  permissive, with explicit MPL-2.0 exceptions for project-owned workspace crates.

- 2026-10-14: scaffold created: workspace (`geom`, `model`, `beamcraft-cli`), dual MIT/Apache-2.0
  licence, CI on Windows and Linux, release workflow, agent rules.
