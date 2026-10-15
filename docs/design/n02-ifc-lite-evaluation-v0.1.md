# N02: ifc-lite evaluation, version 0.1

Date: 2026-10-15. Task: N02, isolated offline spike.
Worktree: `D:\Projects\01_BeamCraft-N02`. Status: **uncommitted**.
Requested branch: `feat/n02-ifc-lite-spike`, based on `origin/master`. Git ownership validation blocked verification of branch/base and changed paths; no workaround or Git write was attempted.

## Scope and recommendation

**Write our own first-slice rectangular primitives and analytic quantities. General triangulation and booleans remain unknown pending scoped acceptance.** Numeric extrusion and a rectangular hole work, but the high-level Mesh is f32 with split vertices, fails F06 closed-by-index topology, and loses precision on admissible dimensions/placements. A public low-level f64 triangle API exists: these findings do not prove every kernel API unsuitable.

This evaluates the owner's conditional 2026-10-15 permission for unmodified MPL-2.0 dependencies. It approves no production integration, freezes no contract and completes no G-task acceptance gate. Changes are confined to `spikes/n02-ifc-lite/**` and this report; prescribed root checks also produce ordinary build artifacts. Root manifests, lockfile, policy, contracts, attribution, notices, roadmap, crates and CI were not edited.

Changed paths: spike Cargo.toml (only `default-features = false`), original Rust library/binary, original Python supervisor/check runner, inventory/metadata/checksum/results/check logs, README, and this report. The empty spike `[workspace]` and isolated Cargo.lock remain; root packages do not depend on it.

## Environment and evidence

Windows x86_64-pc-windows-msvc; rustc `1.96.0 (ac68faa20 2026-05-25)`, LLVM 22.1.2; Cargo `1.96.0 (30a34c682 2026-05-25)`; Python 3.11. Installed Rust targets: x86_64-pc-windows-msvc and aarch64-pc-windows-msvc. WASM compilation: **not run**, target absent. No network or target installation was attempted.

Dependency resolution, compilation, Clippy and tests used offline Cargo. Final formatting checks also use global `--offline`; one initial rustfmt invocation omitted it, performed no resolution and accessed no network. No subagents were used.

AGENTS, CLAUDE, ROADMAP, architecture, F04/F06 and F06 cases, module boundaries, read-only kernel design and deny policy were read. Some combined output was truncated; relevant numeric, mesh, limits and design sections were reread separately. The requested `docs/design/tolerance-comparison-v0.1.md` **does not exist**. Its claimed 1e-6 relative comparison could not be verified. Checked-in F04/F06 instead require quantity comparison at 1e-12 relative, coordinates at `1e-9 mm + 1e-12*abs(expected)`, and do not contain the task-supplied `eps_geom = max(0.001 mm, 2^-22*L)` expression. Those discrepancies are reported, not silently amended.

On supervisor restart, `Get-Location` again confirmed this worktree, existing evidence was inventoried, and `cargo test --offline --manifest-path spikes/n02-ifc-lite/Cargo.toml` passed 2 tests. Existing experiment/check evidence was sound; the missing report was completed without discarding it.

Evidence paths below are relative to `spikes/n02-ifc-lite/`:

