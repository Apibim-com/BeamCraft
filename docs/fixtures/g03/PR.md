## What this changes

G03 adds 56 original, provenance-recorded rectangular-kernel fixture descriptions and an
independent Rust integration harness. Previously G02 had its own integration checks; the new
dataset separates parameter descriptions, declared outcomes, exact rational references and a
hashed support/provenance manifest. Twenty-two origin rectangles execute the real public builder;
26 invalid inputs must return the exact ordered F06 diagnostics and F04 mappings. Every kernel
comparison failure identifies its stable `g03.plate.*` fixture ID.

The oracle recomputes admission and analytic expectations from parameters, infers faces from
geometry and checks edge incidence, connectedness, box and mesh Euler counts, bounds, positive
tetrahedral volume, surface area and canonical zero. It uses no production geometry helper or
constant table. Nine corruptions of each of 22 real outputs are detected (198 checks), while
reordering triangles with their face mapping still passes. An independent Python standard-library
writer reproduces parameter bytes, hashlib digests and exact Fraction references without calling
the kernel. No production code, dependency, Cargo manifest or frozen contract changes.

Four large-coordinate descriptions are explicitly unverified, with ideal rational expectations
kept separate from successful calls. Four unsupported placeholders reserve through holes
(A04/G09), cavities and thin walls (A02/G05/G09), and nonrectangles (A02/G09). None claims execution,
an implemented diagnostic or a completed geometry gate.

Fixture details, schema, proofs, provenance and commands are in
[`docs/fixtures/g03/README.md`](README.md). The fixture set is version 1.

**Stack:** base `d3c20ddd4a2588662823f6e56f42935d1cf6bf5b`, branch
`test/g03-kernel-fixtures`, target `feat/g02-rect-plate`. Prerequisite:
[G02 PR #39](https://github.com/Apibim-com/BeamCraft/pull/39); F07 is integrated. Merge #39 first,
then retarget this draft to `master`. Only G03 changes belong in this PR. Local branch and HEAD
match the supplied base; the GitHub remote could not be refreshed from this environment.

## Roadmap entry

- 2026-10-16: G03 added 56 contributor-original kernel descriptions on G02 PR #39: 22 supported
  origin rectangles, 26 invalid inputs, four unverified large-coordinate cases and four unsupported
  hole/cavity/thin-wall/nonrectangle reservations. An independent Rust harness checks the public
  kernel against rational references, analytic bounds, topology and 198 output corruptions.
  Workspace fmt, Clippy and 202 tests pass locally; WASM verification is not run because the target
  is absent. No kernel implementation, dependency, CLI, USD, UI or frozen contract changes; G05/G09
  numeric and cross-target gates and K0 remain open.

## How you verified it

Windows x64, rustc 1.96.0, cargo 1.96.0, Python 3.11; no dependency downloads.

- [x] `cargo fmt --all -- --check` — PASS.
- [x] `cargo clippy --workspace --all-targets -- -D warnings` — PASS.
- [x] `cargo test --workspace` — PASS: 202 tests, including four new G03 integration tests.
- [ ] `cargo check -p beamcraft-kernel --target wasm32-unknown-unknown` — attempted; exit 101,
      E0463, missing target standard library. WASM verification **not run**. Installed targets
      are x86_64-pc-windows-msvc and aarch64-pc-windows-msvc; neither supplies wasm32 std.
- [x] `python tests/g03/write_fixtures.py --check` — PASS: 56 descriptions, 58 TSV files,
      exact bytes, digests and rational-reference reproduction.
- [x] `git diff --check` and explicit UTF-8/no-BOM/LF inspection — PASS.

Initial harness compilation and a Clippy macro warning were corrected in test code before the
passing checks. These were harness development issues, not kernel defects. No baseline failure
or production fix is claimed. Existing workspace tests remain in the workspace test run.

## Never-crash

- [x] No new `unwrap`/`expect`/`panic!`/`todo!` outside test code.
- [x] The test reader bounds each file before reading, validates schemas and checks triangle
      addresses before dereference; topology traversal has a fixed eight-pass bound. Test-only
      indexing/unwrap/panic lint allowances are explicit. No runtime API or allocation changes.
- [x] No `unsafe`.

## Provenance (clean-room)

- [x] No asset from Tekla, Trimble or any commercial detailing product.
- [x] No commercial binary, API sample, reference assembly or documentation example was used.
- [x] No GPL/AGPL code, third-party test, corpus or geometry implementation copied.
- [x] Numerical behavior and diagnostic expectations come exclusively from frozen F06 0.1.0,
      sections 2/3/4/6/7/10; exclusions use section 1. Mathematical references are the public
      Cartesian-product box volume/bounds formulas, Euler characteristic and signed-tetrahedral
      divergence identity already written in F06 section 6. README proofs and per-row source labels
      cover every expectation. No external oracle or kernel-derived expected number is included.
- [x] All new non-code assets are under the existing `ATTRIBUTION.md` `docs/fixtures/**` entry;
      the dated G03 README and per-file manifest refine path, version, author, source, licence and
      use. Contributor-original MPL-2.0. No edit outside the task allowlist was needed.

Repository context read: AGENTS/CLAUDE, roadmap, architecture, F06 and case descriptions,
tolerance comparison, G04 kernel design, G01 evaluation/owner decision, F08 module boundaries,
kernel README/API and existing repository fixture conventions. Implementation sources read for
orientation are BeamCraft's own code. The oracle is written from the cited contract/formulas;
it does not call builder internals, geom arithmetic, production tolerance helpers or topology
tables. Unestablished prior model-training exposure remains unknown; this task does not certify
implementer admission.

## Anything a reviewer should look at first

The explicit distinction between confirmed origin output and unconfirmed translated ideal
references is essential. The raw dimensions API cannot evaluate authored contours or placement.
F06 rounded-mesh accuracy E-1, G05/G09 numeric certificates, WASM and cross-platform byte
conformance remain open. CR-TOL-001 is only a test comparison rule; thresholds and statuses are
exact. The independent checker is bounded QA for these boxes, not a general mesh validator.

**Failing-test note:** no failing kernel fixture or kernel defect was found. The only outstanding
required verification is the missing WASM target. Unsupported reservations are never run as
successful kernel fixtures. This is QA infrastructure, not a new user-facing feature or engine/CLI
E2E acceptance. Keep the PR draft until the dependency and required CI/acceptance gates are met.

## Handoff

This file preserves the full draft PR description for the authorized Git handoff. It is not
evidence that a remote PR was created. Use title `test: add G03 original kernel fixtures and
independent harness` and base `feat/g02-rect-plate` when publishing the draft.

Current handoff status: **uncommitted**. `git add` failed because the sandbox could not create
`D:/Projects/01_BeamCraft/.git/worktrees/01_BeamCraft-G03/index.lock` (permission denied).
No alternate index, Git-directory relocation, escalation or other metadata-write workaround was
attempted. No commit, push or PR was created. The orchestrator must commit these 62 allowlisted
files and publish one focused draft PR. The local HEAD remains the supplied base. The remote read
also failed to connect to GitHub through the environment's configured loopback proxy; no current
remote-head/CI verification is claimed. Keep the missing WASM and prerequisite gates open.
