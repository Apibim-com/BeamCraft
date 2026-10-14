# Development

Requires stable Rust 1.95+ (`rustup update stable`).

```sh
cargo build --workspace
cargo test --workspace
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo run -p beamcraft-cli -- --version
```

Optional tools: `cargo install cargo-deny` then `cargo deny check` (licences and advisories).

## CI

`.github/workflows/ci.yml` runs on every PR and push to `master`: fmt, clippy (`-D warnings`), tests
on Windows and Linux, docs, a wasm32 check of the library crates, and `cargo deny`.

## Releases

Push a tag such as `v0.1.0`. `.github/workflows/release.yml` builds `beamcraft` for Windows, Linux
and macOS and attaches the archives to a GitHub release.
