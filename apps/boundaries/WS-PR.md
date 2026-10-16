# WS: make BeamCraft desktop-only and scope WASM checks to the three pure crates

Target: `master`. Branch: `chore/desktop-first-wasm-scope`.
Base: `adbc407e09476a803de07c31f3a5adcaabad1304` (HEAD and local origin/master verified).
Open as a draft. No merge is authorized.

## What this changes

The owner's 2026-10-16 decision makes BeamCraft desktop-only on Windows, Linux and macOS.
Only beamcraft-geom, beamcraft-model and beamcraft-kernel retain WASM compatibility as a
layering discipline. All other crates, including later crates, have no WASM requirement.

- Remove browser/web product plans from README, CLAUDE, architecture and roadmap planning text.
- Amend F08 explicitly as task WS. Reserve `domain` for the three pure crates; introduce `native`
  for desktop libraries and classify protocol and the planned engine accordingly. Keep internal
  edge/layer rules and AGENTS dependency admission. No new dependency is admitted.
- Apply pure-source and dependency checks through the WASM-required kind rule. Prove rejection of
  a native-only dependency from a pure crate in normal, dev, build and target-specific sections.
  Prove native libraries, hosts, apps and tools may use native APIs/dependencies without joining
  the WASM list. Parser, real-workspace and CLI tests assert the exact three-package list.
- Keep CI's table-derived list and change its scope to exactly geom, model and kernel.
- Remove the wrapper WASM build from N03 and U01-U04's inherited platform criteria. Retain Linux
  wrapper builds/tests, Windows/Linux deterministic-export comparisons and the available admitted
  SDK oracle. Update all four G03 evidence-plan criteria to desktop checks on the approved MSRV.
  Historical observations and evidence statuses remain unchanged; JSON remains valid.

## Roadmap entry

- 2026-10-16: WS recorded the owner's desktop-only decision, restricted WASM compatibility and CI to geom/model/kernel, amended F08's table and boundary checks with positive/negative and CLI coverage, and removed wrapper WASM gates from U01-U04/N03 while retaining Linux and SDK-oracle verification; native format, Clippy and 205 workspace tests passed, WASM was not run because the target is absent, and no runtime acceptance gate is completed.

This is a proposed entry only; the ROADMAP progress log is unchanged.

## How you verified it

- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --workspace --all-targets -- -D warnings`
- [x] `cargo test --workspace` (205 tests)
- [ ] `cargo check -p beamcraft-geom -p beamcraft-model -p beamcraft-kernel --target wasm32-unknown-unknown`: not run; target absent. Installed targets are x86_64-pc-windows-msvc and aarch64-pc-windows-msvc. No download attempted.
- [x] `cargo run --locked --quiet -p beamcraft-boundaries -- check`
- [x] `cargo run --locked --quiet -p beamcraft-boundaries -- portable`: exactly `-p beamcraft-geom -p beamcraft-model -p beamcraft-kernel`.
- [x] `git diff --check`, valid JSON, UTF-8 without BOM/LF, allowlist and unchanged progress-log checks.

The initial format check identified new-test formatting, and the initial Clippy run identified
an unnecessary cloned assertion value. Both were corrected before the final passing checks.

## Never-crash

- [x] No new unwrap/expect/panic/todo outside tests; no unsafe.
- [x] No input-derived indexing/slicing or new input-sized allocations.
- [x] The parser's existing caps and fail-closed loading remain in effect.

## Provenance (clean-room)

Contributor-original documentation/checker changes under MPL-2.0 implement the owner's dated
scope decision. Mathematical behaviour remains sourced only from public math sources and F06
sections 2, 4 and 7; WS implements or changes no mathematical behaviour, tolerances, fixtures,
geometry encoding or conformance expectations. No third-party source or asset is added or
copied; no commercial-product observation or material is used. No new attribution is required.

- [x] No Tekla/Trimble assets, binaries, API source or samples used.
- [x] No GPL/AGPL code copied.
- [x] No new dependencies, SDK resources, outbound product access or runtime feature added.

## Frozen-contract amendment proposed separately

F06 section 4, the sentence requiring conformance evidence on each target including wasm32:
clarify that WASM target conformance concerns geom/model/kernel only, while engine, query and
viewport consumers are desktop-native. Propose this for the frozen contract's separate review;
do not edit F06 in WS. No F04 amendment is needed. All frozen contracts and fixtures are unchanged.
Historical spike/evaluation/PR records and the ROADMAP progress log are unchanged.

## Limits and handoff

The WASM target is absent, so compatibility compilation is unverified locally. Linux/macOS
execution, remote CI and the N03 wrapper/SDK oracle were not run by this documentation/CI task.
No U-task or K0/K1 acceptance is claimed.

Uncommitted: staging failed with permission denied creating the shared worktree `index.lock`.
The sandbox cannot write Git metadata outside this worktree. Per the task instruction, no
workaround, commit, push or PR creation was attempted after that failure. The orchestrator must
commit these files, push the existing branch and open one draft PR targeting master, using this
file as its body. This handoff file is inside the writable `apps/boundaries/**` allowlist.
