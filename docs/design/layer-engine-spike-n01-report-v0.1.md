# N01 option C: first-slice spike report, version 0.2.0

Task: evaluate the pure-Rust `openusd` crate behind a BeamCraft-owned trait, executing the
owner's 2026-10-15 option C decision ("Variant C. Keep the spike. We should ideally move away
from Python and C++ if that helps."). Branch `feat/n01-openusd-spike`, stacked on PR #22.
Status: first slice of the N01 plan, **not** N01 completion, not a parent acceptance gate and not
an architecture change. This version replaces 0.1.0, which described an acquisition blocker that
no longer applies (the crate is now in the local Cargo cache).

## Scope and method

Read `AGENTS.md`, `CLAUDE.md`, `ROADMAP.md`, `docs/architecture.md`, the F11 decision and
evidence plan, the F04 model/command contract and the F08 module-boundary contract. All
experiment code is contributor-original. No upstream code, tests or fixtures were copied; this
report describes observed behaviour and the public API that was used, not upstream internals.

The spike is an isolated package in `spikes/n01-openusd/` with its own empty `[workspace]`
table and its own `Cargo.lock`. It depends only on `openusd =0.8.0` with
`default-features = false` (optional `mmap` and `serde` disabled). It is not a dependency of any
repository crate and does not go through the repository `cargo deny`.

Contents:

| Path | Purpose |
|---|---|
| `src/lib.rs` | Owned `LayerBackend` trait, `RustBackend`, UUIDv7 path validation, fixture author, structural check, `(UUID, attribute)` key extraction |
| `src/main.rs` | Child-process entry points `parse PATH` (hostile input) and `sanity COUNT DIR` (performance) |
| `tests/evidence.rs` | Nine tests: round trip, adapter edit, composition, foreign reference, delta, guards, unknown metadata, numeric overflow, stub backend |
| `tests/supervision.rs` | Ten-case smoke run of the hostile supervisor under an external deadline |
| `harness.py` | Development supervisor only (Windows Job commit cap, 5 s deadline): `inventory`, `hostile`, `performance`. USD work happens in the Rust binary |
| `checks.py` | Runs the task's check list and verifies that the root `Cargo.lock` is unchanged |
| `EXPECTATIONS.md` | Expected outcomes frozen before the first candidate test |
| `inventory.json`, `hostile-summary.json`, `performance-results.json` | Compact committed results |
| `reproducers/numeric-overflow.usda` | Minimal synthetic reproducer (finding 1) |

Repository hygiene (changed from the first run): raw `cargo metadata` output, the full
`cargo tree` text, per-input hostile logs and per-check logs are **not** committed. Bulk output
goes to `spikes/n01-openusd/artifacts/` (ignored by Git). The earlier bulky files remain in Git
history only.

Regeneration, from `spikes/n01-openusd` with the Cargo cache populated:

```text
python harness.py inventory                 # writes inventory.json
cargo build --release --offline
python harness.py hostile                   # writes hostile-summary.json (bulk log to artifacts/)
python harness.py performance               # writes performance-results.json
python checks.py                            # runs the full check list, bulk log to artifacts/
```

## Environment

| Item | Value |
|---|---|
| OS | Windows 10 Enterprise 10.0.19045 (Python `Windows-10-10.0.19045-SP0`, AMD64) |
| CPU | Intel64 Family 6 Model 165 Stepping 5 (GenuineIntel) |
| rustc / cargo | 1.96.0 (ac68faa20 2026-05-25) / 1.96.0 (30a34c682 2026-05-25) |
| Installed targets | `aarch64-pc-windows-msvc`, `x86_64-pc-windows-msvc` |
| `openusd` | 0.8.0, declared licence `MIT`, declared `rust-version` 1.96, features `mmap`, `serde` available, none enabled |
| Official OpenUSD tooling | Not available (`usdcat`, `usdchecker` not on PATH; not installed) |

## 1. Dependency, licence inventory and MSRV

