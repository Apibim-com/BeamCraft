# N01 option C: first-slice acquisition report, version 0.1.0

Task: evaluate pure-Rust `openusd` behind a BeamCraft-owned trait, executing the owner's
2026-10-15 option C decision. Worktree: `D:\Projects\01_BeamCraft-N01`.
Status: **uncommitted; blocked at dependency acquisition; runtime spike not implemented**.
No capability or mandatory gate passed. This is a blocked first slice, not completion of N01,
U01, a parent acceptance gate or an architecture change.

## Scope and method

Read `AGENTS.md`, `CLAUDE.md`, `ROADMAP.md`, `docs/architecture.md`, the F11 decision and
evidence plan, F04 model/command contract and F08 module-boundary contract. The supplied
owner decision authorizes investigating C; the architecture and contracts remain unchanged.
No other checkout or Desktop directory was accessed. No subagents were used.

Created an isolated acquisition bootstrap with its own empty `[workspace]`, `publish=false`,
MPL-2.0, Rust 1.96 and one exact dependency request: `openusd = "=0.8.0"` with
`default-features=false`. This pins the version named in N01-C-G01 and avoids deliberately
enabling optional features before their audit. It is not a resolved dependency or a tested
feature configuration. `src/lib.rs` contains documentation and `forbid(unsafe_code)` only;
it is not an adapter, mock backend, test suite or USD implementation.

The local registry lookup found neither an `openusd-0.8*` source directory nor a cached
archive. One `cargo fetch` was attempted. Cargo reported offline mode and failed to resolve
the package. The worker stopped dependency and runtime work under task rules 7 and 11.
No setting, environment variable, proxy, Cargo configuration or sandbox restriction was
changed. No alternate download, escalation, installation or manual vendoring was attempted.
This is an offline-resolution failure, not evidence of a failed online HTTP request.

Changed paths:

- `spikes/n01-openusd/Cargo.toml`
- `spikes/n01-openusd/src/lib.rs`
- `spikes/n01-openusd/acquisition.txt`
- `docs/design/layer-engine-spike-n01-report-v0.1.md`
- `docs/design/layer-engine-evidence-plan-v0.1.json`: evidence references on G01 and G03
  only; both statuses remain `not_run` because the gates could not execute.

No `Cargo.lock` was fabricated for the spike: resolution failed before Cargo could produce
one. An independently resolved spike lockfile is still required before implementation.
No root manifest, root lockfile, deny configuration, CI, roadmap, attribution or contract
was edited. Git metadata was not written.

## Environment and preliminary commands

Date: 2026-10-15; environment timezone: America/Santiago. Windows, PowerShell, Python
3.11 executable found on PATH. No OS build number, CPU model or host target triple was
measured; performance work did not start.

| Command / observation | Exact result |
|---|---|
| `Get-Location` (first command) | Exit 0; `D:\Projects\01_BeamCraft-N01` |
| `rustc --version` | `rustc 1.96.0 (ac68faa20 2026-05-25)` |
| `cargo --version` | `cargo 1.96.0 (30a34c682 2026-05-25)` |
| `rustup target list --installed` | `aarch64-pc-windows-msvc`, `x86_64-pc-windows-msvc` |
| `git status --short --branch` | Failed: `fatal: detected dubious ownership in repository at 'D:/Projects/01_BeamCraft-N01'`; no workaround |
| `Get-FileHash Cargo.lock -Algorithm SHA256` | Command unavailable: `The term 'Get-FileHash' is not recognized as the name of a cmdlet, function, script file, or operable program.` |
| Python `hashlib.sha256` of root `Cargo.lock` | Baseline `7b15824fc67c90a837ad19af9d28e12dea2802797e90f541945ddb8e984497eb` |
| `Get-Command usdcat,usdchecker,python -ErrorAction SilentlyContinue` | Only `python.exe` found; no `usdcat` or `usdchecker` on PATH |
| Registry source/cache lookup | Both `openusd-0.8*` glob lists empty |

The environment/ownership probe was one PowerShell command sequence and returned exit 1;
the tool version and installed-target lines above were nevertheless emitted. The branch
`feat/n01-openusd-spike` and stack on PR #22 are supplied by the task, **not independently
verified**, because read-only Git failed. No `safe.directory` change or direct metadata
workaround was attempted.

## 1. Dependency, licence inventory and MSRV

Claim: **N01-C-G01**; verdict **unknown**, status `not_run` (acquisition prerequisite failed).

Command, from `spikes/n01-openusd`: `cargo fetch`. Exit **1**, exact output:

