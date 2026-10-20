# F10: domain vocabulary and discipline-module contract, version 0.1.0

This contract fixes **what an element, a relationship and an opening are called and what they
carry**, so that more than one discipline can read and write the same project without private
dialects, and it fixes the boundary between the shared application and a discipline module. The
examples are acceptance descriptions, not APIs or saved models. Only the **steel** module is in
scope (sections 1.3 and 7.6); every other element kind is `reserved` vocabulary: a name and a
promise of shape, with no behaviour.

## 1. Status and scope

### 1.1 Relation to other contracts

- [F04](f04-model-and-commands-v0.1.md): F10 takes identity (section 2), limits (section 3), authored
  fields (section 4), commands (section 7) and the 23 error codes (section 8). It changes nothing:
  no identity, limit, command or code is changed or added.
- [F06](f06-geometry-contract-v0.1.md): F10 takes `ElementId` as the root of every FeatureId
  (section 5). Geometry stays in the kernel. It changes nothing and adds no collision evaluation.
- [F08](f08-module-boundaries-v0.1.md): F10 takes the crate layers and the one-way dependency rule.

F05 (worker protocol) is not an input. Real-time synchronisation is **not designed here**: section 6
only says what is checked again after a merge.

### 1.2 What the document decides

1. A closed registry of element kinds (section 2).
2. The attributes every element carries (section 3).
3. Namespaces, ownership, unknown-content preservation, versioning and negotiation (section 4).
4. Relationships, including the shared penetration request (section 5).
5. The validation layer: rule IDs, stages, a result envelope (section 6).
6. The discipline-module contract and enable/disable behaviour (section 7).

Normative statements carry a rule ID of the form `DV-<FAMILY>-<NNN>`; section 6.5 lists all
57 of them. Prose without an ID is explanation or policy that no case checks.

### 1.3 First slice and reserved vocabulary

- **Implementable in the first slice:** the kinds `plate`, `beam` and `column` (steel), the generic
  validation layer over them, the module registry, and preservation of unknown content. This is
  the `now` slice of section 6.5: 42 rules.
- **Reserved:** `wall`, `slab`, `opening`, `penetration`, `pipe`, `duct`, `support`, `assembly`,
  `connection`, `reference`. Reserved kinds have names, owners and *proposed* attribute names, and
  nothing else: no command creates them, no behaviour is attached, and authoring one is rejected
  (`DV-KIND-004`). The 15 `reserved` rules and their cases are specification for a future
  module; they are marked `first_slice: false` in the case file and are not implemented work.
- Content that already contains a reserved or unknown kind is **loaded, preserved and shown
  read-only** (`DV-KIND-002`, `DV-MOD-001`). It is never rejected on load and never deleted.

### 1.4 Excluded

Real-time or concurrent-editing design; the USD file layout beyond naming prim kinds and fields; a
second persisted model format (JSON here describes messages and acceptance cases only);
geometry, clash or collision *evaluation*; UI types; permissions and authentication beyond F04
section 8; profile, material or product catalogues; numbering; export formats.

### 1.5 Sources

The vocabulary uses generic engineering words; no product, IFC-library or vendor schema, naming
list or catalogue is copied. IFC export is a separate adapter (E01).

## 2. Element kinds

### 2.1 The registry

The registry is **closed** for a vocabulary version. A kind is a lowercase token stored in
`bim:elementKind` (F04 section 4). F04 fixes the first-slice values `plate`, `beam` and `column`;
this contract adds the reserved tokens below and changes no F04 value. Adding or removing a kind
needs a new vocabulary version (section 4.5). An unknown kind in an authoring request is
`DV-KIND-001`; in loaded content it is `DV-KIND-002`.

| Kind | Owner | Status | Family |
|------|-------|--------|--------|
| `plate` | steel | implemented (first slice) | plate |
| `beam` | steel | implemented (first slice) | straight member |
| `column` | steel | implemented (first slice) | straight member |
| `wall` | arch | reserved | - |
| `slab` | arch | reserved | - |
| `opening` | arch | reserved | - |
| `penetration` | any | reserved | - |
| `pipe` | mep | reserved | - |
| `duct` | mep | reserved | - |
| `support` | any | reserved | - |
| `assembly` | any | reserved | - |
| `connection` | steel | reserved | - |
| `reference` | any | reserved | - |

**Owner** is a discipline token. `steel`, `arch` and `mep` are the discipline tokens. An authored discipline or module token outside
`steel`, `arch`, `mep` is rejected (`DV-KIND-006`). Loaded content with an unknown discipline is
preserved read-only under section 7.4, not rejected on load. `any` means the discipline of the
module that authors the element; it is stored in `bim:discipline` and is then immutable
(`DV-OWN-003`). For every other kind
`bim:discipline` must equal the owner (`DV-KIND-005`). A *straight member* is the family of `beam`
and `column`; F04 treats them as one command with an explicit kind.

### 2.2 Implemented kinds (first slice)

All three carry the common base of section 3. Lengths are millimetres, coordinates are element
local, angles are radians (F04 section 3).

**`plate`** (owner steel).

- Required: `steel:plate:widthMm`, `steel:plate:lengthMm`, `steel:plate:thicknessMm`; each a finite
  number greater than 0 and at most 1,000,000 (`DV-LIM-001`, `DV-LIM-002`).
- Optional: `bim:material`.
- Undefined here: holes, cuts, bevels, grade, numbering, weld preparation, bolt patterns.

**`beam`** and **`column`** (owner steel).

- Required: `bim:axisPointsMm` (two points of three finite numbers, element local, nonzero distance
  at most 1,000,000), and `bim:profile`, the ContentDigest of a pinned profile definition.
- Optional: `bim:material`.
- Undefined here: section kinds beyond what F06 section 2 admits, end cuts, camber, roll beyond the
  placement, position presets, catalogue lookup by name.

The frames, the profile resource and the quantities of both kinds stay in F04 and F06.

### 2.3 Reserved kinds

The attribute names below are *reserved*: they are not validated in 0.1.0 and may change before
a module implements the kind. Units are fixed now so that two disciplines cannot pick different
ones. "Undefined" lists what this contract deliberately does not say.

