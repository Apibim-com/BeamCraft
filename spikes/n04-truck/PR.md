# Evaluate truck B-rep holes and cuts in an isolated offline spike

Draft PR target: `master`; branch: `feat/n04-truck-spike`.
Verified entry base: `adbc407e09476a803de07c31f3a5adcaabad1304`.
Publication status: **uncommitted; PR not opened**. Git staging failed creating the worktree's
`index.lock` with Permission denied because Git metadata is outside the writable sandbox. No
escalation or alternate Git metadata access was attempted. The orchestrator must commit, push
and open this focused draft against master. This file is the review-ready body, not a claim of a
published PR. Creating the draft does not authorize merging or adoption.

## What this changes

N04 supplies an original offline experiment and
[evaluation](../../docs/design/n04-truck-evaluation-v0.1.md) of truck-modeling 0.6.0,
truck-shapeops 0.4.0 and truck-meshalgo 0.4.0 for later A02/A03/A04/G09 work. It builds actual
B-reps and meshes for rectangular/concave plates, cylinder subtraction, a four-hole pattern,
edge contact/notches, half-space-equivalent cuts, minimum wall/slab dimensions, large coordinates
and tolerance/precondition controls. Expected volumes/topology come from public mathematics.

The recommendation is explicitly a **proposal for the owner: reject this pinned default stack
as the holes-and-cuts backend for now**. Ordinary cuts and extrusion show useful logic, but the
degenerate-cut failures, incomplete/inaccurate cylinder tessellation, raw mesh topology,
provenance, bounded execution and unresolved admission work prevent production acceptance.
No decision is made and no production dependency, kernel code, contract, CI or roadmap is changed.
Estimated truck adoption investigation: 200–400 person-hours, uncertain and not a promise.

Each of 44 bounded cases runs in debug/release, caught three times at one thread, caught at four
threads, and directly at one thread: 440 calls with a 30-second child-process deadline.
Per-call metrics and exact triangle hashes, independent Python scalar checks, case/determinism
tables and the full cargo-metadata transitive inventory are under `evidence/`.
Two additional 120-second calls (release pattern and debug hole) are stored in supplement.json;
their budgets and results are separated from the primary 440-call matrix.

<!-- N04-PR-RESULTS-BEGIN -->
Final counters pending.
<!-- N04-PR-RESULTS-END -->

PASS in case evidence is an analytic-volume/B-rep probe result, not F06 mesh conformity. FAIL is
explicit failure or a distinguished timeout/cap; PANIC and WRONG are counted separately.
Repeated timeouts do not imply geometry is wrong and have no byte-determinism result.
No outputs are healed, perturbed or promoted to certified/current engineering quantities.

## Roadmap entry

- 2026-10-16: N04 evaluated pinned truck B-rep crates in an isolated offline spike using original
  analytic plate, hole, contour, cut, minimum-dimension and large-coordinate cases. Debug/release,
  caught/direct calls, repeat/thread byte comparisons and complete Cargo licence/checksum evidence
  are recorded in `docs/design/n04-truck-evaluation-v0.1.md`. The owner proposal is to reject this
  default stack for holes/cuts pending robust geometry, F06 mesh/provenance/bounds work and licence
  admission. No dependency or feature is adopted; WASM and cargo-deny were not run because the
  target/tool are absent. This entry is proposed only; ROADMAP.md is unchanged.

## How you verified it

- [x] Root `cargo fmt --all -- --check` (offline environment).
- [x] Root `cargo clippy --workspace --all-targets -- -D warnings` (offline environment).
- [x] Root `cargo test --workspace` (offline environment).
- [ ] `cargo check --manifest-path spikes/n04-truck/Cargo.toml --target wasm32-unknown-unknown`:
  command attempted; fails E0463 because target absent; WASM compilation **not run**.
- [x] Separate spike fmt, Clippy, six real-call Rust tests in debug and release.
- [x] Independent Python tetrahedron corruption controls and six exact G03 rational references.
- [x] 440 supervised native calls, compared numeric and topology expectations and triangle bytes.
- [x] All 135 registry archive checksums match Cargo.lock; complete inventory checked against declared
  deny-policy expressions, including legacy expressions and OR alternatives.
