# F10: domain vocabulary amendment, version 0.2.0

Status: **draft contract; foundation approved by the owner on 2026-10-22; not implemented**.
Part of the single F12 project-coordination foundation work package.

This document and [F10 0.1.0](f10-domain-vocabulary-v0.1.md) together specify the proposed
0.2.0 vocabulary. The replacements below take precedence; all other 0.1.0 clauses remain
applicable. The frozen 0.1.0 document, schema and acceptance cases are unchanged. This is a
new compatibility line, not a patch to the meaning of existing 0.1.0 projects.

## 1. Separate engineering semantics from project responsibility

Replace the use of one "discipline" concept for both engineering behaviour and the workspace's
write boundary with these distinct concepts:

| Concept | Meaning | Example |
|---------|---------|---------|
| Module | Registered implementation of kinds, typed commands and engineering rules | `steel` |
| Namespace | Semantic attributes owned by that module | `steel:` |
| Section | Independently responsible project deliverable, with a stable SectionId | `KM`, `KMD` |
| Zone | Optional project partition, with a stable ZoneId | `K1` |
| Workspace | Personal history based on a pinned project revision | A detailer's draft |
| Write scope | The section and explicit zone set the trusted context permits this workspace to edit | KMD in K1 |

SectionId and ZoneId are full UUIDv7 identities under [F12](f12-project-coordination-v0.1.md).
Codes and names are labels, not authority. A module may serve several sections. KM and KMD
both use `steel`; enabling steel grants neither access to another section nor permission to
accept its changes. A zone does not become an element's parent path.

`bim:discipline` retains its semantic-module meaning (`steel`, `arch`, `mep`); it does not
become `KM`, `KMD` or a team name. Kind ownership, namespace registration and module manifests
in F10 0.1.0 sections 2, 4.1, 4.2 and 7.1 retain that meaning. Reserved kinds and modules
gain no implementation through this amendment.

## 2. Replace the workspace boundary in section 7.4

**DV-SCOPE-001.** A workspace has one active SectionId and an explicit writable zone set,
including the unzoned partition when needed. Its base pins the section registry and scope.
The trusted execution context must authorize that scope at publication; a client-provided
section, zone, module, actor or filename does not establish authority.

**DV-SCOPE-002.** An ordinary element edit must pass all of these checks:

1. The engineering module is enabled and supports the typed operation and vocabulary.
2. The element's validated authored provenance belongs to the active section and allowed zone.
3. The authenticated or otherwise trusted local execution context authorizes the operation.
4. Namespace, ownership, dependency and transition rules permit the particular fields.

Unknown provenance gives no write permission. Installing or enabling a module cannot expand
the scope. Reading already admitted content through the common read API is unchanged; this
does not define remote access-control policy.

**DV-SCOPE-003.** Publishing ordinary edits must preserve other sections' authored content,
external content and the project's composition. A stronger USD opinion over an out-of-scope
element is still an edit and must be refused. A scope change, layer relocation or change to
the root's sublayers cannot be smuggled through an ordinary element command.

**DV-SCOPE-004.** Shared metadata, grids/levels and coordination membership require explicit
shared commands and separately authorized scopes. Being common data does not make them
writable by every section. Their dependencies and effects participate in validation just as
element edits do. Shared definitions remain immutable content-addressed values.

F10 0.1.0 `DV-MOD-005` is replaced on this line by `DV-SCOPE-001` through `004`; it remains
unchanged on the 0.1 line. `DV-OWN-001` and namespace checks remain additional constraints:
permission within a section never permits writing another module's private attributes.
Module enable/disable and unknown-content preservation otherwise retain section 7.4's rules.

## 3. One shared opening assignment

The engineering-facing name for the reserved `penetration` request is **opening assignment**
(Russian: "задание на отверстие"). There is one assignment identity and one revisioned
history of its proposal and decisions, not a separate request per participant.

The proposer supplies the requested shape, dimensions, placement and subject/host references.
The responsible host section decides whether the opening is acceptable. The eventual physical
opening belongs to the host model and is linked to the assignment; an accepted assignment
does not itself prove that an opening has been authored or checked in that model.

