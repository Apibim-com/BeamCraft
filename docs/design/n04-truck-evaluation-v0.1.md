# N04: truck B-rep evaluation, version 0.1

2026-10-16. N04T trims PR #46 at `5a154b95758284f9854860f074766979e290462c`.
Original integration base: `adbc407e09476a803de07c31f3a5adcaabad1304`; existing branch
`feat/n04-truck-spike`, target `master`. Contributor-original report and probes,
BeamCraft contributors, MPL-2.0. This revision summarizes existing measurements;
no evaluation matrix or supplementary geometry run was added.

## Conclusion and proposal

Reject the evaluated default-feature truck stack as BeamCraft's holes/cuts backend.
The owner decided on 2026-10-16 that holes and cuts use our own implementation,
with circular and oval holes only. This is the record of a rejected route, not an
integration, a general verdict on truck, or completion of A02/A03/A04/G09 or K0.
Oval holes were not evaluated. Rectangular/concave extrusion and several ordinary
plane cuts worked in the sample; singular cuts, inaccurate/incomplete cylinder
meshes, raw indexing, unstable bytes, execution limits and provenance remain gaps.
F06 0.1.0 excludes operations and nonrectangular evaluation; future support needs
its own acceptance amendment.

Pins remain `truck-modeling =0.6.0`, `truck-shapeops =0.4.0`,
`truck-meshalgo =0.4.0`, with the original default features and lockfile.
The [minimal harness README](../../spikes/n04-truck/README.md) gives exact offline
build/test commands and optional reproduction commands. Original runs used Windows
x86_64 MSVC, Rust/Cargo 1.96.0 and Python 3.11.9. No production dependency or
workspace crate, CI, deny policy, attribution or roadmap file changes here.

## Method and result meanings