- **`wall`** (arch). Required: `bim:axisPointsMm`, `arch:wall:thicknessMm`, `arch:wall:heightMm`.
  Optional: `bim:material`. Undefined: layers, joins, curved walls, finishes.
- **`slab`** (arch). Required: `bim:outlineMm` (closed planar polygon, at least 3 points, local XY),
  `arch:slab:thicknessMm`. Optional: `bim:material`. Undefined: slope, edges, drops.
- **`opening`** (arch). A void authored by the host's own discipline, with no request step.
  Required: `bim:opening:shape` (`round` or `rect`), `bim:opening:sizeAMm`, `sizeBMm` for `rect`;
  one `hosted-in` record whose host has the same discipline (`DV-REL-014`). `opening` and
  `penetration` remain separate kinds until a module needs to merge them; merging requires a
  vocabulary amendment. Undefined: frames, reveals, ratings.
- **`penetration`** (any). A request by one discipline for an opening through an element of another.
  The full shape and lifecycle are fixed in section 5.4. Undefined: sleeves, firestopping, more
  than one host, cut geometry.
- **`pipe`** (mep). Required: `bim:axisPointsMm`, `mep:pipe:outerDiameterMm`. Optional:
  `mep:pipe:wallThicknessMm`, `bim:material`. Undefined: fittings, flow, slope, insulation, systems.
- **`duct`** (mep). Required: `bim:axisPointsMm`, `mep:duct:shape` (`round` or `rect`),
  `mep:duct:sizeAMm`, `sizeBMm` for `rect`. Undefined: fittings, flow, insulation, systems.
- **`support`** (any). Required: the common base only. Optional: one `hosted-in` record.
  Undefined: loads, support types, spacing.
- **`assembly`** (any). A grouping target of `part-of`. Required: the common base only.
  Undefined: numbering, nesting depth limit, assembly types, multi-element copy (F04 section 7).
- **`connection`** (steel). Required: one `connects` record with 2 to 1024 targets. Undefined:
  bolts, welds, connection rules, design checks.
- **`reference`** (any). The wrapper of a foreign read-only source (F04 section 1). Undefined:
  everything; R01 defines attachment commands after this contract.

## 3. Common base

### 3.1 Attributes every placed element carries

| Attribute | Value | Written by |
|-----------|-------|------------|
| `bim:id` | Full UUIDv7 (F04 section 2) | engine, at creation |
| `bim:name` | Label, 1 to 256 bytes, no control characters | owning module |
| `bim:elementKind` | Kind token of section 2.1, immutable | owning module, at creation |
| `bim:discipline` | Owner token, immutable (new) | engine, from the authoring module |
| `bim:lifecycle` | `active` or `deactivated` (new) | owning module |
| `bim:schemaVersion` | Vocabulary version that wrote the element, `0.1.0` here | engine |
| `bim:geometryMode` | As F04 section 4 | owning module |
| `bim:createdBy` | As F04 section 4, from trusted context | engine |
| `bim:revision` | As F04 section 4 | engine |
| Placement | Rigid pose, F04 section 3 | owning module |
| `bim:material` | Optional ContentDigest of a pinned immutable definition | owning module |

Rules: identity `DV-BASE-001`, `DV-BASE-002`; label `DV-BASE-003`, `DV-LIM-001`; lifecycle
`DV-BASE-004`; material `DV-BASE-005`, `DV-BASE-006`; placement `DV-LIM-002`, `DV-LIM-003`;
a missing required attribute `DV-ATTR-001`. An element label is never an identity: renaming keeps
the ID and the prim path (F04 section 2).

### 3.2 Lifecycle

`active` is the state of every new element. `deactivated` means *retained but out of play*: the
element, its identity and its relationships stay in the project, and derived results may treat it
as absent. It is not `deleted`; delete is the F04 `DeleteElement` command and removes the element.
A relationship whose target is `deactivated` stays valid and is reported (`DV-REL-012`). F04 has no
command that changes `bim:lifecycle`, so in 0.1.0 a steel element is always created `active` and
`deactivated` appears only in loaded content.

### 3.3 What only a discipline may add

An attribute outside the shared namespace `bim:` is *discipline-specific* and belongs to the module
that owns that namespace (section 4). The common base never contains a steel, architecture or pipe
quantity. A kind-specific attribute listed in sections 2.2 and 2.3 is valid only on that kind
(`DV-ATTR-005`).

## 4. Namespaces and ownership

### 4.1 Two kinds of namespace

- **Shared:** `bim:`. Defined by this vocabulary and changed only by a vocabulary version. Any
  module reads it; writes follow the table of section 4.3.
- **Discipline:** `<token>:` where the token is the registered namespace of one module and equals
  its discipline token (`steel:` already appears in F04 as `steel:plate:widthMm`). Only that module
  writes it (`DV-OWN-002`).

Attribute names are `<namespace>:<name>[:<name>...]`. A namespace token matches
`[a-z][a-z0-9_]{0,31}`. Each following segment matches `[A-Za-z][A-Za-z0-9]{0,63}`; there are at
most four segments and at most 128 bytes in total. A name that does not match is rejected when
authored (`DV-NS-001`). A syntactically valid attribute from an unknown namespace is legal in loaded
content and is preserved (section 4.4).

### 4.2 Registering a namespace

Each module registers exactly one namespace. The shared token `bim`, the tokens `usd` and `xform`,
and any token already registered are refused (`DV-MOD-004`).

### 4.3 Who may author what

| Attribute group | May be written by |
|-----------------|-------------------|
| `bim:id`, `bim:createdBy`, `bim:revision`, `bim:schemaVersion` | engine only |
| Other `bim:` attributes of an element | the module of the element's discipline |
| `bim:penetration:` request fields (section 5.4) | the proposer's module |
| `bim:penetration:` decision fields (section 5.4) | the host element's module |
| `<token>:` attributes | the module registered for that token |

Any other write is `DV-OWN-002` (wrong namespace or engine-only attribute), `DV-OWN-001` (another
discipline's element) or `DV-PEN-005` (the other party's penetration field).
`bim:discipline` and `bim:elementKind` never change after creation (`DV-OWN-003`). Ownership is
derived by the engine from the element, never from a field in the request (F04 section 1).

