# F11: USD layer-engine decision record

## 1. Status and scope

**WS amendment, owner decision of 2026-10-16:** desktop-only on Windows, Linux and macOS.
Only geom, model and kernel retain WASM compatibility as a layering discipline under amended
F08. The USD layer engine, wrapper, protocol, host, IO, CLI, UI and later crates are native-only
with no WASM requirement. U01-U04 and N03 have desktop build/verification criteria.

**Decision 0.1.1, 2026-10-16: variant C selected by the owner.** The pure-Rust
`openusd` crate (0.8.x) is the primary USD layer engine behind a thin BeamCraft-owned wrapper.
The product-path aim is no Python and no C++. Variant A, the official SDK worker, remains only
an explicitly selected fallback and an external test oracle where available. Selection does not
admit a production dependency or establish runtime readiness.

The [architecture](../architecture.md) records this direction. The [F03 SDK
audit](../audits/openusd-26.8.md) stays conditional for fallback/oracle use; its package is not
production-approved. F04 and F06 were frozen at 0.1.0 on 2026-10-16. The [F05
protocol](../contracts/worker-protocol-v0.1.md) and [F08
boundaries](../contracts/module-boundaries-v0.1.md) need reviewed alignment before integration;
this documentation task changes neither contract. In-process module versus separate Rust process
remains an open execution question (section 9), not a consequence of choosing Rust.

F11 depends on F03/F05/F08. U01-U07 retain runtime, schema, identity, client, resources,
publication and recovery acceptance. C-tasks retain collaboration semantics. This record does
not complete N01, K0/K1 or any U-task, choose storage or implement live synchronization.
[Wrapper requirements](layer-engine-wrapper-requirements-v0.1.md) and section 9 define the next
implementation gates; N03 is planned verification, not completed work.

F11D documentation base: `381930cabb00c76f428a5ad49bb4336e1be7eacd`, verified against local
`origin/master`; branch `docs/f11-variant-c-decision`, targeting `master`. The original F11
comparison was prepared on 2026-10-15 for PR #22, initially stacked on F08/F06. Its recommendation
to retain A is superseded by the owner decision below. The private implementation plan was not
opened; the task scope here comes from the owner card and repository contracts/roadmap.

### Public evidence and limits

The following public-source notes are historical F11 research from 2026-10-15, not new
verification by F11D. The later [N01 report](layer-engine-spike-n01-report-v0.1.md), version
0.2.0, supplies the measured first-slice evidence used by this decision. No upstream source,
test corpus or runtime resource is added by F11D.

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
  admit those resources. AGENTS.md section 1.2 now separately covers the recorded OpenUSD-derived
  file under the owner decision of 2026-10-15; it is not blanket crate/graph admission.
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
- At original research time, workspace `Cargo.toml` stated Rust 1.95, edition 2024.
  The owner raised the workspace MSRV to 1.96 on 2026-10-15 (recorded in the roadmap). `deny.toml` has permissive licences and
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
| Core layering | Keep only geom/model/kernel WASM-compatible; the layer engine and other crates are desktop-native. |
| Offline | Resolve only authorized contained local assets; no model-triggered network or plugins. |
| Headless | Engine API, CLI and E2E evidence remain mandatory for every eventual feature. |

F05's wire budgets are not stage-memory limits. N01 proposes explicit stage limits in section 7;
they cannot widen F04/F05 limits. The host remains responsible for validation, replay decisions,
outcome recording and publication under every option. A backend cannot declare acceptance.

## 3. Options (original comparison, updated selection labels)

Section 8 records the current decision; the effort estimates below are unvalidated priors.
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
| First-slice effort | 6-12 weeks | 16-40 weeks | 8-20 weeks | 20-52 weeks |
| Principal risk | Packaging | Semantic gaps | Upstream fit | Two authorities |
| Architecture review | Retain baseline | Required | Required | Required |

### A. Official SDK in an isolated worker: fallback and oracle

- **Gives:** general USD composition and serialization through the established SDK interface.
  A delta layer is authored through it; engineering and merge rules remain BeamCraft's work.
