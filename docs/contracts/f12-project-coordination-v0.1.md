# F12: section projects and pinned coordination, version 0.1.0

Status: **draft contract; foundation approved by the owner on 2026-10-22; not implemented**.

The approved foundation has three parts: separate modules from responsible sections; allow
sections to produce independent projects under one standard; coordinate exact published
revisions. This contract and the [F10 0.2 amendment](f10-domain-vocabulary-v0.2.md) form one
documentation work package, developed in one branch and one PR. The subsequently approved
stage 4 adds [opening assignments and file exchange](f12-opening-assignments-v0.1.md) to the
same package, with exact-version review and separate implementation evidence. Approved
[stage 6](f12-craft-read-capabilities-v0.1.md) defines common reading across Craft applications
and specialized module editing in this same package.
The [external-reference companion](f12-external-reference-workflow-v0.1.md) adds single-section
companies receiving IFC models and exchanging reports with participants outside Craft.

## 1. Invariants and scope

USD remains the authoritative model. F04 identities, SHA-256 byte verification, immutable
complete revisions and atomic publication remain the storage foundation. This contract does
not introduce a second saved model, a cloud service, a database, a Git backend or a remote
permission system. "Repository for engineers" describes the workflow, not textual USD merges.

Two organizations are supported by the design:

1. **Shared project:** several sections have separate authored scopes in one project store.
2. **Independent projects:** each section or zone has its own project store and publication
   history; a coordination project connects their selected revisions read-only.

A company can also run just its own section project with received IFC converted into read-only
USD references. Other participants need no Craft projects or accounts, and a shared coordination
project is optional. The external-reference companion defines source evidence and reporting;
native project pins in this document apply only when the source is actually a Craft project.

The same identity, compatibility and responsibility rules apply to both. Physical file layout
does not authorize writes. Independent projects are not workspaces of the coordination
project; each can have its own engineers' workspaces. Splitting or combining existing stores
is an explicit future migration operation, never a consequence of opening a project.

Rebase, semantic merge, general project review/approval workflows, remote sync and a second
engineering module remain outside this foundation. Their existing tasks are not moved into
the MVP. The stage 4 companion specifies opening-assignment exchange; stage 6 specifies
common read/display and specialized editing capabilities. Concrete file encoding, read APIs
and command protocols remain implementation gates.

## 2. Identity and project descriptor

The descriptor is authored project metadata in the coordination/root USD layer. The existing
revision manifest remains the immutable inventory and integrity envelope for its layers and
assets. The following names describe logical fields; they are not an additional JSON file
format or permission to author arbitrary USD attributes.

| Field | Required meaning |
|-------|------------------|
| ProjectId | Stable full UUIDv7 under F04; remains the same across revisions and transport copies |
| Code, name, organization | Human-readable project metadata; no field grants access |
| Contract requirements | Exact project-contract, vocabulary and model-schema identifiers needed to interpret this revision; retain the existing schema digest |
| Sections | Stable SectionId, code, label and permitted module vocabulary for each deliverable |
| Zones | Stable ZoneId, code and label; empty for a project without zones |
| Authored scope map | Which section/zone or explicit shared scope owns each authored resource |
| Frame | Units, up axis, handedness and the declared project coordinate frame |
| Dependency pins | Exact source revisions used by this revision, including coordination connections |

**PC-ID-001.** SectionId, ZoneId and ConnectionId use the full UUIDv7 representation and
uniqueness checks of F04. They are distinct typed identities. Codes such as `KM`, `KMD`, `AR`,
`OV` and `K1` are labels; changing a label does not replace the identity. Codes are unique
within their respective project registries; section and zone references must resolve there.

**PC-ID-002.** An element reference across projects names `(ProjectId, ElementId)`; a historical
reference also pins the source RevisionId and manifest digest. Bare ElementId lookup across
all connected projects is not sufficient. A coordination occurrence additionally names its
connection path so that repeated placement of one source is not confused with copying its
elements. Source identities and source-local prim paths remain unchanged.

**PC-ID-003.** Inside one project, the F04 path and identity rules remain intact. Grouping by
section or zone must not reparent elements. A dependency connection is a read-only wrapper,
not a new owner of the source elements. The eventual USD mapping must isolate connected roots
under wrappers rather than overlaying unrelated `/World` roots at the same paths.

Independent section projects normally have distinct ProjectIds. A transport copy or an
engineer's workspace does not manufacture a new source-project identity. A fork intended to
become a new independent project requires explicit identity/remapping semantics outside this
version; silently changing ProjectId is not a fork implementation.

## 3. Authored scope and common data

**PC-OWN-001.** Each editable authored element has one validated section/zone scope, or one
explicit shared scope for common data. The root's scope map and contributing layer provenance
must agree. Ambiguous, missing or conflicting ownership refuses mutation. Both file-level
provenance and the operation's actual effects are checked: a file named `KMD` cannot grant
write access to a KM element through a stronger opinion.

**PC-OWN-002.** Ordinary publication changes only the workspace's authorized section/zone
scope under F10 0.2.0. Cross-section transfer, shared metadata changes and coordination
membership updates need explicit shared commands. Common grids/levels have their own shared
scope; section users may reference them without gaining edit rights. Derived output follows
the owning authored inputs and cannot supply authority to rewrite another section.