```text
error: no matching package named `openusd` found
location searched: crates.io index
required by package `beamcraft-n01-openusd-spike v0.0.1 (D:\Projects\01_BeamCraft-N01\spikes\n01-openusd)`
note: offline mode (via `--offline`) can sometimes cause surprising resolution failures
help: if this error is too confusing you may wish to retry without `--offline`
```

The recorded output is also in `spikes/n01-openusd/acquisition.txt`.
`cargo tree`, `cargo metadata`, a build and a complete licence/file/notice inventory:
**not run** after the stop condition. Full transitive list, resolved versions, checksums,
SPDX expressions and native/generated/schema closure: **unknown, not available**.
There is no resolved third-party package inventory to list or approve.

| Component | Version | Licence | Evidence / disposition |
|---|---|---|---|
| BeamCraft acquisition bootstrap | 0.0.1 | MPL-2.0 | Contributor-original manifest and empty library; isolated, not shipped |
| `openusd` request | =0.8.0 | MIT supplied by task; not audited | Not acquired or resolved; bundled licence exceptions remain unverified |
| Transitive packages | Unknown | Unknown | Not resolved; no assertion of allowed closure |

No dependency outside the task's licence list was identified, but **zero findings does not
establish licence closure** when the graph and included files are unavailable. In particular,
F11 warns of schema/generated exceptions: no MIT blanket approval or TOST admission is made.
Optional `mmap` and `serde` were not explicitly enabled; their exact release definitions,
transitive activation and portability effects remain unverified. No source or test files from
any USD implementation were copied.

The task supplies crate Rust 1.96 versus repository Rust 1.95; the root manifest independently
confirms 1.95. Local Rust 1.96 exists, but **whether this spike builds locally is not shown**.
A 1.96 minimum for workspace adoption would require a separate owner decision, toolchain/CI
review and dependency/boundary audit. An isolated manifest requesting 1.96 changes no repository
MSRV and does not prove 1.95 incompatibility from a compiler run.

## 2. USDA / USDC round trip and unknown-field preservation

Claims: **N01-C-G05**, **N01-C-G09**; verdict **unknown**, status `not_run`.
Author, write, fresh read and second-write comparison: **not run**. No resolved public API
was available to implement them honestly. USDC writing capability: **unknown**. No fixture,
UUID, explicit units, unknown custom attribute, dictionary, relationship or metadata was
exercised. No identity/copy/preservation or byte-determinism result is claimed.

## 3. Composition and foreign ownership

Claims: **N01-C-G04**, **N01-C-G05**; verdict **unknown**, status `not_run`.
Sublayer ordering, stronger `over`, deactivation, foreign references, source-byte checks,
variants and payload states: **not run**. Whether the crate structurally prevents a foreign
override is unknown; a read-only source file alone would not demonstrate that invariant.

Required wrapper rule, derived from F04 sections 1, 2, 7 and 8: resolve mutation destinations
from validated full UUID identities and composition provenance; reject unknown ownership,
foreign source edits and opinions overriding foreign prims, including beneath an owned
attachment wrapper. Permit wrapper placement and separate owned measurement results only.
Never expose raw prim-path/field mutation or writable foreign-layer handles. Validate the
candidate composition before acceptance. This is a required design rule, **not tested code**.

## 4. Delta and merge model

Claim: **N01-C-G06**; verdict **unknown**, status `not_run`.
Change observation, diff capture, replay on a copy and same-`(UUID, attribute)` conflict
detection: **not run**. No upstream diff API was inspected or exercised. What the exact
release supplies remains unknown.

BeamCraft still owns command authorization, identity/provenance validation, pinned base and
generation, typed absence/block/removal semantics, read/write/relationship footprints and
lifecycle dependencies. F04 section 8 requires reading the complete shape-input group;
different attribute names alone cannot prove independence. An upstream delta cannot be
accepted or replayed as an authorized command. Idempotency, gap/stale-base handling,
dependency-unavailable behavior and preservation of rejected local intent require later
evidence. A key-by-field merge was not implemented or demonstrated here.

## 5. Untrusted input

Claim: **N01-C-G08**; verdict **unknown**, status `not_run`.
Truncated, malformed, deeply nested, huge-count and invalid-UTF-8 USDA, and corrupted USDC:
**not run**. No parser process started. There are no observed parser panics, hangs, memory
measurements or error-quality results; this is not a zero-defect or safe-parsing result.
No minimal parser reproducer exists because no parser was acquired.

For continuation, retain the evidence plan's 5-second / 512 MiB external backstops plus
pre-allocation byte/count/depth/cumulative-resolution limits and cycle checks. If the host
cannot enforce a required cap, record a gate failure rather than treating process termination
as safe parsing. Neither those limits nor an external enforcement harness were implemented.

## 6. Portability