| File | Evidence |
|---|---|
| metadata-default.json / metadata-no-default.json | Cargo metadata, exact licences/MSRVs/features/graph before and after disabling defaults |
| metadata-wasm.json | Offline platform-filtered metadata, not a WASM compilation |
| tree-default.txt / tree-no-default.txt | Offline dependency trees; latter flattened |
| archive-checksums.json | All 76 registry archives SHA-256 verified against spike Cargo.lock |
| inventory.md | Complete package table, also reproduced below |
| results.json | 20 supervised release processes, outputs, exits, times, peak committed memory and hashes |
| checks.json / check-1.txt through check-6.txt | Exact required commands, exit statuses and full logs |
| src/** / supervise.py / checks.py | Original probes and reproducible runners |

## Inventory, licences and cost

Metadata resolves **76 third-party package versions**, plus the private spike. The native build tree has **71 third-party versions**. Four glam versions and zlib-rs occur only in metadata. Both miniz_oxide versions are active. Every MPL-2.0 package is listed here:

| Package | Version | Treatment |
|---|---|---|
| ifc-lite-geometry | 20.4.0 | Third party; exact exception and source/notice obligations if adopted |
| ifc-lite-core | 20.4.0 | Mandatory transitive third party; separate exact exception and obligations |
| beamcraft-n02-ifc-lite-spike | 0.0.1 | Private BeamCraft-original isolated package, outside root workspace |

Other flags: adler2 2.0.1 offers **0BSD**, memchr 2.8.3 offers **Unlicense**, both outside the supplied allowlist; both also offer MIT, so selecting MIT requires no exception. Legacy `MIT/Apache-2.0` strings occur in the six lexical packages, matrixmultiply, minimal-lexical and rawpointer: retain the declarations below, review/normalize dual-licensing rather than invent a new licence. unicode-ident requires Unicode-3.0 in addition to a MIT/Apache choice. No other out-of-list identifier was observed. Notice obligations remain for selected permissive licences too.

Neither ifc-lite crate declares rust-version. The entire native graph builds on 1.96.0; **actual minimum MSRV is unknown**, since no older toolchain was run. Highest declared third-party minimum is nalgebra 1.89.0 (safe_arch/wide declare 1.89). The private spike declares 1.96. An undeclared MSRV cannot be inferred from dependency maxima alone.

Both ifc-lite default feature sets are empty. Disabling geometry defaults removed only the activated `default` marker: **zero package reduction**. Core is mandatory, not feature-gated. Geometry also exposes observability (optional tracing), triangulation-alt (optional earcut), csg_capture, csg_manifold_gate, csg_topology_gate, debug_geometry, opening-perf-trace and test-support; none was enabled. There is no tested primitives-only configuration that removes parsing, threading or image support.

Mandatory weight unnecessary for these boxes includes STEP/IFC parsing (nom, lexical, fast-float2), futures utilities, rayon/crossbeam, PNG/JPEG/compression, nalgebra/SIMD and integer/rational arithmetic. No Python, wasm-bindgen, network client, C/C++ kernel or sys binding appears in this graph. It appears **pure Rust**, from manifests/builds, not a complete source audit. rayon-core's Cargo `links = "rayon-core"` is not by itself evidence of native FFI. Pure Rust does not imply no unsafe code or OS/thread use in dependencies; only original spike Rust is checked under BeamCraft's unsafe prohibition.

Release probe executable: 1,777,152 bytes. An intermediate target snapshot contained approximately 360 MB release and 677 MB debug artifacts, including repeated builds; these are cache footprints, not distribution sizes or clean-build benchmarks. First debug test compilation reported 38.18 s. No comparative original-kernel benchmark or development-effort measurement was made.

### Complete resolved inventory

Licence strings are declared metadata; missing rust-version is explicit. Metadata-only rows are not part of the observed native tree.

| Package | Resolved version | Declared SPDX expression | Declared rust-version | In host tree |
|---|---|---|---|---|
| adler2 | 2.0.1 | `0BSD OR MIT OR Apache-2.0` | not declared | yes |
| approx | 0.5.1 | `Apache-2.0` | not declared | yes |
| autocfg | 1.5.1 | `Apache-2.0 OR MIT` | 1.0 | yes |
| beamcraft-n02-ifc-lite-spike | 0.0.1 | `MPL-2.0` | 1.96 | yes |
| bitflags | 2.13.2 | `MIT OR Apache-2.0` | 1.56.0 | yes |
| bnum | 0.14.4 | `MIT OR Apache-2.0` | 1.87.0 | yes |
| bytemuck | 1.25.2 | `Zlib OR Apache-2.0 OR MIT` | not declared | yes |
| cfg-if | 1.0.5 | `MIT OR Apache-2.0` | 1.32 | yes |
| crc32fast | 1.5.2 | `MIT OR Apache-2.0` | 1.63 | yes |
| crossbeam-deque | 0.8.8 | `MIT OR Apache-2.0` | 1.61 | yes |
| crossbeam-epoch | 0.9.21 | `MIT OR Apache-2.0` | 1.61 | yes |
| crossbeam-utils | 0.8.23 | `MIT OR Apache-2.0` | 1.60 | yes |
| earcutr | 0.5.0 | `ISC` | not declared | yes |
| either | 1.19.0 | `MIT OR Apache-2.0` | 1.63.0 | yes |
| fast-float2 | 0.2.4 | `MIT OR Apache-2.0` | 1.37 | yes |
| fdeflate | 0.3.7 | `MIT OR Apache-2.0` | 1.67.0 | yes |
| flate2 | 1.1.10 | `MIT OR Apache-2.0` | 1.67.0 | yes |
| futures-core | 0.3.34 | `MIT OR Apache-2.0` | 1.36 | yes |
| futures-macro | 0.3.34 | `MIT OR Apache-2.0` | 1.71 | yes |
| futures-task | 0.3.34 | `MIT OR Apache-2.0` | 1.71 | yes |
| futures-util | 0.3.34 | `MIT OR Apache-2.0` | 1.71 | yes |
| geometry-predicates | 0.3.0 | `MIT` | not declared | yes |
| glam | 0.30.10 | `MIT OR Apache-2.0` | 1.68.2 | metadata only |
| glam | 0.31.1 | `MIT OR Apache-2.0` | 1.68.2 | metadata only |
| glam | 0.32.1 | `MIT OR Apache-2.0` | 1.68.2 | metadata only |
| glam | 0.33.12 | `MIT OR Apache-2.0` | 1.68.2 | metadata only |
| i_float | 4.1.0 | `MIT` | 1.85 | yes |
| i_key_sort | 0.11.0 | `MIT` | not declared | yes |
| i_overlay | 8.1.2 | `MIT OR Apache-2.0` | 1.88 | yes |
| i_shape | 4.0.0 | `MIT` | 1.85 | yes |
| i_tree | 0.19.1 | `MIT` | not declared | yes |
| ifc-lite-core | 20.4.0 | `MPL-2.0` | not declared | yes |
| ifc-lite-geometry | 20.4.0 | `MPL-2.0` | not declared | yes |
| itertools | 0.14.0 | `MIT OR Apache-2.0` | 1.63.0 | yes |
| jpeg-decoder | 0.3.2 | `MIT OR Apache-2.0` | 1.61.0 | yes |
| lexical-core | 1.0.6 | `MIT/Apache-2.0` | 1.60.0 | yes |
| lexical-parse-float | 1.0.6 | `MIT/Apache-2.0` | 1.60.0 | yes |
| lexical-parse-integer | 1.0.6 | `MIT/Apache-2.0` | 1.60.0 | yes |
| lexical-util | 1.0.7 | `MIT/Apache-2.0` | 1.60.0 | yes |
| lexical-write-float | 1.0.6 | `MIT/Apache-2.0` | 1.60.0 | yes |
| lexical-write-integer | 1.0.6 | `MIT/Apache-2.0` | 1.60.0 | yes |
| libm | 0.2.16 | `MIT` | 1.63 | yes |
| matrixmultiply | 0.3.11 | `MIT/Apache-2.0` | 1.75.0 | yes |
| memchr | 2.8.3 | `Unlicense OR MIT` | 1.61 | yes |
| minimal-lexical | 0.2.1 | `MIT/Apache-2.0` | not declared | yes |
| miniz_oxide | 0.8.9 | `MIT OR Zlib OR Apache-2.0` | not declared | yes |
| miniz_oxide | 0.9.1 | `MIT OR Zlib OR Apache-2.0` | not declared | yes |
| nalgebra | 0.35.0 | `Apache-2.0` | 1.89.0 | yes |
| nom | 7.1.3 | `MIT` | 1.48 | yes |
| num-bigint | 0.4.8 | `MIT OR Apache-2.0` | 1.60 | yes |
| num-complex | 0.4.6 | `MIT OR Apache-2.0` | 1.60 | yes |
| num-integer | 0.1.47 | `MIT OR Apache-2.0` | 1.31 | yes |
| num-rational | 0.4.2 | `MIT OR Apache-2.0` | 1.60 | yes |
| num-traits | 0.2.19 | `MIT OR Apache-2.0` | 1.60 | yes |
| pin-project-lite | 0.2.17 | `Apache-2.0 OR MIT` | 1.37 | yes |
| png | 0.18.1 | `MIT OR Apache-2.0` | 1.73 | yes |
| proc-macro2 | 1.0.107 | `MIT OR Apache-2.0` | 1.71 | yes |
| quote | 1.0.47 | `MIT OR Apache-2.0` | 1.71 | yes |
| rawpointer | 0.2.1 | `MIT/Apache-2.0` | not declared | yes |
| rayon | 1.12.0 | `MIT OR Apache-2.0` | 1.80 | yes |
| rayon-core | 1.13.0 | `MIT OR Apache-2.0` | 1.80 | yes |
| rustc-hash | 2.1.3 | `Apache-2.0 OR MIT` | 1.77 | yes |
| safe_arch | 1.2.0 | `Zlib OR Apache-2.0 OR MIT` | 1.89 | yes |
| serde | 1.0.229 | `MIT OR Apache-2.0` | 1.56 | yes |
| serde_core | 1.0.229 | `MIT OR Apache-2.0` | 1.56 | yes |
| serde_derive | 1.0.229 | `MIT OR Apache-2.0` | 1.71 | yes |
| simba | 0.10.2 | `Apache-2.0` | not declared | yes |
| simd-adler32 | 0.3.10 | `MIT` | not declared | yes |
| slab | 0.4.12 | `MIT` | 1.51 | yes |
| smallvec | 1.16.3 | `MIT OR Apache-2.0` | not declared | yes |
| syn | 3.0.9 | `MIT OR Apache-2.0` | 1.71 | yes |
| thiserror | 2.0.21 | `MIT OR Apache-2.0` | 1.77 | yes |
| thiserror-impl | 2.0.21 | `MIT OR Apache-2.0` | 1.77 | yes |
| typenum | 1.20.1 | `MIT OR Apache-2.0` | 1.41.0 | yes |
| unicode-ident | 1.0.27 | `(MIT OR Apache-2.0) AND Unicode-3.0` | 1.71 | yes |
| wide | 1.7.1 | `Zlib OR Apache-2.0 OR MIT` | 1.89 | yes |
| zlib-rs | 0.6.8 | `Zlib` | 1.75 | metadata only |

## Public API survey

These are public names and observed behaviours, not copied implementations. Geometry reexports its nalgebra points/vectors/matrices, so no direct nalgebra dependency is needed.

| API | Input and return | Relevance |
|---|---|---|
| Profile2D::new / add_hole / triangulate | f64 outer points and hole loops; triangulation has points and indices | Plain numeric contours, no IFC IDs |
| ProfileType | Rectangle/circle/hollow-circle/polygon descriptions, profile conversion | Rectangle can use plain width/height; no catalogue used |
| extrude_profile | Profile reference, f64 depth, optional f64 4x4 matrix; fallible Mesh | Profile +Z extrusion then placement; exercised directly |
| extrude_profile_lofted | Two profiles, depth, optional matrix; fallible Mesh | Public, not exercised; not the F06 roll rule |
| Profile2DWithVoids / VoidInfo / extrude_profile_with_voids | Contours and depth intervals; fallible Mesh | Public, not exercised; probe instead uses add_hole + ordinary extrusion |
| triangulate_polygon | f64 2D points; fallible index vector | Standalone API; profile triangulation was exercised |
| Mesh | Public f32 positions/normals, u32 indices, origin/transform metadata | Neither F06 f64 blob nor semantic face table |
| Mesh::vertex_count / triangle_count / bounds | Counts; bounds as f32 point pair | No engineering status/signature/certificate |
| Mesh::welded_by_position | Caller epsilon; new mesh | Not used; F06 disallows welding as a primitive-construction substitute |
| Plane / Triangle / ClippingProcessor | Numeric points, normalized plane normal and meshes; clipping/intersection/subtraction | No IFC required; epsilon and failure drain observable |
| GroupCut / GroupReject / take_failures | Cut/retessellated/rejected and diagnostic records | Not F06 diagnostics or atomic seal semantics |
| kernel::mesh_bridge::{mesh_to_tris,tris_to_mesh,subtract,intersection,union} | Mesh/triangle conversion and mesh operations | Subtract exercised; bridge output still f32 Mesh |
| kernel::arrangement | f64 triangle arrays, BoolOp and boolean/conformity functions | Plain-number low-level API exists; direct f64-only route not exercised |
| kernel::mesh_volume::mesh_volume | Mesh to f64 scalar | Measured independently below; no certification/status |
| ProfileProcessor / IFC processors and extractors | Schema/decoded entities/decoder, IFC bytes or entity IDs | Import-facing surfaces unused, parser remains compiled transitively |

**Use without a STEP document is demonstrated. Building without the parser dependency is not supported by the tested manifest feature surface.** None of these APIs demonstrated a complete F06 validator, FeatureIds, canonical blob/keys, cancellation or analytic quantity records.

## First-slice method and results

Original plate contour: (0,0), (1000,0), (1000,500), (0,500), depth 10. Member: centred section +/-100 and +/-150, depth 4000; our right-handed matrix maps section X to +Y, section Y to +Z and extrusion Z to +X. This reproduces F06-POS-04's particular frame, not a general P-3 implementation. No IFC or catalogue was loaded.

Our audit uses checked indices, independent f64 triangle areas and signed tetrahedron sums, directed-edge incidence and connected components. It separately enumerates exactly equal f32 positions (signed zero normalized); this does not mutate the Mesh or propose production tolerance-welding. For convex boxes, every triangle normal is checked against the box centre. Euler alone is never considered closure proof.

| Measurement | Plate: F06-POS-01/03 | Member: F06-POS-04 |
|---|---|---|
| Analytic volume mm3 | 5,000,000 | 240,000,000 |
| Independent signed mesh volume | 5,000,000; error 0 | 240,000,000; error 0 |
| Crate mesh_volume | 5,000,000 | 240,000,000 |
| Area mm2 | 1,030,000; error 0 | 4,120,000; error 0 |
| Bounds mm | [0,0,0]..[1000,500,10]; error 0 | [0,-100,-150]..[4000,100,150]; error 0 |
| Raw vertices / triangles / edges | 24 / 12 / 30 | 24 / 12 / 30 |
| F06 expected vertices / triangles / edges | 8 / 12 / 18 | 8 / 12 / 18 |
| Raw indexed closedness; Euler | false; 24-30+12=6 | false; 24-30+12=6 |
| Equal-position V/E/T | 8/18/12 | 8/18/12 |
| Equal-position closure; Euler; components | true; 2; 1 | true; 2; 1 |
| Nonzero triangles; outward triangles | 12; 12 | 12; 12 |

The integer geometry agrees, but the returned representation **does not conform** to F06's canonical eight vertices/indices, closed-by-index topology, f64 coordinates, six semantic face roles or 392-byte BCMESH01 digest. The probe does not manufacture that contract after the fact.

### Units and precision

Numeric extrusion performs no observed mm/metre conversion: input width 1000 returns bound 1000. Its public ecosystem has metre-oriented naming (including LARGE_COORD_THRESHOLD_METERS), but a raw profile/mesh carries no unit tag. This establishes numeric behaviour, not unit independence of every algorithm.

A metre-oriented adapter would divide lengths by 1000 on entry, multiply coordinates by 1000 on exit, area by 1e6 and volume by 1e9, converting every physical threshold too. These extra operations plus f32 narrowing cannot be presumed to satisfy F06 arithmetic. Keeping mm avoids those conversions but does not prove every upstream noise/tolerance gate has the intended physical scale.

| Probe and analytic volume | Independent returned-vertex volume | Crate mesh_volume |
|---|---|---|
| 1 x 0.5 x 0.01 m; 0.005 m3 | 0.004999999888241291; -2.235174181e-8 relative | 0.00499725341796875; -5.4931640625e-4 relative |
| 0.001 mm cube; 1e-9 mm3 | 1.0000001424923606e-9; +1.424923605e-7 relative | 1.0213909718004288e-9; +0.0213909718 relative |
| 1000 x 500 x 0.001 mm; 500 mm3 | 500.0000237487256; +4.749745119e-8 relative | 503.5400390625; +0.007080078125 relative |
| 1e6 mm cube; 1e18 mm3 | 9.999999999999999e17 (binary64 value 999999999999999872), one step below expected | 1e18 |
| 1000.123456789 x 500.123456789 x 10.123456789 mm; f64 ordered analytic 5063603.263000589 | 5063603.300262201; +7.358714703e-9 relative | 5063604.254289749; +1.957675412e-7 relative |

The first, minimum, thin and fractional cases fail F04/F06's 1e-12 quantity comparison. The crate scalar also fails the task-supplied looser 1e-6 comparison for metre, minimum and thin cases. Fractional bounds are f32-rounded (debug output 1000.1235, 500.12344, 10.123457), rather than preserved f64 inputs. Their exact f32 coordinate values and absolute errors are:

| Input mm | Returned coordinate mm | Signed error mm | F04/F06 comparison bound mm |
|---|---|---|---|
| 1000.123456789 | 1000.1234741210938 | +0.000017332093761979195 | 2.000123456789e-9 |
| 500.123456789 | 500.1234436035156 | -0.000013185484363020805 | 1.500123456789e-9 |
| 10.123456789 | 10.123456954956055 | +0.00000016595605423219695 | 1.010123456789e-9 |

All three coordinates fail the contract comparison, including the thin dimension. Values come from binary32 rounding of the observed output coordinates, not from another kernel.

At translation (1e9,1e9,1e9) mm, the admissible thin plate returns **4 unique positions**, only 4 nonzero triangles, geometric Euler 7, non-closed incidence, coincident Z bounds and zero volume. Direct extrusion's public metadata does not recover thickness in this probe. A local-origin adapter might avoid this particular loss, but was not demonstrated and would still leave local f32 precision and canonicalization work.

## Triangulation and boolean findings

Rectangle profile triangulation returns four points, two triangles, indices `[2,3,0,0,1,2]`. This works for the tested rectangle; it does not prove general contour correctness.

The authored rectangular hole is [100,300] x [100,200] through the 10 mm plate. Analytic volume is `5,000,000 - 200*100*10 = 4,800,000 mm3`. Analytic area is `1,030,000 - 2*20,000 + 600*10 = 996,000 mm2`.

| Path | Raw V/T | Equal-position V/E/T; Euler | Geometric closure | Independent volume / area |
|---|---|---|---|---|
| Profile with hole, extruded | 48/32 | 16/48/32; 0 | yes, one component | 4,800,000 / 996,000; exact agreement |
| ClippingProcessor::subtract_mesh; cutter 200 x 100 x 30, Z=-10..20 | 48/32; Cut | 16/48/32; 0 | yes, one component | 4,800,000 / 996,000; exact agreement; failures empty |
| kernel::mesh_bridge::subtract, same operands | 168/56 | 28/84/56; 0 | yes, one component | 4,799,999.999999998 / 996,000; volume error about -1.86e-9 mm3; crate volume 4,800,000 |

Euler 0 fits a through-hole. The convex-centre normal statistic in raw logs **is not an orientation certificate for concave solids**; 28/32 and 46/56 are not evidence of inverted hole walls. Hole orientation, embedding and generic self-intersection freedom were not independently certified.

Plane clipping with x=500 and +X normal retains x>=500. A capped half-plate would have volume 2,500,000 mm3. Returned output has 42 raw vertices, 14 triangles, 12 unique positions, 25 geometric edges, Euler 1 and is **open**. Our origin-based sum is 3,333,333.3333333335; crate volume is 2,083,333.3333333333. Neither is a valid solid volume. clip_mesh returned success with no failure record. A verified cap/closure policy is still needed.

The kernel exposes public f64 triangle, sign and implicit-point surfaces. The experiment observes ordinary subtraction and repeatable rounded output; it proves neither exact predicates/intersections/volumes, rational output preservation, near-coplanar correctness nor arbitrary-input work bounds. No README exact-arithmetic claim is treated as evidence. The direct f64-only arrangement route is untested.

## Tolerances

Public `ClippingProcessor::new().epsilon` is **1e-6 numeric length units**. Interpreted in mm, that is 1000 times below the task's floor; in metres it maps to 0.001 mm. One public field does not establish the complete tolerance policy.

| L mm | Task-supplied max(0.001, 2^-22*L), mm |
|---|---|
| 0.001 | 0.001 |
| 1000 | 0.001 |
| 4000 | 0.001 |
| 1,000,000 | 0.2384185791015625 |

Actual F06 specifies inclusive 0.001 mm minimum dimensions, strict 1e-6 parallel branch, 1e-12 frame certificate and no snapping/welding. Its 1e-6 mm pick band and test-comparison tolerances are separate. No constant may be substituted or widened to accept these outputs. f32 spacing near 1e9 units is 64 units, consistent with the observed thin-plate collapse. F06's own f64 construction and numeric certification remain necessary.

## Hostile input, determinism and performance

Each release mode runs in a child process. It waits on a one-byte gate before geometry; the Python supervisor installs a Windows Job Object with **256 MiB committed process-memory limit**, 512 MiB job limit, kill-on-close and a **5 s wall deadline**, then opens the gate. Job assignment failures would stop before geometry; none occurred. This is an OS memory cap and process timeout, not a detached thread. Input contour counts are fixed; our post-call audit caps output at 4096 vertices/8192 triangles, while the Job Object bounds upstream allocation before the audit.

All 12 hostile cases exited 0, with no panic text, timeout or memory termination. Exit 0 means the probe reported an outcome, not that the input passed validation. Across 20 release child processes max observed committed peak was **1,122,304 bytes**, max wall time **0.015543 s**. This small corpus does not prove general input safety.

| Case | Public outcome |
|---|---|
| Zero depth | InvalidExtrusion: depth must be positive |
| Negative depth | Same typed error |
| Zero width | Success, 16 raw vertices/4 triangles, zero volume; no semantic rejection |
| Negative width | Success, box extends to x=-1000, volume +5,000,000; a valid contour shape but invalid BeamCraft dimension |
| Width/height/depth 1e300 | Success, 24 raw vertices/14 triangles, non-finite mesh, NaN independent quantities; crate volume 0 |
| NaN depth | Success, non-finite mesh, NaN independent quantities; bounds hide NaN Z as 0, crate volume 0 |
| Infinite depth | Success, non-finite mesh, NaN independent quantities, crate volume 0 |
| NaN contour width | TriangulationError: non-finite coordinate in polygon ring |
| Infinite contour width | Success, 16 raw vertices/8 triangles, non-finite/open geometry |
| Four collinear points | Success, 24 raw vertices/8 triangles, open geometric shell, volume 0 |
| Four-point bow-tie | Success, 24 raw vertices/10 triangles, open shell, signed sum 166.6666666666667; no self-intersection rejection |
| Empty contour | InvalidProfile: at least three vertices required |

Typed errors help for some failures, but validation is inconsistent. Upstream success or scalar zero is not certification. Any adapter must validate finite/support/range/cap/degeneracy rules before calls and discard invalid output afterward, mapping to BeamCraft diagnostics. These probes cover extrusion and its profile/triangulation path, not malformed indexed-mesh or hostile-boolean fuzzing.

Original Rust forbids unsafe and denies unwrap/expect/panic/todo/unreachable/indexing-slicing via Clippy attributes. Accessors are checked and authored fixture counts bounded. No upstream crash was found/fixed, so no crash-regression claim is made.

Repeated same-build calls compare little-endian positions/normals/indices plus the full public Mesh debug representation, including remaining metadata. Plate, member, hole extrusion, plane clip and kernel subtraction repeated identically. Two separate executable processes emitted **identical 3812 bytes** for plate/member (direct byte comparison). Both SHA-256 values are `5ff9188137be62976ee75186f932b74fd652363c41704479780bf7fc2b2fb647`. Separate full-suite and boolean outputs also had identical hashes. This is local same-build determinism, not cross-platform F06 canonical bytes or stable upstream serialization.

Release timing: Instant surrounds 1000 sequential calls, each creating the profile, extruding/allocating the mesh and consuming/dropping it through black_box. Members also build/apply the explicit matrix. Process startup, compile time, auditing and analytic quantities are excluded. One run, no warmup/statistical benchmark:

| Work | Calls | Total time | Vertices / triangles |
|---|---|---|---|
| Plates | 1000 | 1,705,800 ns = 1.7058 ms | 24,000 / 12,000 |
| Members | 1000 | 1,741,800 ns = 1.7418 ms | 24,000 / 12,000 |

Construction is inexpensive here; this does not establish superiority over original eight-corner code or development time saved. The unmeasured adapter/certification effort drives the recommendation.

## WASM, API stability and MPL obligations

WASM compilation/runtime: **not run**, target absent. Offline filtered metadata succeeds, but is not compilation evidence. No native binding/Python/wasm-bindgen is encouraging; unconditional rayon/crossbeam threading needs target compilation and single-threaded runtime checks, and explicit F08 domain-policy review. A licence exception alone does not approve portability/thread use.

Version **20.4.0 is not pre-1.0**. Frequent releases are task-supplied context; release history/responsiveness were not independently verified offline. Exact pinning and Cargo.lock aid reproducibility, not API stability. Low-level public APIs need upgrade tests for bytes, topology, numeric accuracy and rejection behaviour. Declared authors: IFC-Lite Contributors; repository: https://github.com/LTplus-AG/ifc-lite. A BeamCraft maintenance owner for upgrades, notices, fixes and any fork remains **unassigned**.

Both packages record upstream VCS commit `a8905535c861ca42e69f33ee8be138dea5c828b2`, paths rust/core and rust/geometry. Cached archive SHA-256 matches the spike lock:

- core: `ad276ea751e0320b8d75336d211d6eac72b7dbd4a93141e4d6b7d1926892d46b`.
- geometry: `ecaf1509c5af007e0b42da84acfbdc4c13b16907e660a77067084a40aa698dd0`.

Original package manifests declare MPL-2.0, but **neither archive contains a LICENSE, NOTICE or COPYRIGHT-named file**. This supports evaluating the declared licence and proposing exceptions; it is not a complete rights/notice audit. Obtain the corresponding upstream licence and applicable source notices before shipping or vendoring. No upstream file or unverified licence text is vendored here.

Practical obligations, using the repository LICENSE's MPL-2.0 text (source recorded in ATTRIBUTION), with section numbers:

- **2.1** grants use/distribution of unmodified contributions and modifications, including larger works. Linking an unmodified dependency is possible; BeamCraft policy still requires exact per-package approval.
- **1.10** covers modified covered-source files and new files containing covered source. Patching an upstream file requires that resulting file's distributed modifications under MPL; copying a function into another file does not evade coverage.
- **3.1**, including "including any Modifications", requires distributed covered source to remain MPL and recipients to be informed of its licence and how to obtain it.
- **3.2** requires corresponding covered source when executables are distributed, with recipients told how to obtain it reasonably and promptly. Pin an actual source archive/offer matching the shipped version, including patches; a moving main-branch link is operationally insufficient. Static linking or WASM delivery does not remove the requirement.
- **3.3** allows larger works with separate files under chosen terms while preserving covered-software obligations. BeamCraft's independent files already use MPL; upstream ownership does not spread into them by linking.
- **3.4** preserves licence, copyright, patent, warranty and liability notices. Vendoring must retain applicable notices/licence and corresponding modified source obligations; a NOTICE summary alone does not replace them.
- **2.3** grants no upstream trademark rights. No logo/brand asset is imported.

No network legal research was performed. These are the relevant licence-text requirements, not certification of the missing upstream notice audit.

## Verdict table

| Need | Verdict | Reason |
|---|---|---|
| Plate mesh | **write our own** | Small canonical f64 box; returned split vertices/precision do not satisfy F06 |
| Straight rectangular member | **write our own** | Same mismatch; frame/roll/certification remain ours |
| First-slice rectangle extrusion | **write our own** | Two end sections, six prescribed quads suffice |
| General profile extrusion later | **unknown** | Numeric/hole example works; broader inputs, output precision and budgets unproven |
| Rectangle triangulation | **write our own** | F06 fixes the diagonals and roles |
| General triangulation later | **unknown** | Positive rectangle/hole evidence; invalid contours can succeed with open geometry |
| Plane cuts/general booleans later | **unknown** | Rectangular subtraction works; plane clipping is open; exactness/work bounds and direct f64 route unproven |
| Quantities and bounds | **write our own** | Analytic/status contract absent; crate volume/f32 bounds fail admissible precision probes |

No current need earns an unconditional **use the crate** verdict. The disagreement is with assuming a fast mesher or exact-arithmetic claim automatically accelerates an F06-conforming slice; the owner's conditional licence permission remains respected.

## Exact proposed edits (not applied)

### Root Cargo.toml and deny.toml

**Recommended production edit now: none.** If a later integration passes owner/adapter/acceptance gates, add exactly this under root workspace.dependencies:

```toml
# Conditional N02 candidate; adapter and acceptance gates must pass first.
ifc-lite-geometry = { version = "=20.4.0", default-features = false }
```

Do not add a direct core dependency merely to use geometry; core is already mandatory transitively. A future real consumer would inherit `ifc-lite-geometry = { workspace = true }`. A new BeamCraft kernel package needs its separate project-owned exception, without creating an empty crate now.

For that conditional adoption, replace deny.toml's two introductory policy comment lines with:

```toml
# cargo-deny policy. Third-party dependencies use permissive licences by default.
# MPL-2.0 is allowed only for the exact package/version exceptions below.
```

Append one exception per third-party MPL package, never a global MPL allowlist entry:

```toml
[[licenses.exceptions]]
name = "ifc-lite-geometry"
version = "20.4.0"
allow = ["MPL-2.0"]

[[licenses.exceptions]]
name = "ifc-lite-core"
version = "20.4.0"
allow = ["MPL-2.0"]
```

The private isolated spike is outside root's graph; no root exception is needed for it. cargo-deny was **not run**; these snippets are not a claim of production-policy acceptance.

### ATTRIBUTION.md

Original-work row for the orchestrator to add when recording N02:

```markdown
| `spikes/n02-ifc-lite/**` (excluding build artifacts), `docs/design/n02-ifc-lite-evaluation-v0.1.md` | BeamCraft contributors | Contributor-original N02 numeric probes, synthetic contours, inventory and observations, 2026-10-15; unmodified cached ifc-lite 20.4.0 public APIs; sources/checksums and limitations in the report | `MPL-2.0` | Isolated evaluation only; no production backend or upstream source copied |
```

Dependency provenance rows (research use now; conditional production use):

```markdown
| `ifc-lite-geometry` dependency 20.4.0 (unmodified) | IFC-Lite Contributors | crates.io ifc-lite-geometry 20.4.0; https://github.com/LTplus-AG/ifc-lite/tree/a8905535c861ca42e69f33ee8be138dea5c828b2/rust/geometry ; archive SHA-256 ecaf1509c5af007e0b42da84acfbdc4c13b16907e660a77067084a40aa698dd0 | `MPL-2.0` | Geometry candidate; isolated N02 today; production pending approval and notice audit |
| `ifc-lite-core` dependency 20.4.0 (unmodified) | IFC-Lite Contributors | crates.io ifc-lite-core 20.4.0; https://github.com/LTplus-AG/ifc-lite/tree/a8905535c861ca42e69f33ee8be138dea5c828b2/rust/core ; archive SHA-256 ad276ea751e0320b8d75336d211d6eac72b7dbd4a93141e4d6b7d1926892d46b | `MPL-2.0` | Mandatory transitive parser; no IFC fixture loaded by N02 |
```

These rows do not certify missing notices. Our contours and generated observations are original; no commercial material/profile table or other third-party asset was introduced.

### NOTICE

Conditional exact addition after adoption, retaining existing project/brand text:

```text
IFC-Lite dependencies (unmodified):
  ifc-lite-geometry 20.4.0
  ifc-lite-core 20.4.0
Authors: IFC-Lite Contributors.
Declared licence: Mozilla Public License 2.0 (MPL-2.0).
Upstream source: https://github.com/LTplus-AG/ifc-lite
Pinned upstream commit: a8905535c861ca42e69f33ee8be138dea5c828b2
Corresponding published source packages:
  https://crates.io/api/v1/crates/ifc-lite-geometry/20.4.0/download
  https://crates.io/api/v1/crates/ifc-lite-core/20.4.0/download
Licence text: LICENSE and https://www.mozilla.org/MPL/2.0/.
Recipients can obtain the unmodified corresponding source from these pinned packages.
Distributors of a modified version must also provide its corresponding MPL-covered source
and retain all applicable upstream notices.
```

This is a proposed summary, not an invented upstream copyright notice or complete notice bundle. Distribution still needs verified upstream licence/notices and a maintained source offer. No licence text was copied here.

### Module boundaries and replaceable adapter

Recommended boundary edit now: **none**. Leave the kernel planned and third-party domain list empty. For later admitted pure-domain integration, add this candidate row in F08 section 3 after portability/thread-policy approval, and revise the surrounding empty-list assertion:

```markdown
| ifc-lite-geometry | Exact version 20.4.0; owner-approved MPL exception; private beamcraft-kernel adapter only; portability, thread-policy and F06 acceptance required before admission |
```

Keep internal edges unchanged. The kernel row becomes now only with real tested code in its owner task. The current checker allowlist is global by crate name: the reason cell does not enforce kernel-only use or version pinning. A reviewed checker/table extension or explicit review gate must own those limits. Core stays transitive; direct parser use requires separate scope/allowlist review. Do not reclassify a native backend as domain merely to silence checks. If pure-domain rules cannot be met, a host-isolated candidate needs a separate reviewed boundary proposal.

Proposed BeamCraft-owned trait: `beamcraft_kernel::backend::GeometryBackend`, fallibly evaluating BeamCraft GeometryInput plus resource/cancellation context into BeamCraft GeometryResult. An independent AnalyticBoxBackend handles this slice. A future IfcLiteBackend keeps all upstream types private; validates support/finiteness/ranges/caps before calls; converts units at one audited boundary; validates/discards output; and never promotes a mesh scalar to an analytic current quantity. Canonical f64 mesh, semantic faces/FeatureIds, BCMESH01, keys, status and atomic seal remain BeamCraft responsibilities or the adapter reports unsupported. Backend/algorithm versions and any fallback are explicit. No upstream index, parser, mutable Mesh or error string leaks into model/engine contracts. This is a proposal, not implemented production code.

### ROADMAP.md entry

Exact proposed progress entry, not applied:

```markdown
- 2026-10-15: N02 evaluated unmodified ifc-lite-geometry/core 20.4.0 in an isolated offline spike. Integer plate/member volumes match analytically, but f32 split-vertex meshes do not satisfy F06; thin/large-origin and invalid-input probes expose precision and validation gaps. A rectangular through-hole succeeds; plane clipping is open. The proposal is original first-slice primitives/quantities and further scoped evaluation for general triangulation/booleans. No production dependency, contract freeze or acceptance gate is completed. WASM and cargo-deny were not run; notice completeness and owner adoption remain open. See docs/design/n02-ifc-lite-evaluation-v0.1.md.
```

## Commands and exact results

All six required checks passed; raw output and commands are in checks.json/check-N.txt. Global `cargo --offline fmt` is the offline equivalent of the card's fmt command.

| Directory | Command | Result |
|---|---|---|
| spikes/n02-ifc-lite | `cargo --offline fmt --all -- --check` | exit 0; no output |
| spikes/n02-ifc-lite | `cargo clippy --offline --all-targets -- -D warnings` | exit 0; no warnings |
| spikes/n02-ifc-lite | `cargo test --offline` | exit 0; 2 passed, 0 failed; binary/doc tests 0 |
| repository root | `cargo --offline fmt --all -- --check` | exit 0; no output |
| repository root | `cargo clippy --workspace --all-targets --offline -- -D warnings` | exit 0; no warnings |
| repository root | `cargo test --workspace --offline` | exit 0; 120 passed, 0 failed; doc tests 0 |

Additional completed commands: Get-Location twice, correct path; rustc -Vv / cargo -V / rustup target list --installed; offline cargo tree/metadata (default, no defaults, WASM-filtered); offline release build; python supervise.py (20 child exits 0); python checks.py (six exits 0); 76/76 archive checksum matches; resumed spike tests (2/2 pass). Full commands/inputs for reproducing experimental modes are in README and the runners. One oversized report-writing command hit Windows error 206 before executing; writing the report in bounded explicit-UTF-8 chunks succeeded.

Root Cargo.lock SHA-256 before and after checks is unchanged:
`7b15824fc67c90a837ad19af9d28e12dea2802797e90f541945ddb8e984497eb`.

`git status --short --branch` failed with **exit 128, dubious ownership**. Git-only-changed-path confirmation, actual branch and origin/master freshness are **not verified**. No safe.directory or other workaround was attempted. Get-FileHash was unavailable; Python hashlib supplied SHA-256. Work remains **uncommitted** for the orchestrator. The final encoding check covers every deliverable text file for strict UTF-8, no BOM/CR and no specified mojibake; final results are recorded in verification.json.

## Not run, remaining decisions and exposure

Not run: WASM compilation/runtime (target absent), cargo-deny, older-compiler MSRV, other OS/CPU/compiler determinism, statistical/original-kernel comparison benchmarks, optional features, broad contour/boolean fuzzing, malformed/hostile boolean meshes, generic embedding/hole-orientation certification, direct f64-only arrangement experiments, exact-arithmetic proof, production engine/CLI/USD E2E, network provenance/release-history/notice retrieval. The absent tolerance-comparison file could not be read. Git checks were blocked as described. No blocked action was bypassed.

Owner decisions: accept/replace the per-need recommendation; approve exact MPL exceptions if later adopting; assign a BeamCraft maintainer; retrieve/verify missing upstream notices before redistribution; decide a further f64 boolean/triangulation scope with resource limits/FeatureIds/acceptance; resolve F04/F06 freeze, P-1..P-3, tolerance discrepancy and numeric accuracy gates through their owners. No human response was required/requested during this headless task.

Exposure note: only this worktree's authorized repository documents, own spike/evidence, toolchain output and explicitly authorized Cargo cache were read. Selected public API declarations, package manifests/VCS metadata, archive filenames and checksums were inspected. A broad declaration search also matched declaration lines in upstream test-named files; no test bodies or fixtures were displayed/copied. Tools compile upstream implementation; declaration scans did not expose implementation bodies to this report. No upstream source/test/comment, internal implementation description, commercial software material, catalogue, client project, Desktop directory or other checkout was imported into outputs. Numeric fixtures and audit code are original. Public-API/cache exposure must inform future clean-room admission; this worker is not declared eligible to implement an independent replacement kernel. Prior training/exposure is unknown. No network/external service was used.