Claim **N01-C-G01**: evidence result `failed` (one policy exception outside the allowed list at
measurement time), see below. The owner has since resolved the exception by a licence decision;
the measured result is unchanged (see "Owner licence decision").

Commands (all offline): `cargo metadata --offline --format-version 1` (processed by
`harness.py inventory`; raw output not committed), `cargo tree --offline --edges all` (read, not
committed). Result: 52 locked packages, equal to the packages Cargo resolved: the spike itself,
`openusd 0.8.0` and 50 transitive packages. Versions, declared SPDX expressions and declared
`rust-version` for every package are in `spikes/n01-openusd/inventory.json`.

Licence findings against the task's allowed list (MIT, Apache-2.0, BSD-2/3, ISC, Zlib,
Unicode-3.0, BSL-1.0, CC0-1.0, MIT-0):

| Finding | Detail | Disposition |
|---|---|---|
| **TOST-1.0 file inside `openusd`** | The crate README states that `src/usd/core_schemas.rs` is generated from OpenUSD schema definitions and covered by the Tomorrow Open Source Technology License 1.0; the package ships `third-party/LICENSE` and a Pixar `NOTICE.txt`. The manifest says only `MIT`. | **Outside the allowed list as measured.** F11 rule: MIT headline is not blanket approval. Resolved afterwards by the owner decision below; the measurement itself is unchanged. |
| `Unlicense OR MIT` (`aho-corasick`, `memchr`) | Alternative expression | MIT can be elected; allowed |
| `Apache-2.0 / MIT` (`fnv`) | Non-standard SPDX spelling, both allowed | Allowed; note for `deny.toml` parsing |
| `Apache-2.0` only (`zopfli`) | Single licence, in task list | Allowed |
| `(MIT OR Apache-2.0) AND Unicode-3.0` (`unicode-ident`) | Both in list | Allowed |
| All other packages | MIT, MIT OR Apache-2.0, Zlib, or Zlib/BSD-2-Clause alternatives | Allowed |

**Owner licence decision, 2026-10-15** (recorded in PR #29, not merged at the time of writing;
`AGENTS.md` section 1.2 scoped extension, `NOTICE`, `licenses/TOST-1.0-openusd.txt`,
`ATTRIBUTION.md`): "Everything under TOST belongs to OpenUSD; I do not need their rights. We
adopt all of OpenUSD and all of OpenUSD's rights remain with them; put the notices where they
are needed." The TOST-1.0 file `src/usd/core_schemas.rs` of `openusd` 0.8.x, generated from
OpenUSD's own schema definitions, is accepted. Conditions:

1. Only the exact package and version recorded in `ATTRIBUTION.md` are covered.
2. The upstream licence text and notice are kept.
3. BeamCraft claims no rights in the material and trademark terms are respected.
4. The global licence allowlist in `deny.toml` is unchanged.
5. If the package enters the workspace graph, a package/version exception and the file-level
   notice review are required first.

The decision resolves the policy question only. The crate is still **not adopted**, it is not in
the workspace graph, no file-level review or reviewer sign-off was done here, and no option is
selected.

The graph contains build-time procedural-macro packages (for example `logos`, `strum`,
`derive_more`, `thiserror`, `syn` in two major versions) and compression packages (`zip`,
`flate2`, `zlib-rs`, `zopfli`, `lz4_flex`). Cargo resolves them although optional features are
disabled. No native (C/C++) build step was observed; this was not independently audited file by
file. The full file-level notice review required by the claim (every included file, reviewer
sign-off) was **not** done.

**MSRV.** The crate declares Rust 1.96; the repository MSRV is 1.95. The spike builds,
passes Clippy and passes its tests on the local 1.96.0 toolchain. Building on 1.95 was **not
run** (no 1.95 toolchain installed; Cargo is expected to refuse a dependency that declares a
newer `rust-version`). Highest declared `rust-version` among the other dependencies is 1.88.
Adopting the crate in the workspace would therefore need an owner decision to raise the
workspace MSRV from 1.95 to 1.96, plus toolchain and CI review. This spike changes no MSRV.