### 4.4 Unknown content is preserved

When any module or the engine rewrites an element (rename, move, resave, merge), every attribute
and relationship it does not understand, in any namespace including `bim:`, survives with its
meaning unchanged. Dropping or altering it is `DV-NS-002` and is a defect, not a user error.
Authoring an undefined attribute through a command is a different case and is rejected
(`DV-ATTR-004`). Unknown attributes are never promoted to defined ones by being read.

### 4.5 Reading another discipline's element

Any module may read any element through the engine read API. Reading never needs permission from
the owner. A module sees the shared attributes it knows, and a foreign discipline's attributes
only as opaque data. It cannot change any of it: `DV-OWN-001` (F04 `FOREIGN_READ_ONLY`). A module
that wants a change asks the owner: for an opening this is the penetration request of section 5.4;
for anything else the mechanism is outside this version.

### 4.6 Vocabulary versioning

The vocabulary version is `MAJOR.MINOR.PATCH`, stored per element in `bim:schemaVersion`.

- **Compatibility line.** For `MAJOR >= 1` the line is `MAJOR`. While `MAJOR` is 0 the line is
  `0.MINOR`. A version on another line is *unsupported* for editing (`DV-VER-001`); moving from
  `0.MINOR` to `1.0` is an explicit migration step.
- **Additive change.** A new optional attribute, a new reserved kind or a new rule severity of
  `warning` raises the PATCH (line `0.y`) or MINOR (line `>= 1`). Required attributes, units,
  meanings and rule codes of an existing name never change inside a line.
- **Deprecation.** A deprecated name stays defined and readable for the rest of its line and is
  removed only with the next line. Policy only; 0.1.0 deprecates nothing.
- **Migration.** Moving content to another line is an explicit, reviewed step. No reader rewrites
  `bim:schemaVersion` on its own.

A reader mutates an element only if the element's version is on the reader's line and is not newer
than the reader's own (`DV-VER-002`). A newer element is read-only and its unknown attributes are
preserved. Content on an unsupported line can be inspected and preserved, not mutated
(`DV-VER-001`; F04 section 4). Editing newer-patch elements on the same compatibility line is
forbidden until an explicit migration story exists (`DV-VER-002`).

### 4.7 Capability negotiation

Both sides state the vocabulary versions they support and each module states the version it
requires (section 7.1). The negotiated vocabulary version is the highest version present on both
sides within one compatibility line. A module is enabled only if a version is selected, that
version is on the compatibility line of its `requires_vocabulary`, and that version is at least
that minimum (`DV-VER-003`). Compare versions numerically by (MAJOR, MINOR, PATCH), not as text.
The minimum need not appear in either peer's advertised set: peers advertising only `0.1.1` can
enable a module requiring `0.1.0`. No common selected version, a different line, or a selected
version below the minimum leaves the module disabled. The host's answer is authoritative.
The client validates before sending with the negotiated version only. The carrying message belongs
to F05/U02; this contract fixes the content.

## 5. Cross-discipline relationships

### 5.1 The vocabulary

A relationship is a directed record from a **source** element to a **target** element, except
that section 5.4 permits a historical host identity instead of a live target. It is identified by
the tuple (type, source, target, role), not by a new identity type (`DV-REL-007`).
`role` is an optional token (`[a-z][a-z0-9_]{0,31}`); only `subject` has defined semantics.
`references` accepts any role matching that syntax; no closed list beyond `subject` is imposed
(`DV-REL-013`). Unknown roles acquire no implied behaviour. A record always lives with its source.
F04's `bim:profile` and `bim:material` links to immutable definitions are not element relationships
and are not covered here.

- `hosted-in`. Source: `opening`, `support`. Target: `wall`, `slab`, `plate`, `beam`, `column`.
  Per source: an opening has exactly 1, a support 0 or 1.
- `penetrates`. Source: `penetration`. Target: `wall`, `slab`, `plate`, `beam`, `column`.
  Per source: exactly 1.
- `references`. Source: any kind. Target: any kind. Per source: 0 to 1024; role optional.
- `part-of`. Source: member kinds, see below. Target: `assembly`. Per source: 0 or 1.
- `connects`. Source: `connection`. Target: `beam`, `column`, `plate`, `assembly`.
  Per source: 2 to 1024 distinct targets.

For `part-of` the source may be `plate`, `beam`, `column`, `pipe`, `duct`, `support`,
`connection` or `assembly`. Targets are unlimited in number except for the 1024-target cap of
F04 section 3, which applies per element in total (`DV-LIM-001`). A penetration also needs exactly
one `references` record with role `subject` naming the passing element.

Rules: type `DV-REL-002`; endpoint kinds `DV-REL-003`; cardinality `DV-REL-004`; cycles in
`part-of` or `hosted-in` of any length `DV-REL-005`; self-links `DV-REL-006`; duplicates
`DV-REL-007`; dangling targets `DV-REL-001`.

### 5.2 Direction and ownership

Direction is source to target and is never reversed implicitly. A record belongs to the discipline
that owns its **source** element; only that discipline may author or edit it (`DV-REL-008`). The
host side never gains write access to an incoming record. For `penetrates` the source is the
penetration, so the proposing discipline owns the records, and the host discipline answers through
the decision fields of section 5.4.

### 5.3 Delete and copy

- **Delete.** F04 forbids deleting an element that has unresolved incoming dependencies, with no
  implicit cascade. An incoming live record blocks the delete (`DV-REL-009`). Only a `penetrates`
  record whose source penetration is `rejected` or `withdrawn` is historical (section 5.4) and
  does not block it. Other records from that penetration, including its subject reference, remain
  live. If the incoming index cannot be completed the answer is `DV-REL-010`, never "no links".
  Deleting an element removes its own outgoing records with it. Deleting an `accepted`
  penetration is rejected; the proposer must withdraw it first (`DV-PEN-007`).
- **Copy.** The first F04 `CopyElement` takes one source. A source that has element-to-element
  relationships is refused rather than copied incompletely (`DV-REL-011`).
- **Deactivated targets** are reported, not rejected (`DV-REL-012`).

