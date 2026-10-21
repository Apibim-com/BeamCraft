# Development

Requires stable Rust 1.96 or newer (`rustup update stable`).

## Build and test

```sh
cargo build --workspace
cargo test --workspace
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo run -p beamcraft-cli -- --version
```

## Everything CI checks

CI (`.github/workflows/ci.yml`) runs on every pull request and every push to `master`, on Windows
and Linux. To run the same checks locally:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
cargo run -p beamcraft-boundaries -- check
cargo check --target wasm32-unknown-unknown $(cargo run -q -p beamcraft-boundaries -- portable)
cargo deny check
```

- **Module boundaries.** `beamcraft-boundaries check` compares the crates' dependencies with the
  table in [`contracts/f08-module-boundaries-v0.1.md`](contracts/f08-module-boundaries-v0.1.md); it also
  runs inside `cargo test --workspace`.
- **WASM.** `beamcraft-boundaries portable` prints the crates that must build for WASM
  (`-p beamcraft-geom -p beamcraft-model -p beamcraft-kernel`). In PowerShell, run it first and
  pass its output to `cargo check`; set `$env:RUSTDOCFLAGS = '-D warnings'` for the docs command.
- **Tools.** The WASM check needs `rustup target add wasm32-unknown-unknown`; the dependency check
  needs `cargo install cargo-deny --locked`. A check that could not run is not a passed check.
- **Offline.** Add `--offline --locked` to Cargo commands to work without downloads; never update
  `Cargo.lock` as a side effect.

For K1 acceptance, set `BEAMCRAFT_REQUIRE_PXR=1` and run
`cargo test -p beamcraft-cli --test pxr_oracle -- --nocapture`. Optionally set
`BEAMCRAFT_PXR_PYTHON` to the installed Python interpreter with `pxr`. This mode fails if the
independent reader is unavailable; the default developer mode prints an explicit skip instead.
No interpreter or SDK is installed by the tests. The exact contract case inventory, asserted
fields and remaining deviations are in [K1 acceptance evidence](design/k1-contract-acceptance.md).

## Releases

Push a tag such as `v0.1.0`, or run `.github/workflows/release.yml` by hand. It builds unsigned
packages and attaches them to a GitHub release:

| Platform | Target | Package |
|----------|--------|---------|
| Windows | `x86_64-pc-windows-msvc` | `beamcraft-<tag>-x86_64-pc-windows-msvc.zip` |
| Linux | `x86_64-unknown-linux-gnu` | the same layout as a `tar.gz` |
| macOS | `aarch64-apple-darwin` | `BeamCraft.app` in a `tar.gz`, ad-hoc signed; Gatekeeper warns on a fresh download |

Every package holds `beamcraft`, `beamcraft-ui`, the Inter font with its
OFL-1.1 licence (in the .app under `Contents/Resources/`) and the project documents.
`packaging/package.sh` builds one package from a release build; `packaging/Info.plist` is the .app
template. CI extracts each archive into a fresh directory and runs `beamcraft --version` from it.

Manual check on each OS: extract the package, run `beamcraft --version`, start `beamcraft-ui` and
confirm the window opens with text (without the font the window has no text).

The macOS bundle uses `assets/icons/BeamCraft.icns` for Finder and the Dock. Its original
artwork is `assets/icons/beamcraft-1024.png`; on macOS, regenerate the standard/Retina
icon set with `bash packaging/generate-icon.sh`. The prebuilt ICNS is committed so release
packaging needs no image tools. Release smoke checks decode the packaged ICNS with `iconutil`.
The same PNG ships inside the bundle as `Contents/Resources/beamcraft-icon.png`: while
`beamcraft-ui` runs, the Dock shows the process icon, so the UI loads that file and applies
it (`eframe::icon_data::from_png_bytes`), falling back to the coded I-section outside a
bundle. Run `python3 packaging/test_package.py` to check the icon and archive wiring without
a Rust build. On macOS, also confirm the official BeamCraft icon appears in Finder, the Dock
and the app switcher. The icon and `docs/brand/beamcraft-logo.svg` are the official BeamCraft
logo (see `ATTRIBUTION.md`); the brand terms in `docs/brand/LICENSE-brand.txt` apply to them.