## 2. USDA and USDC round trip and unknown-field preservation

Claims **N01-C-G05**, **N01-C-G09**: partially evidenced, status `not_run`.

Test `usda_usdc_roundtrip_every_authored_field_and_second_write` (passes). Method: author a
synthetic model with the crate (two complete UUIDv7 element prims named
`E_<32 hex digits>`, `bim:id`, explicit-unit double attributes such as `steel:plate:widthMm`,
a relationship, `active = false`, an unknown custom string attribute, prim `customData`, layer
`customLayerData`, `metersPerUnit`, `upAxis`), write USDA, read it back, snapshot every
`(path, field)` pair, write USDC, read that back and compare.

Observed:

- The crate writes both USDA and USDC. Every authored `(path, field)` value, including
  bit-exact doubles, relationship targets, the unknown attribute, prim and layer dictionaries
  and stage metadata, was equal after both round trips.
- A second write of the re-read data is byte-identical for USDA (text equality) and for USDC
  (byte equality). This is determinism within one build only; 20 reordered repeats and
  cross-platform digests were not run.
- Test `unknown_prim_metadata_survives_text_and_binary`: unregistered prim metadata
  (`futureThing`) and a namespaced custom attribute survive USDA rewrite and USDC. This covers
  those two kinds of unknown content, not unknown content in general.
- Test `owned_trait_edit_preserves_unknown_content_and_identity`: one width edit through the
  owned trait changes exactly one field; every other authored field is unchanged and the edited
  file reopens with the new value.

Not run for G05: rename, duplicate/short ID rejection beyond path validation, copy with fresh
IDs and remapped links, digest audit.

## 3. Composition and foreign ownership

Claims **N01-C-G04**, **N01-C-G05**: partially evidenced, status `not_run`.

Test `sublayers_strong_over_and_active_false` (passes), with sublayers `[strong, middle, model]`:

- Strongest sublayer opinion wins (width 1200.5 over 1100.5 over 1000.125). Muting the strong
  layer selects the middle one (1100.5). A stronger `over` on the root layer wins (1300.5).
- `active = false` prim is excluded from default traversal and included after reactivation.
- Zero composition diagnostics.

Test `foreign_reference_bytes_unchanged_but_raw_override_allowed_wrapper_rejects` (passes): a
root layer references a read-only foreign layer's `/Source` into an element path. The composed
width reads 77 and the prim stack names the foreign layer.

**Key finding for the foreign-layer rule:** the crate does **not** structurally prevent a stronger
local override of a foreign prim. A raw attribute set through the stage on the referenced prim
succeeds (width becomes 99 in the composed view) and is authored in the local edit layer; the
foreign file's bytes and read-only flag stay identical, because the edit goes to the edit layer,
not to the foreign file. The repository forbids such overrides, so the rule must be enforced by
a BeamCraft wrapper.

Wrapper rule exercised by `RustBackend` (and rejecting with `FOREIGN_READ_ONLY`): derive the
prim path only from a validated complete UUIDv7; read the prim stack of that path; require
**every** contributing layer to be in the host-selected owned-layer set (canonicalised paths);
require the authored `bim:id` to match the path; require the edit layer itself to be owned;
accept only an allow-listed field with a finite, bounded value; never expose a stage, a path
setter or a writable foreign-layer handle; export only the owned edit layer to a fresh path.
This is a spike of the rule on a trusted synthetic fixture. It is not a contained asset
resolver and does not cover variants, payloads, inherits or specializes, which were **not
tested**. No SDK oracle comparison was made (G04 requires one).

## 4. Delta and merge model

Claim **N01-C-G06**: partially evidenced, status `not_run`.

Test `captured_deltas_replay_and_detect_same_identity_attribute_conflict` (passes). The crate
offers a replay wrapper around a stage that records authored edits as a list of diffs, and a
stage call that applies a diff to the current edit target. Observed:

- A width edit and a label edit were captured as separate diffs of field-set edits keyed by
  attribute path.
