# U01 placement, containment evidence and proposed draft PR body

> **Superseded in part by U17 (owner decision of 2026-10-20).** The separate USD worker
> process and the private worker protocol (F05) were removed: USD is read and written in the
> calling process by the layer engine, under its size and depth budgets. Mentions of the worker,
> of a separate process and of F05 below are the history of this analysis, not the current
> design; see [`architecture.md`](../architecture.md) and [`decisions.md`](../plan/decisions.md).

Date: 2026-10-16. Status: **PARTIAL experimental read slice; U01 acceptance remains open**.
Base and current HEAD: `adbc407e09476a803de07c31f3a5adcaabad1304`.
Branch: `feat/u01-open-read`; proposed PR target: `master`; keep draft.
Proposed PR title: Add U01 experimental bounded USD read wrapper.

## What this changes

Adds `beamcraft-layer-engine` at `crates/layer-engine`, with a public engine-level
open/read/close API and original tests through the real pinned Rust backend. The adapter
accepts verified local asset bytes; it has no filesystem/default-resolver delegation,
network access, Python/C++ product calls, subprocess launcher or raw stage setters.
No CLI, host edge, authored schema, UUID index or publication/recovery feature is added.

`Reader` exclusively owns one private stage and a complete immutable typed snapshot.
Open verifies all declared source digests and the manifest commitment before any parser
call, constructs a private candidate, validates it completely and checks cancellation
before replacing the old read view. Failure leaves the previous view and its full
revision/generation unchanged. A new open always gets a new opaque context, even at the
same generation. Close and reopen invalidate old contexts and cursors; another Reader
cannot consume them. ReadSnapshot pages reuse the existing stage/snapshot, with no
reopen/recomposition, and expose no upstream types or mutable handles.

The manifest commitment is an original length-prefixed SHA-256 preimage over full
project/workspace UUIDv7s, full base digest, generation, root, backend pin/features/schema,
profile, ordered asset names/digests/lengths and sorted owned-layer names. It is **not**
F04 canonical op_id replay or a revision-storage format. Authorization and an independently
verified expected manifest remain host preconditions; computing a digest cannot authorize
an asset. Budget tightening cannot widen the pinned content/profile.

## Placement amendment and topology proposal

The prior placement blocker was resolved by the orchestrator's U01 continuation 1:
it explicitly authorizes this crate and the narrow F08 amendment. The module table now
registers a native `host`-kind adapter at layer 2 depending internally only on `model` at
layer 1, labelled as a U01 amendment proposed pending owner review. A single section 4
row records the read wrapper. There is **no new host dependency edge** and no checker
change. Root Cargo.toml already includes `crates/*`; no membership edit was needed.
The CI portable list remains unchanged because the proposed kind is native, not domain.

The current implementation is an experimental **in-process library**. A separate Rust
parser/stage process remains a **proposal for the owner**, not a chosen topology or an
implemented transport. The native placement does not decide the final process boundary.
F05 and the SDK fallback wording are unchanged. The earlier placement review is retained
here with its blocker superseded by the explicit scope expansion, not by a weakened check.

## Supported profile and diagnostics

`BACKEND` pins openusd 0.8.0, the full archive digest, no optional features and
`u01-static-read-v1`. This is a wrapper snapshot profile, not a U02 BeamCraft schema.
Wrong pin/features/schema return UNSUPPORTED_VERSION. The profile supports static USDA
sublayers and external references with explicit targets and identity offsets. Relative
identifiers anchor within a closed virtual manifest namespace. Source IDs are never
interpreted as operating-system paths. Escapes, aliases, absolute/device paths, package
paths, expression paths and URLs fail before backend resolution. Missing dependencies
return DEPENDENCY_UNAVAILABLE; layer cycles return DEPENDENCY_CONFLICT. Both are rejected
before stage composition. All supplied layers, including unused entries, are validated.

The restricted USDC profile supports scalar values and selected uncompressed vectors;
nonempty binary dictionaries, list operations, binary references and compressed value
arrays are unavailable. Structural compression is supported after preflight. Supported
crate versions are 0.8.0 through 0.12.0 under this exact backend pin. USDZ is unavailable.
Payloads, variants, inherits, specializes, instancing, time samples, expression variables,
nonidentity offsets, relationship delivery and unsupported value types return explicit
UNSUPPORTED_PROPERTY. An unsupported construct never produces a plausible partial view.
Single-quoted/triple-quoted USDA strings are outside this conservative lexical profile.

Results contain complete identity, exact unit metadata, bounded authored root metadata,
prim paths/types/active state, full matching element UUID/path/bim:id where authored,
static typed attribute values and all contributing prim sites with source digests and
host-selected ownership. Unknown supported metadata remains inert; source bytes are
never rewritten. Absence and explicit block are distinct. The read guard returns
FOREIGN_READ_ONLY if any contributor is foreign or provenance is missing. Owned targets
also cannot mutate: they return UNSUPPORTED_COMMAND. This is refusal, never authorization.
Unsupported arcs cannot enter a write path because this slice exposes no mutation path.

