# N04: truck B-rep evaluation, version 0.1

Date: 2026-10-16. Isolated offline research spike, task N04.
Worktree: `D:\Projects\01_BeamCraft-N04`; branch `feat/n04-truck-spike`.
Verified HEAD and local `origin/master` at entry: `adbc407e09476a803de07c31f3a5adcaabad1304`.
The owner supplied this as the verified integration base; no remote refresh was attempted offline.
Publication state: committed on the branch; not pushed. The orchestrator pushes and opens the PR.
`spikes/n04-truck/PR.md` contains the concrete draft description.

## Scope and recommendation

**Proposal for the owner: reject the pinned, default-feature stack as BeamCraft's holes-and-cuts
backend for now.** Rectangular and concave extrusion work in this bounded sample, and ordinary
plane intersections often have correct volume. Degenerate cuts, cylinder subtraction, mesh
conformance, stable bytes, provenance and bounded execution leave substantial work. This is a
proposal, not an adoption decision, a proof that every truck configuration fails, or permission
to replace BeamCraft's kernel. A narrower future contour-only evaluation could still be useful.

The evaluated crates are exactly `truck-modeling =0.6.0`, `truck-shapeops =0.4.0` and
`truck-meshalgo =0.4.0`, using the prepared manifest's defaults. No dependency, patch, foreign
source, catalogue or fixture was added. No BeamCraft workspace crate, root manifest/lockfile,
deny policy, attribution, CI or roadmap was edited. The isolated manifest remains a private
MPL-2.0 package. BeamCraft retains its own small math types under the G01 owner decision;
any future adapter would use truck for algorithms behind those types.

F06 0.1.0 sections 3, 4, 6 and 10 are the comparison boundary. They currently exclude operations
and nonrectangular evaluation. A02/A03/A04/G09 need amended acceptance, topology and FeatureId
rules before integration. Successful spike measurements do not widen that scope, pass K0,
complete those tasks, certify arbitrary solids or implement an engine/CLI feature.

## Environment and evidence

Windows x86_64-pc-windows-msvc, Rust/Cargo 1.96.0; rustc
`1.96.0 (ac68faa20 2026-05-25)`. Debug and optimized release use the same lockfile.
Only x86_64-pc-windows-msvc and aarch64-pc-windows-msvc targets are installed; ARM was not run.
All Cargo resolution/builds/checks use offline mode or `CARGO_NET_OFFLINE=true`.
No browser, network, subagent, external geometry oracle or other checkout was used.

Evidence paths are relative to `spikes/n04-truck/`:

| Evidence | Meaning |
|---|---|
| `src/lib.rs`, `src/main.rs`, `tests/probes.rs` | Original public-API experiment, one-case CLI and actual extrusion/recovery tests |
| `oracle.py`, `run.py` | Independent mathematical references, process deadlines and scalar triangle/topology audit |
| `evidence/expectations.json` | Analytic dimensions, volumes, genus expectations, local floors and formula labels |
| `evidence/results-debug.json`, `evidence/results-release.json` | Every final invocation, status, numeric measurements, hashes, stderr stage and elapsed time |
| `evidence/supplement.json` | Two separate longer-budget calls, release pattern and debug hole, with 120-second deadlines |
| `evidence/case-results.md`, `evidence/determinism.md`, `evidence/summary.json` | Derived complete case and reproducibility tables; reproducible with `summarize.py` |
| `evidence/inventory.md`, `evidence/inventory.json` | Complete metadata package/version/SPDX/checksum table and licence-file hashes |
| `evidence/resolution.json`, `evidence/tree.txt`, `evidence/features.txt` | Cargo metadata resolve graph and native dependency/feature trees |
| `evidence/checks.json`, `evidence/check-*.txt` | Exact required commands and actual results, including missing-target diagnostics |
| `evidence/root-lock-sha256.txt` | Root lockfile digest, identical before/after verification |
| `PR.md` | Review-ready draft PR description and proposed dated roadmap/attribution entries |