- **Licence/graph:** official TOST exception only, with exact provenance, file inventory and
  notices; F03 native/runtime closure is still open. Python/native packages are outside Cargo,
  not outside dependency policy. No cxx/autocxx or unsafe FFI is needed for Python/pxr.
- **Platforms:** audit candidates exist for three desktop platforms, not tested support.
  Native SDK stays in the host worker.
- **Effort:** 6-12 person-weeks for first-slice worker, packaging and adapter evidence.
- **Risk:** native distribution, parser exposure, plugins and runtime maintenance; isolation
  limits failure propagation but does not make malicious files safe.
- **Must prove:** F03 gates, bounded local resolution, F05 recovery, identity/unknown preservation
  and independent saved-file validation; SDK use alone proves none of those project invariants.
- **F03/F05/U01:** retain F03 admission and the F05 process contract for any SDK fallback.
  U01 now implements C as primary (section 9). Oracle validation needs an admitted tool;
  fallback is explicit, never a silent change of backend or composition context.

### B. Own Rust USD subset

- **Gives:** a deliberately restricted USDA authoring/composition profile; explicit field deltas
  and potentially small native code. No promise of USDC/USDZ or arbitrary foreign composition.
- **Licence/graph:** original BeamCraft MPL-2.0 code; no assumed new dependencies. Any parser
  helper or hashing crate still needs admission. External SDK oracle use retains F03 conditions.
- **Platforms:** the subset is desktop-native. Filesystem, publication and process operations
  remain host responsibilities. Each desktop target needs tests.
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

### C. `openusd` behind a BeamCraft-owned wrapper: selected 2026-10-16

- **Gives:** a candidate implementation of authoring and composition with a narrower adapter
  maintenance burden than B. Advertised diff/undo mechanisms need domain-level validation.
- **Licence/graph:** claimed MIT core is promising, but pin release/checksum/features and audit
  normal, build, target and optional dependencies plus generated/schema resources. Do not enable
  `openusd-build` or schema features without a separate resource licence/provenance decision.
  The recorded `src/usd/core_schemas.rs` exception is already in AGENTS.md section 1.2;
  all other files/versions still require their own admission.
- **Platforms:** desktop CI is upstream evidence only. Verify exact features on native
  targets; reject filesystem/mmap/process edges from the three pure crates. Initial placement
  would be a native adapter or Rust worker, not a new domain dependency under F08's empty list.
- **Effort:** 8-20 person-weeks for audit, adapter and first-slice gaps, excluding upstream fixes
  of unknown size. An unpatchable required gap rejects the option within the spike.
- **Risk:** release/development mismatch, API churn, maintainer concentration and parser limits.
  The original 1.96-versus-1.95 MSRV mismatch was resolved by the owner raising the workspace
  minimum to 1.96; exact-feature builds and dependency review still remain.
- **Must prove:** exact-revision behavior, permitted complete graph, declared-MSRV compatibility,
  bounded parsing and upgrade containment. A proposed BeamCraft-owned
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
- **Platforms:** owned logic is desktop-native; full-fidelity operations still require the
  native SDK. Verify each supported desktop target.
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

The N01 differential plan requires an audited official SDK as an **external oracle process**
over original synthetic fixtures; this comparison was not run in the first slice. Candidate and oracle run separately with contained local assets and pinned versions.
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
| G03 | MSRV/desktop platforms | Owner-approved MSRV 1.96; exact-feature target checks. |
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
> select an engine option at that date. Section 8 records the later 2026-10-16 selection.

Maintenance gate G02 applies to SDK/Python maintenance as well as small Rust projects and our
own subset. Stars and README claims do not establish bus factor. Reassess pins and open coverage whenever an engine, schema,
toolchain or specification changes; a previous corpus pass does not admit a new release.

## 7. Evidence plan: spike N01

The following is the original proposed plan, retained for its gate IDs, budgets and scenarios.
N01 option C has since run a first slice; the report and section 8 record its limits.
The original plan is not a claim that all four options ran or that N01 is complete.

