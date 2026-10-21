# Development

Requires stable Rust 1.96+ (`rustup update stable`).

```sh
cargo build --workspace
cargo test --workspace
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo run -p beamcraft-cli -- --version
```

## Required checks

In addition to the native commands above, CI requires documentation, portable domain builds and
dependency checks:

```sh
cargo fmt --all -- --check
cargo doc --workspace --no-deps
cargo run -p beamcraft-boundaries -- check
cargo check --target wasm32-unknown-unknown $(cargo run -q -p beamcraft-boundaries -- portable)
cargo deny check
```

The boundary check compares the workspace manifests with the module table in
`docs/contracts/module-boundaries-v0.1.md` (it also runs inside `cargo test --workspace`). The
`portable` command prints exactly `-p beamcraft-geom -p beamcraft-model -p beamcraft-kernel`
from the three `now` domain rows, so the WASM check list is written in one place. All other
crates, including later crates, are desktop-native and have no WASM requirement. The three pure
crates stay WASM-compatible as a layering discipline (owner decision of 2026-10-16, task WS).
In PowerShell, run it first and pass the output to `cargo check --target wasm32-unknown-unknown`.

Run the documentation command with `RUSTDOCFLAGS=-D warnings`.
For example, PowerShell uses `$env:RUSTDOCFLAGS = '-D warnings'`.

The WASM check requires the `wasm32-unknown-unknown` target. Dependency checks require `cargo-deny`.
If provisioning is authorized, install them with `rustup target add wasm32-unknown-unknown` and
`cargo install cargo-deny --locked`. These commands download tools; an agent must follow the
owner's network policy before running them. Missing tools leave checks unverified, not passed.

Use `--offline --locked` with ordinary Cargo build, test, check, Clippy and docs commands when
validating without dependency downloads. Do not update the lockfile as a side effect of an audit.

## F01 baseline: 2026-10-14

The initial implementation base is `c6a1e73aea9c41bf88db96281040043471b80d0a`, which includes
the project's MPL-2.0 change. At audit time, local `master` was still
`596f929e820b590b8674f2b10cdc2100ba057cbe`. Start foundation branches from the recorded MPL
base until it is integrated; do not silently fall back to the older default branch.
This records the initial snapshot, not a permanent base for future branches.

The source tree was clean. Local tools were Rust/Cargo 1.96.0 on Windows x64
(Windows NT 10.0.19045.0). The workspace declares Rust 1.95 as its minimum; that version was
not separately tested. Cargo.lock contained only the three project-owned packages, each covered
by its package-specific MPL exception in `deny.toml`.

| Check | Local result |
|-------|--------------|
| Format and Clippy with warnings denied | passed |
| Workspace tests | passed: two unit tests and one CLI end-to-end test |
| Documentation with warnings denied | passed |
| CLI `--version` | passed: `beamcraft 0.0.1` |
| Locked offline metadata and clean diff | passed |
| WASM domain check | unverified: target absent, E0463 |
| cargo-deny | unverified: tool absent |
| Rust 1.95, Linux, macOS and remote CI | not run locally |

This baseline validates the scaffold. It does not establish USD feature acceptance or pass K0.

Note (2026-10-15): the owner raised the workspace minimum from Rust 1.95 to 1.96. The baseline
above records the state on 2026-10-14, when the minimum was 1.95, and is left unchanged.

## CI

`.github/workflows/ci.yml` runs on every PR and push to `master`: fmt, clippy (`-D warnings`), tests
on Windows and Linux, the module-boundary check, docs, a wasm32 check of `beamcraft-geom`,
`beamcraft-model` and `beamcraft-kernel` (listed by the boundary tool), and `cargo deny`.

## Build profiles

`cargo build --release` is the optimized build for development: every crate at optimization level
3, with 16 codegen units and no LTO, so a large crate is code-generated on several cores and no
binary is optimized again as a whole when it is linked. Release packages are built with
`--profile dist` into `target/<triple>/dist/`: ThinLTO across crates and one codegen unit per
crate, the settings `release` had until 2026-10-21. They give the smallest and fastest binaries,
but the largest dependency, `openusd`, is then code-generated on a single core and the build ends
with the slow, mostly serial link of each binary. Compare performance measurements only between
builds of the same profile.

Add `--timings` to see where a build spends its time; Cargo writes a per-crate report to
`target/cargo-timings/cargo-timing.html`.

A build with `--target` writes to `target/<triple>/` and reuses nothing from a build without it, so
keep to one form in a dev loop. A new worktree or `cargo clean` starts cold. A local compiler cache
such as [sccache](https://github.com/mozilla/sccache) serves repeated compilations from disk:
install it and set `RUSTC_WRAPPER=sccache` in your shell, or `build.rustc-wrapper = "sccache"` in
your own `~/.cargo/config.toml` (the repository does not require it). Its key includes the source
paths and the `CARGO_*` environment, so a rebuild after `cargo clean` comes almost entirely from
the cache, a new worktree still compiles the workspace crates, and a changed `CARGO_TARGET_DIR`
misses everything.

## Releases

Push a tag such as `v0.1.0`. `.github/workflows/release.yml` builds unsigned packages and
attaches them to a GitHub release (Q02 rehearsal):

- Windows `x86_64-pc-windows-msvc`: `beamcraft-<tag>-x86_64-pc-windows-msvc.zip`.
- Linux `x86_64-unknown-linux-gnu`: the same layout, archived as a `tar.gz`.
- macOS `aarch64-apple-darwin`: a `BeamCraft.app` bundle inside a `tar.gz`, ad-hoc signed
  (`codesign -s -`); the arm64 kernel refuses unsigned executables. No developer identity,
  no notarization — Gatekeeper warns on a fresh download (owner decision; tracked separately).

Every package carries `beamcraft`, `beamcraft-ui` and `beamcraft-usd-worker`, the admitted
Inter font with its `OFL-1.1-Inter.txt` licence (in the .app under `Contents/Resources/`),
and the project documents. `packaging/package.sh` builds one package from a `dist` build;
`packaging/Info.plist` is the .app template. CI extracts each archive into a fresh directory
and runs `beamcraft --version` from it.

Fresh-install verification on each OS (manual, owner): extract the package, run
`beamcraft --version`, start `beamcraft-ui` and confirm the window opens with text rendered
(Inter loaded; without it the window runs without text).