### 5.4 The shared penetration request

`penetration` is one shared type so that a pipe discipline and a structural discipline describe a
requested hole the same way. It is `reserved`: the table below is specification and nothing in the
first slice creates or changes one. The opening/penetration lifecycle belongs in the common
vocabulary; modules must use the shared states and transition rules (`DV-PEN-001` to
`DV-PEN-007`). `opening` is a host-owned void without a request; `penetration` is a request.
They remain separate kinds until a module needs to merge them.

**Fields** (shared namespace; case files use the short names in brackets).

- Request fields, written by the proposer: `bim:penetration:shape` (`round` or `rect`),
  `sizeAMm`, `sizeBMm` (`rect` only), optional `clearanceMm`; each size greater than 0 and at
  most 1,000,000 mm, clearance at least 0. Placement is the common pose.
- Decision fields, written by the host discipline: `bim:penetration:reasonCode` [`reason`].
- State fields, written only through a transition: `bim:penetration:state` [`state`] and
  `bim:penetration:stateSeq` [`stateSeq`], an unsigned integer that is 1 at creation and rises by
  exactly 1 on each accepted transition or request edit.
- Records: one `penetrates` naming the host, one live `references` with role `subject` to the
  passing element. The `penetrates` record is live in `proposed` or `accepted`; in `rejected` or
  `withdrawn` its target is a stable historical ElementId, not a live link. The record and target
  ID stay unchanged with the source; no party edits the other discipline's record on deletion.
  A historical target need not resolve (`DV-REL-001`); target-kind and deactivation checks
  (`DV-REL-003`, `DV-REL-012`) apply only while it is live. It still counts for cardinality and
  duplicate checks. Returning to `proposed` makes it live again and requires the same host to
  resolve; a missing host rejects the transition with `DV-REL-001`. No retargeting is implicit.
- Closed reason codes: `position`, `size`, `structural`, `other`.

**Who.** The **proposer** is the discipline in `bim:discipline`. The **host discipline** is the
discipline of the element at the end of `penetrates`. When both are the same discipline the request
is allowed and follows the same table; one module then holds both rights. Only the host discipline
may accept a penetration (`DV-PEN-002`); being the proposer alone grants no acceptance right.

**States and transitions.** Every transition carries the expected `stateSeq`
(`DV-PEN-003`). A transition not in this table is `DV-PEN-001`; the right edge by the wrong
discipline is `DV-PEN-002`.

| From | To | Actor | Condition |
|------|----|-------|-----------|
| (creation) | `proposed` | proposer | request fields complete |
| `proposed` | `accepted` | host | host exists and is `active` (`DV-PEN-004`) |
| `proposed` | `rejected` | host | reason code from the closed list (`DV-PEN-006`) |
| `proposed` | `withdrawn` | proposer | none |
| `rejected` | `proposed` | proposer | revised request, `stateSeq` rises |
| `rejected` | `withdrawn` | proposer | none |
| `accepted` | `proposed` | proposer | request fields changed; acceptance is void |
| `accepted` | `rejected` | host | revoke, reason code required (`DV-PEN-006`) |
| `accepted` | `withdrawn` | proposer | none |

`withdrawn` is final. The host writes only decision fields and the transition (`DV-PEN-005`); the
proposer writes only request fields and its own transitions. A request edit while `proposed` raises
`stateSeq` so that a concurrent decision is detected.

**Conflict behaviour.** The transition is a compare-and-set on `stateSeq` inside the host
transaction. When two actors act on the same state (for example the proposer withdraws while the
host accepts) the first applied wins and the second is rejected with `DV-PEN-003`
(`DEPENDENCY_CONFLICT`). The loser's intent is returned in the diagnostics and its draft is kept
(F04 section 8: conflicts preserve local edits). UUID clock order gives no priority (F04
section 2). If both transitions arrive through a merge, the merged result is refused by
`DV-MRG-001` together with `DV-PEN-003`, and a person re-issues the transition against the new
state. Deleting a host that has a `proposed` or `accepted` penetration is `DV-REL-009`; the host
discipline first rejects the request. Its `penetrates` record then becomes historical, which
unblocks deletion if no other live incoming records remain and the incoming index is complete.
The retained record remains valid after host deletion. Deleting a penetration is not a transition.
An `accepted` penetration must first become `withdrawn` through the proposer's sequence-checked
transition before deletion (`DV-PEN-007`); deletion never withdraws it implicitly. Other F04 delete,
ownership, enabled-module and incoming-index checks still apply, including after withdrawal.
At M, the recorded withdrawal must precede the delete; a missing final element alone is not proof.

## 6. Validation layer

### 6.1 Stages

The same rule catalogue runs at three points. The letter is used in the catalogue.

- **C, client before send.** Advisory. It lets a user see a rejection without a round trip. A
  client result is never trusted by the host.
- **H, host on apply.** Authoritative for a command. It runs inside the host's workspace
  transaction after F04's checks of version, context, op_id replay, workspace and base.
- **M, post-merge re-check.** After a merge or rebase the host re-runs every model and
  relationship rule on the merged result and ignores any client-reported pass (`DV-MRG-001`).
  All element and module rules also run when applicable. Checks use the merged state and recorded
  operation provenance: authoring, ownership, transition, delete, copy and module-operation rules
  re-check the relevant operations against the merged inputs. Authoring-only checks apply to
  those writes, not to untouched loaded content; unknown and reserved content stays preserved.
  A valid final state alone cannot prove that competing transitions were valid.

A failing H or M run changes nothing: authored state, derived state and generation stay as they
were (F04 section 8), and local drafts are kept.

### 6.2 Scope

A rule has one scope. **element:** one element and its own attributes. **relationship:** one record
and its two endpoints. **model:** the whole set of elements and records. **post_merge:** only
meaningful on a merged result.

### 6.3 Result envelope

A validation result is a logical message, not a stored file.