**Original proposal:** timebox N01 to ten working days, at most 80
person-hours. This is feasibility evidence, not delivery of U01 or the engines. Day 1 pins the
profile, sources, tools and licences (8 h); days 2-3 prepare original fixtures and the oracle
(16 h); days 4-7 exercise thin candidates without production integration (32 h); days 8-9 cover
failures, portability and measurements (16 h); day 10 records results and the owner comparison
(8 h). No parallel staffing is assumed. Stop at the timebox; unfinished options are inconclusive.
If SDK admission or crate acquisition is unavailable, finish documentary gates and record the
runtime-dependent claims as not run. Any later downloads/builds require their own authorization.

The [machine-readable evidence plan](layer-engine-evidence-plan-v0.1.json) contains **40 claims:
10 for each of A, B, C and D**, grouped by G01-G10. The JSON now carries N01 first-slice
evidence: C-G01 remains `failed` as measured (policy subsequently resolved); all other claim
statuses remain `not_run`, including partially evidenced claims. It defines
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
that leak upstream types, missing declared-MSRV/target checks or timebox overruns also
prevent promotion. No arbitrary weighted score can cancel a mandatory failure.

## 8. Owner decision, evidence and maintenance policy

### Decision and reasons (2026-10-16)

The owner chooses **variant C**, the pure-Rust `openusd` 0.8.x crate behind a thin BeamCraft
wrapper, as the primary USD layer engine. The aim is to remove Python and C++ from the product
path while avoiding a new BeamCraft parser/compositor. The wrapper contains upstream API changes
and retains BeamCraft ownership, validation and transaction rules. Variant A remains only a
fallback and an external test oracle where available; its F03 gates still apply. No automatic
fallback, acquisition or production dependency is authorized by this record.

The reason is directional plus measured feasibility, not a claim of full USD conformance.
[N01 report 0.2.0](layer-engine-spike-n01-report-v0.1.md) tested `openusd =0.8.0` with default
features disabled on Windows x64, Rust 1.96.0:

| N01 evidence | Decision relevance and limit |
|--------------|------------------------------|
| Sections 2-3, C-G04/G05 | USDA/USDC round trips preserved tested authored fields, IDs, relationships and unknown data; sublayers, stronger `over`, `active` and a foreign reference composed. No independent SDK comparison or general arc coverage. |
| Section 3, finding 5 | A raw local override of a foreign prim succeeds. The spike wrapper refused it with `FOREIGN_READ_ONLY`; production protection is BeamCraft's duty. |
| Section 4, C-G06 | Captured field-set deltas replayed; `(UUID, attribute)` detected same-field collisions. Semantic footprints, idempotency, lifecycle and stale/gap handling remain unproved. |
| Section 5, C-G08 | 1184 synthetic hostile inputs caused no observed panic, crash or timeout. Non-finite values and some corrupt/truncated inputs were accepted; allocation bounds were not proved. |
| Sections 2/7, C-G09 | Second writes were byte-identical within one build; three warm 1000/10000-element runs were feasible. Reordered repeats, cold-run statistics and cross-platform determinism were not run. |

The N01 report is a historical measurement: its then-current "not adopted/no option selected"
and MSRV-blocker statements are superseded by the owner decisions, not rewritten as passing
results. The 2026-10-15 licence exception and MSRV raise resolve policy questions only.
File-level notices, dependency closure and runtime acceptance remain open. The spike is isolated
from the product workspace; this task adds no dependency or executable behaviour.

### Explicit remaining risks

- Foreign-prim write refusal must cover every mutation route and contributing composition source,
  including variants/payloads/inherits/specializes absent from the N01 tests.
- Non-finite values can parse successfully. Some truncations and USDC header corruptions are
  accepted. Engineering validation and verified manifests/digests must establish validity/integrity.
- Parser allocations, cumulative resolution, cycles and maximum-plus-one limits remain unproved;
  the cheap 64 KiB corpus and Windows process cap establish no production safety bound.
- Linux, macOS and WASM builds were not run. SDK-oracle comparison was not run; same-crate
  round trips establish self-consistency only. N03 addresses Linux and available oracles;
  macOS runtime evidence remains open.