Expected behaviour comes from public mathematics: OpenStax Calculus Volume 3
[5.4, triple integrals](https://openstax.org/books/calculus-volume-3/pages/5-4-triple-integrals),
[6.8, divergence theorem](https://openstax.org/books/calculus-volume-3/pages/6-8-the-divergence-theorem),
and [Euler characteristic](https://mathworld.wolfram.com/EulerCharacteristic.html).
[F06 sections 3, 4, 6 and 10](../contracts/geometry-contract-v0.1.md) supply result,
tolerance, mesh and resource policies. Original G03 rational references are only
cross-checks of the same box formulas. Public truck API documentation was used to
call the library, never as a source of expected geometry. No third-party source,
test, fixture, catalogue or commercial-product material was copied.

The plate is 100 x 80 x 10 mm. Expected volume is `abc`; a radius-5 through hole
subtracts `pi*25*c`, four holes subtract four such cylinders, and a crossing edge
removes half a cylinder. Near-edge ligaments are 0.0005 and 0.002 mm; touching
contact is singular. Plane cuts use an oversized 1000-mm prism equivalent to a
half-space on these specimens: central inversion proves half volume; a vertex cut
retains a tetrahedron `10^3/6`. Coincident-face cuts should keep the whole box.
The L contour has area `100*30+30*50`, extruded 10 mm. The ring wall reference is
`(100*80-99.998*79.998)*10`. Large offsets add 1e6 mm on each axis.

Independent volume sums signed tetrahedra about a local anchor; Python repeats it
with scalar determinants and `math.fsum`. B-rep chi is `V-E+sum(2-boundary_loops)`;
expected regular boundary chi is `2-2*genus`. A closed B-rep does not certify a mesh.
The raw indexed and exact-position-alias audits are separate; no mesh is healed.
Volume comparison uses relative 1e-6; bounds use `0.001+1e-6*abs(expected)` mm.
Topology and byte equality are exact. The admission floor is separately
`max(0.001, D/4194304)` mm, excluding thickness and translation. `scaled-below`
passes the numerical probe while violating that floor; it is not BeamCraft admission.

PASS means the volume/B-rep probe passes, **not F06 mesh conformance**. WRONG means
returned geometry fails quantities, expected B-rep counts/topology, bounds, complete
surface tessellation or nondegeneracy. FAIL means returned error/None or timeout
(the historical set contains no audit-cap failures). PANIC means unwinding.
Open/partial mesh signed volume is only a diagnostic scalar, never an engineering
quantity. Missing face counts are checked before flattening a mesh.

## Observed per-case results (partial experiment)

The planned schedule was five calls per case/profile: caught one-thread repeats
0/1/2, caught four-thread repeat 0, then direct one-thread repeat 0.
**366/440 calls exist: debug 146/220, release 220/220.** All 44 debug direct calls
and 30 debug four-thread calls are absent. There is no `supplement.json`; the
proposed 120-second pattern/hole probes have no recorded results. The former
343-record summary was stale; the tables below use all 366 raw records.
Numbers are the first release caught one-thread call; no mesh means no quantity.

| Case | Expected mm3 | Debug observed / 5 | Release observed / 5 | Release volume / relative error | B-rep V/E/F; chi | Mesh V/T; missing faces |
|---|---:|---|---|---|---|---|
| plate | 80000 | PASS x 4 | PASS x 5 | 80000 / 0 | 8/12/6; 2 | 24/12; 0 |
| g03-unit | 1 | PASS x 4 | PASS x 5 | 1 / 0 | 8/12/6; 2 | 24/12; 0 |
| g03-unequal | 3243 | PASS x 4 | PASS x 5 | 3243 / 0 | 8/12/6; 2 | 24/12; 0 |
| g03-dyadic | 0.001953125 | PASS x 4 | PASS x 5 | 0.001953125 / 0 | 8/12/6; 2 | 24/12; 0 |
| g03-decimal | 0.006 | PASS x 4 | PASS x 5 | 0.006 / 1.4456e-16 | 8/12/6; 2 | 24/12; 0 |
| thin-slab | 8 | PASS x 4 | PASS x 5 | 8 / 0 | 8/12/6; 2 | 24/12; 0 |
| minimum-cube | 1e-09 | PASS x 4 | PASS x 5 | 1e-09 / 4.1359e-16 | 8/12/6; 2 | 24/12; 0 |
| scaled-minimum | 15360 | PASS x 4 | PASS x 5 | 15360 / 0 | 8/12/6; 2 | 24/12; 0 |
| scaled-below | 12582.912 | PASS x 4 | PASS x 5 | 12582.912 / 0 | 8/12/6; 2 | 24/12; 0 |
| large-scale | 14305114746.1 | PASS x 4 | PASS x 5 | 14305114746.1 / 0 | 8/12/6; 2 | 24/12; 0 |
| large-offset | 80000 | PASS x 4 | PASS x 5 | 80000 / 0 | 8/12/6; 2 | 24/12; 0 |
| large-offset-thin | 8 | PASS x 4 | PASS x 5 | 8.00000037998 / 4.7497e-08 | 8/12/6; 2 | 24/12; 0 |
| hole | 79214.6018366 | FAIL x 4 | WRONG x 5 | 79215.4372725 / 1.0546e-05 | 14/21/9; 0 | 3121/5618; 0 |
| pattern | 76858.4073464 | FAIL x 4 | FAIL x 5 | no mesh | - | - |
| touching-hole | 79214.6018366 | FAIL x 3 | PANIC x 5 | no mesh | - | - |
| near-hole | 79214.6018366 | FAIL x 3 | WRONG x 5 | 79215.4378422 / 1.0554e-05 | 14/21/9; 0 | 3071/5568; 0 |
| near-hole-outside-band | 79214.6018366 | FAIL x 3 | WRONG x 5 | 79215.4381972 / 1.0558e-05 | 14/21/9; 0 | 3059/5556; 0 |
| crossing-hole | 79607.3009183 | FAIL x 3 | WRONG x 5 | 79607.9147683 / 7.711e-06 | 14/21/9; 2 | 1132/1564; 0 |
| thin-wall | 3.59996 | WRONG x 3 | WRONG x 5 | 3.62495936277 / 0.0069443 | 16/24/10; 0 | 228/200; 0 |
| contour | 45000 | PASS x 3 | PASS x 5 | 45000 / 0 | 12/18/8; 2 | 36/20; 0 |
| plate-cut-0 | 40000 | PASS x 3 | PASS x 5 | 40000 / 0 | 8/12/6; 2 | 108/96; 0 |
| plate-cut-30 | 40000 | PASS x 3 | PASS x 5 | 40000 / 1.6371e-15 | 8/12/6; 2 | 114/102; 0 |
| plate-cut-45 | 40000 | PASS x 3 | PASS x 5 | 40000 / 5.457e-16 | 8/12/6; 2 | 120/108; 0 |
| plate-cut-60 | 40000 | PASS x 3 | PASS x 5 | 40000 / 2.9104e-15 | 8/12/6; 2 | 120/108; 0 |
| plate-cut-vertex | 166.666666667 | PASS x 3 | PASS x 5 | 166.666666667 / 5.6275e-15 | 4/6/4; 2 | 18/10; 0 |
| plate-cut-coincident | 80000 | FAIL x 3 | FAIL x 5 | no mesh | - | - |
| box-cut-0 | 500 | PASS x 3 | PASS x 5 | 500 / 0 | 8/12/6; 2 | 24/12; 0 |
| box-cut-30 | 500 | PASS x 3 | PASS x 5 | 500 / 4.5475e-16 | 8/12/6; 2 | 80/68; 0 |
| box-cut-45 | 500 | FAIL x 3 | FAIL x 5 | no mesh | - | - |
| box-cut-60 | 500 | PASS x 3 | PASS x 5 | 500 / 1.5689e-14 | 8/12/6; 2 | 80/68; 0 |
| box-cut-vertex | 166.666666667 | PANIC x 3 | PANIC x 5 | no mesh | - | - |
| box-cut-coincident | 1000 | FAIL x 3 | FAIL x 5 | no mesh | - | - |
| hole-tol-small | 79214.6018366 | FAIL x 3 | FAIL x 5 | no mesh | - | - |
| hole-tol-large | 79214.6018366 | FAIL x 3 | WRONG x 5 | 79215.4310149 / 1.0467e-05 | 14/21/9; 0 | 3819/6316; 0 |
| hole-tol-scaled | 79214.6018366 | FAIL x 3 | WRONG x 5 | 79215.4372725 / 1.0546e-05 | 14/21/9; 0 | 3121/5618; 0 |
| hole-fine-mesh | 79214.6018366 | FAIL x 3 | WRONG x 5 | 81110.8291191 / 0.023938 | 14/21/9; 0 | 139660/269300; 1 |
| tess-tol-below | 80000 | PANIC x 3 | PANIC x 5 | no mesh | - | - |
| tess-tol-equal | 80000 | PASS x 3 | PASS x 5 | 80000 / 0 | 8/12/6; 2 | 24/12; 0 |
| nonunit-axis | 79214.6018366 | PANIC x 3 | FAIL x 5 | no mesh | - | - |
| plate-cut-consistency | 40000 | PANIC x 3 | PANIC x 5 | no mesh | - | - |
| hole-consistency | 79214.6018366 | PANIC x 3 | PANIC x 5 | no mesh | - | - |
| hole-coarse | 79214.6018366 | WRONG x 3 | WRONG x 5 | 79215.4420061 / 1.0606e-05 | 14/21/9; 0 | 3309/5806; 0 |
| hole-medium-mesh | 79214.6018366 | FAIL x 3 | WRONG x 5 | 79214.7425902 / 1.7769e-06 | 14/21/9; 0 | 17473/29362; 0 |
| hole-robust-mesh | 79214.6018366 | FAIL x 3 | WRONG x 5 | 79214.7425907 / 1.7769e-06 | 14/21/9; 0 | 17473/29362; 0 |


## Robustness and reproducibility

| Profile | Calls | PASS | FAIL | PANIC | WRONG | 30-second timeouts (within FAIL) |
|---|---:|---:|---:|---:|---:|---:|
| Debug | 146 | 78 | 47 | 15 | 6 | 38 |
| Release | 220 | 110 | 30 | 25 | 55 | 10 |
| Total | 366 | 188 | 77 | 40 | 61 | 48 |

The remaining 29 FAIL calls are 24 boolean-None outcomes (coincident-face and cube
45-degree cuts) and five release invalid-axis plane-construction failures.
The `(0,0,2)` rotational-axis control violates the documented normalized-axis
precondition: its debug panic/release error difference does not describe valid input.
The below-tessellation-floor control panics in both profiles. Optional consistency
checks can panic after boolean construction returns; these are separate cases.

Every call runs in a fresh child process. Caught calls construct/discard all state
inside `catch_unwind`; direct panics terminate the child. The 30-second supervisor
kills/reaps timed-out children. This does not bound upstream peak memory, prove
termination without a supervisor, catch aborts or implement cooperative cancellation.
No additional robustness/fuzzing or cross-platform guarantee is established.

Byte comparisons require every scheduled run and its hash. Missing runs, timeout
outputs or absent hashes are **unmeasured**, excluded from measured denominators;
FAIL in a byte comparison is reserved for observed mismatches. SHA-256 compares
raw indexed mesh bytes and emitted triangle coordinates; no tolerance is applied.

| Profile | Comparison | Identical / measured | Mismatched | Unmeasured |
|---|---|---:|---:|---:|
| debug | repeat mesh | 14/24 | 10 | 20 |
| debug | repeat triangles | 14/24 | 10 | 20 |
| debug | thread triangles | 12/12 | 0 | 32 |
| release | repeat mesh | 14/33 | 19 | 11 |
| release | repeat triangles | 14/33 | 19 | 11 |
| release | thread triangles | 15/33 | 18 | 11 |

Three identical runs on one compiler/host do not prove general determinism. A
partial mesh's anchor-dependent scalar may vary even with equivalent surviving
triangles. Report numbers can be regenerated without geometry by
`python spikes/n04-truck/summarize.py --input-dir <archive-evidence-directory>`.
For a fresh matrix, run `python run.py` then `python summarize.py` from the spike
as documented in the README. Outputs stay under ignored `target/`; elapsed times
and nondeterministic hashes will differ. These commands were not used to add runs
for N04T.

## Mapping to BeamCraft needs

| Need | Observed fit and remaining gap |
|---|---|
| Quantities and supported holes/cuts | Boxes and many ordinary cuts match analytic volume. Ordinary holes miss relative 1e-6; refinement can omit a face and exceed caps. No certified generic boolean quantity path. |
| Mesh, F06 section 6 | A box returns 24 positions/12 triangles, raw chi 6 and 24 boundary edges; exact-position aliases give 8 vertices/chi 2. Needs shared indexing, BCMESH01 encoding, canonical ordering, roles, face mapping and keys. Sorting or tolerance welding cannot establish conformance. |
| FeatureId/history | Boolean API returns a Solid without generated/modified/deleted history. Runtime IDs can retain four plate faces for a hole but give no stable split-parent or semantic names. |
| Tolerances, finite values and bounds | Approximation parameters do not implement the F06 admission floor, staged input validation or exact curved bounds. Need typed conversion, local construction, caps and explicit unsupported/uncertain diagnostics. |
| Limits, F06 section 10 | Synchronous calls have no tested budget/cancellation interface; output caps cannot bound prior allocation. Process discard is only a research safeguard. |
| Portability | Native default graph includes C compression. Host success does not establish WASM, Linux/macOS or alternate-feature support. |

## Licence and supply-chain findings

These are recorded findings from the original offline audit, not a new licence
admission. All three direct manifests declare Apache-2.0 and identify
`https://github.com/ricosjp/truck`; the cached archive pins are:

| Package | Version | Cargo.lock archive SHA-256 |
|---|---|---|
| truck-modeling | 0.6.0 | `8082c2dfac8c2732a014d81807fb5d24dbac0d72ee4de2a705b7fbc05fa05b18` |
| truck-shapeops | 0.4.0 | `fd6e83ea4d36d00229d38ab985a991e32f58c3ed2971083011ac66fb57e26d5c` |
| truck-meshalgo | 0.4.0 | `e18f325738e822c79bb1a48897ec789cbf50cbeeb0841105295f9da85bdf3613` |

The original inventory lists 135 registry versions plus the private spike, including
target-specific packages; it records matching cached archive checksums for all 135.
Each declared expression has an allowed selection, including MIT/Apache alternatives
to 0BSD/Unlicense. This metadata screen is not a file-level licence proof or cargo-deny.
The candidate crate roots package no LICENSE files; original publisher licence/NOTICE
retrieval and exact attribution remain unresolved. No text is invented or relicensed.

Default VTK/STEP features broaden the graph. `vtkio -> xz2 -> lzma-sys 0.1.20`
introduces C code: the XZ summary distinguishes public-domain liblzma from GPL build
material and conditional LGPL getopt. No GPL source was read/copied; exact native
file/link/notice closure remains unaudited and unapproved for redistribution.
Minimal features were not tested. Original inventory/graph files are identified by
hash below; `cargo metadata --offline --locked --format-version 1` and
`cargo tree --offline --locked -e features` in the spike reproduce graph declarations.
Original audit/environment/document-generation scripts were removed; only the small
Python supervisor, scalar oracle, table calculator and their tests are necessary
for the retained numerical report. No runtime backend or redistribution is admitted.

ATTRIBUTION.md is outside this task's write allowance. Proposed original-material
entry: BeamCraft contributors; retained `spikes/n04-truck/**` source/docs and this
report; original public-math/F06 probes, 2026-10-16; MPL-2.0; research record only.
Exact upstream notice/attribution admission is separate and remains open.

## Copilot review disposition (head 5a154b9)

| Thread | Disposition for the orchestrator to post |
|---|---|
| 1, PR.md:31 | moot: file removed under the evidence policy. The underlying report defect is fixed: experiment explicitly partial, 366 observed calls, missing 74 schedules and absent supplementary file stated. Completion claims and stale 343-record counts removed; no new evaluation authorized. |
| 2, checks.py:35 | moot: file removed under the evidence policy. finalize.py was also removed. Current verification records actual status/diagnostics below; E0463 alone is not proof that a target is absent. |
| 3, run.py:135 | Fixed. The default schedule checkpoints separate debug/release files, clears selected files before starting, and keeps partial runs visible. Synthetic tests exercise both profile files and interruption without geometry calls. |
| 4, summarize.py:49 | Fixed. Missing schedule/hash returns unmeasured; only complete observed byte comparisons can pass/fail. Aggregate denominators exclude unmeasured. Tables recomputed from all 366 records; missing four-thread contour and timed-out repeats have regression coverage. finalize.py consumer is moot: file removed under the evidence policy. |

## N04T verification and handoff

All commands ran offline on Rust/Clippy 1.96.0. Root Cargo build outputs were redirected
to `spikes/n04-truck/target/root-checks`; spike outputs used its `target/` directory.

| Directory | Command | Actual result |
|---|---|---|
| Root | `cargo fmt --all -- --check` | PASS |
| Root | `cargo clippy --workspace --all-targets -- -D warnings` | PASS |
| Root | `cargo test --workspace` | PASS |
| Spike | `cargo fmt --all -- --check` | PASS |
| Spike | `cargo clippy --workspace --all-targets -- -D warnings` | PASS |
| Spike | `cargo test --offline --locked` | PASS, six probe tests |
| Spike | `cargo test --offline --locked --release` | PASS, six probe tests |
| Root | `python spikes/n04-truck/test_harness.py` | PASS, four tests; synthetic review controls and scalar corruption checks |
| Spike | `cargo check -p beamcraft-n04-truck-spike --target wasm32-unknown-unknown` | FAIL, E0463: cannot find crate for core; WASM compilation not run |
| Root | `git diff --check` | PASS |

The WASM diagnosis is supported independently by `rustup target list --installed`,
which lists only aarch64-pc-windows-msvc and x86_64-pc-windows-msvc. E0463 by itself
would not prove the target absent. This process/filesystem supervisor is native;
no portability acceptance is claimed. No target/tool installation was attempted.
Cargo also warns of future incompatibilities in nom 3.2.1 and quick-xml 0.22.0;
current local Clippy passes, while current-stable CI has not been run here.
The original supervisor was reproduced with synthetic calls: it wrote 440 records
to results.json and neither per-profile file. The corrected regression passes.
No full matrix or supplementary geometry was rerun; only existing probe tests ran.

Publication: **uncommitted**. Git staging cannot create the external worktree
index.lock (Permission denied); no alternate metadata access was attempted.
GitHub PR lookup also fails through the configured proxy (connection refused),
so push, PR-body publication and draft-state verification were not possible.
The existing PR is [#46](https://github.com/Apibim-com/BeamCraft/pull/46); no duplicate
was opened. A full proposed body is prepared locally at ignored
`spikes/n04-truck/target/n04t-pr-body.md` for the orchestrator to publish after
committing/pushing. Copilot replies are left to the orchestrator.
Entry HEAD remains `5a154b95758284f9854860f074766979e290462c`.
The local origin/master ref is now `64655c353371cab8c8b7ac3944cd00543c18ab21`;
no remote refresh, rebase or change to the existing PR branch was attempted.

Proposed roadmap entry (unapplied): 2026-10-16: N04T reduced the rejected truck
route to its report and offline harness, corrected partial-run and byte-comparison
claims, and removed generated evidence with SHA-256/size references. Circular/oval
holes and cuts remain our own implementation; no production feature is adopted.

## Archived locally, not in the repository

The orchestrator archived the original tree outside this repository; N04T neither
accessed nor verified that archive. Sizes and SHA-256 below were computed from each
worktree file **before deletion**. Paths are relative to `spikes/n04-truck/` and
identify archived material, not live repository links. Earlier summary/check files
can contain stale claims; the corrected report takes precedence. Binary build
products and per-run meshes remain ignored, never repository evidence.

| Removed file | Bytes | SHA-256 |
|---|---:|---|
| `PR.md` | 8779 | `12d78b35bd4b14545135b96c6e1a930c2fd3f23938725614be1897def676bd3e` |
| `audit.py` | 5921 | `8c6da7c13de4325f79bb25c3d5f18a76f99baf98c8c4af23bcc6814163a7c426` |
| `checks.py` | 2104 | `7dd2c0b6ebe3b130bec2035de83d853d5a28ec7a522239f15c41ecf15e7a45aa` |
| `doc_text.py` | 957 | `e7d4792b55ff2fac38b341790f6a9ce5b179551959b6486298e1467d1ef915f6` |
| `environment.py` | 1304 | `838760c0ad7a14a9e58d41a1bce1ab4af9b72db60be554e0ad435cecc07bc88f` |
| `evidence/case-results.md` | 4307 | `d11cf289e1a4ff21a90e3b3b689fc5475f03c6fb86fe92685b48ecaae09cb104` |
| `evidence/check-1.txt` | 0 | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| `evidence/check-2.txt` | 72 | `59639588a6f1826f129cbcd121edc07279dab94786f5fa5bde6b3439b2e845cc` |
| `evidence/check-3.txt` | 17081 | `ae02538ac67c112a4ff72f8a291a241d446d922f5816719bd19d353a6419a32e` |
| `evidence/check-4.txt` | 957 | `a65f12d7d87e468913d139d19ee2f7aa01cc639768a9070fb15231446f4a200c` |
| `evidence/check-5.txt` | 0 | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| `evidence/check-6.txt` | 426 | `982006bfce98eeb394820b601f62a82eb1e4db75e1a5c5bdabefe7fe62b4f01b` |
| `evidence/check-7.txt` | 1619 | `7695ad909009c84c576ff710e313e6fe7bf2da349bb920ed1684c160315684a6` |
| `evidence/check-8.txt` | 1614 | `0582737b7ffaa6224195cbc04f8fe1f31b4cbecd0a7cc041b7b00d1ff21893fc` |
| `evidence/checks.json` | 1344 | `f36715de7e4940772ac19af8e0bb215d6448457e38bc1982e6f95c086edc19cc` |
| `evidence/determinism.md` | 3013 | `99b2a140f7719953e72c1fb5ea26b2849cc3d7327a0ed619d8d3bb51aa128f2e` |
| `evidence/environment.json` | 1447 | `79b3b27c3f58a1011807bb1eee72930cefa0feca0d21be9c41271869e32ac2de` |
| `evidence/expectations.json` | 15508 | `6030bdd8cc8df94af3e30e6a3de134f167dae6b5eaf2b6470d2f9118fa90e35b` |
| `evidence/features.txt` | 61113 | `8336318eed9d1c0c72bb1cb428964b1cf654e3fa3e4168044c2bb512af389d28` |
| `evidence/inventory.json` | 130525 | `c92e9721d4660a1f54c2e1a5582586711325925c92e719b256b210ab3b471bcf` |
| `evidence/inventory.md` | 21263 | `423230e0fd943861c5fef32121668da86e8690dc4d9a81a887b3573973051df7` |
| `evidence/resolution.json` | 129545 | `0e07c67e0acaf70c220559ea424304ee2c121b51305c3cce4f84c057848d3fc9` |
| `evidence/results-debug.json` | 201920 | `8d591d2ef834552da616d008944d85edb8252e5c4d178c5bf15d26ccf2487d8d` |
| `evidence/results-release.json` | 382703 | `6a55954dde5b8b0efe7cc99ada2b7788493523ce0780f95654392e5905470d51` |
| `evidence/root-lock-sha256.txt` | 65 | `14fc0ae41cf112da199086b9edbd01ee5771b379fa06c45a8af4a9dda9d2e4bd` |
| `evidence/summary.json` | 208832 | `9bd56bbc9767afa2a844fded1ec2f471a0912f2235bfc122d0cbcb9a1762d122` |
| `evidence/tree.txt` | 10885 | `1ee557f5b6a647afdaf3875b1b59769dfa9349a74beb8f15263b000cd05c5a40` |
| `finalize.py` | 6309 | `35100427399977f4901bceb1c701690aef889dbcdb71d8afb2341da04d034f1a` |
| `supplement.py` | 593 | `f4a7d68bf9bdcb12df0547bad04be32c29eeedce21fe29a100bb444ea98d8fd3` |

Removed 29 files, 1,220,206 bytes in total.