| Field | Meaning |
|-------|---------|
| `vocabulary_version` | Negotiated version used |
| `stage` | `C`, `H` or `M` |
| `status` | `pass`, `fail` or `incomplete` |
| `primary_code` | `DEPENDENCY_UNAVAILABLE` if incomplete; else first error code or `null` |
| `incomplete` | True if any required input was unavailable or a traversal limit prevented checking |
| `total_findings` | Count of findings found, saturating at 1,000,000 |
| `truncated` | True if `findings` is shorter than `total_findings` |
| `findings` | At most 256 entries, ordered as in 6.4 |

A finding has `rule_id`, `severity` (`error`, `warning`, `info`), `code` (an F04 code for an
error, `null` otherwise), and as needed `element_id`, `relationship` (`type`, `source`, `target`,
`role`), `attribute` and `message`. The message is a fixed text of at most 256 bytes per rule; it
never contains a credential, a private path or client data. Every C, H and M report uses this exact
precedence, independently of finding order:

1. Required input unavailable (`DV-REL-010`) or traversal stopped at a host limit: `status` is
   `incomplete`, `primary_code` is `DEPENDENCY_UNAVAILABLE`, and `incomplete` is `true`, even if
   definite error findings are also present. Retain all definite findings that were found.
2. Otherwise, any error: `status` is `fail`, `primary_code` is the first ordered error's code,
   and `incomplete` is `false`.
3. Otherwise: `status` is `pass`, `primary_code` is `null`, and `incomplete` is `false`.
   Warnings alone give `pass`.

In every branch, `truncated` is exactly `len(findings) < total_findings`: unavailable input alone
does not set it. `total_findings` counts findings actually found, not unvisited possibilities.
Unavailable-only and unavailable-plus-one-definite-error reports have `truncated: false` when
their one or two findings fit. Neither `fail` nor `incomplete` permits H or M to accept a change.

### 6.4 Bounds and determinism

1. Findings are ordered by severity (`error`, `warning`, `info`), then by family in this order:
   VER, MOD, KIND, BASE, NS, OWN, REL, PEN, COL, ATTR, LIM, MRG, then by rule ID, `element_id`,
   `attribute`, and the relationship tuple. Comparison is bytewise on UTF-8.
2. The report keeps the first 256 findings in that order. `truncated` is true only when findings
   are omitted by this cap; it is independent of `incomplete`.
3. Select `primary_code` by the precedence in 6.3 before truncation: unavailable inputs override
   the first ordered error, even if the unavailable finding falls beyond the report cap.
   Otherwise errors sort first, so the first error is retained. This is a fixed validation order
   under F04 section 8; tests must not depend on collection order.
4. The checker visits each element and each record a bounded number of times: duplicate and cycle
   detection are linear in the number of elements plus records. The input is bounded by the F04
   caps and the host's stage-traversal limits; an input that exceeds a host limit gives
   `incomplete` with `DEPENDENCY_UNAVAILABLE`, not a partial pass.
5. The result depends only on the content, recorded operations, available validation inputs,
   configured limits, negotiated version and enabled modules. It does not depend on a clock,
   a random source, map iteration order or the platform.

### 6.5 Rule catalogue

Run points: C client, H host, M post-merge. `now`: implementable in the first slice. `reserved`:
specification only. An F04 code of `-` means the rule is a warning and never rejects. No new F04
code is introduced: every error maps to one of the existing 23 codes. A run point checks a rule
when its triggering content or operation is present, as defined in 6.1.

| Rule | Severity | Scope | Run | F04 code | Slice |
|------|----------|-------|-----|----------|-------|
| DV-VER-001 | error | model | C H M | UNSUPPORTED_VERSION | now |
| DV-VER-002 | error | element | C H M | UNSUPPORTED_VERSION | now |
| DV-VER-003 | error | model | C H M | UNSUPPORTED_VERSION | now |
| DV-KIND-001 | error | element | C H M | INVALID_ARGUMENT | now |
| DV-KIND-002 | warning | model | H M | - | now |
| DV-KIND-003 | error | element | C H M | UNSUPPORTED_COMMAND | now |
| DV-KIND-004 | error | element | C H M | UNSUPPORTED_COMMAND | now |
| DV-KIND-005 | error | element | C H M | INVALID_ARGUMENT | now |
| DV-KIND-006 | error | model | C H M | INVALID_ARGUMENT | now |
| DV-BASE-001 | error | element | C H M | INVALID_ID | now |
| DV-BASE-002 | error | model | C H M | DUPLICATE_ID | now |
| DV-BASE-003 | error | element | C H M | INVALID_ARGUMENT | now |
| DV-BASE-004 | error | element | C H M | INVALID_ARGUMENT | now |
| DV-BASE-005 | error | element | C H M | INVALID_ARGUMENT | now |
| DV-BASE-006 | error | element | H M | NOT_FOUND | now |
| DV-ATTR-001 | error | element | C H M | INVALID_ARGUMENT | now |
| DV-ATTR-002 | error | element | C H M | INVALID_ARGUMENT | now |
| DV-ATTR-003 | error | element | C H M | NON_FINITE_NUMBER | now |
| DV-ATTR-004 | error | element | C H M | UNSUPPORTED_PROPERTY | now |
| DV-ATTR-005 | error | element | C H M | UNSUPPORTED_PROPERTY | now |
| DV-LIM-001 | error | element | C H M | LIMIT_EXCEEDED | now |
| DV-LIM-002 | error | element | C H M | INVALID_ARGUMENT | now |
| DV-LIM-003 | error | element | C H M | INVALID_ARGUMENT | now |
| DV-OWN-001 | error | element | C H M | FOREIGN_READ_ONLY | reserved |
| DV-OWN-002 | error | element | C H M | PERMISSION_DENIED | now |
| DV-OWN-003 | error | element | C H M | UNSUPPORTED_PROPERTY | now |
| DV-NS-001 | error | element | C H M | INVALID_ARGUMENT | now |
| DV-NS-002 | error | element | H M | INTEGRITY_ERROR | now |
| DV-REL-001 | error | relationship | C H M | DANGLING_RELATIONSHIP | now |
| DV-REL-002 | error | relationship | C H M | INVALID_ARGUMENT | now |
| DV-REL-003 | error | relationship | C H M | INVALID_ARGUMENT | reserved |
| DV-REL-004 | error | relationship | C H M | DEPENDENCY_CONFLICT | reserved |
| DV-REL-005 | error | model | C H M | DEPENDENCY_CONFLICT | reserved |
| DV-REL-006 | error | relationship | C H M | INVALID_ARGUMENT | now |
| DV-REL-007 | error | relationship | C H M | INVALID_ARGUMENT | now |
| DV-REL-008 | error | relationship | C H M | FOREIGN_READ_ONLY | reserved |
| DV-REL-009 | error | relationship | H M | DEPENDENCY_CONFLICT | now |
| DV-REL-010 | error | relationship | H M | DEPENDENCY_UNAVAILABLE | now |
| DV-REL-011 | error | relationship | C H M | DEPENDENCY_CONFLICT | now |
| DV-REL-012 | warning | relationship | H M | - | now |
| DV-REL-013 | error | relationship | C H M | INVALID_ARGUMENT | now |
| DV-REL-014 | error | relationship | C H M | FOREIGN_READ_ONLY | reserved |
| DV-PEN-001 | error | element | C H M | INVALID_ARGUMENT | reserved |
| DV-PEN-002 | error | element | C H M | PERMISSION_DENIED | reserved |
| DV-PEN-003 | error | element | C H M | DEPENDENCY_CONFLICT | reserved |
| DV-PEN-004 | error | element | C H M | DEPENDENCY_CONFLICT | reserved |
| DV-PEN-005 | error | element | C H M | FOREIGN_READ_ONLY | reserved |
| DV-PEN-006 | error | element | C H M | INVALID_ARGUMENT | reserved |
| DV-PEN-007 | error | element | C H M | DEPENDENCY_CONFLICT | reserved |
| DV-COL-001 | error | element | C H M | INVALID_ARGUMENT | reserved |
| DV-COL-002 | error | model | C H M | INVALID_ARGUMENT | reserved |
| DV-MOD-001 | error | element | C H M | FOREIGN_READ_ONLY | now |
| DV-MOD-002 | error | model | H M | INTEGRITY_ERROR | now |
| DV-MOD-003 | error | model | H M | INVALID_ARGUMENT | now |
| DV-MOD-004 | error | model | H M | INVALID_ARGUMENT | now |
| DV-MOD-005 | error | model | C H M | PERMISSION_DENIED | now |
| DV-MRG-001 | error | post_merge | M | DEPENDENCY_CONFLICT | now |