Numeric checks reject USDA NaN/infinity, binary nonfinite values and text overflowing or
nonzero-underflowing binary64. Exposed floating values have a conservative absolute
1,000,000,000 cap. Arrays and dictionary values are checked recursively within the declared
limits. Units are explicitly metersPerUnit=0.001 and upAxis=Z. No shape is evaluated or
certified: typed field semantics, minimum/positive dimensions, contour/pose validation and
F06 operation-order acceptance still need the U02/engine schema adapter. Negative or zero
arbitrary scalar attributes cannot be classified as dimensions by this raw read slice.
No new geometric tolerance, normalization, clamping or upstream math is introduced.

Diagnostics use existing F04 codes and fixed bounded ASCII messages. Backend error text,
private paths and model strings are not included in API errors. Parse/decode corruption
maps to INTEGRITY_ERROR; a backend unwind maps to BACKEND_UNAVAILABLE. A digest mismatch
fails even when the parser might accept the mutated/truncated bytes as a valid prefix.

## Stage budgets and containment evidence

All budgets are explicit and may only tighten these experimental maxima:

| Budget | Maximum |
|---|---:|
| Source bytes per layer | 65,536 |
| Sum of all supplied source bytes | 262,144 |
| Manifest layers | 8 |
| Lexical nesting / dependency chain / source path depth | 16 |
| Authored or composed prims | 256 |
| Authored fields / structural entries / snapshot properties / dependency work | 4,096 |
| Array scalars / vector entries | 1,024 |
| Binary declared expansion reservation | 4,194,304 bytes |
| Retained snapshot accounting, including root metadata | 1,048,576 bytes |
| Prims per page | 64 |
| Request deadline | At most 120 seconds, host-established monotonic deadline |

These are smaller than F11's experimental proposal, distinct from F05 wire budgets and
not a production allowance. USDA preflight scans bounded bytes, string/token lengths,
balanced/matching delimiters, nesting, array leaf counts, prim declaration counts and
numeric text before parsing. Actual authored fields and typed results are also capped.
USDC preflight checks the header/version, exactly six known nonoverlapping structural
sections, checked offsets/lengths, a complete terminal TOC, declared counts, token expansion
and compressed-block spans before CrateFile::open. It reserves 1,024 bytes per declared
structural entry plus declared token bytes against the aggregate expansion reservation.
After structural open/validation and before lazy value decode it restricts encodings and
checks vector counts. Nested/compressed encodings lacking independent value preflight
remain unavailable. The final composed snapshot and delivery budgets are checked before
installation; partial candidate data is never returned.

**This is not a proved aggregate parser heap/work bound.** The backend synchronous parser,
path builder, schema/token caches, lazy decoder and compositor expose no per-call allocation
or cancellation budget. Traversal callbacks bound retained results, not upstream traversal
work. Structural reservations and post-parse checks cannot prove total allocator use or
prevent a stack overflow, allocation abort or a process abort. A Rust unwind catch is only
an error adapter; the default panic hook can still write upstream text to process stderr.
It is not an OS sandbox or a bounded diagnostic-drain implementation. No hostile large
corpus or enforced process-cap claim is made. Only small synthetic regression fixtures
were dispatched in this in-process run. `parser_contained=false` is returned explicitly.
Do not integrate this slice as a production parser for untrusted files until LEW-03 is
reviewed and proved. Separate-process containment remains the proposed next step; process
caps alone would still not replace declared-count/decompression/work checks.

Cancellation is checked before admission, throughout wrapper loops, before/after upstream
calls and immediately before install. Deadline/cancellation cannot interrupt a synchronous
upstream call. A cancellation racing after the final check can observe a completed read open;
there is no hard cancellation-latency claim. Tests cancel after a real candidate prepare and
prove the old context remains installed. A synthetic backend unwind exercises the same
prepare adapter and also preserves the old context. Neither event advances generation,
records a host outcome, claims a rollback or retries a mutation.

## F05 semantic mapping and proposed amendments for review

| F05 obligation | Current in-process read slice | Proposed separate-process route |
|---|---|---|
| Private pipes, frames and attachments | Function arguments with explicit byte/digest caps; no wire channel is opened | Retain full F05 framing/attachment validation and real-pipe tests |
| Handshake/capabilities/backend identity | Exact BACKEND comparison plus returned profile/capabilities/budgets; no wire handshake claimed | Review Rust runtime identity against the exact backend/schema pin; SDK identity stays fallback-only |
| Correlation and one stage owner | Exclusive Reader owner and opaque incarnation handles; no pipelined transport | Retain request/session correlation, serialization and one worker stage owner |
| Pinned contexts, pages and invalidation | Complete revision/generation tuple plus unforgeable in-process marker; cursor rejects reopen/close/another Reader | Use F05 random wire tokens, nonce rotation and cursor/session invalidation |
| Deadlines, cancellation and liveness | Monotonic deadline, cooperative checks and private candidate install gate; no hard liveness/latency proof | Enforce process caps, cancellation, deadlines, bounded diagnostic drain and loss/restart over real pipes |
| Failure and restart | Failed read prepare preserves old projection; no process restart semantics implemented | Reconstruct only from verified host state; old-session results cannot install |
| Prepare/install/discard and host authorization | Read-only private prepare/install, no mutation command or acceptance claim | Retain isolated mutation candidates and proven host acceptance before installation |
| Canonical op_id replay and serialized acceptance | No mutation/replay API is exposed; remains mandatory for its later owner | Host remains authoritative; no backend replay/acknowledgement proves acceptance |
| Durable outcome queries and uncertainty | No fabricated recorded result or rollback is returned; U06/U07 remain open | Host-ledger QueryOutcome must work without the worker; preserve OUTCOME_UNKNOWN after loss |

