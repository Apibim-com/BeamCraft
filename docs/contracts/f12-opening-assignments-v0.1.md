# F12 stage 4: opening assignments and file exchange, version 0.1.0

Status: **draft contract; workflow approved by the owner on 2026-10-22; not implemented**.
Companion to [F12](f12-project-coordination-v0.1.md) and the
[F10 0.2 amendment](f10-domain-vocabulary-v0.2.md), in the same branch and PR.

## 1. Approved workflow

1. The services engineer connects a pinned structural project revision as a read-only reference.
2. The engineer models pipes/ducts in the services project without cutting the structural model.
3. The engineer collects locations requiring openings and exports a file containing a batch of
   assignments. Manual collection is sufficient; automatic clash detection is not assumed.
4. The structural engineer imports and reviews the assignments individually, accepts or rejects
   them with reasons, or requests corrections. A batch can have different outcomes per item.
5. The structural engineer authors accepted openings in the structural model through authorized
   commands and returns decisions and, where available, implementation records in a response file.

An intersection is a reason to propose an opening, not permission to create it. Delivery,
engineering acceptance, implementation in the host model and selection of a new structural
revision in coordination are separate events. Receipt of a file performs none of the latter
three implicitly. This scope does not introduce general project PRs or automatic semantic merge.

## 2. One identity, immutable proposal versions

**OA-ID-001.** One assignment has one persistent identity throughout proposals, responses and
implementation. Its identity is scoped by the proposing ProjectId and uses the ElementId of
the reserved `penetration` request; there is no independent assignment copied for each party.
Changing requested dimensions or placement creates a new proposal version under the same
identity. Creating an unrelated assignment requires a new identity.

Each proposal version pins these logical inputs:

| Input | Meaning |
|-------|---------|
| Assignment identity | Proposing ProjectId and request ElementId |
| Proposal version | Proposing revision and manifest digest, plus exact request-record content digest |
| Responsibility | Proposing SectionId and responsible host ProjectId/SectionId |
| Geometry | Requested shape, dimensions, clearance, placement and declared coordinate frame |
| Subject | Scoped identity and pinned source revision of the pipe/duct or other passing element |
| Host basis | Scoped host element identity and exact structural revision/manifest used to prepare the request |
| Explanation | Purpose and any correction/rejection explanation, bounded and explicitly authored |

F04 units and value limits apply. F10's supported request shapes remain the reserved baseline;
this document does not add a pipe, duct or concrete geometry implementation. References use
F12's scoped identities and connection placement where necessary. A filename is never an
assignment identity, proposal version, source resolver or permission grant.

**OA-VER-001.** A proposal version's content is immutable. A new version identifies its
predecessor; a receiver never replaces a newer known proposal with an older arrival. Competing
successors of one proposal are a conflict to resolve explicitly, not a timestamp ordering rule.
The owning project's accepted history establishes succession; proposal labels do not.

## 3. Batches are transport, not shared live storage

**OA-PKG-001.** One request file can carry many assignments. It has a versioned envelope,
stable package identity, content digest, sender/recipient project and section identities,
an explicit item inventory, exact item versions and the pinned evidence needed to review them.
A response identifies the request package and the exact item versions it answers. Omitted
items remain unanswered; a batch-level "received" or "complete" flag cannot approve them.

The file is an exchange package of revisioned records and referenced resources. Admitted
assignment/decision content belongs in versioned USD project records under the existing model
standard; transport metadata is not a second authoritative engineering model. Import validates
into an isolated draft and records a receipt atomically before advancing that local history.
No receiving process writes the sender's project. No distributed atomic transaction is assumed.

**OA-PKG-002.** Importing identical package bytes again returns the recorded receipt and
creates no duplicate assignments, decisions or openings. Reusing the same package/record
identity with different bytes is an integrity refusal. The same assignment version delivered
in another package is likewise deduplicated by its identity/version, not its file path.
An older but valid response can be retained as historical evidence without changing the
current proposal or its approval status. Corrupt or incomplete required evidence refuses
admission of the package; partial engineering acceptance happens later, per admitted item.

**OA-AUTH-001.** A digest verifies bytes, not the sender's authority. Project, section and actor
strings in a file cannot authorize decisions. Imported decisions require validated provenance
and an authorized local admission action under the trusted context; unverifiable attribution
is retained only as untrusted evidence, not effective acceptance. Only the responsible host
section may accept or reject an assignment. The proposer cannot accept its own request merely
because both sections use the same engineering module. Source request fields remain proposer-owned.

Exact container encoding, extension, USD mapping, authentication/provenance mechanism and
typed import/export commands remain implementation gates. Before implementation they must
specify bounded sizes/counts/depth, unsupported-version handling, containment checks, a ban
on executable payloads and implicit network fetches, and crash/retry behaviour. No concrete
format or interoperability with another product is claimed by this contract.

## 4. Decisions bind exact proposals and reviewed host revisions

**OA-DEC-001.** A decision is an immutable, identified record from the responsible host
section, pinned to the assignment identity, exact proposal version/digest and exact host
revision/manifest reviewed. It records the reviewer provenance, outcome and reason, and any
previous decision it explicitly supersedes. Rejection/correction reasons retain F10's reason
categories; a correction request leaves that proposal unapproved and does not edit its inputs.

**OA-DEC-002.** Acceptance applies only to the reviewed proposal and host basis. Changes to
requested size, position, shape, subject or target require a new proposal and review. If the
selected host revision differs from the reviewed revision, applicability is "needs review"
until explicit revalidation records the new basis. No automatic carry-forward is inferred
from matching names, a displayed mesh, or an engineer's earlier acceptance.

