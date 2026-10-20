# G01: math-crate evaluation, version 0.1.0

Date: 2026-10-15. Task: G01, owner-rescoped dependency selection and thin adapter spike.
Worktree: `D:\Projects\01_BeamCraft-G01`. Status: **uncommitted**.
Requested branch: `feat/g01-math-crates-spike`, based on `origin/master`. Actual branch,
HEAD and base are unverified: Git rejected repository ownership. No workaround, staging,
commit, push or Git metadata write was attempted.

## Owner decision (2026-10-15, after this evaluation)

**Do not adopt glam or nalgebra.** A library is worth taking only for its logic; here only the
small scalar types would be taken, and BeamCraft can write those itself. The F06 ordered
arithmetic and certificates stay in BeamCraft's own types. The owner also fixed the scope of
rotation: plates are never rotated (they stay as drawn), and only members roll, about their own
local Z axis. No general orientation, and so no quaternion, is needed in the kernel, which removes
the operations where glam and nalgebra differed from F06. The results below remain as evidence.
The sections "Result and scope" and "Recommendation in five lines" show the original proposal
(glam as the type provider) and are superseded by this decision. The open question that remains
is how `sin`/`cos` for a member's roll are made reproducible (section "Risks and remaining owner
decisions").

## Result and scope

Original proposal (superseded, see the owner decision above): propose **glam 0.34.1** for\nfixed-size f64 math types, with a BeamCraft adapter owning F04
admission, F06 operation order, canonicalization and frame certificates. Use **approx 0.5.1
only in tests**. Retain **robust 1.2.0 as a test-only predicate candidate** until its numeric
domain and permission to use auxiliary exact predicates are reviewed. Omit nalgebra from the
first-slice runtime: its broader dependency closure brings no necessary benefit here.

This is dependency-selection evidence, not kernel integration, F06 freeze, K0 acceptance or
completion of the broader runtime G01 allocation in F08. All integration edits below are
proposals, unapplied. The owner's permission to use permissive math libraries supersedes
G04's earlier "no new dependency" allocation for this task.

Authored paths are confined to `spikes/g01-math-crates/**` and this report. The spike retains
its own empty `[workspace]`, lockfile and manifest lints. The only manifest change adds glam
features `f64` and `scalar-math`; the initial std-only selection does not expose all f64 types
in glam 0.34. The root workspace does not depend on the spike. Its local `.gitignore` excludes
`/target/`. Root Cargo files, production code, contracts, policy, roadmap and attribution
were not edited.

Files: `src/lib.rs` is the experimental adapter; `tests/conformance.rs` holds the independent
scalar reference and native comparisons; `tests/predicates.rs` holds original predicate probes;
`tests/support/mod.rs` contains positive F06 numeric inputs. `evidence/` records the active
crate/feature trees, inventory/licence hashes, raw check output and conformance CSV.

## Environment and inventory method

- rustc 1.96.0 (ac68faa20 2026-05-25), LLVM 22.1.2; cargo 1.96.0 (30a34c682 2026-05-25).
- Stable Clippy from that toolchain; host x86_64-pc-windows-msvc; Windows PowerShell.
- Installed targets: x86_64-pc-windows-msvc and aarch64-pc-windows-msvc. No wasm target.
- RUSTFLAGS and CARGO_ENCODED_RUSTFLAGS unset. Default target cfg: cmpxchg16b, fxsr, sse,
  sse2, sse3; **no fma, AVX or AVX2**. Debug and release conformance tables are identical.
- No fast-math or explicit mul_add in adapter/reference. Rust ordinary arithmetic does not
  contract multiply/add into FMA. Inspected native f64 paths do not explicitly fuse; differences
  below follow formula/order changes, not contraction.
- glam's scalar-math disables its explicit SIMD backend. LLVM may select SIMD instructions for
  independent operations without reassociating reductions; no disassembly audit was run.
  nalgebra std includes wide/safe_arch and matrixmultiply (including its avx512 feature), but
  fixed Vector3<f64>/Matrix3<f64> paths are not thereby proven to execute those general kernels.

