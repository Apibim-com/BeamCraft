# F11: USD layer-engine decision record

## 1. Status and scope

**Draft 0.1.0, 2026-10-15. Recommendation, not a decision.** No option has been runtime-verified
by F11. This record asks which engine should author, preserve and compose BeamCraft USD layers.
It adds no implementation, dependency, architecture amendment or permission to acquire an SDK.

The accepted [architecture](../architecture.md) remains authoritative: official OpenUSD in an
isolated worker, initially Python/pxr. The [F03 audit](../audits/openusd-26.8.md) is conditional;
its package is not production-approved. The [F05 protocol](../contracts/worker-protocol-v0.1.md)
is a draft. The [F04 contract](../contracts/model-and-commands-v0.1.md) is a freeze candidate;
[F08 boundaries](../contracts/module-boundaries-v0.1.md) do not implement a worker or engine.

F11 depends on F03/F05/F08. U01-U07 still own runtime, schema, identity, client, resources,
publication and recovery acceptance. C-tasks still own collaboration semantics. This document
does not freeze those contracts, complete K0/K1, choose a storage mechanism or implement live sync.
It proposes a separate N01 evidence spike before the owner considers changing the baseline.

Original task-card base: `20d26cfb107d51e7a2c76c25ca0191b8115ec301`, then stacked on F08 PR #17.
For PR #22, the branch has since been merged with `docs/f06-geometry-contract` (PR #20).
The current PR base branch is `docs/f06-geometry-contract` (PR #20), which carries the F06
corrections, the merged F08 module-boundaries head and the G04 kernel design.
Retarget PR #22 to `master` after PR #20 lands. This stack update comes from the correction card;
the original read-only Git checks failed the repository ownership check, so no current base SHA
is independently verified here.
The private implementation-plan revision 1.2 was not opened. Task IDs below use only the card,
the contracts and [roadmap](../../ROADMAP.md), not inferred private task specifications.

### Public evidence and limits

Public pages were read on 2026-10-15. Documentation claims establish investigation candidates,
not BeamCraft suitability. No third-party source, test corpus or library was acquired.

- [OpenUSD introduction](https://openusd.org/release/intro.html): layers and composition are
  interchange mechanisms; USD does not supply BeamCraft's persistent UUID identity policy.
- [OpenUSD glossary][usd-terms]: public semantics of opinions, list edits, activation and arcs.
- [OpenUSD toolset][usd-tools]: validation options and their scope, rather than proof of an
  installed validator. These release pages identified themselves as 26.08 when read.
- [mxpv project README][mxpv]: development claims include USDA/USDC/USDZ read/write; sublayers,
  references, payloads, variants, inherits, specializes, LIVERPS, list edits, instancing and
  relocates; Stage and namespace editing; change observation, undo and replayable diffs;
  custom schema generation with `openusd-build`; AOUSD supplemental compliance testing;
  Linux/macOS/Windows CI; and possible API changes before 1.0. These are upstream claims.
  The README explicitly describes development, not necessarily release 0.8.0.
- That README advertises MIT **with exceptions** for vendored schemas and generated code under
  TOST. This is not an audit of the release or its dependency closure. An MIT headline cannot
  admit those resources. The official-SDK exception does not automatically cover this project.
- Supplied, not re-verified: crates.io `openusd` 0.8.0, release date 2026-10-11, Rust 1.96,
  edition 2024 and roughly 80,000 Rust lines. The [release metadata][crate-meta] and
  [docs.rs release page][crate-docs] were inaccessible through the public browsing tool.
  Line count is not evidence of coverage, safety or effort saved.
- [FloatyMonkey README][floaty] now describes USDA/USDC reading and initial composition, with
  USDA composition parsing still developing, USDZ planned and major API changes expected.
  It advertises MIT or Apache-2.0. This contradicts the supplied flattened-only characterization;
  neither view proves authoring support or fit for an authoritative engine. Keep it as a later
  reader candidate, outside the four N01 implementations. No release was pinned or audited.
- The [AOUSD standards landing page][aousd] and a candidate USD Core subpage were inaccessible;
  search found no usable official result. Specification 1.0.1 testing is supplied/upstream-claimed,
  not re-verified conformance. A README-linked PDF wrapper was visible; its contents were not
  read or used. N01 must pin the publisher's specification before mapping clauses to tests.
- Workspace `Cargo.toml` states Rust 1.95, edition 2024. `deny.toml` has permissive licences and
  project-specific MPL exceptions; no global TOST admission. These local facts were checked.
  Supplied C++ cxx/autocxx bindings offer SDK access with native builds and no WASM route;
  no particular binding crate, dependency graph or capability was investigated.

[usd-terms]: https://openusd.org/release/glossary.html
[usd-tools]: https://openusd.org/release/toolset.html#usdchecker
[mxpv]: https://github.com/mxpv/openusd
[floaty]: https://github.com/FloatyMonkey/openusd-rs
[crate-meta]: https://crates.io/crates/openusd/0.8.0
[crate-docs]: https://docs.rs/crate/openusd/0.8.0
[aousd]: https://aousd.org/standards/

## 2. Requirements for the layer engine

These are retained requirements, not proposed relaxations. F04 sections 1-8, F05 sections 2-11,
F08 sections 1-4 and the architecture provide the local provenance.

| Requirement | Acceptance obligation |
|-------------|-----------------------|
| USD authority | Reopen authored and derived state from USD; JSON is only control/journal data. |
| Full identity | Keep UUIDv7, RFC variant, all 32 path digits and full `bim:id`; reject aliases. |
| Independent copy | Allocate new identities; remap owned links; shape edits cannot change source. |
| Immutable resources | Share verified bytes by full SHA-256; copy-on-write creates new content. |
| Foreign read-only | Derive rights from composition provenance; reject every foreign override. |
| Personal layers | Preserve drafts and journals after restart; exclude them from published root. |
| Changesets | Retain base, op_id, read/write/relationship footprints and rejected local intent. |
| Semantic conflicts | Map `(UUID, attribute)` to F04 groups with lifecycle and dependencies. |
| Whole revision | Validate complete candidate; serialize expected-head check and atomic publish. |
| Unknown content | Keep unknown namespaces, attributes and links; reject unsupported mutation. |
| Bounded input | Cap bytes, arrays, traversal, recursion and allocations before trusting lengths. |
| Domain portability | Keep geom/model/protocol/kernel/engine pure and portable to `wasm32`. |
| Offline | Resolve only authorized contained local assets; no model-triggered network or plugins. |
| Headless | Engine API, CLI and E2E evidence remain mandatory for every eventual feature. |

F05's wire budgets are not stage-memory limits. N01 proposes explicit stage limits in section 7;
they cannot widen F04/F05 limits. The host remains responsible for validation, replay decisions,
outcome recording and publication under every option. A backend cannot declare acceptance.

## 3. Options

The following comparison uses the same criteria. Effort is an analyst's planning estimate in
person-weeks after prerequisites, with one experienced implementer, existing audited tools and
only the first supported slice. It excludes broad USD conformance, UI, full collaboration and
geometry development. Estimates have at least factor-two uncertainty and are not commitments.

| Criterion | A: SDK worker | B: own subset | C: Rust crate | D: hybrid |
|-----------|---------------|---------------|---------------|-----------|
| Authored engine | Official SDK | BeamCraft | `openusd` | BeamCraft |
| Broad composition | SDK candidate | Unsupported | Claimed | SDK boundary |
| New Cargo graph | None required | None assumed | Audited closure | None assumed |
| SDK obligations | Runtime | Test oracle | Test oracle | Runtime and oracle |
| WASM engine | No | Possible | Unproven | Owned slice possible |
| First-slice effort | 6-12 weeks | 16-40 weeks | 8-20 weeks | 20-52 weeks |
| Principal risk | Packaging | Semantic gaps | Upstream fit | Two authorities |
| Architecture review | Retain baseline | Required | Required | Required |

### A. Official SDK in an isolated worker: accepted baseline

- **Gives:** general USD composition and serialization through the established SDK interface.
  A delta layer is authored through it; engineering and merge rules remain BeamCraft's work.
- **Licence/graph:** official TOST exception only, with exact provenance, file inventory and
  notices; F03 native/runtime closure is still open. Python/native packages are outside Cargo,
  not outside dependency policy. No cxx/autocxx or unsafe FFI is needed for Python/pxr.
- **Platforms/WASM:** audit candidates exist for three desktop platforms, not tested support.
  Native SDK stays in the host worker; pure domain crates retain their WASM requirement.
  This does not deliver browser USD composition or a web application.
- **Effort:** 6-12 person-weeks for first-slice worker, packaging and adapter evidence.
- **Risk:** native distribution, parser exposure, plugins and runtime maintenance; isolation
  limits failure propagation but does not make malicious files safe.
- **Must prove:** F03 gates, bounded local resolution, F05 recovery, identity/unknown preservation
  and independent saved-file validation; SDK use alone proves none of those project invariants.
- **F03/F05/U01:** keep F03 integration gates and F05 draft behavior; U01 implements the accepted
  verified worker launch/handshake/read path. Oracle validation needs a separately admitted tool.

### B. Own Rust USD subset

- **Gives:** a deliberately restricted USDA authoring/composition profile; explicit field deltas
  and potentially small portable code. No promise of USDC/USDZ or arbitrary foreign composition.
- **Licence/graph:** original BeamCraft MPL-2.0 code; no assumed new dependencies. Any parser
  helper or hashing crate still needs admission. External SDK oracle use retains F03 conditions.
- **Platforms/WASM:** a pure in-memory portion could be portable. Filesystem, publication and
  process operations remain host-only. All targets need tests; Rust alone does not prove WASM.
- **Effort:** 16-40 person-weeks for a bounded first slice; general composition is outside that
  estimate and would be a separate potentially multi-year project.
- **Risk:** apparently simple values conceal USD type, namespace, list and preservation semantics.
  Unsupported foreign arcs must produce diagnostics, never a plausible incomplete scene.
- **Must prove:** a closed supported grammar/profile, lossless safe edits, differential composition
  and a usable foreign-reference policy. Dropping foreign composition requires an owner scope
  decision and would not meet the current architecture's later reference gate.
- **F03/F05/U01:** F03 becomes oracle-only only after explicit architecture approval. Review F05
  if execution moves in-process; keeping a Rust worker can retain isolation. U01 changes backend
  construction and capabilities, not the host's acceptance/recovery responsibilities.

### C. Adopt `openusd` behind a BeamCraft-owned trait

- **Gives:** a candidate implementation of authoring and composition with a narrower adapter
  maintenance burden than B. Advertised diff/undo mechanisms need domain-level validation.
- **Licence/graph:** claimed MIT core is promising, but pin release/checksum/features and audit
  normal, build, target and optional dependencies plus generated/schema resources. Do not enable
  `openusd-build` or schema features without a separate resource licence/provenance decision.
  TOST material outside official-SDK scope is blocked unless the owner changes policy explicitly.
- **Platforms/WASM:** desktop CI is upstream evidence only. Verify exact features on native
  targets and WASM; reject filesystem/mmap/process edges from domain crates. Initial placement
  would be a native adapter or Rust worker, not a new domain dependency under F08's empty list.
- **Effort:** 8-20 person-weeks for audit, adapter and first-slice gaps, excluding upstream fixes
  of unknown size. An unpatchable required gap rejects the option within the spike.
- **Risk:** release/development mismatch, API churn, maintainer concentration and parser limits.
  The supplied 1.96 MSRV exceeds workspace 1.95; a newer local compiler does not resolve policy.
- **Must prove:** exact-release behavior, permitted complete graph, 1.95 compatibility or an
  explicit MSRV decision, bounded parsing and upgrade containment. A proposed BeamCraft-owned
  trait would expose bounded prepare/read/install/export capabilities with BeamCraft IDs,
  values, errors and contexts; no upstream handles escape. It is not designed or added here.
- **F03/F05/U01:** retain an audited SDK oracle and independent reader. F05 needs explicit backend
  identity/capability review, even if a worker is retained; no silent Python-to-Rust substitution.
  U01 must prove adapter equivalence, process behavior and the revised runtime manifest.

### D. Own authoritative layer model plus SDK oracle and full-fidelity adapter

- **Gives:** direct deltas on owned authored opinions with SDK composition at foreign/import/export
  boundaries. It can test whether the owned slice benefits from a smaller engine.
- **Licence/graph:** BeamCraft model code is MPL-2.0; official SDK and resources keep F03's scoped
  exception and notice requirements. This can retain both maintenance costs; it saves no licence
  audit automatically. No new Cargo dependency is presumed.
- **Platforms/WASM:** owned in-memory logic could be portable; full-fidelity operations still
  require the native SDK. A browser would need an explicitly limited mode or later approved path.
- **Effort:** 20-52 person-weeks for the first slice and reconciliation boundary, excluding general
  import/conversion. Foreign-to-native conversion remains out of initial scope.
- **Risk:** divergent authorities and round-trip information loss. To remain eligible, the own
  model must be a USD authored-opinion model persisted as USD, with reconstructible indexes;
  a separate authoritative JSON/database model would contradict the accepted architecture.
- **Must prove:** one source of truth, exact unknown/source preservation, pinned composition
  contexts and no flattening-based replacement of authored layers. SDK import/export must retain
  unsupported content or refuse the operation; "full-fidelity" is an objective, not a result.
- **F03/F05/U01:** keep F03 for both runtime adapter and oracle. F05 needs an explicit dual-path
  ownership/candidate protocol review. U01 gains SDK availability and reconciliation failures;
  no fallback may guess values or publish without the required foreign dependency view.

## 4. Fit to diff/merge/live

A map `(prim path, field) -> typed authored opinion` is a useful abstraction for BeamCraft-owned
static fields. A workspace can record sparse opinions; a three-way comparison can identify
unchanged, one-sided and competing edits. Full UUID identity links paths to semantic attributes.
It simplifies sparse storage and review, but bytes saved in USD remain authoritative, and the
map must distinguish absence, explicit block, authored value and removal of a local opinion.

| Option | Delta/merge/live fit | Remaining BeamCraft work |
|--------|----------------------|--------------------------|
| A | SDK-authored delta layers | UUID index, footprints, replay and live envelopes |
| B | Direct owned-opinion deltas | Same semantics plus supported composition |
| C | Candidate upstream edit diffs | Translate and validate; never trust as footprints |
| D | Own deltas; SDK boundary view | Reconcile contexts and retain foreign provenance |

Merge is a three-way semantic operation against a pinned base, not "strongest user wins".
Two label edits to one UUID conflict; a label edit and a placement edit can be independent only
after read/write checks. Width and thickness edits can touch distinct attributes yet both read
the complete F04 shape-input group. Material and shape edits may invalidate the same mass result.
Delete versus any dependent read/write conflicts through lifecycle; relationship changes require
the pinned incoming-link index. Missing payload/index evidence is `DEPENDENCY_UNAVAILABLE`.
Copy needs fresh IDs and owned-link remapping, not a second path to mutable source state.

The public [USD terms][usd-terms] explain why a field map alone is insufficient: `over` authors
opinions without a complete defining prim; `active=false` changes subtree participation rather
than erasing bytes; list operations can edit target sets across layers. Reference namespaces,
variant selection, payload load state and arc strength affect the resulting scene. A sorted
lookup in a flat layer stack cannot stand in for those semantics. Source ownership also needs
composition provenance. Read-only foreign content still needs correct composition for display,
measurement and dependency checks; the prohibition on overrides does not simplify it away.

Later live deltas must carry accepted operation/revision identities, base context, ordering and
idempotency; preserve rejected local edits; detect gaps; and resnapshot under bounded budgets.
F05 incremental events are local transport, not a collaboration protocol or a CRDT. None of A-D
provides authorization, rebase, durable undo or live conflict resolution for free. Upstream diff
replay must not bypass the F04 command surface or F05 host ledger.

## 5. Interoperability and conformance

"Open by ordinary USD tools" means that an independently launched, pinned official SDK reader
can open `project.usda` offline, resolve every declared local dependency, compose the intended
prims and inspect standard geometry/transforms plus authored custom data. No BeamCraft parser,
private resolver or plugin is required for base geometry and custom attribute inspection.
Specialized engineering behavior is not promised by generic readers. Omniverse, Blender and
Houdini compatibility requires named versions and separate smoke evidence; no such tool was
opened and no compatibility claim is made here.

N01 uses an audited official SDK as an **external oracle process** over original synthetic
fixtures. Candidate and oracle run separately with contained local assets and pinned versions.
For A, validate in a fresh independent reader process, not through the authoring stage's cache.
No upstream test file, baseline or code is copied. Reuse the [F07 style](../fixtures/README.md):
original inputs, provenance/hash manifest, independent expected results and explicit support
statuses. F07 descriptions are not USD fixtures or F04 commands; adaptation is new N01 work.

Publication fault tests bracket six points: blob persistence, manifest persistence, candidate
validation, host decision recording, root switch and worker installation. The before/after pairs
give 12 failure points per option. Storage details still belong to U06/U07; N01 must identify the
actual persistence points before claiming coverage.

Differential checks compare authored opinions and composition separately: full prim IDs/types,
active state, explicit values and blocks, target order, relationships, units, transforms, mesh
arrays, selected variants, payload load behavior and contributing source ownership. Assert the
expected result first, then compare the oracle; agreement between two wrong runs is insufficient.
Numeric authored binary64 values must round-trip exactly. Analytic geometry comparisons use
the applicable existing contract's test rule, never a newly invented engineering tolerance.

Test candidate-write -> oracle-read, oracle-write of our fixture -> candidate-read, and
candidate-edit -> save -> fresh oracle-read. Test all supported selection/load combinations
in the small fixture matrix. Preserve unknown fields in authored form and untouched foreign
files byte-for-byte; composed-value equality alone cannot prove preservation of layering.

Run an admitted validator with documented rules, version, arguments, output and exit status.
[usdchecker documentation][usd-tools] describes evolving checks and incomplete time-sample
coverage; a clean result is one gate, not proof of semantic merge, safe parsing or full conformance.
F03 found no `usdchecker` executable in its wheels. Obtain approval for a separately audited
distribution or build before using it; do not claim wheel import passes U02 validation.

Record a coverage ledger with format, construct, specification clause, fixture ID, oracle version,
expected result and supported/preserved/rejected/deferred status. Known unverified areas include
USDC/USDZ (especially B), general arcs and list edits, time samples/value clips, instancing,
relocates, resolver behavior, unknown schema data and malicious inputs. N01's owned authoring
profile is static USDA; unsupported animation remains preserved or explicitly rejected. A narrow
profile pass must never be reported as AOUSD or complete USD conformance.

## 6. Risk register and gates

G01-G10 are gate groups in the evidence plan; each has one claim per option. A failed mandatory
gate rejects promotion for that scope. Missing prerequisites mean **not run/inconclusive**, not
pass. A proposed policy or scope change needs the owner before a rejected option can be retried.

| Gate | Risk | Required evidence before promotion |
|------|------|------------------------------------|
| G01 | Licence/provenance | Exact inventory and graph; zero unresolved terms or notices. |
| G02 | Bus factor/API churn | Named maintainer and backup; bounded upgrade/exit exercise. |
| G03 | MSRV/platform/WASM | 1.95 or owner-approved change; exact-feature target checks. |
| G04 | Composition gaps | Zero mismatches in declared scope; no silent foreign omissions. |
| G05 | Identity/preservation | Full IDs, independent copy, unknown opinions and source hashes. |
| G06 | False merge/live safety | Expected conflicts and dependencies; ordered idempotent replay. |
| G07 | Partial publication | Old/new complete revision at every injected failure point. |
| G08 | Parser/security | Capped malformed-input runs; no escape, panic or unbounded work. |
| G09 | Nondeterminism/cost | Stable own bytes and snapshots; measured bounded resource costs. |
| G10 | Spec drift/oracle gaps | Pinned clauses/tools; coverage ledger and review of all warnings. |

Licence gate G01 covers build tools, generated outputs and separately licensed bundled resources,
not just crate metadata. No TOST global allowlist change is proposed.

> **Note, 2026-10-15 (owner decision).** The owner decided to accept the TOST-1.0 material of
> OpenUSD origin inside the `openusd` crate (0.8.x, file `src/usd/core_schemas.rs`), under the
> conditions in `AGENTS.md` section 1.2: the upstream licence and notice are kept (`NOTICE`,
> `licenses/TOST-1.0-openusd.txt`), BeamCraft claims no rights in that material, trademark terms
> are respected, and the exact package and version are recorded in `ATTRIBUTION.md`. The licence
> blocker found by spike N01 is thereby resolved as an owner decision, not by a change of the
> crate's declared MIT licence. The `deny.toml` allowlist is unchanged, and this note does not
> adopt the crate or select an engine option. Maintenance gate G02 applies
to SDK/Python maintenance as well as small Rust projects and our own subset. Stars and README
claims do not establish bus factor. Reassess pins and open coverage whenever an engine, schema,
toolchain or specification changes; a previous corpus pass does not admit a new release.

## 7. Evidence plan: spike N01

**Proposed, not authorized or executed here:** timebox N01 to ten working days, at most 80
person-hours. This is feasibility evidence, not delivery of U01 or the engines. Day 1 pins the
profile, sources, tools and licences (8 h); days 2-3 prepare original fixtures and the oracle
(16 h); days 4-7 exercise thin candidates without production integration (32 h); days 8-9 cover
failures, portability and measurements (16 h); day 10 records results and the owner comparison
(8 h). No parallel staffing is assumed. Stop at the timebox; unfinished options are inconclusive.
If SDK admission or crate acquisition is unavailable, finish documentary gates and record the
runtime-dependent claims as not run. Any later downloads/builds require their own authorization.

The [machine-readable evidence plan](layer-engine-evidence-plan-v0.1.json) contains **40 claims:
10 for each of A, B, C and D**, grouped by G01-G10. All have status `not_run`. It defines
12 original scenario families, N01-S01 through N01-S12, not 12 implemented fixtures. Each family
may generate boundary variants under a versioned manifest; actual fixture counts must be reported
separately. Claims can depend on future tasks; that dependency never implies their completion.

| Family | Original scenario and independent expected result |
|--------|---------------------------------------------------|
| N01-S01 | Plate/static fields: exact explicit units, dimensions and standard mesh. |
| N01-S02 | Same-millisecond UUIDs, rename and copy/edit: unique full IDs, unchanged source. |
| N01-S03 | Three ordered sublayers and `over`: expected opinion plus missing definition case. |
| N01-S04 | Deactivate/reactivate parent: expected traversal and retained descendant opinions. |
| N01-S05 | Explicit/prepend/append/delete target edits; valid and dangling owned links. |
| N01-S06 | Owned wrapper with local foreign reference, two variants and loaded/unloaded payload. |
| N01-S07 | Unknown namespace, dictionary, relationship and unsupported animation preserved. |
| N01-S08 | Independent and conflicting edits, delete/read, link-index absence and live replay. |
| N01-S09 | Persistent personal workspace excluded from root; crash around publication points. |
| N01-S10 | Truncation, bad types/lengths, deep/cyclic arcs, path escape and plugin directives. |
| N01-S11 | 1, 1000 and 10000 owned static prims with fixed seeded insertion permutations. |
| N01-S12 | SDK interchange directions; known unsupported constructs and validator diagnostics. |

Freeze the generated manifest and hand-derived expected outcomes before running candidates.
Derive additional conformance cases from pinned public specification clauses, not copied suites.
For S06 B must either support the required composition or report unsupported; a diagnostic passes
the safe-failure test but does **not** pass the current foreign-reference capability requirement.
For D use SDK composition at that boundary and prove the own view cannot replace it with guessed
data. C's claimed composition is tested identically to A's; neither receives a presumed pass.

Proposed spike resource profile: each layer <= 16 MiB, resolved local bytes <= 64 MiB,
<= 10000 prims, <= 100000 properties, arrays <= 1000000 scalars, composition depth <= 32,
and <= 128 resolved layers. Preflight declarations and cumulative allocations before use;
cycle detection is required. These are experiment limits, not accepted production contracts.
Preserve F04/F05 command/IPC limits unchanged. Reject maximum-plus-one cases without publication.
Use an external 5-second per malformed-input limit and 512 MiB process working-set cap as a
backstop, not a substitute for parser checks. Record platform enforcement gaps as failures.

For each option capture pin/checksum/features, toolchain, OS/CPU, dependency inventory, fixture
hashes, commands, exit status, snapshots, validator rules and sanitized bounded diagnostics.
Use one declared machine per target; timing is not compared across unlike hosts. G09 measures
30 cold runs at each S11 size; the 10000-prim target is p95 open/compose/save <= 5 seconds and
peak working set <= 512 MiB on the recorded reference machine. Report startup separately.
Repeat output generation 20 times with fixed IDs; own serialized bytes/digests and canonical
semantic snapshots must match within an option. Cross-engine byte equality is not required.

Exit deliverables: a result for all 40 claim IDs (pass/fail/not run with artifacts), a coverage
ledger, dependency/provenance report, maintenance owner/backup, measured effort/cost and a
comparison for the owner. Promotion requires all ten gates for the selected scope and explicit
owner approval. One hard licence, safety, authority, preservation, conflict or atomicity failure
rejects promotion; unsupported required arcs reject B without a scope decision. API changes
that leak upstream types, unresolved 1.95 policy, missing target checks or timebox overruns also
prevent promotion. No arbitrary weighted score can cancel a mandatory failure.

## 8. Recommendation for the owner

**Recommendation, not a decision: retain A for the first deliverable and commission N01 before
any replacement decision.** Keep the official SDK worker as the accepted integration direction,
subject to F03 admission. Evaluate C as the first replacement candidate inside N01 because it
could avoid owning a new parser/compositor, while B and D remain bounded comparison prototypes.
Do not adopt C, change MSRV or introduce a second authority on the strength of its README.

This path preserves F04/F05/F08 responsibilities and keeps U01's evidence useful. It tests the
owner's map-of-fields hypothesis against both owned editing and foreign composition. A future
BeamCraft-owned adapter boundary can contain upstream changes, but cannot establish semantic
equivalence or exempt dependencies from audit. A baseline choice is not a claim that F03 passed.

Unknowns include exact 0.8.0 licence/features/graph, release versus development capabilities,
maintainer/backup capacity, MSRV resolution, WASM behavior, hostile-parser limits, unknown-data
round trips, differential results, performance and actual first-slice effort. B's required
foreign coverage and D's single-authority reconciliation are also unproven. All remain open.

Exact owner decisions required, after reviewing this record:

1. Approve or decline the proposed 80-hour N01 scope, mandatory gates and experiment budgets.
2. Assign an evidence owner and backup, and authorize exact tool/package acquisition separately;
   decide the F03 notice/native/runtime admission route before an SDK oracle is used.
3. Keep Rust 1.95, or separately approve an MSRV change after pinned-release evidence. Until
   then, the supplied 1.96 candidate is blocked for workspace adoption.
4. Confirm whether full read-only foreign composition remains mandatory. If it does, B cannot
   be selected with a field-stack-only evaluator. Any scope reduction needs revised acceptance.
5. After N01, explicitly select A/B/C/D or continue evidence; approve any resulting F05/F08/U01
   and architecture amendments before implementation. Decide any out-of-scope TOST admission
   separately; this record requests no policy exception or contract freeze.

## 9. Effects on the plan

These are proposed impacts only. This decision record changes no architecture, dependency or
runtime behavior and edits no task specification or shared contract. The only shared-file edits
in PR #22 are the `ROADMAP.md` progress-log entry and the `ATTRIBUTION.md` row recorded by the
orchestrator.
The inputs do not include full private U/C/G/P task definitions; sequencing beyond the named
contract obligations must be confirmed by their owners. N01 completion would not complete F11's
parent architecture/implementation acceptance gates.

| Area | A | B | C | D |
|------|---|---|---|---|
| F03 | Close runtime gates | Oracle audit | Oracle plus crate audit | Runtime plus oracle audit |
| F05 | Retain draft | Review execution | Review backend | Review dual-path context |
| F08 | Retain boundaries | Review adapter | Review graph/trait | Review model/adapter |
| U01 | SDK worker | Subset runtime | Rust adapter/worker | Own engine plus SDK |
| U02 | SDK schema authoring | Subset schema mapping | Verify schema features | Reconcile schemas |
| U04 | F05 client | Review local calls | Prove same CLI results | Expose boundary failures |
| U06 | Host CAS/publish | Same; own writer | Same; crate writer | Same; cross-view check |
| C01-C05 | Domain merge | Plus subset gaps | Plus diff mapping | Plus reconciliation |
| G/P | Stable domain rules | New round-trip checks | New adapter checks | New boundary checks |

U03 identity and U05 resource hashes stay mandatory for all options. U07 must retain draft
recovery, durable outcomes and complete revision recovery; changing engines cannot remove it.
U02's ordinary-reader/validator gate survives every choice, including A.

C01-C05 retain publication idempotency, semantic rebase/conflict checks, retries and preservation
of local intent as applicable to their owners' detailed cards; upstream diffs are not a
substitute. This record does not invent a per-ID allocation absent those cards. Later live work
needs a separate reviewed transport/coordination scope; no server is enabled by F11 or N01.

G-tasks keep the independent Rust kernel, derived geometry and signature contracts. P01/P03
creation, P04 invalidation, P05 independent copy, P07 undo, P08 edits/deletion and personal
workspaces need backend-independent API -> CLI -> USD -> independent-reader evidence. P02's profile
resource format remains an existing open decision. Geometry/frame/tolerance gaps and F04 freeze
conditions are unaffected. Options B-D add validation work and may delay U/G/P integration;
they do not make adjacent tasks complete.

Provenance: contributor-original analysis and evidence definitions, MPL-2.0, derived from the
listed repository inputs, task card and linked public documentation. No third-party code, test
fixtures, commercial assets or runtime resources are included. The orchestrator has recorded
this PR's progress-log entry in `ROADMAP.md` and attribution row in `ATTRIBUTION.md`.