**OA-DEC-003.** An offline host may decide an older proposal without knowing that the proposer
has already changed it. On receipt, keep the decision attached to that older version and show
the newer proposal as awaiting its own decision. Do not copy the old acceptance to the new
proposal or overwrite the new proposal. Preserve both records and identify the exact mismatch.

For example: proposal V1 requests diameter 150 mm; V2 changes it to 200 mm. A response accepting
V1 remains a decision about 150 mm. V2 is not accepted, whichever order the files arrive in.
The user can inspect V1 and its decision alongside V2 without creating two physical openings.

**OA-DEC-004.** Each project serializes its own authored records against its expected local
head/generation. A decision supersession is explicit and checked against the expected previous
decision; simultaneous competing decisions do not win by arrival order or UUID time. Retain
both as evidence, report a conflict and require an authorized resolution before showing an
effective current acceptance. A withdrawn proposal cannot be restored by a delayed response.

The current view distinguishes pending review, current acceptance, rejection, withdrawn,
needs review and conflicting evidence, with historical decisions still accessible. These are
logical presentation outcomes, not newly prescribed USD attribute tokens.

## 5. Acceptance and implementation are separate

**OA-IMPL-001.** Accepting or importing an assignment/decision does not cut the host model.
An authorized host command creates the physical opening only after checking the exact accepted
proposal and the current structural basis. It records the assignment, proposal, effective
decision, resulting opening identity and resulting host revision. The publication is atomic
within the host project and idempotent under F04; a retry must not create a second opening.

The decision names the reviewed pre-edit structural basis; the implementation record names
the resulting revision. Creating the approved opening therefore has its own explicit evidence
of the authorized transition, rather than invalidating itself merely because a new revision
was created. Subsequent changes need revalidation. If an opening is changed or removed, retain
the old implementation record as history and do not report it as current implementation.

If a 150 mm opening already exists when the request changes to 200 mm, retain the actual 150 mm
opening and its historical basis; mark the revised request as needing review. Resizing or
removing the opening requires a new authorized host operation. Neither delivery of V2 nor
import of an old response performs that operation.

## 6. Relation to F10 and F12

For the proposed F10 0.2 file-exchange workflow, this companion replaces section 5.4's assumption
of one globally shared mutable state and one `stateSeq` compare-and-set across both parties.
The proposer authors proposal history; the host authors decision and implementation history;
receivers retain immutable copies as evidence. They describe one logical assignment. The
combined status is derived from matching versions and validated provenance, never by merging
arbitrary USD fields from two owners. Local mutations still use F04 concurrency controls.

The F10 0.1 contract, its local transition table and fixtures remain unchanged. In 0.2,
`DV-PEN-003`'s stale/competing-state protection is supplied by OA-VER-001 and OA-DEC-001 through
004 for exchanged assignments; other existing field, role, preservation and deletion
constraints remain where applicable. Withdrawals and decisions are retained; deleting a
local display item cannot erase the other participant's evidence or remove a physical opening.
Detailed command/state encoding must be reviewed before implementing this new compatibility line.

Pin evidence under F12 retention rules while the assignment/history requires it. Composition
connections remain acyclic by source project along a composition path. Historical evidence
may return to an older revision of a project (OV proposal based on KR10, KR11 decision based
on that OV proposal); it is not a new composition connection. Its closure is traversed by
exact project/revision identities with cycle and resource-budget checks, never by mutable heads.

## 7. Required acceptance cases

These are contract requirements, not executed tests. Refusals use existing F04 codes.

| Rule | Positive case | Negative/conflict case |
|------|---------------|------------------------|
| OA-ID-001 / OA-VER-001 | Editing 150 mm to 200 mm retains identity and publishes V2 | Different content under V1 refused (`INTEGRITY_ERROR`); competing successors require resolution (`DEPENDENCY_CONFLICT`) |
| OA-PKG-001 | A file contains three assignments; the host accepts one, rejects one and leaves one unanswered | Missing item evidence refuses package admission (`DEPENDENCY_UNAVAILABLE`); receipt alone approves none |
| OA-PKG-002 | Identical retransmission replays the receipt; repackaging V1 creates no duplicate | Reused package identity with changed bytes refused (`INTEGRITY_ERROR`) |
| OA-AUTH-001 | Authorized host records its own decision; source inputs survive unchanged | Claimed actor/section alone cannot approve (`PERMISSION_DENIED`); foreign request-field edit refused (`FOREIGN_READ_ONLY`) |
| OA-DEC-001/002 | Matching proposal and reviewed host basis can show current acceptance | Changed proposal/host basis has no effective acceptance until revalidated (`DEPENDENCY_CONFLICT` when attempting implementation) |
| OA-DEC-003 | V1 acceptance arriving after V2 is retained as historical evidence | V2 never inherits V1 acceptance and is never overwritten; test both delivery orders |
| OA-DEC-004 | Explicit authorized supersession changes the effective decision | Competing decisions remain visible and unresolved; old response cannot undo withdrawal (`DEPENDENCY_CONFLICT` for conflicting application) |
| OA-IMPL-001 | Authorized implementation links the opening and resulting revision; exact retry creates nothing | Imported acceptance creates no hole; unreviewed V2 cannot resize an implemented V1 opening |

Also require interrupted-import/publication recovery, unknown package versions, hostile paths,
truncated input, bounded repeated evidence and retention of live proposal/decision bases.
Existing C03–C06 remain deferred; no automatic rebase, second engineering module or network
coordinator is needed merely to specify this file-based workflow. The broader points 5
(`builtFrom` and other inter-section dependencies) and 6 (generic viewing) remain separate work.