Proposed amendments belong to review, not frozen-contract edits: accept or revise the
F08 native row and final topology; specify the exact Rust backend/runtime identity mapping
in F05 while preserving fallback/oracle admission. Review the experimental snapshot/profile
and count/expansion accounting with U02/U04. F04/F06, their frozen cases and all numeric
engineering rules remain unchanged. This PR must remain draft while these gates are open.

## Provenance and dependencies

All BeamCraft code, documentation and synthetic fixtures are contributor-original MPL-2.0.
Tests declare static expected identities, units, values and source ownership; there is no
copied upstream suite, fixture, implementation, commercial material or observation screenshot.
No geometry algorithm was implemented. Numeric authority remains F04 section 3 and F06
sections 2/4/7/10; no external math code or data is incorporated. Wrapper calls use only the
cached public openusd APIs, re-derived rather than copied from the N01 spike.

The exact crates.io archive SHA-256 is
`54f38d869c64c96492e1a482fb621eda620d120d91dbe93072112ee315a8923c`.
Its VCS metadata records `9c007a00fd2ca243a7b6812dd0bf34eca01ee878`, path `crates/openusd`.
The task authorizes an exact cached version; no git download or inferred fork pin was used.
Cargo.lock records 51 third-party packages, matching the cached N01 closure. All 51 archives
matched their lockfile SHA-256s; every extracted archive member matched the corresponding
cached source file. ATTRIBUTION records exact versions, original repository URLs, authors,
declared licences and archive hashes. Hash verification proves provenance/immutability,
not licence completeness or parser safety. A full redistribution/notice review remains open.

openusd's MIT-covered code remains MIT; src/usd/core_schemas.rs remains under the scoped
LicenseRef-TOST-1.0 exception. Existing licenses/TOST-1.0-openusd.txt and NOTICE are preserved;
no TOST material is relabelled or copied. deny.toml adds only exact package exceptions for
the new MPL crate and openusd 0.8.0; the global allowlist is unchanged. Optional mmap/serde
are disabled, though upstream's normal pure-Rust compression and Windows dependencies remain
in the graph. No package acquisition, installation or network access occurred.

## How verified

Windows x64, rustc/cargo 1.96.0, offline Cargo execution:

- `cargo fmt --all -- --check`: PASS.
- `cargo clippy --workspace --all-targets -- -D warnings`: PASS (offline).
- `cargo test --workspace`: PASS (offline), 220 tests including 18 wrapper tests.
- `cargo run --offline -p beamcraft-boundaries -- check`: PASS, no checker edits.
- `cargo check -p beamcraft-layer-engine --target wasm32-unknown-unknown`: not run;
  native proposed placement and target not installed. Only the two Windows targets exist.
  This is not a successful WASM build or an N03 completion claim.
- `cargo deny`: not run, tool not installed; no network installation attempted.
- Dependency metadata/features and full archive/source hash checks: PASS, offline.

Wrapper tests cover original USDA and generated USDC values/IDs/units, metadata and unknown
inert content, blocks/absence, sublayer anchoring, stronger reference opinions and foreign
provenance/refusal, integrity/path/identity/pin faults, missing/cyclic dependencies, numeric
NaN/infinity/overflow/underflow and finite bounds, byte/layer/field/prim/array/depth/output
caps, binary structural count/decompression reservations, malformed/truncated binaries,
pinned paging, stale/cross-reader contexts, cancellation and injected backend unwind.
The binary fixtures are generated from our original text with the pinned crate. Those
reads establish self-consistency only; independently produced USDC/oracle evidence is absent.
U04 API-to-CLI E2E and N03 Linux/WASM/admitted SDK evidence were not run and remain required.
No production safety, full LEW acceptance, F05 real transport or K1 completion is claimed.

## Delivery state

Uncommitted: sandbox Git metadata writes are denied (index.lock creation failed in this
same session). No metadata-write workaround, push or network PR creation was attempted.
HEAD remains the supplied base. The orchestrator must commit/push and create a focused draft
PR targeting master, using this document as the prepared body. Creating that PR does not
authorize integration. The wrapper remains an experimental native library until the owner
reviews placement, topology and the explicitly incomplete containment/acceptance evidence.
