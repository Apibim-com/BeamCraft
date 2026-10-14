# BeamCraft — instructions for agents

BeamCraft is a clean-room, open-source, Rust-native structural steel detailing application,
developed by Apibim. It targets the workflows of tools like Tekla Structures without using any of
their code or assets. Runs natively on Windows, Linux and macOS, and later on the web via WASM.

## Start every session here

1. Read `AGENTS.md` (it wins over this file) and `ROADMAP.md` (current stage and next tasks).
2. Read `docs/architecture.md` and the README of the crate you are touching.
3. Work in small steps: orient → plan → implement + test → verify → record → commit.
4. At the end of a session, add a line to the progress log in `ROADMAP.md`.

## Non-negotiables

- **Clean-room and assets: `AGENTS.md` §1 and §2.** Nothing from Tekla/Trimble, ever.
- **Never crash on input:** `AGENTS.md` §4. Clippy denies `unwrap`/`expect`/`panic`/indexing in
  non-test code.
- **Headless first:** every feature has an engine API and a CLI command (`AGENTS.md` §3).
- **Units:** millimetres, radians internally, right-handed, Z up. Convert only at the UI/IO edge.

## Commands

```sh
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p beamcraft-cli -- --version
```

CI (`.github/workflows/ci.yml`) runs the same on Windows and Linux, plus a wasm32 check of the
library crates and `cargo deny`. A change is not done until these pass.

## Conventions

- Edition 2024, workspace lints in the root `Cargo.toml`. Library crates stay free of UI and OS
  dependencies and must compile for `wasm32-unknown-unknown`.
- Crates live in `crates/`, binaries in `apps/`. Dependencies point one way: `geom` ← `model` ←
  everything else. Do not add a back-edge.
- Every public item has a doc comment (`missing_docs` is on).
- Match the surrounding code's style; keep PRs small and focused. Commit messages: imperative
  mood, explain the why.
- Branch from `master`; open a PR using the template.