Rule text:

- **DV-VER-001** The content's vocabulary version (`bim:schemaVersion`) lies on a compatibility
  line the reader does not support. The content may be inspected and preserved, never mutated.
- **DV-VER-002** A mutation targets an element written by a newer version on the reader's own
  compatibility line. The element is read-only and every unknown attribute on it is preserved.
- **DV-VER-003** Capability negotiation has no selected version, or the selected version is not
  on the module minimum's compatibility line or is below its `requires_vocabulary` minimum.
  The module is enabled only when the selected version is on that line and at least that minimum;
  otherwise it is not enabled and its elements follow DV-MOD-001.
- **DV-KIND-001** An authoring request names an element kind that is not in the registry of
  section 2.
- **DV-KIND-002** Loaded content has an element whose kind is not in the registry. It is preserved
  as an opaque read-only element and listed; it is never dropped and never rejected on load.
- **DV-KIND-003** A kind is authored through a module that is not enabled in the project. A
  disabled module registers no commands.
- **DV-KIND-004** A reserved kind is authored. In vocabulary 0.1.0 no module declares a reserved
  kind as implemented, so the request is rejected even when the owning module is enabled.
- **DV-KIND-005** `bim:discipline` is not the owning discipline of the kind (for kinds owned by
  `any`: the discipline of the authoring module).
- **DV-KIND-006** An authoring request or module registration names a discipline or module token
  outside `steel`, `arch`, `mep`. This is an authoring check, not rejection of loaded content.
- **DV-BASE-001** `bim:id` is not a full canonical lowercase UUIDv7 with the RFC variant (F04
  section 2).
- **DV-BASE-002** Two placed elements in one model carry the same `bim:id`.
- **DV-BASE-003** `bim:name` is empty or contains a control character U+0000-001F or U+007F (F04
  section 3).
- **DV-BASE-004** `bim:lifecycle` is not `active` or `deactivated`. A new element is created
  `active`.
- **DV-BASE-005** A material reference is not a ContentDigest: 64 lowercase hexadecimal digits
  (F04 section 2).
- **DV-BASE-006** A well-formed material ContentDigest does not resolve to a pinned immutable
  definition.
- **DV-ATTR-001** A required attribute of the common base or of the kind is missing. No fallback
  value is supplied (F04 section 4).
- **DV-ATTR-002** An attribute value has the wrong type or unit. Numbers are JSON numbers, never
  strings; lengths are millimetres and angles radians.
- **DV-ATTR-003** A numeric attribute is not finite, or a nonzero literal overflows or underflows
  f64 (F04 section 3).
- **DV-ATTR-004** An authoring request sets an attribute that the vocabulary does not define.
  Loaded unknown attributes are preserved instead (DV-NS-002).
- **DV-ATTR-005** An attribute that the registry does not list for this kind is set on an element
  of that kind.
- **DV-LIM-001** A value exceeds an F04 section 3 cap: dimension or axis length above 1,000,000
  mm, coordinate above 1,000,000,000 mm, string above 4096 bytes, label above 256 bytes, or more
  than 1024 relationship targets.
- **DV-LIM-002** A dimension that must be greater than zero is zero or negative (F04 section 3).
- **DV-LIM-003** A placement quaternion has a component outside [-1, 1] or a squared norm farther
  than 1e-12 from one (F04 section 3).
- **DV-OWN-001** A module mutates an element whose `bim:discipline` is another enabled discipline.
  Reading is allowed; every write is rejected. The `penetration` kind is governed by DV-PEN-005.
- **DV-OWN-002** A module writes an attribute in a namespace it does not own, or an engine-
  reserved `bim:` attribute (identity, creator, revision).
- **DV-OWN-003** A patch changes `bim:discipline` or `bim:elementKind` of an existing element.
  Both are immutable.