Authored, derived and external remain origin roles, independent of section and zone.
Material/profile definitions stay immutable by content. Foreign projects and reference bytes
remain read-only. KM and KMD can both use `steel` while remaining separate writable sections.
The storage implementation must validate these distinctions without weakening F04 ownership.

## 4. A complete independent section revision

**PC-REV-001.** A published revision pins its descriptor, authored and derived resources,
external resources and dependency revisions by their verified manifests and exact bytes.
Composition connections are bounded and acyclic by ProjectId on a composition traversal path;
repeated siblings are permitted as distinct connections. Historical evidence pins, such as
opening-assignment responses referencing a proposal based on an earlier host revision, are
not composition connections. Their bounded dependency graph must be acyclic by exact
(ProjectId, RevisionId) identities with verified manifest digests; it may revisit a project
at an older revision. Neither graph is traversed by mutable heads. Missing or damaged required resources
prevent publication of a complete coordination revision. Lazy geometry loading can defer
decoding, but cannot substitute for verifying that its required bytes are available.

**PC-REV-002.** A revision advertises only the capabilities its reader/module combination can
actually supply: interpretation, display, editing and engineering validation are separate.
An absent engineering module does not authorize deletion or rewriting of its content. An
unsupported semantic kind can be retained read-only when the underlying encoding is safely
admitted. An unsupported composition arc, corrupt resource or exceeded traversal limit is
not excused as an "unknown module".

**PC-VER-001.** Contract and schema requirements are checked before editing or evaluating
their semantics. Unsupported content is read-only where bounded interpretation is possible,
otherwise unavailable with a diagnostic. No automatic schema upgrade, guessed units or
silent substitution of a supported version is permitted. The F10 0.2 line requires explicit
upgrade; this draft itself changes neither installed readers nor existing schema pins.

Local byte supply is enough to exercise this contract. Future remote resolution supplies the
same verified bytes through a separately authorized transport; a URI, branch name or mutable
"latest" alias cannot stand in for an immutable revision pin.

## 5. Coordination connections and coordinate frames

Each connection in the coordination project's authored root records:

| Logical field | Meaning |
|---------------|---------|
| ConnectionId | Stable identity of this occurrence in the coordination project |
| Source ProjectId | Identity of the independent source project |
| Source RevisionId and manifest digest | Exact published source snapshot; both must agree |
| Selected section/zone scope | Explicit source scope, or an explicit whole-project selection |
| Source frame and placement | Declared source coordinate frame and full rigid transform into the parent frame |
| Label | Human-readable description, never a resolver or identity |

**PC-COORD-001.** Opening a coordination revision resolves only its pinned source revisions.
Publishing a newer source revision neither changes these pins nor rewrites this coordination
revision. A source being temporarily unreachable cannot cause fallback to another revision.

**PC-COORD-002.** Updating a pin is an explicit operation on the coordination project's
workspace: name the connection, expected old pin and candidate new pin; resolve and verify
the candidate; report changes and compatibility/dependency findings; then publish against
the expected coordination head. Publication is atomic and idempotent under F04. Failure keeps
the previous complete coordination revision and the user's draft. Concurrent changes to the
head are refused under current fast-forward rules; this contract supplies no automatic merge.

**PC-COORD-003.** A scope selection controls participation/display, not permission or permission
to discard dependencies. All dependencies required to interpret and validate the selection must
remain pinned and available. Queries identify omitted scopes. A partial selection cannot prove
that there are no incoming relationships or that the whole project has no clashes.

**PC-FRAME-001.** Conforming native section projects use F04 millimetres, radians, right-handed
Z-up coordinates. Each connection specifies a finite rigid transform (translation and rotation)
into its parent's declared frame. It does not modify the source placement. Nested placements
compose in a documented order; unknown alignment is an explicit unavailable coordination
state, not identity placement. Non-rigid scale/shear or unsupported georeferencing requires a
separate admitted conversion; no implicit best fit, unit guessing or geometric certification.

**PC-STATE-001.** Report resource completeness, compatibility, frame availability and engineering
validation separately. A loaded viewport alone proves none of the other three. A partially
available historical coordination revision can be inspected with diagnostics, but cannot be
reported as completely validated. Unknown or skipped domain checks are not passed checks.

## 6. Example: independent publication and explicit coordination update

These labels abbreviate full identities for explanation only; they are not serialized IDs.

```text
Coordination C12
  connection A -> AR project, revision A17 + its manifest digest
  connection S -> KM project, revision S09 + its manifest digest
  connection M -> OV project, revision M23 + its manifest digest

AR publishes A18: C12 still resolves A17, S09 and M23.
Coordinator explicitly proposes connection A: A17 -> A18.
After complete validation and publication against C12:
Coordination C13 resolves A18, S09 and M23.
C12 remains reproducible with its original dependency closure.
```

Publishing in a section and selecting its version in coordination are different operations.
Neither operation implies formal engineering approval or release for construction. The
stage 4 companion defines version-pinned review of opening assignments only; general project
approval and release workflows still need their own specification.