- [ ] `cargo deny`: **not run**, tool absent; no security/advisory freshness claim.
- [x] Root lockfile unchanged; original text UTF-8 without BOM, LF; all generated build/mesh files
  remain in the allowed spike's ignored target directory.

Exact command logs are in `evidence/checks.json` and `evidence/check-*.txt`. The root checks exercise
the existing product. Research findings such as upstream panics are not hidden by passing harness
tests and are not accepted feature behaviour. No production engine/CLI/USD E2E claim is made.

## Never-crash

- [x] No new unwrap/expect/panic/todo/unreachable in original non-test Rust.
- [x] Fixed synthetic case count; checked mesh index access and bounded audit outputs.
- [x] No unsafe in authored Rust; manifest `unsafe_code=forbid`.
- [x] Fresh state inside catch_unwind; direct calls in separate processes; no post-unwind reuse.
- [ ] Upstream no-panic / allocation / recursion / cancellation contract: **not established**;
  caught panics and process timeouts are explicit evidence. A post-output cap cannot bound upstream
  peak memory or work. Dependencies have not been audited for absence of unsafe.

## Provenance (clean-room)

- [x] No Tekla/Trimble or other commercial-product asset, code, sample, catalogue, screenshot or test.
- [x] No decompilation or copying of GPL/AGPL source or third-party implementation/tests.
- [x] Behaviour/expectations derive only from public mathematics and F06 policies: Cartesian triple
  integrals `abc`; disk integral `πr²`; disjoint-area extrusion; tetrahedron `abc/6`; central-inversion
  symmetry for half volume; Euler `2-2g`; divergence theorem for independent mesh measurement.
- [x] Public math sources: OpenStax Calculus Volume 3 sections 5.4 and 6.8; Euler characteristic;
  exact formulas and bibliography are derived in the report. F06 sections 3/4/6/10 supply result,
  tolerance, buffer and resource rules. Original G03 rational references cross-check boxes.
- [x] Cached rendered public truck API documentation is used only to call the API. No upstream
  implementation bodies or tests were opened; compiler errors/backtraces are runtime observations.
- [ ] Every new asset has an ATTRIBUTION.md entry: the task **forbids editing ATTRIBUTION.md**.
  Original outputs have provenance in the report/README and the concrete proposed row below.
  No upstream code/licence text or fixture asset is copied into the repository.

Proposed original-output attribution row, intentionally unapplied:

| Asset | Author | Source (with version) | SPDX licence | Use |
|---|---|---|---|---|
| `spikes/n04-truck/**` excluding build artifacts, `docs/design/n04-truck-evaluation-v0.1.md` | BeamCraft contributors | Contributor-original N04 probes, analytic references, measurements and audit, 2026-10-16; exact pinned public-API package/checksum provenance in the evaluation and inventory | MPL-2.0 | Isolated research and owner-decision evidence; no upstream implementation, catalogue, commercial-product asset or fixture copied |

If adopted later, each exact truck package/version needs its own attribution row with repository
revision/archive checksum, actual original licence text/NOTICE provenance and intended algorithm
use. The report records all three direct checksums and every transitive version. Do not fill those
rows with inferred missing text: none of the three candidate archives packages a root LICENSE*.
The native optional lzma-sys/XZ file-level closure and licence distinctions are unresolved; no
redistribution is authorized by this PR. Apache-2.0 is already allowed; any new project-owned crate
requires its package-specific MPL exception and F08 entry. No global licence exception is added.

## Anything a reviewer should look at first

The chosen tolerance controls approximation, whereas F06's scaled length floor controls admission;
they are not interchangeable. A returned Solid may flatten to a partial mesh. A correct scalar
volume does not establish indexed closure, provenance, limits, stable bytes or a certified solid.
The default graph also enables optional VTK/STEP functionality and native C compression.

Limits: one Windows compiler/host; no WASM/other-platform execution, general fuzzing, exact geometry
certificate, file-level native audit, current advisory check, minimal-feature build, source-level
patch design, stable operation-id scheme or production integration. The owner should decide any
follow-up scope separately. No merge or adjacent task is undertaken.