Claim: **N01-C-G03**; verdict **unknown**, status `not_run` (target preflight only).
`rustup target list --installed` returned only the two Windows targets listed above.
`cargo check --target wasm32-unknown-unknown --lib --offline`: **not run**, WASM target
not installed, and dependency acquisition failed. No target was installed. Linux, macOS,
Windows compilation and all domain-WASM checks: **not run**. Native target installation
does not establish a passing build. Feature-specific `mmap` / `serde` effects are unknown.
No repository dependency edge was added.

## 7. Performance sanity

Claim: **N01-C-G09**; verdict **unknown**, status `not_run`.
Open/compose/traverse at 1,000 and 10,000 element prims: **not run**. No timings, working-set
numbers, release/debug comparison or benchmark claim is available. The bootstrap is empty
and must not be timed as a substitute. The plan's 30 cold runs, p95 target and 20 output
repeats were not executed.

## 8. Interoperability and structural validation

Claim: **N01-C-G10**; verdict **unknown**, status `not_run`.
**Oracle not available: not run.** PATH discovery found no official `usdcat` or `usdchecker`.
No exhaustive installation search or Python `pxr` import probe was performed; an undiscovered
installation is not ruled out. No official SDK was invoked or installed.
Same-crate USDA reread and an original structural check: **not run**, no USD was written.
A future reread would establish only self-consistency; a structural check can check its
declared syntax/shape subset, not independent USD grammar or composition conformance.
No specification clause, validator hash or differential result is claimed.

## Claim table

All ten option C claim IDs are retained. None passed or failed a runtime assertion.
Only G01/G03 have evidence-plan references for the attempted acquisition and platform
preflight; the remaining claims and all A/B/D claims are untouched. Counts and IDs stay
40 claims, 10 per option, 10 gate groups and 12 scenario families.

| Claim | Status | Verdict | Recorded result / section |
|---|---|---|---|
| N01-C-G01 | not_run | unknown | Acquisition attempted, blocked; section 1 |
| N01-C-G02 | not_run | unknown | No API perturbation, stub replacement or maintainer/backup exercise |
| N01-C-G03 | not_run | unknown | Environment/target discovery only; section 6 |
| N01-C-G04 | not_run | unknown | Composition unavailable; section 3 |
| N01-C-G05 | not_run | unknown | Round trip/identity/copy unavailable; sections 2 and 3 |
| N01-C-G06 | not_run | unknown | Diff/merge unavailable; section 4 |
| N01-C-G07 | not_run | unknown | Atomic publication outside this slice; no storage runtime |
| N01-C-G08 | not_run | unknown | Parser unavailable; section 5 |
| N01-C-G09 | not_run | unknown | Determinism and performance unavailable; sections 2 and 7 |
| N01-C-G10 | not_run | unknown | No independent oracle, writer or validator; section 8 |

## Findings and proposed adapter surface

1. **Acquisition blocker:** one permitted fetch failed in Cargo offline mode with no cached
   `openusd`. Minimal reproduction: from the isolated package, run `cargo fetch` under the
   same environment. Do not infer package absence from crates.io or a library defect.
2. **Verification blocker:** read-only Git rejected repository ownership. Branch, stack and
   pre-existing working-tree changes cannot be independently established with Git.
3. **Portability gap:** no installed `wasm32-unknown-unknown` target. No WASM result.
4. **Bootstrap limitation:** no spike lockfile, adapter, executable, fixture, test or runtime
   evidence exists. The empty library is explicitly labelled and provides no API evidence.

Proposed BeamCraft-owned trait, in logical terms only: bounded open/read with contained
asset resolution and explicit revision context; enumerate composed identities and source
provenance; inspect typed authored opinions and relationships; prepare supported owned edits
in a candidate; capture normalized changes for engine-derived footprints; validate/install
or discard that candidate; export authored layers while retaining unknown opinions.
Expose BeamCraft IDs, values, bounded diagnostics and context tokens, never upstream handles.
Host storage, authorization, atomic publication and durable outcome ledger remain outside
the backend. No trait was implemented and no exact upstream API mapping is asserted.

## Python/C++ avoidance verdict

| First-slice need | Can avoid Python/C++ on this evidence? | Reason |
|---|---|---|
| Open/read | not shown | Crate not acquired |
| Author | not shown | No API exercised |
| Compose | not shown | No stage opened |
| Write | not shown | No USDA or USDC output |
| Foreign read-only references | not shown | No provenance or override test |
| Delta/merge | not shown | No diff capture/replay or merge test |

No negative capability verdict about the crate follows from an environment failure.
No suitability, complete conformance or replacement of the accepted worker is established.

## Checks and exact outcomes