For F10 section 5.4, replace the authority terms "proposer discipline" and "host discipline"
with the responsible proposer and host **sections**. Module capability remains required.
Sections may use the same module and still have different decision rights. The shared
transition is the explicit, narrowly authorized exception to ordinary active-section writes:
the host may record a decision on the assignment but cannot alter the proposer's request
fields or other source content. It must run as a shared command, with both participants and
the exact target state validated, never as a raw foreign-layer edit.

The owner-approved stage 4 workflow is specified in the
[F12 opening-assignment companion](f12-opening-assignments-v0.1.md): engineers design services
against a pinned structural reference, then send batches of assignments as files. Decisions
bind exact proposal and reviewed host revisions; acceptance of a 150 mm request never applies
to its 200 mm successor. Accepted and implemented remain distinct facts.

For file exchange on the proposed 0.2 line, the companion replaces the assumption of a global
`stateSeq` transaction with immutable, version-pinned proposals and host decisions, each
published in its owner's local history. The shared decision command records a decision in the
host's project and never writes the proposer's store. Received records are immutable evidence;
only validated, authorized records can affect the projected assignment status. The companion
specifies stale-response handling and partial batch outcomes. The old 0.1 table and fixtures
remain unchanged; concrete 0.2 commands, encoding and provenance admission are implementation
gates. No KR opening implementation or distributed transaction is added by these documents.

## 4. Compatibility and implementation boundary

**DV-SCOPE-005.** A 0.1 project must not acquire section authority by inference from its file
names, namespace or installed modules. Moving to 0.2 requires an explicit upgrade that assigns
sections, zones and authored provenance, validates the complete result and publishes a new
revision. The original revision remains intact. No upgrade is performed merely by reading.

The logical section and provenance records are specified in F12. Their exact USD serialization,
schema digest, typed command envelopes and bounded parsers must be defined and tested before
an implementation advertises vocabulary 0.2.0. Current 0.1 commands and stores continue to use
their current contracts. No module crate, dynamic plugin, network service or new engineering
kind is introduced by this document.

## 5. Rules and acceptance descriptions

These are contract cases to implement, not executed tests. Structural and scope rules run at
client preflight, authoritative publication and any future post-merge validation. A client pass
is advisory; the publisher derives provenance and permissions again from trusted inputs.

| Rule | F04 refusal | Positive case | Negative case |
|------|-------------|---------------|---------------|
| DV-SCOPE-001 | `PERMISSION_DENIED` | Authorized KMD/K1 workspace publishes within its scope | Caller changes its claimed scope to KM |
| DV-SCOPE-002 | `FOREIGN_READ_ONLY` for foreign scope; existing module refusals otherwise | Steel edits a KMD element in its admitted zone | The same steel module edits a KM element or a KMD element outside the writable zones |
| DV-SCOPE-003 | `FOREIGN_READ_ONLY` | KMD edit preserves KM, AR and root composition | KMD authors a stronger override of a KM prim or rewrites the root to hide it |
| DV-SCOPE-004 | `PERMISSION_DENIED` | Explicitly authorized shared command updates grids | Ordinary section edit changes common grids or coordination pins |
| DV-SCOPE-005 | `UNSUPPORTED_VERSION` | Explicit upgrade produces a validated new revision | Reader assigns section rights to a legacy file automatically |

Additional reserved lifecycle cases: the host section can decide an assignment while its
request fields remain unchanged; the proposer cannot accept merely because both sections use
the same module; competing decisions cannot both become the effective acceptance; acceptance
alone does not create a physical opening. The companion maps the exchanged-assignment
concurrency rules to F04 refusals and explains their relationship to `DV-PEN-003`.

## 6. Provenance

This amendment develops the owner's approved project organization and the existing F04/F10
contracts. The owner-supplied Speckle study informed the separation of generic data handling
from engineering modules; it is design input, not normative evidence of BeamCraft behaviour.
No third-party source, assets, schema text or implementation was copied.