- **DV-NS-001** An authored attribute name is not `<namespace>:<name>` with namespace and name
  matching the token syntax of section 4.
- **DV-NS-002** A rewrite of an element dropped or altered an attribute or relationship that the
  writer does not understand. Unknown content must survive byte-for-byte in meaning.
- **DV-REL-001** A live relationship target does not resolve to an element of the model.
  Only `penetrates` from a `rejected` or `withdrawn` penetration is historical: its unchanged
  target ElementId may outlive the host (section 5.4). Every other target must resolve.
- **DV-REL-002** An authoring request uses a relationship type outside the five of section 5.
- **DV-REL-003** The source or target kind is not allowed for the relationship type (section 5
  table). Historical `penetrates` targets are exempt from target-kind checks (section 5.4).
- **DV-REL-004** A cardinality bound of the relationship type is violated for a source or a
  target.
- **DV-REL-005** `part-of` or `hosted-in` records form a cycle of any length.
- **DV-REL-006** A relationship has the same element as source and target.
- **DV-REL-007** Two records have the same type, source, target and role.
- **DV-REL-008** A relationship record is authored or edited by a discipline other than the owner
  of its source element.
- **DV-REL-009** A delete targets an element that has an incoming live record. Only historical
  `penetrates` records from `rejected` or `withdrawn` penetrations are nonblocking; they remain
  with their sources and keep the host ID after deletion. Other incoming records still block.
  No cascade or cross-discipline record edit is implicit.
- **DV-REL-010** The incoming-relationship index needed to decide a delete is incomplete or
  unavailable.
- **DV-REL-011** A copy source has element-to-element relationships. The first copy supports none
  (F04 section 7).
- **DV-REL-012** A live relationship target is `deactivated`. The record stays valid; the target is
  listed.
- **DV-REL-013** An authored relationship role is present but does not match
  `[a-z][a-z0-9_]{0,31}`. A syntactically valid `references` role is accepted even when unknown.
- **DV-REL-014** An `opening` has a `hosted-in` target whose discipline differs from its own.
  An opening is host-owned and has no penetration request or acceptance step.
- **DV-PEN-001** A penetration state change is not an edge of the transition table of section 5.4.
- **DV-PEN-002** The actor's discipline is not the one entitled to the requested transition.
  Only the host discipline may accept; proposer rights alone never permit acceptance.
- **DV-PEN-003** The transition's expected `stateSeq` differs from the stored value (a concurrent
  change), or merge provenance contains competing transitions from the same sequence. Check at
  C, H and M; a valid-looking final state does not erase a sequence conflict.
- **DV-PEN-004** `accepted` is requested while the host element is missing or `deactivated`.
- **DV-PEN-005** The host discipline edits a request field, or the proposer edits a decision
  field.
- **DV-PEN-006** A transition to `rejected` has no reason code, or a code outside the closed list.
- **DV-PEN-007** A delete targets a penetration while its state is `accepted`. Reject without
  changing state; require a sequence-checked withdrawal by the proposer before deletion. At M,
  check the recorded operation order, including the pre-delete state, rather than absence alone.
- **DV-COL-001** An element names a collision class that no enabled module declares.
- **DV-COL-002** A module's collision-class declaration has a severity outside `hard`, `soft`,
  `clearance`.
- **DV-MOD-001** A mutation, including delete, targets an element of a module that is not enabled.
  The element is preserved and shown read-only.
- **DV-MOD-002** Disabling a module changed, hid-by-deletion or dropped an element or attribute of
  that module in the stored project.
- **DV-MOD-003** A module manifest declares a dependency on another module.
- **DV-MOD-004** A module manifest registers the shared namespace `bim`, a reserved token, or a
  namespace that another module already registered.
- **DV-MOD-005** A change to the project's enabled-module list lacks owner-level permission
  from trusted context. A discipline token or module ownership does not grant that permission.
  U02 defines storage and authoring; this rule specifies permission only, not an implementation.
- **DV-MRG-001** A merged result or its recorded operations violate a definite error-severity
  rule. The host re-runs all applicable catalogue rules and ignores any client-reported pass.
  Unavailable inputs alone do not prove a violation; they block acceptance as `incomplete` under
  section 6.3, including when a definite violation also raises DV-MRG-001.

### 6.6 Collision classes (declared only)

A discipline may declare named collision classes in its manifest: `<token>:collision:<name>` with
a severity `hard`, `soft` or `clearance`. This contract validates only that an element names a
declared class (`DV-COL-001`) and that a declaration has a valid severity (`DV-COL-002`). It
defines no geometry and no evaluation; clash checks are a later task on top of F06.

## 7. Discipline-module contract

### 7.1 What a module provides

A module is a unit of one application, not a separate program. It provides a **manifest**:

1. `module`: its token, equal to its discipline and namespace token (section 4.2).
2. `version` of the module, and `requires_vocabulary`: the lowest vocabulary version it needs
   (section 4.7). The selected version must be on this minimum's compatibility line and at least
   this minimum; an exact match in the advertised sets is not required (`DV-VER-003`).
3. `kinds_implemented`: kinds from the registry whose owner is its discipline (or `any`).
   Reserved kinds appear here only in a later vocabulary version (`DV-KIND-004`).
4. `commands`: the logical names of the commands it adds to the engine. Each follows F04: typed
   arguments, full IDs, derived footprint, no raw USD setter.
5. `rules`: discipline rules with IDs `<TOKEN>-<FAMILY>-<NNN>`. They map to F04 codes, use the
   envelope of section 6.3 and may add errors but never weaken or disable a `DV-` rule.
6. `collision_classes` (section 6.6).
7. `panels` and `tools`: logical identifiers and titles only. The contract fixes no widget, layout
   or rendering type.
8. `depends_on`: always empty (`DV-MOD-003`).

### 7.2 What a module may not do

A module may not depend on another module (`DV-MOD-003`); write outside its namespace
(`DV-OWN-002`); change an element of another discipline (`DV-OWN-001`); edit the shared registry or
register `bim` (`DV-MOD-004`); author a reserved kind (`DV-KIND-004`); add a persisted model
format, filesystem, process or network access; or accept raw USD paths or attributes in a command
(F04 section 7). The checks that apply to modules in code are the F08 boundary rules.

