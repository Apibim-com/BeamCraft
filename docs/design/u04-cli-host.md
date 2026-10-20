# U04 shared CLI/host and separate Rust USD process

Date: 2026-10-16. Status: **PARTIAL experimental read integration; keep PR draft**.
Base: `bca89e23671470ba0633497b82d38a83fb5cc11d`, verified local `origin/master`.
Branch: `feat/u04-cli-host`; intended target: `master`.
Proposed title: Add U04 shared native CLI dispatcher and separate Rust read worker.

## What this changes

CLI and native callers use one typed command dispatcher. A thin separate process constructs
the U01 Reader, owns its stage on one thread, and services private-pipe control on another.
The ordinary path invokes no Python/C++ runtime, shell, plugin discovery or listening socket.
Explicit SDK selection fails unavailable; no fallback is attempted. The existing CLI package
holds both binaries, so no package, third-party dependency or licence exception is added.
The module table amendments record that placement and the two additional CLI dependency edges.
No other checkout, authored-layer implementation, UUID index, roadmap log or frozen contract
is changed. Cargo.lock changes only existing internal package dependency edges.

The host verifies a packaged executable digest before each spawn and permits only controlled
stdio pipes and a cleared environment (Windows SystemRoot retained). A protected existing U05
root supplies its working directory. Protected package/store namespaces are prerequisites;
safe std checks cannot defeat hostile concurrent filesystem replacement. No OS sandbox or
unrelated-handle inheritance proof is claimed.

OpenStage retains F05's five body fields. Its manifest digest identifies a host-produced
U05 grant containing `manifest` and `owned_layers`. The separately verified read manifest
contains exactly `identity`, `root`, `assets`, `backend_archive`, `profile`; each asset has
`name`, `digest`, `byte_length`. Ownership comes from the caller's explicit host grant, not
the read manifest or USD. The worker revalidates all bytes/digests/identities and constructs
U01's existing manifest commitment. Source bytes remain immutable files addressed by full
digest. No source bytes, derived mesh or large array is streamed as a JSON numeric buffer.
The profile negotiates zero wire attachments. Large derived-blob delivery is not implemented.

Read results expose finite typed values, metadata and contributing sources, never upstream
handles. Each host page check verifies source names/digests against the authorized set and
recomputes the owned flag. It checks static value shape/depth/counts, numeric bounds, path
length/depth, full UUID/path/bim:id consistency, closed property records and duplicate paths.
The host checks continuation progression and total complete delivery counts across pages.
Page metadata is at most 64 prims under U01's 256-prim profile. Generic JSON ceilings may
reject deeply nested or large typed U01 output before delivery; no unbounded paging fallback
is used. The CLI `read` returns the first page; the native retained session supports paging.

Open failure retains the previous authorized read intent and known tuple. Backend loss
terminates/reaps the old child before restarting with new pipes/nonce/tokens. The host retries
reads only, from the same verified manifest and nonzero generation. Each attempt permits at
most two delayed restarts (250/1000 ms); malformed/incompatible peers are not automatically
retried. A continuation cannot survive restart. A forced-loss handle belongs to one child;
an old handle cannot kill its replacement. No unsent application change is stored in or
deleted with this disposable projection.

The host deadline includes package hashing, transmission, recovery and result validation.
Hashing polls it by chunk. The receive side tracks first-byte assembly and request deadlines;
workers can process ping/cancel during composition. A read timeout returns backend unavailable
with IPC_DEADLINE, never inferred rollback. A cancellation flag or advisory frame alone never
proves mutation cancellation: no mutation is dispatchable here. Native errors retain the
host-known workspace tuple. CLI JSON uses fixed safe text and stable exit codes; raw child
stderr, caller paths and exceptions are never forwarded. Diagnostic reading stops after
64 KiB and retains zero raw bytes. Reaping uncertainty blocks replacement.

## Capabilities and acceptance limits

The open result explicitly reports false for parser containment, mutations, save, relationships,
publication and durable outcomes. The host rejects a worker claiming otherwise. QueryOutcome
works without a worker and returns OUTCOME_UNKNOWN; it never treats `not_observed`, a repeated
op_id or a backend acknowledgement as a recorded rejection/acceptance. There is no canonical
F04 replay ledger, candidate prepare/install/discard, operation collision decision, event stream
or revision recovery implementation. Those capabilities cannot be inferred from read retries.
U02/U03 API -> CLI -> save -> fresh independent reader acceptance remains unavailable.

LEW-01: the read dispatcher exposes no mutation/setter route; foreign provenance is validated,
and unknown mutation/save commands are unavailable. Full foreign mutation refusal is not proved.
LEW-02/04: real-process file numeric/digest faults and host response checks are tested.
LEW-03: byte/tree/page/count/depth bounds and process deadlines are implemented, but upstream
aggregate allocation/work bounds and enforced OS memory caps remain unproved. Process separation
does not satisfy those gates. LEW-05/06 and mutation-side LEW-07 remain unavailable. Read-side
context rotation, loss, cancellation and uncertainty evidence does not complete those requirements.