Raw numeric triangle/mesh bytes remain under ignored `target/probes`; final evidence retains their
SHA-256 digests. No screenshot, third-party geometry asset, upstream source/test or licence text
is copied into deliverables. All authored data/documents are contributor-original, MPL-2.0;
dependency licence declarations do not relabel upstream material.

## Public API and methodology

Local `cargo doc --offline` generated public documentation from the cached packages. Only rendered
API documentation, manifests, Cargo VCS packaging metadata, checksums and licence/notice files
were inspected; source links, upstream implementation bodies and tests were not opened. Compiler
diagnostics and panic backtraces necessarily name upstream functions/locations. Those observations
are runtime evidence, not source inspection.

Relevant public documentation references, available locally at these exact versions:

- [truck-modeling builder](https://docs.rs/truck-modeling/0.6.0/truck_modeling/builder/index.html):
  vertex/line, translational and rotational sweep, planar-face construction.
- [truck-shapeops `and`](https://docs.rs/truck-shapeops/0.4.0/truck_shapeops/fn.and.html):
  intersection of two solids with caller tolerance, returning `Option<Solid>`.
- [MeshableShape](https://docs.rs/truck-meshalgo/0.4.0/truck_meshalgo/tessellation/trait.MeshableShape.html)
  and [RobustMeshableShape](https://docs.rs/truck-meshalgo/0.4.0/truck_meshalgo/tessellation/trait.RobustMeshableShape.html):
  tessellation can leave a solid's mesh open; robust tessellation projects boundary vertices and
  does not promise closure even after the documented optimization filter.
- [truck-base tolerance](https://docs.rs/truck-base/0.5.0/truck_base/tolerance/constant.TOLERANCE.html):
  the documented general constant is `1e-6`, independent of body size.
- [Solid](https://docs.rs/truck-topology/0.6.0/truck_topology/struct.Solid.html) and
  [Face](https://docs.rs/truck-topology/0.6.0/truck_topology/struct.Face.html):
  fallible constructors coexist with panicking/debug-only constructors and runtime face identities.

The hyperlinks identify public references; no online freshness or upstream maintenance claim is
made. Public formulas below and the frozen F06/G03 references are the only sources of expected
behaviour. Truck output never supplies an expected quantity or topology.

The primary plate is `[0,100] × [0,80] × [0,10]` mm. Boxes are built by three translational
sweeps. The cylinder has radius 5, base z=-10, height 30; therefore its end caps are outside the
plate. A rotational sweep constructs the circular wire, a plane closes it, and extrusion makes
the tool. Subtraction uses intersection with the reversed tool (`tool.not()`), the oriented-solid
complement convention being explicitly exercised. A failed result is not retried after silently
perturbing coordinates. Pattern subtraction is four sequential cylinders at `(25,20)`, `(75,20)`,
`(25,60)`, `(75,60)`; this is not a batch-operation benchmark.

The plane experiment uses an oversized finite prism as a half-space adapter: its base is the
target plane and it extends in `-normal`. Every specimen corner is less than 150 mm from the
plane's chosen centre, whereas the four side planes and far plane are at least 1000 mm away.
Those other planes cannot clip these specimens. This tests solid intersection with a half-space
equivalent on the specimen, not a claimed dedicated infinite-half-space API.

Measurements enumerate actual B-rep vertices/edges/faces, oriented edge incidence and shell
conditions. For a genus-zero trimmed face with b boundary loops, its contribution is `2-b`;
the shell Euler value is `V-E+sum(2-b)`. Counting every multiply trimmed cap as one disk would
give a misleading Euler result for holes. Integer topology expectations are exact. Euler plus
closed incidence is not a generic proof of freedom from self-intersection.

Tessellation returns a solid whose face surfaces are `Option<PolygonMesh>`. The probe explicitly
counts missing surfaces **before** converting to one mesh; `to_polygon()` returning triangles is
not treated as complete success. The independent volume uses signed tetrahedra about a local
anchor and compensated accumulation. Python repeats the measurement with explicit scalar
determinants and `math.fsum`, checks indices, directed incidence, unused vertices and bounds, and
checks its answer against the Rust measurement. The library's volume computation is not used.
For an open or partially omitted shell this signed integral is only a diagnostic scalar: it can
depend on the chosen anchor and is not the volume of a valid solid. A reordered partial mesh can
therefore change that scalar even if the surviving sorted triangles are identical. Such output is
never an engineering quantity; completeness and closure must be checked first in any adapter.

Raw indexed topology and exact-position alias topology are reported separately. Alias enumeration
normalizes signed zero only for comparison and never changes or returns a welded/healed mesh.
Small coordinate discrepancies can still leave exact-position seams. There is no generic
self-intersection certification, analytic B-rep integration or manufacturing-accuracy claim.

## Analytic references and formulas

The derivations are original applications of public mathematics. Bibliographic sources already
identified in F06/G04/G03 are cited here without downloading text or code:
[OpenStax, Calculus Volume 3, triple integrals](https://openstax.org/books/calculus-volume-3/pages/5-4-triple-integrals),
[OpenStax, divergence theorem, section 6.8](https://openstax.org/books/calculus-volume-3/pages/6-8-the-divergence-theorem),
and [Euler characteristic](https://mathworld.wolfram.com/EulerCharacteristic.html).
F06 sections 4/6/10 supply policies, not new mathematical algorithms. G03's original rational
references are in `docs/fixtures/g03/expectations.tsv`; six are independently checked by the
Python oracle. No values are copied from another kernel or commercial product.

| Case family | Derivation of the expectation |
|---|---|
| Rectangular plate / G03 boxes | Triple integral of 1 over Cartesian intervals: `V=(a*b)*c`. Bounds are interval endpoints. Eight endpoint combinations, twelve axis edges, six faces; `8-12+6=2`. Canonical F06 triangulation is 8 vertices / 12 triangles / 18 mesh edges. |
| One through hole | Integrate disk area in z: `A=∫0^(2π)∫0^r ρ dρ dθ=πr²`; `V=abc-πr²c=79214.60183660255 mm3`. A regular through hole adds one handle, so boundary chi=`2-2g=0`. |
| 2×2 pattern | Four disjoint disk cylinders: `V=abc-4πr²c=76858.40734641021 mm3`, genus 4, chi=-6. |
| Near edge | Centres x=5.0005 and 5.002 leave positive ligaments 0.0005 and 0.002 mm. Ideal volume and chi equal the ordinary hole; the first ligament is inside the 0.001 mm geometric band. No snapping/merging is authorized by F06. |
| Touching edge | Centre x=5 makes the cylinder tangent to x=0. Removed volume is still `πr²c`; the boundary has singular tangential contact, so ordinary closed-manifold success is not assumed. Explicit failure is acceptable; a panic is still recorded. The ordinary-hole chi in the parameter record is not a certificate for this singular solid. |
| Crossing edge | Centre on x=0, y=40: half the disk is inside the plate. `V=abc-πr²c/2=79607.30091830128 mm3`; a regular open notch leaves genus 0, chi=2. |
| Ordinary plane cuts | Normals `(cos θ,sin θ,0)` at θ=0°,30°,45°,60° through the body centre. Central inversion `x→2*centre-x` exchanges the two sides and preserves volume, proving `V=abc/2`: 40000 for the plate and 500 for the 10 mm cube. Axis bounds follow the positive half-space inequality by setting the other coordinate to zero. |
| Through a vertex | `x+y+z<=10` on nonnegative coordinates, with all extents >=10: `∫0^10∫0^(10-x)∫0^(10-x-y) dz dy dx=1000/6`. The retained tetrahedron has 4 vertices, 6 edges, 4 faces, chi=2; the plane passes through existing cube vertices. |
| Coincident face | Closed half-space `x<=a` contains the whole box, so expected volume and bounds are unchanged. No empty intersection is expected. |
| Concave L contour | Original polygon `(0,0),(100,0),(100,30),(30,30),(30,80),(0,80)`. Disjoint rectangles have area `100*30+30*50=4500`; extrude 10 gives 45000 mm3. Six vertices per cap: B-rep V/E/F=12/18/8, chi=2. |
| Thin slab / wall | Slab `100*80*0.001=8 mm3`. Ring wall: `(100*80-99.998*79.998)*10=3.59996 mm3`, ideal ligament 0.001 mm, genus 1. The wall uses box subtraction, not a copied sample model. |
| Large translation | Translate every construction coordinate by 1e6 mm on each axis. Ideal volume/topology are translation invariant; bounds add 1e6. The independent tetrahedron anchor limits cancellation during measurement without changing returned vertices. |
| Local scale | For 3072×4096, `D=5120`, hence `eps_geom=5/4096=0.001220703125` mm. A 0.001 mm thickness is below that floor. For 300000×400000, `D=500000`, floor=0.11920928955078125 mm. |

Test volumes use CR-TOL-001, `abs(actual-expected)<=1e-6*abs(expected)`; coordinates use
`0.001 mm+1e-6*abs(expected)`. Topology, ids and byte equality have no tolerance. The distinct
geometric admission floor is `max(0.001 mm,D/4194304)` and excludes world translation and thickness.
This probe deliberately passes some below-floor dimensions to truck to identify wrapper work;
their numeric PASS is not valid BeamCraft admission. Operations themselves remain outside F06 0.1.0.

## Case results

PASS means the analytic-volume/B-rep probe met its checks, not F06 buffer compliance. WRONG means
the API returned geometry but the volume, expected B-rep topology/counts, bounds, complete face
tessellation or nondegeneracy check failed. FAIL means explicit error/`None`, post-output audit
cap, or 30-second process timeout; these are distinguished in raw records. PANIC distinguishes
unwinding from a returned error. Singular tangency and invalid-axis controls are labelled above.
The table reports five invocations per case/profile: caught runs 1/1/1/4 threads, then direct/1.
Baseline values are the first release caught invocation; all variations remain in the raw evidence.

<!-- N04-CASES-BEGIN -->
Pending final supervised runs.
<!-- N04-CASES-END -->

<!-- N04-SUPPLEMENT-BEGIN -->
Longer-budget probes pending.
<!-- N04-SUPPLEMENT-END -->

Every rectangular case agrees with the G03-style B-rep reference V/E/F=8/12/6, chi=2 and closed
incidence. A translated 0.001 mm slab measures about `8.00000037997961 mm3` (relative error
`4.749745e-8`), within the 1e-6 test rule. This does not prove every F06 placement or magnitude.
The below-scaled-floor case is accepted numerically by truck: a future guard must reject it.

Ordinary successful cut volumes do not establish robust cuts: coincident faces return `None`,
the cube's 45° case returns `None`, and a cube cut through vertices panics in the sampled calls.
A consistency-check probe is separate: the public `is_geometric_consistent` path can panic in
`Surface::include` after boolean construction has returned an intersection curve. That is a
checker limitation, not evidence that the preceding boolean never returned. Source locations and
exact diagnostics are retained only as observations; no vendor patch was designed from source.

## Panic isolation, debug/release and determinism

Each case runs in a fresh child process, both directly and inside `std::panic::catch_unwind`.
The closure constructs and drops all truck state; no interior-mutable solid is reused after an
unwind and no `AssertUnwindSafe` or unsafe code is needed. Direct panics terminate only their child.
`catch_unwind` cannot catch aborts, stop a nonterminating call, recover allocation failure, prove
absence of dependency undefined behaviour, or implement cancellation. The supervisor kills and
reaps a timed-out child. Its 30-second limit is an experimental budget, not a proved F06 latency.
Profiles may run concurrently; elapsed times are observations under load, not comparative benchmarks.

The invalid rotational-axis control deliberately passes `(0,0,2)` where public docs require a
normalized axis and warn of a debug-only panic. Its profile difference must not be generalized to
valid input. The below-tessellation-tolerance control exercises the documented 1e-6 lower bound;
the equal case records actual inclusive behaviour without rewriting the documentation's wording.

Raw mesh bytes contain f64 positions and emitted position indices; raw triangle bytes contain nine
f64 coordinates per triangle in emitted order. Three caught runs at one thread and a changed
`RAYON_NUM_THREADS=4` process compare exact bytes using SHA-256 and retained per-run digests.
Environment changes request the Rayon pool size; no additional Rayon dependency or affinity API is
introduced to instrument every upstream scheduling choice. A diagnostic sorted-triangle hash
ignores triangle order and cyclic starting corner but preserves winding; it is not a canonical
BeamCraft encoding. Different hashes may represent different order, diagonals or constructed points.

<!-- N04-DETERMINISM-BEGIN -->
Pending final supervised runs.
<!-- N04-DETERMINISM-END -->

No triangle result means byte determinism is **not measured**, even if errors/panics repeat.
Aggregate counters include the explicitly invalid precondition controls, singular tangency and
optional consistency-check probes; the per-case rows separate them from ordinary valid operations.
One host, compiler, dependency graph and finite set do not prove cross-platform determinism.
The default graph contains randomized/hash-based machinery; byte comparisons, rather than that
dependency fact, are the evidence. No universal explanation of upstream nondeterminism is inferred.

## Mapping to BeamCraft

| Need | Observed fit and missing work |
|---|---|
| FeatureId and face provenance | `and` returns only a Solid, no generated/modified/deleted input-face history. Runtime face ids can recognize some unchanged faces (the ordinary hole retains four original plate faces), but split faces receive new ids with no reported parent. New constructions also receive new identities. These are not fid1 semantic names or persistent history. Surface matching alone is ambiguous for coincident/repeated faces. Need original operation/face history, stable split naming, deletion rules, and remeshing maps. |
| Mesh output, F06 section 6 | f64 positions are useful, but polygon meshes have separate position/normal/UV indices and platform-sized indices; conversion returns no BCMESH01 blob, u32/u16 tables, canonical roles or digest/key. A box returns 24 positions / 12 triangles and raw chi=6, with 24 boundary mesh edges; exact-position aliases give 8 vertices / chi=2. Raw indexing fails F06 closure. The probe never welds it into a claimed conforming result. |
| Mesh completeness and quantities | Meshed surfaces can be None; flattening can still return a partial mesh. Holes can exceed both the 1e-6 quantity comparison and the vertex/triangle caps, especially on refinement. Analytic boxes can keep analytic quantities; arbitrary boolean volumes need a new validated/certified quantity path. A closed B-rep is insufficient to certify the emitted mesh. |
| Bounds and finite input | No supplied F06 result/diagnostic envelope, staged finiteness/cap checks, local-size policy or numeric certificates. Need typed numeric conversion, local construction before placement, input/output budgets and explicit uncertain/unsupported errors. Returned sampled bounds are not automatically exact bounds of every curved face. |
| Work limits and cancellation, F06 section 10 | Tested synchronous calls take solids/tolerance and have no token, deadline, allocation budget or progress callback. Checking outputs after allocation cannot bound peak upstream work. Guarded inputs and caught panics alone are insufficient. A separately reviewed worker could be killed with discard proven; in-process use needs cooperative limits/patches with measured latency. No control server is added here. |
| WASM | **Not run:** wasm32-unknown-unknown is absent. The prescribed command was attempted and fails E0463. Host compilation and target-filtered manifests cannot establish browser compatibility. Threading, randomness and optional native dependencies need a separately compiled configuration audit. |
| Fillets | Outside current A02/A03/A04/G09 need; zero implementation allocation proposed now. No claim that generic fillets are easy, available or needed to adopt this scope. |

The public robust tessellation alternative is probed explicitly, without claiming it guarantees
closure or exact engineering quantities. A production mesh adapter cannot merely sort triangles:
F06 first-scope boxes fix vertex order, face order, exact roles and diagonals; future operations
need newly reviewed rules. A topology-aware shared-vertex construction is preferable to silently
welding by distance, which F06 forbids.

## Supply chain and licence evidence

The complete cargo-metadata list is [inventory.md](../../spikes/n04-truck/evidence/inventory.md),
with all declared expressions and **each Cargo.lock checksum**, not just direct dependencies.
There are **135 registry package versions plus the private spike**. All 135 cached `.crate`
archive SHA-256 hashes match Cargo.lock. Native and target-specific package differences are
visible in the resolve/feature trees; the list is not falsely labelled 135 host-linked libraries.
No downloaded code or crate archive is committed.
The three candidates declare no rust-version in metadata; compilation on 1.96.0 does not establish
an older minimum supported Rust version. Cargo emits future-incompatibility warnings for
nom 3.2.1 and quick-xml 0.22.0; passing today's harness Clippy does not promise future-toolchain acceptance.

| Candidate | Version | Declared SPDX | Cargo.lock archive SHA-256 |
|---|---|---|---|
| truck-modeling | 0.6.0 | Apache-2.0 | `8082c2dfac8c2732a014d81807fb5d24dbac0d72ee4de2a705b7fbc05fa05b18` |
| truck-shapeops | 0.4.0 | Apache-2.0 | `fd6e83ea4d36d00229d38ab985a991e32f58c3ed2971083011ac66fb57e26d5c` |
| truck-meshalgo | 0.4.0 | Apache-2.0 | `e18f325738e822c79bb1a48897ec789cbf50cbeeb0841105295f9da85bdf3613` |

Original publisher URL in all three cached manifests is `https://github.com/ricosjp/truck`.
Per-package `.cargo_vcs_info.json` revisions and manifest hashes are in inventory.json. These
are archive packaging claims, not a fresh verification of GitHub ownership or release tags.
**No LICENSE* file is packaged in any of these three crate roots.** No licence was inferred from
a README. Their Apache declarations are recorded, but original repository licence/NOTICE retrieval
and exact package/file attribution remain admission gates. This report does not invent missing
licence texts or consider the task's Apache label a complete redistribution audit.

Every declared expression was screened against the current deny.toml allowlist, retaining the
original string. Legacy `MIT/Apache-2.0` is interpreted as a dual choice for this metadata screen
only; a future actual cargo-deny run must verify its treatment. `AND` obligations must all be
satisfied, including unicode-ident's Unicode-3.0; `WITH LLVM-exception` is compared as the full
allowed expression. Every registry package has a selectable allowed declaration. The out-of-list
identifiers, including those avoidable by choosing an allowed alternative, are:

| Package | Identifier outside allowlist | Declared available treatment |
|---|---|---|
| adler2 2.0.1 | 0BSD | Select MIT or Apache-2.0 from its OR expression |
| aho-corasick 1.1.5 | Unlicense | Select MIT |
| byteorder 1.5.0 | Unlicense | Select MIT |
| memchr 1.0.2 and 2.8.3 | Unlicense | Select MIT |
| beamcraft-n04-truck-spike 0.0.1 | MPL-2.0 | Private `publish=false` package; deny policy ignores private packages; no production exception added |

Available dependency licence files were read and hashed; missing packaged texts are explicit
`none packaged` entries. This is a declaration-and-file-presence audit, not a complete file-level
licence proof of the transitive graph. `cargo deny` is **not run**, because cargo-deny is absent;
there is consequently no advisory/yanked/security clean bill and no network DB fetch.

The default feature graph is broader than the geometry need: truck-meshalgo enables VTK output
and truck-shapeops enables step-test, pulling STEP parsing and compression. In particular
`vtkio -> xz2 -> lzma-sys 0.1.20` builds a native C dependency, so this **default configuration is
not a pure-Rust closure**. The packaged `xz-5.2/COPYING` summary says liblzma is public domain,
while scripts/build material include GPL and a conditional getopt route can involve LGPL.
Those are per-file/usage distinctions, not a GPL declaration in Cargo metadata. No GPL source
was read or copied into BeamCraft. The exact native build/link/file closure was not audited and
is not admitted for redistribution. Its licence-summary hash is recorded in inventory.json.
Disabling unused features is a candidate mitigation; feature unification and the resulting graph
must be checked before claiming native code disappears. No minimal-feature build is claimed here.

## What adoption would require; estimated effort

These are rough engineering estimates in person-hours, not measured effort, commitments or
promises. Tasks overlap; adding every upper bound is not a schedule. A maintainer would need to
refine them after public-API reproductions and an authorized source review.

| Missing piece | Work proposal | Estimate |
|---|---|---:|
| Numeric/input guard and BeamCraft type boundary | Preserve original small types, F06 staged validation, units/local origin, caps, panic-to-diagnostic policy and atomic discard | 16–32 h |
| F06 mesh/result adapter | Shared indexing by construction, semantic triangle/face table, encoding/keys, checks, limits and conformance cases | 32–64 h |
| Face provenance / stable operation features | Original parent-face lineage, split naming, generated-cut/hole identities, deletion/remesh/edit stability tests; likely upstream hooks or maintained patches | 64–120 h |
| Reliable degenerate cuts | Reproduce None/panic cases, define boundary policy without snapping, patch or replace missing paths and prove volume/topology invariants | 40–100 h |
| Cylinder booleans / certified quantities | Fix/reject incomplete surfaces and numeric failures, check refinement and cap interaction, bound quantities for supported analytic families | 80–160 h |
| Bounded execution/cancellation | Reviewed worker discard or cooperative upstream budgets, memory/latency measurements, cancellation tests | 24–48 h |
| Licence/features/targets | Retrieve missing original notices, select/verify optional features, native closure audit or elimination, WASM and cross-target runs | 16–40 h |
| Fillets | No current scope requirement | 0 h now |

A practical truck integration investigation is estimated at **200–400 person-hours** before a
credible scoped acceptance review, with substantial uncertainty and no guarantee the current
geometry paths become suitable. A smaller original implementation of restricted contour extrusion,
analytic through holes and plane cuts is estimated at **100–200 person-hours**, also uncertain;
this is not a general CAD B-rep kernel or an independently measured comparison. Re-evaluate these
ranges against concrete later contracts rather than treating spike wall time as development cost.

No vendored patch is delivered or approved. A guard wrapper is necessary but cannot repair a
missing face, supply lineage, guarantee generic geometry, bound an already-running synchronous
call or make output deterministic. If the owner authorizes another trial, define one small
operation family with numeric/topology/cancellation gates and resolve its licence/feature graph
first. Do not adopt all three default-feature crates solely because rectangular extrusion passes.

Potential future repository edits, **none applied**: exact dependency pins and selected features;
per-package F08 domain allowlist entries and production lockfile audit; a package/version MPL
exception for any new BeamCraft-owned crate if needed; ATTRIBUTION rows for exact upstream packages,
notices, any authorized vendored patch and the original N04 outputs; and acceptance amendments for
new topology/FeatureIds/quantity certification. Apache-2.0 is already globally allowed, so an
unnecessary broad deny exception is not proposed. Any newly discovered non-allowed file needs
its own owner disposition; a package's manifest cannot override its bundled file licences.

## Verification and limitations

<!-- N04-CHECKS-BEGIN -->
Pending final verification.
<!-- N04-CHECKS-END -->

Root checks exercise the existing workspace, not an integrated truck adapter. Spike tests and
the supervised actual calls exercise the new experiment. The WASM command was attempted but
compilation is **not run** without the target; no target/tool/dependency installation is attempted.
All build outputs are inside the writable spike's ignored target directory, including root checks.

Not established: arbitrary input robustness or fuzzing; generic self-intersection and mathematical
exactness; stable persistent operation names; hostile allocation/recursion bounds; reliable native
dependency closure; older MSRV; minimal features; WASM/ARM/Linux/macOS/cross-compiler execution;
advisory status or upstream maintenance; production engine/CLI/USD integration; complete licence
notice redistribution; clean-room eligibility for later replacement-kernel implementation.

Read-only input documents include AGENTS, CLAUDE, architecture, ROADMAP, F06, the kernel design,
G01's superseding owner decision, N02's evaluation format, G03 originals, deny.toml and ATTRIBUTION.
No other project source/tests or private commercial material was read. Rendered public truck API
examples were visible but no upstream example/algorithm was copied. Compiler tools necessarily
compile cached dependencies. Exposure to the public APIs and runtime results must be considered
in any later clean-room admission. Prior model-training exposure is unknown; fresh context is
not an admission certificate. All findings remain owner-decision evidence only.

The proposed dated roadmap entry and original-output/upstream attribution text are in PR.md.
ATTRIBUTION.md and ROADMAP.md remain untouched under the task's explicit write restriction.
