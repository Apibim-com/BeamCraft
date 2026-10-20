# V01: own viewport spike, report 0.1.0

Date: 2026-10-16. Base: `2a8c33add3d1049e0b462cf3e509ab6e6d8834d2`.
Branch: `feat/v01-viewport-spike`. Target: `master`. Isolated native workspace:
`spikes/v01-viewport`, package `beamcraft-v01-viewport-spike` 0.0.1, MPL-2.0.

## Result and scope

The own wgpu renderer draws 1k, 10k and 100k real kernel plates offscreen with both
strategies. Measurements and an actual pixel-readback test ran successfully on a
hardware adapter. The eframe windowed app was built as a ready-to-run Windows
release executable, **built but not run** in the headless worker session. No
windowed FPS, camera interaction acceptance, Linux/macOS runtime or software-adapter
performance is claimed. No Kit, Storm, USD, picking, networking, font file or
commercial software asset is used. This does not complete V02, K5 or product admission.

The implementation consists of:

- A bounded session scene API that calls `beamcraft_kernel::rectangular_plate` for
  every object, with varied width (65..125 mm), length (70..118 mm) and thickness
  (6..20 mm). Ids are stable synthetic ordinals, not production ElementId UUIDs.
  Grid columns are fixed at 100, pitch 160 mm, so objects retain placement when
  count changes. Counts 1..100,000 are admitted before allocation.
- Instancing: a unit plate built by the same kernel, 36 expanded vertices with
  flat face normals, plus 24 bytes per object (translation and positive scale).
  Scale is display-only reuse for boxes, never an authored F06 engineering transform.
- Merging: the actual individual kernel meshes expanded to 36 position/normal
  vertices (864 bytes) per object, translated for drawing into one merged buffer.
  No extra per-object draw calls or index buffer are used. Both modes draw exactly
  12 triangles per object with one draw call, CCW/back-face culling, Lambert shading,
  Depth32Float and four samples when both color and depth formats support them.
- A headless benchmark CLI; a camera API and tests; an eframe app using the same
  renderer through an offscreen texture registered with egui. The owner app has
  orbit, pan, zoom, strategy/count controls, reset and an FPS overlay when Inter is
  available. Its image is fixed at 1024 x 768 and fitted without distortion.
  Rebuilds are synchronous. FPS is smoothed event-loop cadence, including UI and
  presentation, and is deliberately distinct from the benchmark timings.