- Pre-1.0 API churn, maintenance owner/backup, broader unknown-content preservation, copy/remap,
  deterministic serialization, semantic replay and publication/recovery still need evidence.

### Fork and upgrade policy

Yes: MIT permits a fork of the MIT-covered crate code, including if upstream maintenance stops
or a required feature is missing. Preserve the MIT copyright and licence. The OpenUSD-derived
`src/usd/core_schemas.rs` keeps its own TOST-1.0 licence, upstream notices and required
modification notices. Retain `licenses/TOST-1.0-openusd.txt` and `NOTICE`; do not relabel that
material MIT or MPL-2.0, claim ownership, or imply endorsement through upstream trademarks.

AGENTS.md section 1.2 covers only the exact recorded package/version. A fork or another version
requires its own ATTRIBUTION entry and an owner decision before use; the current exception does
not transfer automatically. Keep both upstream and any fork behind the same BeamCraft wrapper.
Pin adoption by **exact git revision**, record the corresponding package/version, source,
checksum and feature/dependency inventory, and upstream fixes where possible. N01's crates.io
0.8.0 measurement is evidence, not the future git pin; no exact adoption revision is invented here.
Every pin change repeats affected licence, safety, serialization and oracle checks. A maintainer
and backup must be assigned before integration; permission to fork does not supply staffing.

## 9. Re-scoped implementation and verification tasks

These cards replace the SDK-primary allocation for U01-U04. All are **planned, not complete**.
They consume the [wrapper requirements](layer-engine-wrapper-requirements-v0.1.md) (LEW IDs),
F04/F06's unchanged authority and numeric rules, F08 boundaries, and the N01 evidence above.
They do not authorize edits to frozen contracts or new dependency edges. The wrapper exposes
BeamCraft-owned IDs, typed values, contexts and bounded errors; upstream handles never escape.
Its precise package/module placement requires F08 review, with no new domain dependency assumed.
U01-U04 are desktop-native; their criteria impose no WASM build. N03 retains Linux builds and
the available SDK oracle; the three pure crates have a separate CI portability check.

### Execution question and F05 applicability

**Open question:** should C run as an in-process module or a separate Rust process? F11's earlier
comparison left this open, and the owner selected the engine, not its execution topology.
U01 must present the placement, parser containment and cancellation evidence for review before
integration. Choosing a pure-Rust engine alone does not justify removing failure isolation.

For a separate Rust process, F05 private pipes, framing, attachments, handshake/capabilities,
correlation, liveness, deadlines, cancellation, restart and diagnostics remain the transport
baseline, with reviewed backend/runtime identity alignment. For an in-process module, pipes,
wire framing, process handshakes/restarts and OS child caps are not function-call requirements;
review an explicit semantic mapping rather than silently treating them as satisfied or deleted.
In either case retain bounded inputs/results, pinned contexts and invalidation, one stage owner,
isolated prepare/install/discard, host authorization/validation, canonical op_id replay,
serialized acceptance, durable outcome queries and uncertainty handling. F05 remains unchanged;
its SDK wording continues to describe fallback pending the proposed review in the PR body.

### U01: bounded open/read through the Rust wrapper

**Change from SDK plan:** construct the pinned Rust backend and contained resolver instead of
making Python/pxr startup a primary-path prerequisite. Keep any SDK launcher confined to the
explicit fallback/oracle route and its F03 admission. Do not expose raw stage setters or paths.

**Inputs:** verified project/workspace manifest and local asset bytes/digests; project/workspace
IDs, base revision and generation; owned-layer set; declared composition/load profile; exact
backend pin/features/schema identity; explicit stage budgets and cancellation/deadline context.
**Outputs:** opaque pinned stage context, supported-capability/profile record, bounded typed
snapshot pages with composition provenance, or safe F04-mapped diagnostics with no partial view.

**F05:** apply the execution mapping above to OpenStage/ReadSnapshot, capabilities, limits and
context invalidation. Actual separate-process transport still needs real-pipe failure tests.
In-process execution needs equivalent limits/context tests and a proved parser containment plan.

