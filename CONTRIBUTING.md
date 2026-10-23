# Contributing to BeamCraft

Thanks for helping. Humans and AI agents follow the same rules.

1. Read [`AGENTS.md`](AGENTS.md). The asset and provenance rules are absolute: nothing from Tekla
   Structures or any Trimble product may enter this repository; engineering rules come from public
   standards or are our own.
2. Branch from the verified current `origin/master`, keep the change small and focused.
   Use `<type>/<short-description>` with lowercase hyphenated words: `feat/`, `bug/`, `docs/`,
   `chore/`, `refactor/` or `test/`, as defined in `AGENTS.md` section 6. Include the task ID when
   applicable (for example `feat/f07-fixtures`); do not use agent/provider prefixes.
3. Run before you push:
   ```sh
   cargo fmt --all
   cargo clippy --workspace --all-targets -- -D warnings
   cargo test --workspace
   ```
   The full list of CI checks is in [`docs/development.md`](docs/development.md).
4. Add a test for new behaviour. For a bug fix, confirm the test fails without the fix.
5. Open a PR with the template filled in. CI must be green.
6. Keep the fork aligned with upstream: `bash tools/fork-sync.sh status` inspects the
   divergence, `bash tools/fork-sync.sh sync` merges `upstream/master`, gates it on the local
   triple and pushes `master`. The fork inherits upstream 1:1, so `sync` never auto-resolves:
   a merge conflict stops the run for manual investigation. The script doubles as the reference
   example for the small, single-purpose shell tools that live in [`tools/`](tools/).

Mirrors (forks) of this repository keep their own `master` aligned with this one. This
repository ships only [`tools/sync.schema.example`](tools/sync.schema.example) — the schema
of such a sync, given as an example. No runnable sync script is committed here: upstream has
no write access to any mirror, so sync automation cannot run from here. Keep your script
inside your mirror and hold it to the schema: merge upstream commits without rewriting them,
stop on a conflict for a human decision, and run the triple from step 3 before committing
the merge.

An AI agent that works through a hosted GitHub integration needs write access for that
integration. For example, OpenAI Codex cloud fails to create a branch or a pull request with
`403 Resource not accessible by integration` until the owner of the account that holds the fork or
the repository installs the ChatGPT Codex Connector GitHub App and grants it this repository.
Claude Code uses the contributor's own credentials and needs no extra app.

Contributions to BeamCraft's original source code and documentation are licensed under `MPL-2.0`,
as described in the README and `LICENSE`. Add `// SPDX-License-Identifier: MPL-2.0` to each new
Rust source file. Separately licensed third-party dependencies and assets must follow `AGENTS.md`
and preserve their original notices; do not relabel them as MPL-2.0.

Found a security problem? See [`SECURITY.md`](SECURITY.md) instead of opening a public issue.
