# V01 native viewport spike

Contributor-original BeamCraft implementation and synthetic scene parameters, MPL-2.0.
Isolated Cargo workspace, desktop only (Windows, Linux, macOS). This is a measurement
harness, not an integrated product feature. No main-workspace dependency is added.

## Offline commands

From `spikes/v01-viewport` with Rust 1.96 or newer and the prepared Cargo cache:

```sh
cargo build --offline --locked --release --bins
cargo fmt --all -- --check
cargo clippy --offline --locked --workspace --all-targets -- -D warnings
cargo test --offline --locked --workspace
cargo test --offline --locked --test headless -- --nocapture
python audit.py
cargo run --offline --locked --release --bin benchmark -- --output results/local-offscreen.json
```

The benchmark defaults to 1,000, 10,000 and 100,000 objects, both strategies,
30 warmup frames then 300 measured frames per case. Stdout is one small JSON object;
progress goes to stderr. Optional filters:

```sh
cargo run --offline --locked --release --bin benchmark -- --count 10000 --strategy instanced
```

Windows, after building, can also observe whole-process peak working set:

```powershell
./measure-windows.ps1
```

This overwrites `results/windows-offscreen.json` and `results/windows-memory.json`.
The wrapper starts the benchmark with no window and polls the OS process high-water
working-set value every 250 ms. The last interval can be missed at process exit.
Memory is for the complete six-case run, not an allocation or GPU-memory comparison.
The standalone benchmark uses `null` for unavailable per-case peak memory.

## Owner hardware run

Run from the repository root (or the isolated spike directory). After the parallel
font PR is admitted, its `assets/fonts/inter/Inter-Regular.ttf` is loaded at runtime.
No font is embedded or supplied by this spike. If absent/unreadable, the app writes
an explicit stderr notice and shows only the viewport, without text.

From the repository root:

```sh
cargo run --offline --locked --release --manifest-path spikes/v01-viewport/Cargo.toml --bin viewport
```

Ready-to-run Windows executable after the build:
`spikes/v01-viewport/target/release/viewport.exe` (run with repository root as the
working directory). Linux/macOS: `target/release/viewport` in the isolated workspace.
Binaries are local build artifacts, never committed.

- Primary mouse drag: orbit; secondary drag: pan; mouse wheel: zoom over the image.
- Count buttons: 1,000 / 10,000 / 100,000; keyboard 1 / 2 / 3 works without text.
- Instanced/merged buttons or M switch the submission strategy; R resets framing.
- With Inter present, the overlay shows smoothed event-loop FPS, actual adapter,
  sample count and object count. FPS includes UI and presentation, unlike benchmark
  completion timings. Continuous repaint and the surface's default vsync apply.
- The offscreen image remains 1024 x 768, fitted within the window without distortion.
- Rebuilding a scene is synchronous; a 100k rebuild can temporarily pause the window.

The worker built the app but did not run it in its headless session. Owner acceptance
must record windowed results and input behaviour; this README claims no windowed FPS.

## Measurement and integration boundaries

Meshes and face normals come from `beamcraft-kernel` G02. Every object is evaluated,
including in instanced mode; the shared unit plate also comes from G02. The display
scale is an optimization for these rectangular primitives, not an authored F06 scale.
Merged mode expands actual kernel triangles into a single vertex buffer; both modes
use one draw call and the same shader lighting, camera, target, culling, depth and MSAA.

Time fields are CPU wall-clock milliseconds. Upload excludes CPU serialization and
pipeline creation and includes an empty submission and device completion. Every timed
frame starts with encoding/camera upload, records draw/submit, then waits for its
submission before recording completion. There are no GPU timestamp queries, presentation,
readbacks, occlusion culling, streaming or USD in the benchmark. Counts are capped at
100,000 before allocation; targets at 2048 per dimension; device waits at 60 seconds.

The pixel test uses an actual adapter and checks background and lit-face values plus
strategy equivalence. It clearly prints SKIP only when no adapter exists; device or
render failures fail the test. Use `--nocapture` to see adapter or skip messages.
The crate intentionally has no web/WASM entry point or target acceptance requirement.

See [the report](../../docs/design/v01-viewport-spike-report-v0.1.md) for measured
results, licence inventory, F06 mapping, limitations and the proposal. `audit.py` is
an offline manifest inventory, not a complete licence-text audit or cargo-deny substitute.
The complete lockfile is included in the change; dependency source trees and raw metadata are not.
