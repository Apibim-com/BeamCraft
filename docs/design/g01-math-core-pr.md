# G01: implement the kernel math core

## What this changes

Adds the headless fixed-size f64 math API in the existing `beamcraft-geom`
crate: checked points/vectors, finite planes, derived body rigid maps,
translation-only element placement, ordered local-size tolerance derivation,
default member frames, member-only axial roll, absolute frame certificates and
canonical f64 key bytes. Invalid input or arithmetic returns typed errors.

Base: `c36313420ba6870081eb2b51df43ee93f2461c14`, verified local HEAD and
`origin/master` at task entry; branch `feat/g01-math-core`; target `master`.
No dependencies, manifest/lockfile changes or model-layer changes are needed.

Delivery status: **uncommitted**. The attempted `git add` failed because Git
could not create the worktree `index.lock` in the shared Git metadata directory
(permission denied). No workaround or escalation was attempted. Commit, push and
PR creation are not performed; the orchestrator must commit the allowlisted
changes and use this prepared description to open one draft PR targeting master.

This is the G01 math slice only, not the broad historical G01 allocation in
F08: no kernel crate, solids, meshing, extrusion, topology, quantities, keys,
worker protocol, USD or UI. The math library is the engine-facing API now;
**no CLI command is needed for pure math**, as explicitly scoped by the owner.
Integration tests exercise the real public math API, not mocks.

`Point3` retains its original raw const constructor, fields and unchecked
`distance_to` for existing model scaffold compatibility. The new `try_new`,
`validated`, `checked_distance_to` and all checked consumers validate raw points.
New vectors, frames, planes and maps cannot contain nonfinite data. The old
model's unchecked beam distance remains outside this math integration slice.

## Roadmap entry

- 2026-10-16: G01 added dependency-free checked f64 vectors/points, planes, body-frame rigid maps, translation placement, F06 local-size tolerances, certified default member frames and isolated axial roll in geom, with 51 math tests covering 42 F06 case projections; native format, Clippy and workspace tests passed, WASM target was unavailable, reproducible trig and contract errata remain open, and K0/K1 are not completed.

## How you verified it

- [x] `cargo fmt --all -- --check` - PASS.
- [x] `cargo clippy --workspace --all-targets -- -D warnings` - PASS.
- [x] `cargo test --workspace` - PASS, 185 tests, zero failures.
- [ ] `cargo check -p beamcraft-geom --target wasm32-unknown-unknown` - command
  attempted, exit 1 / E0463: target standard library is absent. WASM compilation
  is **not run**; no target installation or outbound download was attempted.
- [x] `cargo test -p beamcraft-geom --release` - PASS, 51 math tests.
- [x] `git diff --check` - PASS for tracked changes; explicit Python UTF-8,
  no-BOM/LF and whitespace validation covers the newly added files too.

The math suite comprises one unit test and 50 integration tests, representing
42 distinct F06 case IDs. `crates/geom/README.md` contains the complete grouped
case map and explicitly distinguishes math projections from whole-kernel
acceptance. Positive/negative tests cover both frame branches, up/down axes,
slopes, the corrected +Y fallback, exact threshold equality and adjacent f64s,
positive/negative dot certificate injections, all seven residual bounds,
zero axes, NaN/Inf, finite overflow and norm underflow, range/cap precedence,
scaled/absolute floors, plane bands, signed zeros and canonical byte encoding.
Omitted and signed-zero roll have bit-exact equal frames. Native debug and
release assert the same recorded golden bits for ordered arithmetic/transforms.
A deterministic synthetic axis grid exercises signed meridians and scales.

The tests do not establish cross-target runtime conformance. F06's approximate
frame residual gate is implemented; rigorous accuracy enclosures, exact contour
predicates and mesh/quantity certificates are outside this task.

## Never-crash

- [x] No new `unwrap`/`expect`/`panic!`/`todo!` outside tests; `unwrap_or` only
  supplies the explicitly specified default roll.
- [x] No indexing or slicing with input-derived positions in production code.
- [x] No input-sized allocations, recursion or unsafe code.
- [x] Plate count is bounded at 1024 before traversal; coordinate, dimension
  and local-size caps precede geometric minima.
- [x] Nonpositive section dimensions contribute zero to the half-diagonal;
  huge negative inputs cannot overflow the scale derivation.
- [x] A nonfinite derived scale fails without a substituted floor.

## Provenance (clean-room)

- [x] Contributor-original code and synthetic math tests under MPL-2.0.
- [x] No asset or behaviour copied from commercial detailing products.
- [x] No Tekla/Trimble binaries, APIs, documentation examples or observations used.
- [x] No GPL/AGPL or other upstream implementation copied.
- [x] No new dependency, external asset, catalogue, fixture file or licence text.

Normative behaviour and case inputs come only from frozen F06 0.1.0 sections
2 (finite input and arithmetic), 4 (scale, default frame, roll and certificate),
7 (body/world spaces and canonical f64 bytes) and 10 (bounded work). The
underlying public math sources are the IEEE 754-2019 binary64 arithmetic/sqrt
reference cited by F06 and elementary right-handed dot/cross products and basis
maps. No external source implementation was read or imported for this task.
The later owner decision in the G01 evaluation and task determines rotation
scope. Other required repository design documents were reviewed for context;
older draft constants or proposed auxiliary arithmetic do not override F06.

New text documents implementation, case coverage and original test derivations;
no third-party non-code material is included. Existing asset/licence files,
ATTRIBUTION, frozen contracts and ROADMAP remain unchanged within the allowlist.

## LIMITS and review gates

1. **OPEN owner decision: reproducible sin/cos.** Nonzero member roll goes through
   the single private `roll_sin_cos` function using separate `f64::sin` and
   `f64::cos` calls. Omitted/zero roll bypasses trig. No trig backend/profile is
   chosen and no cross-target byte identity is claimed.
2. WASM target is missing; portability compilation and runtime conformance remain
   open. The library adds no UI/OS or other dependencies.
3. **Proposed F06-POS-09 erratum:** final Y is not bit-exact `(0,1,0)` after the
   mandated step c. With dx=0.0005 and dz=1000, written binary64 expressions give
   Y.y=`1.0000000000000002` (bits `0x3ff0000000000001`). The independent scalar
   calculation `ell=sqrt(dx*dx+dz*dz); zx=dx/ell; zz=dz/ell;
   n=sqrt(zz*zz+zx*zx); xx=zz/n; xz=-zx/n; yy=zz*xx-zx*xz` agrees. The regression
   follows the normative operation order and asserts these bits; approximate
   reference comparisons also pass. Amend the case's exact-Y statement; do not
   silently skip re-orthogonalization or alter the frozen file.
4. **Rotation scope amendment needed:** F04/F06 quaternion placement text and
   quaternion-dependent cases POS-11/POS-12/POS-25, plus F08's reserved `quat`
   module, need reconciliation with the later binding owner decision. This
   implementation has translation-only authored placement, plates as drawn,
   derived body bases, and member-only local-Z roll. No general orientation or
   quaternion is added. Frozen contracts are unchanged.
5. The legacy raw `Point3` API remains for scaffold compatibility. Callers must
   use checked methods at kernel boundaries. Full diagnostics/F04 mapping and
   result/state/cancellation semantics belong to subsequent integration tasks.

Keep this PR **draft** while WASM verification and contract review gates are open.
No merge is authorized by creating the PR.