No geometry computation is delegated to a UI library. The session owns G02 results;
the renderer serializes them and applies display transforms. The USD layer engine
is absent from the dependency graph and interactive path, following the
[UPI section 3 data path](usd-process-integration-v0.1.md#3-ui-data-path-state-description).

## Measurement environment and method

All rows below use **NVIDIA GeForce RTX 3060 / DiscreteGpu / Vulkan**, NVIDIA driver
560.94, Windows x86_64 (Windows build 19045), Rust 1.96.0
`ac68faa20`, Cargo default **release** profile, 1024 x 768 RGBA8Unorm,
Depth32Float, **4x MSAA**. CPU identifier: Intel64 Family 6 Model 165 Stepping 5.
The harness enumerates native adapters, prefers discrete then integrated then virtual,
other and CPU adapters, and tries device creation in that order. Equal-type adapters
retain enumeration order. This host unexpectedly provided hardware; no software-adapter
numbers were collected. Adapter name, type, backend and software classification are
in each result file's shared envelope and apply to every row.

Commands and reproduction are in the [README](../../spikes/v01-viewport/README.md).
The evidence prepared for commit is the small [benchmark JSON](../../spikes/v01-viewport/results/windows-offscreen.json)
and [memory JSON](../../spikes/v01-viewport/results/windows-memory.json), each under 50 KB.
There are no logs, screenshots or full cargo metadata dumps in the change.

Scene build is measured once per count and reused by both strategies. Packing is
CPU serialization only. Upload measures buffer creation, writes, empty submission
and device completion; packing and pipeline/target setup are excluded. The fixed
80-byte camera uniform is excluded from scene-upload bytes and updated every frame.
After 30 unmeasured warmups, 300 frames are measured per case. Each frame measures
encoding plus camera write plus submission, then waits for that submission to finish
before starting the next. Median is the sorted upper middle sample; p95 is nearest
rank. Time is CPU wall clock in milliseconds, not GPU timestamp-query time.
No readback occurs during timing. The actual readback test is separate.

### Build, packing and upload (milliseconds except bytes)

| Count | Strategy | Scene build | Pack | Upload bytes | Upload + completion |
|---:|---|---:|---:|---:|---:|
| 1000 | instanced | 7.969 | 0.031 | 24,864 | 0.389 |
| 1000 | merged | 7.969 | 0.688 | 864,000 | 0.312 |
| 10000 | instanced | 78.157 | 0.374 | 240,864 | 0.356 |
| 10000 | merged | 78.157 | 5.660 | 8,640,000 | 1.393 |
| 100000 | instanced | 804.036 | 3.638 | 2,400,864 | 0.709 |
| 100000 | merged | 804.036 | 56.555 | 86,400,000 | 30.172 |

### Frame timings (median / p95 / max, milliseconds)

| Count | Strategy | Encode + draw/submit | Through device completion |
|---:|---|---:|---:|
| 1000 | instanced | 0.0655 / 0.0815 / 0.1067 | 0.1539 / 0.1766 / 0.3885 |
| 1000 | merged | 0.0615 / 0.0788 / 0.9636 | 0.1424 / 0.1786 / 0.9697 |
| 10000 | instanced | 0.0597 / 0.0804 / 0.1291 | 0.2448 / 0.2805 / 0.3559 |
| 10000 | merged | 0.0595 / 0.0659 / 0.0856 | 0.1727 / 0.1852 / 0.2824 |
| 100000 | instanced | 0.0822 / 0.1372 / 0.9425 | 1.2509 / 1.3345 / 1.5565 |
| 100000 | merged | 0.0609 / 0.0916 / 0.1343 | 0.5135 / 0.5640 / 0.6425 |

The Windows wrapper observed **341,598,208 bytes (325.8 MiB)** as the highest
OS process peak-working-set value sampled at 250 ms intervals across the whole run.
The final interval may be missed at exit. This includes scene data, packing,
CPU-side graphics resources and driver allocations; it excludes the compiler/wrapper.
It is not per-strategy memory, private bytes or GPU memory. Per-case peak fields
remain `null`; no unsupported memory estimate is presented as a measurement.

At 10k the instanced upload is 0.241 MB versus 8.64 MB merged (about 35.9x smaller).
Merged is faster through completion on this adapter at all three sizes; the largest
case is roughly 2.4x faster merged. Instancing substantially reduces packing/upload
cost and bytes. These observations justify retaining both paths; they do not identify
one universal winner for mutable engineering models.

### Measurement limitations

One short, ordered run on one GPU is a spike, not a statistical hardware survey.
OS scheduling and CPU-driver synchronization affect p95/max. Submission-to-completion
includes CPU overhead and serialized GPU waits, so its reciprocal is not windowed FPS
or sustained pipelined throughput. No GPU timestamps, thermal control or vendor profiler
was used. Driver 560.94 is reported factually, not recommended as current.

The camera frames the entire grid, so plates at 100k can be subpixel. The 100k grid
is 100 x 1,000, whereas the 10k grid is square. Raster coverage differs by count;
these are real submitted triangle scaling measurements, not equal screen-coverage
fragment benchmarks. No frustum/occlusion culling, shape cache, edit workload, LOD,
operation mesh, holes, transparencies, material variety, selection overlay or text
rendering is measured. Both representations coincide for these uncut boxes; the
architecture's future full/simplified operation representations are not implemented
or separately measured by V01. A real workload must measure them once they exist.

## F06 engine-to-renderer mapping

This maps frozen [F06 sections 5, 6, 7 and 9](../contracts/f06-geometry-contract-v0.1.md).
It does not change those contracts or add an engine query implementation.

| Engine input | Spike mapping | Production implication |
|---|---|---|
| Element-local positions, f64 millimetres | G02 `PlateMesh::vertices`; display conversion to f32 | Keep engineering values in f64; consider camera-relative origin for large world coordinates |
| Triangle indices, u32 | G02 canonical triangles expanded to 36 vertices | Validate indices and declared counts before upload; indexed/deduplicated buffers can be measured later |
| Triangle face indices and ordered local face table | Used to select G02 outward face normals | Preserve the association to ElementId-qualified FeatureIds; GPU indices are never engineering addresses |
| Face normals | G02 separate normals, expanded per triangle | F06 BCMESH01 has no normal section; pass normals separately or compute display-only normals from an admitted mesh |
| Element transform | Session translation applied at drawing; unit-box scale is presentation optimization | Keep local shape reusable; a placement edit does not require rebuilding engineering shape or quantities |
| Shape key / mesh digest | Kernel digest remains in retained G02 result; no cache implemented | Group truly identical meshes by shape key and verified bytes; unit-box instancing is narrower and cannot represent arbitrary shapes |
| Presentation state | Camera uniform plus fixed original blue appearance | Camera changes only drawing; no shape/world-quantity invalidation |
| Queries, engineering bounds and pick | Not implemented | Resolve selection via engine APIs and snapshot face table, never infer a stable feature from a draw index |

The binary blob is not repurposed as a graphics vertex struct: F06 section 6 uses
f64 positions and face-index sections, whereas this spike uses f32 position/normal
buffers and explicit LE serialization. No unsafe casts or F06 layout changes occur.
F06 section 9 logical snapshot APIs remain future integration work.

F06 section 7 reserves the presentation-key preimage to V01. This harness has no
cache and therefore introduces no competing key implementation. Proposal for later
review: `BCGEO-PRESENTATION` plus NUL, version u16 BE, resolved appearance digest
32 bytes, visibility u8, camera projection kind u8, ordered finite f64 BE camera
parameters with canonical zero, then physical viewport size u32 BE twice. Exclude
shape, placement, material density, selection and hover. Canonical digest wiring
and contract review remain open; this report does not freeze that proposal.

## Fonts, assets and licences

Pins here: eframe/egui 0.36.2, wgpu 30.0.1; craft-ui and craft-quit only at
`83bf4cb25c01ed46e7645c4172974239868aaf06`. No PdfCraft winit, egui-wgpu or
vello_common vendor patch is applied. `craft-ui` is compiled as an admitted dependency;
no craft-ui component-specific behaviour is claimed. `craft-quit::quit` handles
macOS close requests in app logic, but macOS execution is unverified.

The prepared eframe `wgpu` feature would transitively turn on wgpu defaults,
including web backends. It is replaced with `wgpu_no_default_features` plus explicit
native `dx12`, `vulkan`, `metal`, `std`, `wgsl` on wgpu. Every relevant direct
`default-features` stays false. Resolved eframe features: accesskit, links, wayland,
wgpu_no_default_features, x11. egui/epaint enable only `bytemuck` through eframe;
`default_fonts` is absent throughout. Web-related packages can still appear in
cross-platform Cargo metadata, but no web/WASM frontend or wgpu web backend is enabled.

`python audit.py` consumes `cargo metadata --offline --locked --format-version 1`
and the actual base `deny.toml`. It reports **399 resolved packages**, including
platform-specific dependencies and the private harness, no empty manifest licence,
no `epaint_default_fonts`, craft-print, craft-pdf, craft-cms or openusd. Scanning
resolved source directories found no TTF/OTF files. This inventory is manifest-based;
it is not a separate reading of every upstream file or an SPDX satisfiability tool.

Every expression containing an identifier outside the global deny allowlist is
listed here, including disjunctive alternatives rather than hiding them:

| Expression | Exact packages | Disposition |
|---|---|---|
| `0BSD OR MIT OR Apache-2.0` | adler2 2.0.1 | Select MIT or Apache-2.0; 0BSD is not globally allowed |
| `Apache-2.0 OR GPL-2.0-only` | self_cell 1.3.0 | Select Apache-2.0; no GPL admission |
| `MIT OR Apache-2.0 OR LGPL-2.1-or-later` | r-efi 5.3.0, r-efi 6.0.0 | Select MIT/Apache-2.0; no LGPL admission |
| `Unlicense OR MIT` | byteorder-lite 0.1.0, memchr 2.8.3, winapi-util 0.1.11 | Select MIT |
| `Unlicense/MIT` | same-file 1.0.6, walkdir 2.5.0 | Upstream legacy slash expression; audit precedent treats as alternatives, select MIT |
| `MPL-2.0` | beamcraft-geom, beamcraft-model, beamcraft-kernel, beamcraft-v01-viewport-spike, each 0.0.1 | Project-owned, not third-party: existing exact domain exceptions; isolated harness is private (`publish=false`, policy ignores private packages) |

**BSL-1.0 and Unicode-3.0 are already on this base's allowlist.** Neither is a new
exception. BSL packages: clipboard-win 5.4.1 and error-code 3.4.0. Unicode packages:
icu_collections, icu_locale_core, icu_normalizer, icu_normalizer_data,
icu_properties, icu_properties_data and icu_provider each 2.3.0;
litemap 0.8.3; potential_utf 0.1.6; tinystr 0.8.4; writeable 0.6.4;
yoke 0.8.3, yoke-derive 0.8.4; zerofrom 0.1.8, zerofrom-derive 0.1.8;
zerotrie 0.2.5; zerovec 0.11.8, zerovec-derive 0.11.6; unicode-ident 1.0.27
has `(MIT OR Apache-2.0) AND Unicode-3.0`, so its Unicode obligation is retained.
All other identifiers in manifest expressions are in the existing global allowlist.
No allowlist, exception, CI or attribution file is changed.

`cargo deny` is **not run**: the command is not installed, and no advisory database
is cached at the default Cargo advisory-db location. No tool/database was fetched.
Therefore this report claims an offline metadata inventory, not a passing new
cargo-deny or vulnerability scan. The upstream audit's earlier passing result does
not substitute for running it on this lockfile. Full redistribution notices and
asset attribution remain integration gates; this binary is a local spike artifact.

| Material | Author/source/version/licence | Use and limits |
|---|---|---|
| Synthetic dimensions, shader, camera math, report and result JSON | BeamCraft contributors, V01 0.1.0, contributor-original MPL-2.0 | Code-generated test scene and evidence; no imported model or table |
| Runtime Inter | Parallel `chore/ui-font-inter` admission, OFL-1.1; not in this base | Only load `assets/fonts/inter/Inter-Regular.ttf` at runtime from repo root or spike working directory; no bytes copied or embedded here |
| Window icon override | Contributor-original transparent RGBA pixel generated in Rust, MPL-2.0 | Override eframe branding without copying artwork |
| eframe's internal fallback icon | emilk/egui eframe 0.36.2 package `MIT OR Apache-2.0`, audit records hash and unknown separate pixel author | Upstream code contains built-in fallback; override supplied, not copied to source tree; do not assert that optimizer removes upstream fallback bytes |

If Inter is absent, unreadable or over the 4 MiB read cap, the app writes an explicit
notice and emits no egui text widgets. Keyboard count/strategy/reset and mouse
navigation remain available. This branch is compiled but has not been exercised in
a window. No Ubuntu, Bitstream, substitute font, logo, screenshot, icon pack, profile
catalogue or third-party sample is added. The attribution rows above are also supplied
for the owner's later integration attribution update; that excluded file is untouched.

## Verification

Run in the isolated spike directory, Windows x86_64, Rust 1.96.0:

| Command | Result |
|---|---|
| `cargo fmt --all -- --check` | PASS |
| `cargo clippy --offline --workspace --all-targets -- -D warnings` | PASS, no warnings |
| `cargo test --offline --workspace` | PASS, four unit tests and one real headless render integration test |
| `cargo test --offline --test headless -- --nocapture` | PASS; adapter reported RTX 3060 / DiscreteGpu / Vulkan, not skipped |
| `cargo build --offline --locked --release --bins` | PASS; benchmark.exe and viewport.exe built |
| `python audit.py` | PASS inventory checks; limitations above |
| `./measure-windows.ps1` | PASS; six cases, 300 measured frames plus 30 warmups each |
| `cargo check -p beamcraft-v01-viewport-spike --target wasm32-unknown-unknown` | NOT RUN: native-only by owner decision; target also not installed |
| `cargo deny --offline check` | NOT RUN: cargo-deny and cached advisory DB absent |
| Windowed app, Linux/macOS builds and runtime | NOT RUN in this headless Windows task |

Root workspace fmt check, offline Clippy with `-D warnings` and offline locked
workspace tests also passed. Tracked workspace sources remained unchanged.
Scene tests verify all required counts, stable prefix ids/placements and identical
G02 output, plus count limits. Camera tests verify an orthonormal Z-up basis,
center/right projection, finite navigation caps and rejection of NaN/Inf/aspect faults.
The pixel test checks four known background pixels, a known lit-face pixel and
both complete images (at most one channel unit difference), using real G02 data.
It also rejects a non-finite display transform. Adapter absence alone produces a
clear SKIP; device or rendering errors fail. Allocations are capped/reserved,
input-derived positions use checked lookups, authored Rust forbids unsafe and
panic/unwrap/expect/indexing lints, and device waits have finite timeouts.

## Risks, owner questions and recommendation

**Proposal:** continue with the own wgpu + egui/eframe stack for the next viewport
integration review. Retain instancing for validated box reuse and merged buffers as
a measured alternative for static batches. The 10k offscreen target worked without
a renderer failure; this experiment gives no justified problem requiring Storm.
Do not turn the offscreen result into a calendar promise or a general FPS guarantee.

Owner follow-up questions and acceptance still open:

1. Run the built windowed app with admitted Inter on actual owner workflows and
   record input behaviour, FPS, resize/DPI, sustained memory and device-loss response.
   Test Windows as delivered, then Linux/macOS; no platform acceptance is inferred.
2. Choose batching policy after measuring editing, visibility and realistic reusable
   shapes. Merged mode duplicates data; instanced boxes cannot cover arbitrary
   operations. Measure camera-close coverage and full/simplified meshes separately.
3. Complete exact-lockfile cargo-deny, licence/notice and integration attribution
   gates before product distribution. An integrated project crate also needs its
   reviewed module-boundary registration and package-specific MPL exception.
4. Review large-coordinate f32 precision, snapshot limits, incremental upload,
   cached geometry/face-id mapping and the proposed presentation-key encoding in
   V02 rather than claiming those production features from this harness.

## Git handoff

Work is **uncommitted**. Staging was attempted and rejected because the sandbox
cannot create `D:/Projects/01_BeamCraft/.git/worktrees/01_BeamCraft-V01/index.lock`.
No workaround, commit, push, PR creation or merge was attempted after that failure.
HEAD remains the base SHA. The orchestrator can commit the allowlisted files and
open one draft PR targeting master; a prepared template is at
`spikes/v01-viewport/PR.md`. No required review or integration gate is claimed closed.

## Provenance

Contributor-original scalar display math and synthetic parameters, MPL-2.0. Engineering
mesh topology, positions and normals come only from the repository's real G02 API,
whose provenance is F06 sections 5/6/7/9 and public arithmetic. Camera basis uses
spherical coordinates and dot products; perspective is derived by dividing screen
coordinates by positive forward depth and mapping near/far to [0,1]. Shading is
Lambert's public cosine law. No external implementation was copied. Coordinate
conventions follow the public [WebGPU coordinate systems specification](https://www.w3.org/TR/webgpu/#coordinate-systems)
and F06's right-handed Z-up millimetres. No observation note, commercial software,
Tekla/Trimble material or GPL/AGPL source was consulted.