Standard RandomState outputs provide experimental non-authorizing incarnation tokens. This is
not a guaranteed 32-byte cryptographic entropy API. Exact F05 nonce/stage randomness admission
remains open; no entropy/dependency claim is fabricated. Idle sessions have no background host
keepalive: the worker expires idle input after 20 seconds and a later read recovers. In-flight
ping/pong is implemented; full idle liveness admission remains open. There is no full negotiated
small-limit implementation: this first profile requires its exact tested limits.

The request profile is explicit and narrower than full F05. Reserved unquoted USDA
`beamcraft:plugin`/`beamcraft:script` directives are refused before parser entry; inert comments
and strings are not execution. Unknown worker plugin options are refused at startup. There is
no plugin/script loader for any encoding. This is not a general USD conformance/sandbox proof.
Platform containment and production admission must be resolved before treating the CLI as a
production hostile-parser boundary. No new crate is requested or acquired in this slice.

## Proposed contract amendments for review

- F05 Rust identity mapping: backend name `beamcraft-rust-openusd`, version 0.8.0; sdk_version
  carries the admitted archive identity, python_version is null. Confirm naming against the
  verified executable manifest; retain the SDK fallback's F03 admission separately.
- Review `u04-rust-read-v1`'s schema commitment, closed read bodies/static value representation,
  zero attachments and actual features (`private_pipes`, `sha256_buffers`, `cancel`). Full F05
  requires outcome/event features; this profile does not falsely advertise them. It is not
  a compliant full-F05 handshake or a U02 authored schema.
- U01 uses a digest base revision while frozen F04 distinguishes RevisionId UUIDv7 from content
  digest. This integration preserves U01 exactly; resolve that mismatch explicitly before U06.
- Review final OS containment, guaranteed random-byte source, idle liveness and diagnostic
  schema admission; neither a worker acknowledgement nor std process isolation closes them.

Frozen F04/F06, F05 and their acceptance case files are unchanged.

## How verified

Windows x64, Rust/Cargo/Clippy 1.96.0, offline Cargo; no dependency acquisition.
Final verification:

- `cargo fmt --all -- --check`: PASS.
- `cargo clippy --workspace --all-targets -- -D warnings`: PASS.
- `cargo test --workspace`: PASS, 236 tests including eight real-process tests.
- `git diff --check`: PASS (tracked edits; new text separately checked for LF/no BOM).
- `cargo check -p beamcraft-cli --target wasm32-unknown-unknown`: not run. No new crate was
  created; this package and its new worker binary are native-only under the WS amendment.
  Only Windows x64/ARM64 targets are installed. This is not a WASM pass.
- Linux/macOS, admitted SDK/independent-reader comparison and cargo-deny: not run.
  No runner, admitted oracle or extra tool was acquired. No evidence dumps are committed.

The real-process suite covers API/CLI read parity with cleared Python/SDK/search environments,
valid original USDA, corrupt/non-finite inputs, executable pin mismatch, unknown plugin options,
explicit reserved directives, source provenance, corrupt U05 bytes, bounded paging, stale contexts,
close, mid-request forced child loss and automatic reconstruction, deadline expiry, cancellation
over an actual pipe, correlation reuse, a process killed during partial bundle assembly, and
host-only unknown outcomes. The crash injection is at the host's pending request boundary and
partial frame assembly, not a claimed deterministic instruction inside the upstream parser.
Small pure validator tests reject forged ownership, invalid IDs/paths, oversized pages, extra
fields, malformed JSON/surrogates/decoded duplicate keys/depth and lossless generation faults.
Tests do not claim memory-cap containment, an independent USD reader, SDK parity or full F05 cases.

An initial external taskkill-based test could not terminate the child (Access denied in this
environment). Tests now use the existing owned Child handle and the host's incarnation-specific
supervision handle. An intermediate concurrent test relink hit Windows LNK1104 while its old
test executable was running; final checks are sequential. These are not passing evidence.

## Never-crash and provenance

All new code and inline synthetic inputs are contributor-original MPL-2.0. No external asset,
model, test suite, catalogue, implementation, commercial software material or observation was
copied. No new distributed asset or attribution exception is introduced. Existing U01 upstream
pin/notices remain unchanged. No unsafe or production unwrap/expect/panic/todo is introduced;
untrusted lengths are checked and allocations capped. No new mathematical algorithm or geometry
policy is implemented: public math provenance and F06 sections 2/4/7/10 remain exactly as admitted.
Transport behavior derives from repository F05; USD behavior stays behind the existing U01 API.

## Delivery state

Uncommitted: `git add` could not create
`D:/Projects/01_BeamCraft/.git/worktrees/01_BeamCraft-U04/index.lock` (Permission denied).
No Git metadata workaround, escalation, push or network PR creation was attempted. HEAD remains
the supplied base. The orchestrator must commit/push and create one focused draft PR targeting
master using this prepared body. No merge is authorized. Keep the PR draft while the above
gates remain open. The worktree contains the implementation and tests; U04 is not complete.