## 7. History and storage reachability

**PC-KEEP-001.** Every retained coordination revision is a retention root for its pinned
dependency closure. A workspace base and retained historical provenance link, including a
future `builtFrom` record, likewise pin the revisions/resources needed to interpret them.
Compaction must not remove these resources merely because they are no longer the latest head.

The coordination store must retain verified local copies of the closure or an explicit,
enforced storage retention guarantee before treating it as durable. This foundation assumes
local retained copies; an external project's untracked promise is insufficient. The source
store cannot discover unknown consumers and is not required to retain everything forever.
Removing a connection in a new revision does not release pins of older retained revisions.
Retention policy may explicitly retire history, but must expose the loss of reproducibility
and preserve all still-live pins. Cross-store retention and transport protocols remain future
work; this clause does not claim current U14 compaction already implements them.

## 8. Acceptance descriptions and refusals

These are required future cases, not a report of executed tests. Use F04's existing refusal
codes and attach the contract rule ID; no new top-level error vocabulary is introduced here.

| Case / rule | Required positive outcome | Required negative outcome |
|-------------|---------------------------|---------------------------|
| PC-ID-001 | Rename K1 without changing ZoneId or element paths | Duplicate IDs/codes or unresolved registry entries refused (`DUPLICATE_ID` / `INVALID_ARGUMENT`) |
| PC-ID-002/003 | Two source projects with the same local ElementId remain distinct; two occurrences of one source remain distinct | Bare ambiguous lookup or colliding composed roots refused (`INVALID_ARGUMENT`) |
| PC-OWN-001/002 | KMD edit preserves KM, AR and shared grids | Foreign override refused (`FOREIGN_READ_ONLY`); unauthorized shared operation refused (`PERMISSION_DENIED`) |
| PC-REV-001 | Complete pinned resources reopen without a remote server | Missing resource (`DEPENDENCY_UNAVAILABLE`), wrong digest (`INTEGRITY_ERROR`), cyclic graph (`DEPENDENCY_CONFLICT`), exhausted budget (`LIMIT_EXCEEDED`) |
| PC-REV-002 / PC-VER-001 | Admitted unknown semantics round-trip read-only | Unsupported editing/version refused (`UNSUPPORTED_VERSION`); unsafe encoding refused by the reader |
| PC-COORD-001 | A18 publication leaves C12's A17 pin and bytes unchanged | Resolving `latest` instead of A17 refused (`INVALID_ARGUMENT`) |
| PC-COORD-002 | Explicit verified update creates C13; exact retry replays | Stale coordination head/pin refused (`BASE_REVISION_MISMATCH` / `DEPENDENCY_CONFLICT`); interruption leaves C12 or complete C13 |
| PC-COORD-003 | Selected section retains and reports its supporting dependencies | Partial index cannot approve dependency-sensitive change (`DEPENDENCY_UNAVAILABLE`) |
| PC-FRAME-001 | Known nested rigid transforms give independently expected positions without modifying sources | Missing frame (`DEPENDENCY_UNAVAILABLE`) or unsupported transform (`UNSUPPORTED_GEOMETRY`) prevents validated coordination |
| PC-STATE-001 | Missing data and unavailable module checks are visible separately | An incomplete scene reported as completely validated fails acceptance |
| PC-KEEP-001 | C12 reopens after C13 and a compaction attempt | Compaction deleting a live pinned closure is refused (`DEPENDENCY_CONFLICT`) |

## 9. Implementation gates and relation to the plan

Before declaring these capabilities implemented, the corresponding tasks must define exact USD
attributes/relationships, scope-map and wrapper encoding, schema pins, typed commands, resolver
rules and numeric budgets (connection count, depth, resources, bytes and text). Existing F04
limits remain ceilings where applicable; a new unbounded field is not an acceptable default.
Every rule above needs a positive and a negative executable case, including crash/retry checks
at publication and independent expected coordinates. No existing test is silently relabelled
as evidence for this new contract.

- U19: section/zone ownership and workspace scope under F10 0.2.
- U20: project descriptor, stable registries and explicit shared metadata commands.
- U21: independent section/zone projects, pinned connection resolution, atomic updates and
  retained dependency closure; coordinate with U14 before adding live cross-store pins.
- U18: explicit compatibility-line upgrade; never infer legacy authority.
- R06: future source-provenance references use the scoped identity and pin rules here.
- C03–C06: remain deferred; operation replay and conflict semantics are unchanged in scope.

The first implementation proof can use two small synthetic projects and the existing steel
module for both KM and KMD. This tests distinct responsibility without pretending that AR/OV
engineering modules, a remote coordinator or a general approval system exist.

## 10. Provenance

Based on the owner's decisions of 2026-10-20 and approval of the three foundation proposals on
2026-10-22, F04/F10 and the existing project architecture. The owner-supplied Speckle design
study informed separation of generic records, representations and engineering interpretation.
The study's source observations were not independently re-audited for this contract; no claim
is made that Speckle implements these coordination, authority or approval semantics. No
third-party code, assets or documentation text were copied.
