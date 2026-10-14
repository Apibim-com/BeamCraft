# Contributing to BeamCraft

Thanks for helping. Humans and AI agents follow the same rules.

1. Read [`AGENTS.md`](AGENTS.md). The clean-room and asset rules are absolute: nothing from Tekla
   Structures or any Trimble product may enter this repository, and behaviour must come from public
   standards or black-box observation of synthetic fixtures.
2. Branch from `master`, keep the change small and focused.
3. Run before you push:
   ```sh
   cargo fmt --all
   cargo clippy --workspace --all-targets -- -D warnings
   cargo test --workspace
   ```
4. Add a test for new behaviour. For a bug fix, confirm the test fails without the fix.
5. Open a PR with the template filled in. CI must be green.

By contributing you agree that your work is licensed under `MIT OR Apache-2.0`, as described in the
README.

Found a security problem? See [`SECURITY.md`](SECURITY.md) instead of opening a public issue.