**Acceptance:** LEW-01/02/03/04/07; open original USDA/USDC fixtures without Python/C++; verify
full identities, metadata and reference provenance; reject digest/path escapes, numeric faults,
cycles and cap-plus-one before exposing state. Unsupported arcs produce explicit diagnostics.
Reopen and page over one pinned context; reject stale cursors. Test cancellation/backend failure
without state change or inferred rollback. Provide engine API -> CLI open/read E2E via U04 and
N03 platform/oracle evidence; mocks and same-crate reads alone cannot close acceptance.

### U02: codeless schemas and authored/derived layers

**Change from SDK plan:** define BeamCraft codeless schema/property mappings and write layers
through C, without requiring Python schema generation or a C++ plugin in the product path.
Do not enable `openusd-build`, import generated resources or choose their licences implicitly.
Keep USD as authority and preserve authored layering rather than replacing it with flattening.

**Inputs:** reviewed schema mapping/digest and closed change/snapshot field definitions; validated
F04 commands, owned candidate context and engine-derived changes; F06 geometry/results; unknown
inert content and resource digests. **Outputs:** isolated candidate, typed authored opinions,
owned authored/derived layer bytes and digests, schema identity and bounded validation results.

**F05:** U02 still owns closed change/snapshot item schemas and schema mismatch checks. Apply
prepare/install/discard and schema identity in either topology; wire bodies/attachment budgets
apply on separate-process and SDK routes, while the in-process mapping needs reviewed bounds.

**Acceptance:** LEW-01/02/05/06/07; author/reopen a supported plate/member and relationships,
units `metersPerUnit=0.001`, `upAxis=Z`, unknown fields and explicit absence/block/removal.
Foreign overrides fail atomically. Run 20 reordered deterministic saves and independent-reader
checks, including N03's available SDK directions and an admitted validator with recorded rules.
Ordinary readers inspect standard geometry/custom data without a BeamCraft plugin. Validate
schema/pin mismatch and invalid-number rejection. No U06 atomic publication claim follows from save.

### U03: full UUID and relationship index

**Change from SDK plan:** build the reconstructible index from wrapper-owned typed snapshots and
composition provenance instead of pxr objects. Index authority remains the pinned USD state;
upstream diff paths or a persisted index cannot become element identity or engineering authority.

**Inputs:** complete pinned U01/U02 snapshot pages, full UUIDv7 `bim:id`/32-digit paths, typed
relationship opinions/list edits, lifecycle state and source ownership/dependency coverage.
**Outputs:** UUID-to-element/provenance lookup, outgoing/incoming relationship index and explicit
completeness/context token, plus bounded duplicate/dangling/foreign or missing-dependency errors.

**F05:** context/generation and invalidation rules apply in either topology. Separate-process
pages/cursors/events retain F05 delivery limits; in-process snapshots must preserve equivalent
bounded delivery and reject incomplete or mixed-context indexing.

**Acceptance:** LEW-01/06/07; reject short, malformed, duplicate and mismatched IDs; labels do
not change IDs. Rebuild an identical index after save/reopen. Test copy with fresh IDs and
remapped owned links, unchanged source, deactivate/reactivate, delete/dependent-read collisions,
and missing payload/incoming-link evidence as `DEPENDENCY_UNAVAILABLE`. Stale or partial indexes
cannot authorize writes or certify conflict freedom. Integrate real API/CLI E2E with U04;
copy and broader collaboration behaviour remain owned by their P/C tasks.

### U04: shared CLI and native host API

**Change from SDK plan:** route CLI/host operations through the same BeamCraft wrapper API with
C as primary. An ordinary operation has no Python/C++ runtime prerequisite. Backend fallback
must be explicit, report its verified identity and capabilities, and retain F03/F05 admission.

**Inputs:** bounded CLI/API requests, authorized project/workspace/resource handles, pinned
revision/generation, op_id, deadline/cancel context and explicit backend selection.
**Outputs:** the same typed logical result/context, bounded snapshots/events, verified saved-layer
or resource digests and safe diagnostics; proven host-recorded outcomes or `OUTCOME_UNKNOWN`.