All resolution/builds were offline. `cargo metadata --offline --format-version 1`,
`cargo tree --offline` and its `-e features` variant supplied the inventory. Cached original
package manifests/licence files and selected source were read. `evidence/inventory.json`
records original manifest and licence-file SHA-256 hashes, versions, features and metadata;
`tree.txt` and `features.txt` distinguish active host dependencies from metadata-only entries.
Upstream URLs identify publishers recorded by the cached packages; no URL was visited.

| Candidate | Version | SPDX licence | Declared rust-version | Default features | Enabled for main comparison | no_std capability |
|---|---|---|---|---|---|---|
| nalgebra | 0.35.0 | Apache-2.0 | 1.89.0 | std, macros | defaults off; std, which activates matrixmultiply | Yes, defaults off with appropriate libm backend; not compiled here |
| glam | 0.34.1 | MIT OR Apache-2.0 | 1.85.0 | std, all-types | defaults off; std, f64, scalar-math | Yes, defaults off with libm/nostd-libm; not compiled here |
| robust | 1.2.0 | MIT OR Apache-2.0 | Not declared | No default set | no features; supplemental no_std feature check passed | Yes, no_std feature; no dependencies |
| approx | 0.5.1 | Apache-2.0 | Not declared | std | default, std | Yes, defaults off; not separately compiled here |