- Replaying both on a fresh copy of the base gave an authored snapshot equal to the original
  edited stage, identity unchanged.
- BeamCraft-side normalisation to `(prim identity, attribute)` keys (function
  `normalized_keys_for_edits`, which requires a known prim-to-UUID mapping and the `default`
  field) detected the collision between two competing width edits (one shared key) and no
  collision between width and label edits (zero shared keys).

What the crate gives: change capture, a diff representation, replay onto another stage. What
BeamCraft must write: command authorisation and provenance validation; the prim-to-UUID mapping;
engine-derived read/write/relationship footprints (different attribute names do not prove
independence: F04 section 8 requires the complete shape-input group); typed absence, block and
removal; pinned base and generation; idempotent replay (duplicates change no generation);
gap and stale-base handling; recoverable rejected local intent. None of these was run, nor was
the 100-delta replay. The spike handled only field-set edits and refuses anything else.

## 5. Untrusted input

Claim **N01-C-G08**: partially evidenced, status `not_run`.

Method: `harness.py hostile` generates all inputs, writes each to a file of at most 64 KiB, and
runs the release binary on it in a fresh child process with a 5 s deadline and a 512 MiB
Windows Job commit cap, with the parent assigning the Job before the child is released. The
child reads the bytes, parses them with the crate (text or binary detected by the crate),
traverses every spec and reads every field value, and reports a result line. A panic is caught
and reported by the child.

Result (`hostile-summary.json`): **1184 cases, 0 panics, 0 crashes, 0 timeouts, 0 cap
terminations.** Maximum 0.007 s per case, maximum job commit 1 064 960 bytes, maximum working
set 4 612 096 bytes.

| Class | Cases | Rejected | Accepted |
|---|---|---|---|
| Single hand-written cases: truncated, malformed bracket, invalid UTF-8, huge integer, corrupt `PXR-USDC` header | 5 | 5 | 0 |
| Huge exponent `1e999999999` | 1 | 0 | 1 |
| Brace nesting depth 32, 33, 64 | 3 | 0 | 3 |
| Huge `faceVertexCounts` literal | 1 | 0 | 1 |
| Every byte-prefix truncation of a 45-byte USDA | 45 | 42 | 3 |
| Fixed-seed random token streams | 1000 | 1000 | 0 |
| USDC with 8 header bytes set to 0xFF | 1 | 1 | 0 |
| USDC with one of the first 128 bytes inverted | 128 | 100 | 28 |

Observations (each is a finding or a limit, not a safety proof):

1. **Non-finite numbers accepted** (`reproducers/numeric-overflow.usda`): `1e999999999` is read
   as positive infinity without error (test `hostile_nonfinite_value_is_not_engine_validity`).
   BeamCraft must reject non-finite engineering input itself.
2. Three truncations were accepted: two are the bare header (empty layer) and one is the
   complete document minus its final newline. Truncation to an empty or valid-prefix layer is
   not an error in this crate; integrity must come from BeamCraft's digest/manifest layer.
3. 28 single-byte corruptions of a small USDC were accepted with the same spec and field count
   (85 fields). They sit in the file header region (bytes 10-15, 24-39, 60-65). Whether these
   bytes are ignored padding/reserved or whether values changed was not analysed. Corruption is
   not reliably detected.
4. Nesting up to 64 was accepted; no depth limit was found in that range and larger depths
   were not tried. The huge-count case is a one-element literal, not an allocation test.
5. Error text quality: rejected inputs return an error value (truncated to 350 characters in
   the log); no panic message was produced. Quality was not graded further.