### 7.3 Dependency direction

`module -> engine -> model -> geom` (and `kernel` where F08 places it). Modules never depend on
each other. The shell (the application binary and later the UI) never depends on a module's
internals: it reaches a module only through the module interface that the engine defines, and
learns its panels and tools from the manifest. Concretely this needs a module crate above the
engine layer; section 10 proposes the F08 edit and flags that it is not applied.

### 7.4 Enabled and disabled

A module is **enabled** for a project when the project's module list includes it and the host has
it installed, and the selected vocabulary version is on the compatibility line of its minimum and
at least that minimum (`DV-VER-003`). U02 defines the authoring design and storage location of
the project's enabled-module list. Changing that list requires owner-level permission from trusted
context (`DV-MOD-005`); neither a discipline token nor module ownership grants it. This is a U02
contract pointer, not a storage design or implemented permission mechanism. A module that is
absent, not installed or refused by negotiation is **disabled**.

For a project that contains elements of a disabled module:

1. The elements, their attributes and relationships are **preserved** exactly (`DV-MOD-002`).
   Disabling never deletes, rewrites or hides-by-deleting anything.
2. They are **listed and shown read-only**. Their kind is still registered, or it falls under
   `DV-KIND-002`.
3. Any mutation, including `DeleteElement`, is rejected with `FOREIGN_READ_ONLY` (`DV-MOD-001`).
4. The module's commands, panels, tools and discipline rules are absent. The shared `DV-` rules
   still run on those elements, structurally.
5. Enabling the module again restores editing without migration if the versions remain compatible
   under sections 4.6 and 4.7.
6. Authoring a kind through a disabled module is `DV-KIND-003` (`UNSUPPORTED_COMMAND`).

### 7.5 Multiple modules at once

When more than one module is enabled, each owns its namespace and its elements. Cooperation goes
only through shared kinds and records: relationships (section 5) and the penetration request
(section 5.4). Neither module imports the other, calls its commands or reads its private attributes.

### 7.6 The first slice

Exactly one module is in scope: **steel**. Its manifest would declare the kinds `plate`, `beam`,
`column`; the F04 commands `CreatePlate` and `CreateStraightMember`; and no collision class, panel
or discipline rule. `UpdateElement`, `CopyElement` and `DeleteElement` are generic engine commands
that dispatch on the element's kind. The module and the registry are specification until their
tasks create code.

## 8. Where the rules apply

| Area | Sections | Rules |
|------|----------|-------|
| USD schema and adapter (U02) | 2 to 4: registry, base attributes, namespaces, preservation | `DV-NS-*`, `DV-VER-*`, `DV-KIND-*` |
| Identity (U03) | 3.1 identity and label | `DV-BASE-001` to `DV-BASE-004` |
| Profile and material (P02) | material reference, profile digest | `DV-BASE-005`, `DV-BASE-006` |
| Straight member (P03) | `beam`, `column` kinds, attributes, limits | `DV-ATTR-*`, `DV-LIM-*`, `DV-KIND-004` |
| Validation layer (P09) | modules and the validation report (6 and 7) | `DV-MOD-*` |
| Assemblies (A01) | `assembly`, `part-of`, `connects` | `DV-REL-003` to `DV-REL-005` |
| Collaboration (C04) | post-merge re-check, penetration conflicts | `DV-MRG-001`, `DV-PEN-003`, `DV-BASE-002` |
| IFC export (E01) | the registry is the set of exportable kinds | `DV-KIND-002`, `DV-NS-002` |

F05 and U02 define how the validation report and the negotiation travel; F06 owns geometry and any
collision evaluation.

## 9. Acceptance cases

[The case file](../../tests/fixtures/contracts/f10-domain-vocabulary-cases-v0.1.json) holds original acceptance descriptions. They
are proposed future tests, **not** a runnable engine, USD fixture or module mock.

There are 161 logical cases: 79 positive and 82 negative. 114 are
rule pairs: each of the 57 rules `DV-FAM-NNN` has a positive case `DVC-FAM-NNN-P` and a
negative case `DVC-FAM-NNN-N`. The other 47 are extra cases with IDs `DVC-X-<group>-<NN>`:

- valid steel elements (`DVC-X-STEEL-*`);
- reserved or disabled kinds rejected, and a project with foreign-module content preserved
  (`DVC-X-DISABLED-*`);
- unknown attribute preserved (`DVC-X-PRESERVE-01`);
- vocabulary version negotiation (`DVC-X-VERSION-*`);
- read-only foreign discipline (`DVC-X-READ-01`, and `DV-OWN-001`, `DV-MOD-001` pairs);
- cardinality, cycle, delete and copy for penetrations (`DVC-X-REL-*`);
- unavailable-only and mixed-error validation precedence (`DVC-X-VALIDATION-*`);
- the penetration lifecycle: eight allowed and eight forbidden transitions (`DVC-X-LIFE-A1`
  to `A8`, `F1` to `F8`), two conflict cases and a conflict-free merge (`DVC-X-CONFLICT-*`).

Of the cases, 101 are `first_slice: true` and 60 are `first_slice: false`, which
are specification only. A negative case lists the findings its rule raises with severity and F04
code; `primary_code` follows the unavailable-input precedence of sections 6.3 and 6.4. The case
file uses short logical keys (`kind`, `discipline`, `state`, `stateSeq`, `reason`, `attrs`);
they stand for the attributes named in sections 2 to 5 and are not USD names. Cases that name an
`f04_refs` entry
refer to the [F04 case file](../../tests/fixtures/contracts/f04-model-command-cases-v0.1.json) and agree with its code.

F04 cases referenced:

`F04-NEG-01`, `F04-NEG-03`, `F04-NEG-04`, `F04-NEG-07`, `F04-NEG-11`, `F04-NEG-12`, `F04-NEG-13`,
`F04-NEG-15`, `F04-NEG-19`, `F04-NEG-22`, `F04-POS-03`, `F04-POS-05`, `F04-POS-08`, `F04-RAW-02`.

## 10. Open questions

Tracked in [issue #143](https://github.com/Apibim-com/BeamCraft/issues/143).