Original publisher repositories recorded in the cache:
[nalgebra](https://github.com/dimforge/nalgebra),
[glam](https://github.com/bitshifter/glam-rs),
[robust](https://github.com/georust/robust),
[approx](https://github.com/brendanzab/approx).
Licence texts were present for every active package; file names and hashes are in the inventory.
No upstream implementation or licence text was copied into the repository.

The active nalgebra transitive closure is every package below except glam and robust.
approx's closure is num-traits plus build-only autocfg. glam and robust have **zero selected
transitive dependencies**.

| Active transitive/candidate package | Version | SPDX licence |
|---|---|---|
| approx | 0.5.1 | Apache-2.0 |
| autocfg (build only) | 1.5.1 | MIT OR Apache-2.0 |
| bytemuck | 1.25.2 | Zlib OR Apache-2.0 OR MIT |
| glam | 0.34.1 | MIT OR Apache-2.0 |
| matrixmultiply | 0.3.11 | MIT OR Apache-2.0 |
| num-complex | 0.4.6 | MIT OR Apache-2.0 |
| num-integer | 0.1.47 | MIT OR Apache-2.0 |
| num-rational | 0.4.2 | MIT OR Apache-2.0 |
| num-traits | 0.2.19 | MIT OR Apache-2.0 |
| rawpointer | 0.2.1 | MIT OR Apache-2.0 |
| robust | 1.2.0 | MIT OR Apache-2.0 |
| safe_arch | 1.2.0 | Zlib OR Apache-2.0 OR MIT |
| simba | 0.10.2 | Apache-2.0 |
| typenum | 1.20.1 | MIT OR Apache-2.0 |
| wide | 1.7.1 | Zlib OR Apache-2.0 OR MIT |

Historical manifest syntax MIT/Apache-2.0 is normalized to SPDX OR above; raw values remain in
inventory.json. Full metadata also contains inactive optional glam 0.30.10, 0.31.1, 0.32.1,
0.33.12 and num-bigint 0.4.8 (all MIT OR Apache-2.0). They are absent from the active cargo tree
and were not compiled/evaluated as candidates. The inventory preserves rather than conflates them.

**No third-party licence lies outside the allowlist.** The isolated project-owned spike is
MPL-2.0; it is outside root cargo-deny's graph. No policy exception is needed. The dependency
closure is pure Rust: no C/C++ build, BLAS/native SDK or external service. Pure Rust does not
mean unsafe-free: nalgebra and performance dependencies contain internal unsafe/intrinsics;
BeamCraft's adapter forbids unsafe. Fixed-size APIs and cfg-selected portable implementations
make the closure plausible for wasm32-unknown-unknown, judged from features/dependencies.
**WASM compilation was not run.** std support on wasm and no_std capability are separate issues.

## API fit and adapter

Small native API constructions, concretized in the compiled tests:

```rust
let v = glam::DVec3::new(3000.0, 0.0, 0.0);
let q = glam::DQuat::from_xyzw(x, y, z, w);
let r = glam::DMat3::from_quat(q);
let world = r * v + translation;

let v = nalgebra::Vector3::<f64>::new(3000.0, 0.0, 0.0);
let q = nalgebra::Quaternion::new(w, x, y, z); // constructor w first!
let q = nalgebra::UnitQuaternion::new_normalize(q);
let r = q.to_rotation_matrix();
```

Tests also exercise Isometry3::from_parts(Translation3, UnitQuaternion).transform_point and
DAffine3::from_rotation_translation(...).transform_point3(...). Both crates provide f64 vectors,
dot/cross, matrices, rigid transforms, Hamilton composition (i*j=k asserted), active rotations
and rotate-then-translate. These snippets illustrate API fit, not the approved arithmetic path.
approx is a comparison library, not a vector/pose library; robust provides predicate coordinates
and signs, not vectors/rotations. Native math conformance for those two is **not applicable**.

Neither math constructor enforces F04 admission. The adapter rejects nonfinite orientation,
components outside [-1,1] and squared-norm error above 1e-12. It evaluates
`n=((x*x+y*y)+z*z)+w*w`, divides each component by sqrt(n), applies canonical sign (w positive,
otherwise first nonzero x/y/z positive) and cleans zeros. Authored values are not rewritten.
Tests cover zero quaternion, invalid norm, NaN, accepted norm margin and translation caps.
Canonical q/-q equality is asserted for every comparison sample and all positive-case
quaternions. Pose output is bit-identical for antipodes. The +90-degree +Z example maps local
(3000,0,0) to world (100,3200,300), compared under a test-only absolute tolerance.

The adapter writes the nine F06 matrix formulas and ordered row dots plus translation. Pose
keeps admitted fields private and rejects nonfinite intermediates. Derived member corners can
exceed the authored 1e9 cap, so world() checks their finiteness rather than imposing a new cap.
The experimental Error enum is not a full F06 diagnostic envelope or staged validator.

Frame::new follows F06 section 4 rule 2: ordered differences/norm, strict hh<1e-12*LL,
horizontal projection or projected +Y fallback, normalized Y cross Z, then Y=Z cross X.
The certificate checks absolute norm errors, all pairwise dots and handedness, each <=1e-12.
The 14 positive F06 axes pass; POS-09/10/41/42/46 assert branches; NEG-31/35 inject +1e-9 and
-1e-9 dot failures. A fixed-seed loop attempts 100,000 further axes and certifies admitted
lengths while comparing native cross/dot. A nalgebra version of the scalar branch/projection
recipe is tested on the F06 axes. Neither crate supplies this BeamCraft policy automatically.

**roll_rad conflict:** both support axis-angle APIs, DQuat::from_axis_angle(z, roll_rad) and
UnitQuaternion::from_axis_angle(&Unit::new_normalize(z), roll_rad). An exploratory test rotates
+X to +Y about +Z, preserves Z and certifies the rolled basis. Trigonometry occurs only in tests.
F04/F06 have no admitted roll_rad field; P-3 expresses other roll through placement. F06 forbids
trigonometric kernel functions. Use an admitted placement quaternion, or amend the contract to
specify angle conversion/reproducibility; do not silently add an arbitrary runtime angle.

## Bit-level conformance

The independent reference follows these exact F06 sentences:

> Arithmetic uses binary64 with round-to-nearest-even, only `+ - * /` and `sqrt` (no
> trigonometric or other library functions), no fused multiply-add contraction, and the operand
> order written in this contract.

> A dot product of three terms is `(r1*v1 + r2*v2) + r3*v3`; world coordinate `i` of a vertex
> `(x, y, z)` is `(((R_i0*x + R_i1*y) + R_i2*z) + t_i)`.

Section 4 additionally specifies `L=sqrt((dx*dx+dy*dy)+dz*dz)`, the cross-product component
formulas, left-associated four-term quaternion norm, component division and nine quadratic
matrix entries. The reference writes those expressions as arrays/scalars, independently of
candidate/adapter implementations. F06 normalizes negative zero in outputs and keys. Primitive
comparisons retain raw signed zeros; normalization comparison handles canonical sign/zero
separately, and Pose tests check normalized outputs.

The xorshift64 seed is 0x6a09e667f3bcc909, shifts 13/7/17. It supplies 53-bit uniform fractions,
vector scales at exact powers of two from 2^-30 through 2^20, and normalized four-component
quaternions. Each operation has **100,000 generated samples + 64 positive-case vectors + 19
positive-case quaternions + 15 extra edge combinations = 100,098 samples**. Extras include
cancellation, large origins, signed zero, minimum subnormal components and half turns.
POS-45's sqrt(10025) is separately asserted. Frame/injected-error tests add coverage; this is
not an implementation of all 83 F06 cases or uniform sampling of the entire binary64 domain.

Native normalization is measured separately. Rotation/matrix conversion then receive the
**same reference-normalized quaternion**, isolating algorithms. Matrix-vector and world rows
use the **same F06 reference matrix**, not each crate's native quaternion matrix. Consequently,
zero world mismatches do not certify an Isometry3/native quaternion-based pose pipeline.

A sample mismatches if any component differs by f64::to_bits(). Maximum ULP is monotonic
integer bit-distance over finite components, including zero crossings; signed zeros are adjacent
in that metric. The CSV also preserves component mismatch counts, signed-zero counts,
nonfinite counts and maximum absolute errors.

| Crate / operation | Samples | Mismatching samples | Max ULP | Result |
|---|---:|---:|---:|---|
| nalgebra/dot | 100098 | 0 | 0 | Identical in this run |
| nalgebra/cross | 100098 | 0 | 0 | Identical in this run |
| nalgebra/norm | 100098 | 0 | 0 | Identical in this run |
| nalgebra/quat_normalize | 100098 | 12411 | 2 | Last-bit differences |
| nalgebra/quat_matrix | 100098 | 95937 | 4372995238176751617 | Different evaluation structure; near-zero residuals |
| nalgebra/quat_rotate | 100098 | 96048 | 2093470139285504 | Different evaluation structure; near-zero residuals |
| nalgebra/matrix_vector | 100098 | 0 | 0 | Identical in this run |
| nalgebra/world | 100098 | 0 | 0 | Identical in this run |
| glam/dot | 100098 | 0 | 0 | Identical in this run |
| glam/cross | 100098 | 0 | 0 | Identical in this run |
| glam/norm | 100098 | 0 | 0 | Identical in this run |
| glam/quat_normalize | 100098 | 25317 | 1 | Last-bit differences |
| glam/quat_matrix | 100098 | 0 | 0 | Identical in this run |
| glam/quat_rotate | 100098 | 97976 | 4424628304217112577 | Different evaluation structure; near-zero residuals |
| glam/matrix_vector | 100098 | 0 | 0 | Identical in this run |
| glam/world | 100098 | 0 | 0 | Identical in this run |
| adapter/dot | 100098 | 0 | 0 | Identical in this run |
| adapter/cross | 100098 | 0 | 0 | Identical in this run |
| adapter/norm | 100098 | 0 | 0 | Identical in this run |
| adapter/quat_normalize | 100098 | 0 | 0 | Identical in this run |
| adapter/quat_matrix | 100098 | 0 | 0 | Identical in this run |
| adapter/quat_rotate | 100098 | 0 | 0 | Identical in this run |
| adapter/matrix_vector | 100098 | 0 | 0 | Identical in this run |
| adapter/world | 100098 | 0 | 0 | Identical in this run |

All rows match between debug and release. No compared result was nonfinite. Native quaternion
rotation has one signed-zero component difference per crate. Large ULP distances near zero
must not be described as one or two ULPs: nalgebra's matrix maximum absolute difference is
6.661338147750939e-16, but a cancellation residual versus zero spans many representable values.
Native rotation maximum absolute differences are 4.673004150388543e-8 (nalgebra) and
1.287460327150519e-8 (glam). These are changed evaluation structure/near-zero residuals,
not a handedness or component-order mismatch.

Cached source explains the findings:

- nalgebra src/base/blas.rs special-cases three-element dot as a+b+c; norm uses dot and sqrt.
  Fixed-size matrix multiplication matched. Quaternion normalization uses a different four-term
  reduction (up to 2 ULP here). quaternion.rs::to_rotation_matrix forms diagonals from w*w,
  x*x, y*y, z*z instead of 1-2*(...). quaternion_ops.rs direct vector rotation uses a cross
  identity instead of the prescribed matrix path.
- glam src/f64/dvec3.rs dot/length/cross and dmat3.rs::mul_vec3 have the required scalar order
  for these tested finite values. Quaternion normalize uses reciprocal multiplication instead
  of component division (up to 1 ULP here). dquat.rs::mul_vec3 uses vector/dot/cross algebra.
  dmat3.rs::from_quat doubles components before products: all tested matrix entries matched,
  but that is not the literal contract expression or a proof over the full underflow domain.
- Both can honor ordered dot/cross/norm/matrix-vector operations through these fixed f64 paths;
  composing explicit scalar components supplies the literal order for either crate. Keep ordered
  quaternion normalization and nine matrix formulas in BeamCraft. Do not silently substitute
  native direct quaternion rotation, reciprocal division or generic dynamic matrix kernels.

Sampling on one host/compiler/two profiles is not a universal proof. F06 still needs every-target
conformance, gradual underflow and a round-to-nearest-even environment. No FTZ/rounding-register
instrumentation was added. An output-changing algorithm/backend upgrade requires the contract's
algorithm/key version review, even if approximate comparisons still pass.

## Robust predicates and F06 policy

robust provides orient2d, orient3d and incircle. Original fixtures use N=2^27; coordinates are
exactly representable and below 1e9. Expected signs follow integer algebra, not library output.

| Query | Input | Naive | robust | Exact derivation |
|---|---|---:|---:|---|
| orient2d | A=(N,N-1), B=(N+1,N), C=(0,0) | 0 | +1 | N*N-(N-1)*(N+1)=1 |
| orient3d | A=(N,N-1,0), B=(N+1,N,0), C=(0,0,0), D=(0,0,1) | -0 | -1 | Same minor with opposite orient3d sign |
| incircle | A=(N,0), B=(0,N), C=(-N,0), D=(1,-N) | 0 | -36028797018963968 | CCW circle radius N; D outside by squared radius 1; determinant=-2*N*N |
| Coplanarity control | Same three 3D points; D=(1,0,0) | 0 | 0 | All z=0 |
| Underflow limit | A=(0,0), B=(1e-200,0), C=(0,1e-200) | 0 | 0 | Exact submitted-dyadic orient2d determinant is positive |

For future contours, orient2d can certify winding in a specified projection; orient3d can test
vertices against a nondegenerate three-point plane. First prove the reference triple is
noncollinear. Exact signs on rounded constructed vertices do not certify the ideal authored
surface. F06 first-slice inputs are rectangles, not arbitrary contours; no contour feature
is implemented by these probes.

The cached robust source explicitly warns at its orient2d error-bound calculation that adaptive
arithmetic does not guarantee correctness under underflow. The last example demonstrates it.
Finite input and an upper coordinate cap are insufficient. Runtime use needs a proved exponent
domain or reviewed rescaling/exact-dyadic fallback, plus construction-error accounting. No such
fallback is implemented here. A returned zero cannot unconditionally certify coplanarity.

A determinant sign is not a tolerance comparison. F06 names its dimensional floor **eps_dim**,
0.001 mm (the task's generic eps_geom): it rejects short dimensions/endpoints, without snapping
or declaring every small determinant zero. Keep it separate. POS-46 direction (0.5,0,500000)
is not exactly parallel to Z, yet hh<1e-12*LL selects near-vertical; a nonzero exact projected
orientation must not replace that decision. The picking parallel dot threshold likewise remains
the written binary64 comparison.

Certified decisions/no unspecified rounding band can benefit from predicate signs in a proved
domain, with uncertainty handled explicitly. F06's current written f64-only rule does not yet
admit expansion arithmetic as a replacement for specified decisions; G04's auxiliary arithmetic
admission remains open. Recommend robust as a test oracle/candidate now, with production
predicate authority conditional on contract permission and domain proof. incircle is not needed
for the two first-slice boxes, but was tested to assess the candidate.

## Alternatives not evaluated, no network

| Alternative | Licence from prior knowledge, unverified here | Fit / reason deferred |
|---|---|---|
| parry3d / parry3d-f64 | Apache-2.0 | Collision/query geometry; broader than this math scope; absent from cache |
| geo | MIT OR Apache-2.0 | Primarily 2D algorithms, not a 3D pose provider; absent from cache |
| truck-modeling / truck family | Commonly Apache-2.0; every package needs verification | B-rep/CAD exceeds scope; truck-modeling absent from cache; architecture does not authorize it as backend |
| libm 0.2.16 | MIT from prior knowledge, not audited here | Cached possible no_std backend; not needed for std/wasm proposal and not evaluated |

These are leads, not admitted dependencies. No additional crate was needed, downloaded or
vendored. No production proposal relies on an unverified alternative licence.

## Recommendation in five lines

Superseded by the owner decision above (no glam, no nalgebra); kept as the original proposal.
Still valid: items 3 and 4 (approx for tests, robust as a test-only candidate).

1. Pin glam =0.34.1, defaults off, features std/f64/scalar-math, behind BeamCraft types.
2. Keep F06 ordered normalization/matrix/transform operations and certificates in the adapter.
3. Pin approx =0.5.1 for tests only; exact decisions/signatures never use approximate equality.
4. Keep robust =1.2.0 test-only with no_std pending domain proof and contract admission.
5. Omit nalgebra from first-slice runtime; retain the isolated comparison for reevaluation.

BeamCraft still writes finite typed boundaries, staged validation, accuracy/frame certificates,
error envelopes, canonical sign/zeros/encoding, FeatureId, versioned keys, mesh builders and
validators, resource limits and cancellation rules. These are domain contracts, not a reason
to duplicate general linear algebra. No kernel crate is created here.

## Exact proposed edits, none applied (superseded for glam; see the owner decision)

Add to root Cargo.toml [workspace.dependencies], retaining all internal entries:

```toml
# audited first-slice math; G01 evaluation v0.1.0
# Preserve exact versions until arithmetic and target conformance are re-evaluated.
glam = { version = "=0.34.1", default-features = false, features = ["std", "f64", "scalar-math"] }
approx = { version = "=0.5.1", default-features = false, features = ["std"] }
robust = { version = "=1.2.0", default-features = false, features = ["no_std"] }
```

Append to crates/geom/Cargo.toml, keeping its package/lints unchanged:

```toml
[dependencies]
glam.workspace = true

[dev-dependencies]
approx.workspace = true
robust.workspace = true
```

**deny.toml: no change.** MIT and Apache-2.0 are already allowed. No third-party MPL, TOST or
other exception is proposed. Integration must update root Cargo.lock normally and repeat
cargo-deny/target checks; this spike deliberately did not apply those changes.

In docs/contracts/module-boundaries-v0.1.md section 3, replace the paragraph after its heading
and empty table with this exact text:

```markdown
A domain crate may use a third-party crate only if it is listed here with the reason, after the
`AGENTS.md` section 5 review. G01 records pinned versions, licences, selected features and target
limitations in `../design/g01-math-crates-evaluation-v0.1.md`. Only beamcraft-geom uses these
crates in the first slice. approx and robust are dev-dependencies only. robust has no authority
over runtime decisions until its numeric domain and auxiliary predicate arithmetic are admitted.
Cross-target conformance remains an integration gate.

<!-- third-party:begin -->
| Crate | Reason |
|-------|--------|
| glam | beamcraft-geom fixed-size f64 types; pinned scalar features and F06 ordered adapter |
| approx | beamcraft-geom dev-dependency only; test comparisons, never runtime decisions |
| robust | beamcraft-geom dev-dependency only; predicate evaluation pending numeric-domain and contract review |
<!-- third-party:end -->
```

Also replace section 1 rule 2's `section 3 (none today)` with `section 3`. Replace section 2's
sentence `No package has a third-party dependency today: Cargo.lock holds project-owned
packages only.` (retaining its existing backticks when matching) with:

```text
beamcraft-geom uses the audited third-party math crates listed in section 3; Cargo.lock records
their exact resolved versions. No other domain crate adds a third-party dependency in this change.
```

The section 2 geom row retains `-` under Allowed dependencies, since that column is for
internal packages. External names belong in the machine-read section 3 table. The checker
examines dev-dependencies too, so both test crates must be listed. Its two-column external
allowlist grants crate names globally: geom-only/dev-only restrictions also require review,
not an invented parser guarantee. No package is proposed to depend on the spike.

## Risks and remaining owner decisions

Exact pins/lockfiles constrain API churn; every upgrade must repeat source, feature and bit-level
review. Current maintenance/advisory status was not checked online, and these are not claimed to
be the latest versions. Feature unification can broaden backends/assertions/dependencies;
audit the resolved graph rather than a single manifest line. Keep crate types behind the adapter.

Compilation cost is broader for nalgebra's numeric/SIMD closure than glam/robust's zero selected
runtime closures. The initial combined optimized build finished in 24.87 seconds here; that is
not a controlled per-crate benchmark. Warm check timings are preserved in evidence/checks.json.

Evidence covers one Windows compiler/host, two profiles and a finite dataset, not all binary64
inputs or G04 accuracy proofs. WASM/other platforms and floating-point environments remain open.
No test success is promoted to integrated engine/CLI acceptance.

Owner decisions remaining: accept this dependency/adapter proposal; confirm/admit P-1 through
P-3 and unresolved F06 accuracy rules; decide whether arbitrary-angle roll is in scope; admit
auxiliary exact predicates and choose a proved domain/fallback. No response was assumed and
no production changes were applied while these remain proposals.

## Commands and exact results

All required checks passed. `--offline` precedes fmt as a Cargo global option, equivalent to
the task's formatter command without dependency resolution. Test logs add --nocapture to retain
numeric evidence; plain cargo test --offline also passed on continuation. Root builds used
CARGO_TARGET_DIR=spikes/g01-math-crates/target/root-checks (absolute value), keeping generated
writes inside the authorized spike.

| Directory | Command | Exact result |
|---|---|---|
| root | Get-Location | Exit 0; D:\Projects\01_BeamCraft-G01 |
| spike | cargo metadata --offline --format-version 1 | Exit 0; inventory extracted |
| spike | cargo tree --offline | Exit 0; active graph saved |
| spike | cargo tree --offline -e features | Exit 0; feature graph saved |
| spike | cargo --offline fmt --all -- --check | Exit 0; no diff |
| spike | cargo clippy --offline --all-targets -- -D warnings | Exit 0; no warnings |
| spike | cargo test --offline | Exit 0; 5 passed, 0 failed |
| spike | cargo test --offline -- --nocapture | Exit 0; 5 passed, 0 failed; numeric rows saved |
| spike | cargo test --offline --release -- --nocapture | Exit 0; 5 passed, identical numeric rows |
| spike | cargo check --offline --features robust/no_std | Exit 0 |
| root | cargo --offline fmt --all -- --check | Exit 0; no diff |
| root | cargo clippy --workspace --all-targets --offline -- -D warnings | Exit 0; no warnings |
| root | cargo test --workspace --offline | Exit 0; 120 passed, 0 failed |
| root | git status --short | Exit 128; dubious ownership; not bypassed |
| root | git branch --show-current / git rev-parse HEAD origin/master | Rejected for dubious ownership; branch/base unverified |

Initial development attempts encountered console encoding, a wrong-relative-path edit, a Rust
method-pointer type error and test-helper Clippy warnings; all were corrected before the passing
checks. Get-FileHash is unavailable in this shell; Python hashlib supplied the digest instead.
One report-writing command exceeded Windows' command-length limit before process creation;
the report was then written in explicit-UTF-8 LF sections. No blocked action was bypassed.
The supervisor restart left sound spike/evidence files; continuation preserved them, reran tests
and wrote the missing report.

Root Cargo.lock SHA-256 before and after is identical:

```text
7b15824fc67c90a837ad19af9d28e12dea2802797e90f541945ddb8e984497eb
```

The digest is in evidence/root-lock-sha256.txt. Authored text is checked for UTF-8 without BOM,
LF and the specified mojibake patterns. Git could not confirm exclusive changed paths because
of ownership checking; only authorized write paths were used. The orchestrator must verify
branch/base/status before committing. No safe.directory modification or alternate Git access.

**Not run:** wasm compile (target absent); no_std glam/nalgebra/approx builds; ARM execution;
Linux/macOS or cross-compiler/target conformance; disassembly/FMA/rounding-register audit;
full-domain accuracy proof; integrated engine/CLI E2E; cargo-deny/advisories; per-crate performance
benchmarks; and checks of the proposed production graph. Root tests exercise existing code,
not an integrated adapter. The robust no_std feature check does not prove a whole no_std spike.
No network-based verification was attempted.

## Proposed ROADMAP and ATTRIBUTION text, none applied

Proposed ROADMAP.md progress entry:

```markdown
- 2026-10-15: G01 evaluated cached nalgebra 0.35.0, glam 0.34.1, robust 1.2.0 and approx 0.5.1 in an isolated offline spike. It proposes pinned scalar glam types with an F06 ordered adapter, approx for tests and robust as a test-only predicate candidate pending numeric-domain and contract review. Each compared operation used 100,098 samples in debug and release; native quaternion normalization/rotation differ from F06, while the adapter matched the written scalar reference. Spike and workspace format, Clippy and tests passed; the root lockfile is unchanged. Git ownership blocked branch/base/status verification; WASM and cross-target conformance were not run. No production dependency or kernel feature is integrated and K0 is not passed.
```

Proposed ATTRIBUTION.md row:

```markdown
| `docs/design/g01-math-crates-evaluation-v0.1.md`, `spikes/g01-math-crates/tests/support/mod.rs`, `spikes/g01-math-crates/evidence/**` | BeamCraft contributors | Contributor-original G01 evaluation and synthetic numeric probes, 2026-10-15; positive numeric inputs from F06 0.1.0; cached upstream package/version/licence provenance and hashes recorded in the evaluation and inventory | `MPL-2.0` | Offline dependency decision evidence and ordered-arithmetic tests; no upstream code, binaries, catalogue, artwork or licence text copied into the repository |
```

These original outputs are MPL-2.0. Upstream crates retain their own licences; this row does not
relabel dependencies. The attribution proposal remains unapplied because this task explicitly
forbids editing ATTRIBUTION.md.