Not run: each declared stage cap at maximum and maximum+1 (inputs were limited to 64 KiB by the
harness, far below the plan's 16 MiB layer cap), cumulative resolution caps, composition
cycle detection, network and plugin denial. Opaque parser allocation limits therefore remain
unproved: the Job cap was never reached, which shows these inputs are cheap, not that the
parser bounds allocation from declared counts. The harness requires Windows (Job objects) and
refuses to run elsewhere.

## 6. Portability

Claim **N01-C-G03**: status `not_run`.

`rustup target list --installed` lists only the two Windows targets, so
`cargo check --target wasm32-unknown-unknown --lib --offline` was **not run** (target not
installed; nothing installed). Linux and macOS builds were **not run**. Windows x64 on Rust
1.96.0 builds and passes (section "Verification"). Rust 1.95 not run (section 1). The optional
`mmap` feature adds a memory-mapping dependency and `serde` adds `serde` and `half/serde`;
both are disabled in this spike and their portability effect is unknown (mapping a file is
likely unavailable in a browser build, unverified).

## 7. Performance sanity

Claim **N01-C-G09**: partially evidenced, status `not_run`. Not a benchmark.

Method (`harness.py performance`): release build; fixture generated before timing (a project
root sublayering one model layer, one `Xform` prim per element named by its full 32 hex digit
UUID, explicit string `bim:id` and double width); three fresh child processes per size; warm OS
file cache; open measured separately from compose-and-traverse (traversal reads every width
and checks the sum) and from save (export of the model layer). Process start-up is excluded
from the in-process timings.

| Elements | Open (ms) | Compose + traverse (ms) | Save (ms) | Peak working set |
|---|---|---|---|---|
| 1 000 | 5.7 | 24 | 68-71 | about 12 MB |
| 10 000 | 54 | 245-268 | 634-636 | about 61 MB |

Values are from the three runs in `performance-results.json` (ranges, not statistics). Total
open + compose + save at 10 000 elements is about 0.95 s on this machine, well below the
plan's 5 s / 512 MiB ceilings, but the plan's 30 cold runs, p95 and 20 reordered repeats were
not performed, so G09 is not evaluated.

## 8. Interoperability and structural validation

Claim **N01-C-G10**: status `not_run`.

**Oracle not available: not run** for every differential claim; no official OpenUSD tool was
found on PATH, none was installed, and a Python `pxr` import was not attempted. Two weaker
checks exist: files written by the crate are re-read by the same crate (self-consistency only,
sections 2-4), and `structural_check` is an original bounded check of the header line and
balanced quoting/brackets/comments/asset paths with a depth cap (tests
`identity_and_structural_guards` and the round trip). Neither can show conformance to the
OpenUSD grammar or composition semantics: they cannot detect an error the crate shares with
itself. Interchange in either direction with the official SDK is **not shown**.

## Claim table (option C)

Statuses are limited to what was run. `not_run` is inconclusive, never passed.

| Claim | Status | Evidence / what is missing |
|---|---|---|
| N01-C-G01 licence and provenance | **failed** (as measured); resolved by owner decision | Inventory done (section 1). TOST-1.0 generated file inside an MIT-declared crate was outside the allowed list; owner decision 2026-10-15 accepts it under five conditions (PR #29). No file-level review or reviewer sign-off; crate not adopted |
| N01-C-G02 maintenance and API containment | not_run | Owned trait keeps upstream types out of its signatures and a std-only stub passes the same numeric exercise (test `adapter_contract_accepts_a_std_only_stub`), but no upgrade or perturbation exercise, no maintainers named |
| N01-C-G03 MSRV, platform, WASM | not_run | Windows x64 + Rust 1.96.0 only (sections 1, 6) |
| N01-C-G04 composition and ownership | not_run | Sublayer/over/active/reference observed; raw foreign override accepted by crate, rejected by wrapper; no oracle, variants or payloads (section 3) |
| N01-C-G05 identity, copy, unknown fields | not_run | Unknown content and identity preserved on edit; copy/remap/rename not run (sections 2, 3) |
| N01-C-G06 semantic conflicts and live deltas | not_run | Capture, replay and key collision shown; footprints, replay-once, gap/stale not run (section 4) |
| N01-C-G07 atomic publication and outcome | not_run | Outside this slice; no storage runtime |
| N01-C-G08 bounded untrusted input | not_run | 1184 cases, zero panics/hangs; caps at limit, allocation bounds, cycles not run (section 5) |
| N01-C-G09 determinism and cost | not_run | Same-build second write identical; three warm runs; 30 cold / 20 repeats not run (sections 2, 7) |
| N01-C-G10 interoperability and drift | not_run | Oracle not available (section 8) |

The evidence plan JSON carries the same statuses and per-claim `evidence` references for the
claims that were run (G01, G03, G04, G05, G06, G08, G09, G10). G02 and G07 are untouched. IDs and
counts (40 claims, ten per option, ten gate groups, twelve scenario families) are unchanged.

## Findings

1. **Licence exception:** TOST-1.0 generated file in an MIT-declared crate (section 1). Resolved by
   owner decision 2026-10-15 (PR #29) under five conditions; the measured result is unchanged.
2. **MSRV mismatch:** crate declares 1.96, repository 1.95 (section 1).
3. **Non-finite numbers accepted** as values (`reproducers/numeric-overflow.usda`; section 5).
4. **Truncation and header corruption can be accepted silently** (section 5).
5. **Foreign-override prevention is not in the crate**: a raw set on a referenced prim succeeds; BeamCraft's wrapper must enforce the rule (section 3).
6. **Parser allocation bounds unproved** from declared counts; only cheap inputs were tried (section 5).
7. **Supervision harness is Windows-only**; Linux/macOS isolation was not built.
8. `wasm32-unknown-unknown` target missing locally; portability unverified.
9. No panic, hang or crash was observed in the 1184-case corpus or the nine evidence tests.

Reproducer for finding 3 (`reproducers/numeric-overflow.usda`):

```text
#usda 1.0
def "A" { double x = 1e999999999
}
```

## What BeamCraft would have to write on top (logical terms)

A BeamCraft-owned backend trait exposing only BeamCraft identities, values and bounded
diagnostics: bounded open/read with contained asset resolution and an explicit revision
context; enumeration of composed identities with source provenance; typed reads of authored
opinions and relationships; preparation of owned edits in a candidate that can be validated,
installed or discarded; normalised change capture mapped to engine-derived footprints;
export of owned authored layers retaining unknown content; and the ownership rule of
section 3. Host storage, authorisation, atomic publication, a durable outcome ledger,
input-size and count limits before parsing, non-finite rejection, integrity digests and
cross-platform process isolation stay outside the crate. The spike's `LayerBackend` (read a
number, write a number, export the edit layer) is a minimal slice of this surface.

## Python/C++ avoidance verdict (final answer for the owner)

Question: can the Python/C++ OpenUSD worker be avoided for each first-slice need? The answer
below is limited to the synthetic fixtures run here, on Windows with one crate version, with
no official-SDK comparison. "Shown" means the behaviour was observed; it does not mean the
output is conformant.

| First-slice need | Avoidable on this evidence? | Basis and limit |
|---|---|---|
| Open / read | **Yes, shown for the tested subset** | USDA and USDC read, composed values, metadata, relationships; hostile corpus survived. Limits: only self-consistency; corruption and truncation can be accepted; allocation bounds unproved |
| Author | **Yes, shown for the tested subset** | Prims, typed attributes, relationship, metadata, `active`, `over` authored through the crate |
| Compose | **Partly shown** | Sublayers, stronger `over`, `active`, internal reference to a foreign layer. Variants, payloads, inherits, specializes and an oracle comparison not done |
| Write | **Yes, shown for the tested subset** | USDA and USDC both written; same-build second write byte-identical. No independent reader checked the output |
| Foreign read-only references | **Yes, only with a BeamCraft wrapper** | The crate permits a local override of a foreign prim; the wrapper rule rejected it in the spike. Needs a contained resolver and wider cases |
| Delta / merge | **Partly shown** | Capture, replay and `(UUID, attribute)` collision detection shown; footprints, idempotency, gap/stale handling must be written by BeamCraft and were not run |

Overall: nothing observed requires Python or C++ for these needs, but the evidence does not
yet establish that the worker can be dropped. Open blockers: the MSRV change (the licence
exception is resolved by owner decision, with conditions), the lack of an oracle for
interchange and composition correctness, and unproved parser limits. The crate is **not**
shown to be suitable as the replacement backend, is **not adopted**, no option is selected,
and no conclusion about the accepted SDK worker baseline follows.

## Risks, effort and next steps

Risks: licence conditions (TOST file; accepted by owner decision), MSRV, pre-1.0 API
stability and maintenance ownership, silent acceptance of damaged input, spec drift without
an oracle, unproved allocation bounds, platform coverage.

Order-of-magnitude effort only (not a measurement): a few days to extend this slice to the
remaining option C claims that can run locally (variants/payloads, copy/remap, footprints,
cold-run statistics, Linux/macOS/WASM when targets and machines exist); weeks for a production
adapter. F11's 8-20 person-week first-slice integration estimate remains an unvalidated prior.

Next steps: obtain an official-SDK oracle in an admitted environment for differential claims;
install the WASM target and run the Linux/macOS checks; then decide (the TOST question is
resolved by owner decision).

Owner decisions that remain:

1. G01: admissibility is decided (2026-10-15, PR #29); the package/version exception and file-level
   notice review remain required before the crate enters the workspace graph.
2. Raising the workspace MSRV from 1.95 to 1.96 or rejecting the crate on that basis (G03).
3. Maintenance owner and backup, and upstream stability policy for a pre-1.0 crate (G02).
4. Whether to continue N01 for option C (remaining gates) or stop; backend selection after evidence.

## Verification

Run on 2026-10-15 in `D:\Projects\01_BeamCraft-N01` on branch `feat/n01-openusd-spike` (base
head `8957c40`). All Cargo commands used `--offline`.

| Command | Exact result |
|---|---|
| `git status -sb`, `git log --oneline -3` at start | Branch `feat/n01-openusd-spike`, clean, head `8957c40`: matched expectation |
| `cargo fmt --all -- --check` (spike) | Exit 0 |
| `cargo clippy --all-targets -- -D warnings` (spike) | Exit 0, no warnings |
| `cargo test` (spike) | Exit 0: `tests/evidence.rs` 9 passed; `tests/supervision.rs` 1 passed; no unit/doc tests |
| `python -c "import json; json.load(open('docs/design/layer-engine-evidence-plan-v0.1.json', encoding='utf-8')); print('json ok')"` | Exit 0, `json ok` |
| `cargo fmt --all -- --check` (repository) | Exit 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` (repository) | Exit 0 |
| `cargo test --workspace` (repository) | Exit 0 |
| `python checks.py` (all seven above, plus lock integrity) | All exit 0; root `Cargo.lock` SHA-256 `7b15824fc67c90a837ad19af9d28e12dea2802797e90f541945ddb8e984497eb` before and after (unchanged) |
| `python harness.py hostile` | 1184 cases; 36 accepted, 1148 rejected; max 0.007 s; max job commit 1 064 960 bytes |
| `python harness.py performance` | Figures in section 7 |
| `git diff --check` | Exit 0 (LF endings; no CR in changed files) |
| `cargo check --target wasm32-unknown-unknown` | **Not run** (target not installed) |
| Rust 1.95 build, Linux, macOS | **Not run** |
| `cargo deny` on the spike | **Not run** (spike is outside the repository CI) |
| Official OpenUSD tooling | **Not run** (not available) |

## Exposure note

Exposure was limited to repository instructions, design and contract documents, the local
toolchain and Cargo cache, the `openusd 0.8.0` package manifest and README licence statement and
bundled notice file (for the licence finding), and the dependency metadata. No Tekla/Trimble or
other commercial detailing assets, personal data, client projects or other checkouts were
accessed. No upstream source or tests were copied. No network access was used; no service,
worker or plugin was started other than the spike's own child processes. All new material is
contributor-original and MPL-2.0.

## ROADMAP and ATTRIBUTION text

Applied in this change: one `ROADMAP.md` progress-log entry at the top of the log and one
`ATTRIBUTION.md` row, with LF endings. PR URL: none (the orchestrator publishes).