**F05:** native process client/supervision remains for separate-process C and SDK fallback.
In-process calls keep the reviewed bounded semantic adapter. Both preserve host deadlines,
canonical replay, snapshot invalidation, host-ledger QueryOutcome and prepare/install ordering;
never fabricate cancellation/acceptance from a backend acknowledgement.

**Acceptance:** LEW-01 through LEW-07 at the host boundary; API -> CLI -> save -> fresh-reader
E2E parity for U01-U03, including foreign refusal, corrupt/non-finite input, stale contexts,
replay/collision and cancellation/loss. No service or network access starts by default.
Use existing U05 byte storage without claiming U06/U07 publication/recovery complete. Expose
those capabilities only with their implementation; test safe unavailable/uncertain outcomes now.
Run primary-path tests with Python and SDK absent. Process-specific claims require real processes.

### N03: Linux wrapper builds and available SDK oracle

**Status: planned, not run by F11D.** This is a separate verification task/PR, not a completed N01
gate. Inputs are the reviewed U01/U02 wrapper slice, exact git pin/features/lock inventory,
LEW requirement tests, original fixtures with prewritten expectations/hash manifest, and an
admitted pinned `pxr` environment where available. Filesystem/process adapters remain host-only.

**Outputs:** reproducible commands, compiler/target/feature inventory, build results, per-case
semantic snapshots/digests and a differential coverage ledger with pass/fail/not_run, diagnostics
and explicit unsupported constructs. No SDK or Python becomes a product dependency.

**Acceptance:**

1. Build/check the actual wrapper and its selected dependency closure on Linux; run the
   wrapper suite on Linux. Name the wrapper package and exact commands after placement is
   reviewed. A pure-crate check or stub does not pass this gate.
2. Compare deterministic exports and semantic snapshots from fixed/reordered fixtures on Windows
   and Linux. Preserve full IDs, unknown opinions, relationships and foreign source bytes.
3. Where admitted `pxr` is available, run candidate-write -> fresh oracle-read, oracle-write of
   our fixtures -> candidate-read, and candidate-edit/save -> fresh oracle-read. Compare authored
   and composed state separately: fields/types, values/blocks, target ordering, units, transforms,
   geometry and source provenance for the declared supported arcs. Require zero unexplained
   mismatches against independent expectations; preserve/reject unsupported constructs explicitly.
4. If a target, runner or admitted oracle is unavailable, record **not_run** and the reason;
   missing oracle evidence leaves G04/G10 and independent-reader acceptance open. Do not turn
   a missing prerequisite into a pass. N03 does not establish macOS support, general USD
   conformance, hostile-parser safety or completion of U01-U04/K1 by itself.

### Unchanged downstream ownership and provenance

U05 resource hashes, U06 expected-head/atomic complete publication and U07 preserved drafts,
durable outcomes and recovery remain mandatory. C-tasks own semantic rebase/live coordination;
upstream capture/replay is not their acceptance. G/P tasks retain the independent Rust kernel,
F06 operation order, tolerance policy and API -> CLI -> independent-reader gates.

No glam/nalgebra is selected: small fixed-size f64 types stay original in `geom` under F08.
Plates stay as drawn; members roll only about their local Z; no general orientation or quaternion
is introduced into the kernel. This decision changes no numeric rule: finite f64, the 0.001 mm
absolute minimum, local-size `eps_geom`, absolute test/pick rules and F06 operation order remain
binding. Integration must resolve the recorded contract alignment before implementing affected
surfaces; frozen contracts and their cases are untouched here.

Provenance: contributor-original F11D documentation, MPL-2.0, based on the owner's 2026-10-16
card, the repository contracts/designs and N01's original synthetic observations. Public-source
links above retain the original research provenance; no new upstream code, fixture or runtime
resource is included. Proposed attribution rows, frozen-contract amendments and the dated roadmap
progress entry belong only in the F11D PR body. This task leaves ATTRIBUTION and the progress log
unchanged.