The following required commands are **not run** after the acquisition stop condition:

```text
# spikes/n01-openusd
cargo tree --offline
cargo metadata --format-version 1 --offline
cargo fmt --all -- --check
cargo clippy --all-targets --offline -- -D warnings
cargo test --offline
cargo check --target wasm32-unknown-unknown --lib --offline
# repository root
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --offline -- -D warnings
cargo test --workspace --offline
```

No formatting, Clippy, tests, build, dependency closure or boundary test pass is asserted.
Only report/plan consistency and root-lock integrity were checked after recording the stop.
The task's explicit stop takes precedence over continuing the otherwise requested build/test
sequence. Post-recording checks actually run:

| Command / check | Exact result |
|---|---|
| `python -c "import json; json.load(open('docs/design/layer-engine-evidence-plan-v0.1.json', encoding='utf-8')); print('json ok')"` | Exit 0; `json ok` |
| Python comparison of parsed plan before/after insertion | Exit 0; `Evidence update verified: 40 IDs/counts unchanged; only G01/G03 evidence added; all statuses unchanged.` |
| Python `hashlib.sha256(Path('Cargo.lock').read_bytes()).hexdigest()` | Exit 0; `7b15824fc67c90a837ad19af9d28e12dea2802797e90f541945ddb8e984497eb`, identical to baseline |
| Python spike-file/lockfile check | Exit 0; lockfile absent; only `acquisition.txt`, `Cargo.toml`, `src/lib.rs` under the spike |

Root `Cargo.lock` is byte-for-byte unchanged according to matching SHA-256 digests. This does
not replace the unrun workspace checks or independently establish a clean Git working tree.

## Risks, effort and next steps

The unresolved risks are release licence/resources, MSRV, exact public API, upstream stability,
composition/unknown-field correctness, parser allocation bounds, platform coverage and lack
of an independent oracle. Two maintenance owners and an upgrade/exit exercise remain open.

Order-of-magnitude planning only: completing acquisition/audit and this small runtime slice
could take days if prerequisites become available; missing capabilities or upstream fixes
cannot be bounded from this run. F11's existing 8–20 person-week first-slice integration
estimate is an unvalidated prior estimate, not a measurement or commitment from N01.

Continuation requires an environment with authorized registry access or an already populated
Cargo cache. Resolve the exact pin, generate a real independent lockfile, inventory every
package and included resource/notice before selecting APIs, and then implement the original
bounded fixture/harness/trait work. Revisit the SDK oracle only if an admitted tool is already
available; do not install Python/C++ tooling as an implicit fallback.

Owner decisions remain: any repository MSRV change, licence findings/closure after actual
inventory, maintenance owner/backup and upstream stability policy, and eventual backend
selection after evidence. This session changes none of those decisions or shared contracts.

## Proposed shared-file text (not applied)

Exact proposed `ROADMAP.md` progress-log entry:

```markdown
- 2026-10-15: N01 option C first slice attempted an isolated `openusd` 0.8.0 acquisition bootstrap in `spikes/n01-openusd`, stacked on PR #22 as supplied by the task (Git ownership prevented independent verification). Cargo offline resolution failed because the crate was not cached; no retry or bypass was attempted. Runtime, licence-closure and portability gates remain not run; the report records the blocker. No backend, MSRV or architecture change is accepted, and no N01 parent acceptance gate is completed.
```

Exact proposed `ATTRIBUTION.md` row (third-party crates are unresolved dependencies, not
shipped assets; no upstream code/resources are included):

```markdown
| `spikes/n01-openusd/**`, `docs/design/layer-engine-spike-n01-report-v0.1.md`, N01 evidence references in `docs/design/layer-engine-evidence-plan-v0.1.json` | BeamCraft contributors | Contributor-original N01 option C acquisition bootstrap and blocked first-slice report 0.1.0, 2026-10-15; owner task card and repository F04/F08/F11 documents; no third-party code, fixtures or runtime resources copied | `MPL-2.0` | Isolated unshipped spike bootstrap, acquisition diagnostic and evidence limitations; no USD runtime implemented |
```

## Exposure note

Exposure was limited to repository instructions/design/contracts, local toolchain and cache
availability, and Cargo's acquisition diagnostic. No Tekla/Trimble/commercial detailing assets,
installations, personal data, client projects, private observation notes or other checkouts
were accessed. No third-party implementation source or tests were read or copied. No outbound
connection was observed; Cargo reported offline resolution before any successful acquisition.
No service, control channel, worker or plugin was started. All new material is
contributor-original MPL-2.0; the attribution row is proposed for the orchestrator because
this task forbids editing root attribution. Work remains uncommitted.
