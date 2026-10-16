# Draft PR: Add the G02 rectangular plate kernel slice

Target: `master`. Verified base: `f31f12a1dac374bc395bb1988d469a657d34c594`.
Prerequisite [PR #38](https://github.com/Apibim-com/BeamCraft/pull/38) (G01) is merged;
this change is no longer stacked. Keep the PR a draft pending CI and acceptance.

## What this changes

Adds `beamcraft-kernel` at the F08-reserved path. A headless dimensions API now
validates width, length and thickness and returns the standard closed rectangular
plate mesh plus analytic volume. It uses G01 points, vectors and local-size
tolerance and the existing model error codes and SHA-256. It adds no third-party
dependency or outbound access.

The plate spans origin +X/+Y/+Z. The builder emits eight positions, twelve
counter-clockwise triangles, six outward unit normals, the face table, local
roles for six faces/twelve edges/eight vertices, the fixed 392-byte BCMESH01
buffer and its digest. Subdivision is explicitly `none`. Analytic volume is
`(width*length)*thickness`; cubic metres divide it by 1e9.

Finiteness precedes caps/scale, which precede ranges/minima. Diagnostics collect
every failure of the first failing stage in dimension order, with fixed fields,
typed GEO codes and existing F04 mappings. A rejected/cancelled call returns no
geometry. Four host cancellation polls follow F06 P0-P3; no poll follows seal.
Before returning geometry, bounded checks verify valid/distinct indices,
positive outward triangle area, paired opposite edge incidence, every vertex
used, Euler characteristic and positive finite signed volume. Positive orthogonal
origin dimensions certify a convex, non-self-intersecting primitive by construction.

### Registration edits

- `crates/kernel/Cargo.toml`: MPL-2.0 workspace package; only geom/model dependencies.
- `Cargo.lock`: register that package and its two internal dependencies.
- `deny.toml`: add only the package/version-specific beamcraft-kernel 0.0.1 MPL-2.0 exception.
- `docs/contracts/module-boundaries-v0.1.md` section 2: kernel row becomes `now`, owner G02; layer/dependency edges unchanged.
- The same document, section 5: record the real G02 crate and its remaining reserved interfaces; remove kernel from the uncreated-crates note.

Root Cargo.toml already discovers `crates/*` and needs no edit. The boundary
checker and CI WASM list derive from the F08 table, so neither needs an allowlist
or CI edit. G01 files/exports, frozen F04/F06 and ROADMAP.md are untouched.

## Roadmap entry

- 2026-10-16: added the G02 origin rectangular plate primitive in beamcraft-kernel,
  with canonical buffers, local feature roles, staged diagnostics, cancellation,
  analytic volume and independent mesh checks; CLI integration remains P01 and
  local WASM validation awaits the target. K0 is not passed.

## How you verified it

- [x] `cargo fmt --all -- --check` — PASS.
- [x] `cargo clippy --workspace --all-targets -- -D warnings` — PASS.
- [x] `cargo test --workspace` — PASS, including 13 new kernel tests.
- [x] `cargo run --locked -p beamcraft-boundaries -- check` — PASS.
- [ ] `cargo check -p beamcraft-geom --target wasm32-unknown-unknown` — command FAIL, E0463; compilation not run successfully because target std is absent.
- [ ] `cargo check -p beamcraft-kernel --target wasm32-unknown-unknown` — command FAIL at geom, E0463; compilation not run successfully because target std is absent.

`rustup target list --installed` lists only x86_64-pc-windows-msvc and
aarch64-pc-windows-msvc. No target installation or new network access attempted.
All changed/new text was verified UTF-8 without BOM and LF. `git diff --check`
passes for tracked changes.

Independent tests read actual buffers, check the frozen F06-POS-02 SHA-256
vector, enumerate incidence/connectivity without the production checker, compute
triangle normals/area/signed volume with separate scalar formulas, and compare
mesh volume to analytic volume under CR-TOL-001 relative 1e-6. An admitted
dimension grid tests canonical zeros and identical repeated bytes. Internal
corruption injections reject reversed/duplicated/degenerate triangles, invalid
indices and a collapsed corner. No mock kernel or external geometry oracle is used.

Case coverage and exclusions are tabulated in the crate README: POS-01/02,
volume-only POS-03, plate POS-14, local-role projections POS-16/17/18,
POS-37, scale/dimension projections POS-53/54; NEG-01/03/16/17/45 and
builder-corruption projections NEG-18/20. Additional checks exercise all three
dimension boundaries, huge negatives, stage precedence, aggregation and mappings.
These projections do not claim completion of full engine/contour acceptance cases.

## Never-crash

- [x] No new unwrap/expect/panic/todo outside tests.
- [x] No unchecked production indexing or slicing; all buffers and loops have fixed bounds.
- [x] No unsafe, recursion or allocation in the primitive API.
- [x] Submitted nonfinite values/cap excesses stop before derived arithmetic.
- [x] Host cancellation discards candidates without mutating authored state.

## Provenance (clean-room)

- [x] No assets or code from Tekla/Trimble or another commercial detailing product.
- [x] No decompilation, disassembly, commercial API samples or private observations.
- [x] No GPL/AGPL code copied; no external implementation copied.
- [x] Behaviour and original synthetic inputs come from frozen F06 0.1.0 sections 2-6, 7 encoding conventions and 10 plus its cases.
- [x] Public mathematics only: F06's IEEE 754-2019 arithmetic, divergence-theorem triangle volume and Euler characteristic references.
- [x] No new external asset, catalogue, licence text or third-party dependency requiring attribution.

The existing model SHA-256 implementation is reused through its public API.
No third-party source or reference website was fetched for this implementation.

## Anything a reviewer should look at first

This is the requested dimension-based rectangular primitive API, not the full
F06 GeometryInput/GeometryResult request adapter. It accepts no authored
contour; F04 contour admission must precede its use for authored geometry.
Arbitrary as-drawn rectangle starts/directions (POS-48/49), nonrectangles,
placement, generic blob parsing, complete quantity records/density, keys,
lookup/picking, state publication, USD, worker protocol, members, cuts and UI
remain outside this slice. The complete F06 kernel surface is not claimed.

The CLI command and real engine-to-CLI end-to-end gate are owned by **P01**.
Engine FeatureId qualification likewise belongs above local role generation.
Member roll sin/cos reproducibility stays OPEN and is not needed by plates.
Native repetition is not cross-target conformance evidence; CI/fixture gates,
F06 E-1 and K0 remain open.

### Handoff state

**Uncommitted.** Git staging failed with permission denied creating the worktree
`index.lock` in its shared Git metadata. No metadata workaround, escalation,
commit, push or PR creation was attempted after that failure. The orchestrator
can commit the allowlisted files and open this focused draft PR against master
using this description; no merge is authorized.
