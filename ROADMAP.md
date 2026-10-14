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

- 2026-10-14: addressed F03 review findings with bounded ZIP directory preflight and text reads,
  plus input/output alias protection. Twelve synthetic audit tests pass; added their Windows/Linux
  CI execution. SDK integration remains gated separately from audit-tool correctness.

- 2026-10-14: established one focused PR per implementation-plan task in AGENTS.md, including
  verified bases, explicit stacked-PR dependencies, English descriptions and no agent co-author trailers.
  Opened separate draft reviews for the MPL prerequisite, F01 and F03; no merge was performed.

- 2026-10-14: F03 pinned and statically audited three `usd-core` 26.8 CPython 3.12 wheels.
  Added an offline inventory tool with seven passing synthetic tests. SDK admission remains open:
  TOST/resource policy, missing native notices, runtime/native closure and platform smoke tests.
  See [the audit](docs/audits/openusd-26.8.md); no runtime dependency or policy exception was added.

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
