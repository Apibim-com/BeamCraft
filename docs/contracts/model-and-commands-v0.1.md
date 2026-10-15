# F04: model and command contract, version 0.1.0

Status: **freeze candidate, not frozen**. Date: 2026-10-14. Only the owner can freeze it
(see [section 11](#11-freeze-status)).
Draft base: `b34205281ab8a1f95052c4dc7d84f0995633e179`. Freeze-candidate amendments (task F04.2)
are made on `478f330e15819b574c62227bc23009729adb4fdc`; see [section 12](#12-amendments-log).
F01 is merged. F02 is available locally: 35 source files, 160 occurrences and 156 unique IDs;
source hashes were independently checked against its pinned manifest. F02 is not an input to this
contract's amendments. The F09 review result 0.1.0 and the first-scope handoff package 0.1.0 are
integrated (PR #10): see the [review result](../clean-room/review-result.md) and the
[package](../clean-room/handoff/first-scope-v0.1.md). Requirement admission covers only that
package's scope (rectangular plate, straight member, identities, units, rigid placement, declared
failures). Source-exposure admission and eligible implementers are separate and undecided.
K0 is not passed. This document does not close any of these.

This specifies domain semantics before F05 transport, F06 geometry and F08 module implementation.
No command described here is implemented by this PR. The current CLI still reports its version.
JSON examples are control messages and acceptance descriptions, never a second saved model format.

## 1. Authority and ownership

USD contains the authoritative authored parameters, placement, operations and relationships.
Published revisions also include corresponding derived geometry and quantities. Rust owns
engineering validation, commands, rights, dependency tracking and transactions; a USD adapter
handles composition and serialization. UI and CLI call the same engine API.

An owned element, immutable definition and foreign reference are different targets.
Mutation commands address owned elements by ID. The engine derives ownership from validated
composition provenance, not an `owner` string submitted by a caller. Unknown ownership is not
write permission. Edits and overrides of foreign prims are forbidden even in our own layers.
Own reference wrappers and measurement annotations have separate identities; they never make
their foreign source writable. R01 defines attachment commands after this contract.

## 2. Identity, revision and paths

| Type | Representation and invariant |
|------|------------------------------|
| ElementId, ProjectId, WorkspaceId, OpId | Full UUIDv7, canonical lowercase `8-4-4-4-12` form; validate version 7 and RFC variant |
| ActorId | Opaque identity from authenticated execution context; request claim must match that context |
| RevisionId | Full UUIDv7 identifying one immutable accepted project revision; never a shortened timestamp or array index |
| ContentDigest | SHA-256 of exact published bytes, all 64 lowercase hexadecimal digits |
| WorkspaceGeneration | Unsigned 64-bit counter; logical JSON form is a canonical decimal string, without leading zeros except `"0"` |
| FeatureId | Opaque stable identifier within a versioned geometry result; representation and loss rules belong to F06 |

UUID layout follows [RFC 9562 section 5.7](https://www.rfc-editor.org/rfc/rfc9562.html#section-5.7).
Creation checks actual uniqueness. UUID clock order gives neither authority nor conflict priority.
Same-millisecond elements must retain different complete IDs.

An owned steel element maps to `/World/Steel/E_<all 32 UUID hex digits without hyphens>`.
`bim:id` is the full hyphenated ID; `bim:name` is a label. Rename does not change ID or prim path.
Prim paths are adapter/index results, not caller-selected mutation destinations.
The initial namespace does not support arbitrary element reparenting.

A revision ID and a content digest have different roles. Each immutable revision has one
validated layer/asset manifest; reusing its ID for different content is an integrity error.
U06 fixes the root publication/CAS mechanism, and U05 verifies complete blob bytes.
Commands never accept shortened digests, absolute asset paths or remote resource URIs instead
of a ContentDigest. The host resolves authorized project-relative resources with containment checks.

## 3. Units, types and initial limits

Lengths and coordinates use finite f64 millimetres, rotations use radians where expressed as
angles, and the coordinate system is right-handed with Z up. Volume is m3, density kg/m3 and
mass kg. The USD adapter authors `metersPerUnit=0.001` and `upAxis=Z` explicitly.
[USD stage linear units](https://openusd.org/release/api/group___usd_geom_linear_units__group.html)
are an IO concern; standard USD angular operations use their documented units at that boundary.

Logical messages follow [RFC 8259](https://www.rfc-editor.org/rfc/rfc8259.html): reject duplicate
object keys, invalid UTF-8 and non-JSON NaN/Infinity literals. Numeric fields accept numbers only,
not booleans or numeric strings. Reject f64 overflow/underflow that changes a nonzero submitted
number to infinity/zero. Do not silently round or clamp engineering inputs.
Ordinary nearest-f64 conversion is the numeric representation; no extra dimension rounding is allowed.
Positive and negative zero are equivalent for signatures; normalize only their sign for that purpose.

These are original initial operating caps, not vendor tolerances or promises of kernel accuracy:

| Input | Initial constraint |
|-------|--------------------|
| Encoded logical command | At most 1 MiB before parsing; nesting depth at most 16 |
| Single string | At most 4096 UTF-8 bytes unless a narrower field cap applies |
| Element label | 1–256 UTF-8 bytes; reject control characters U+0000–001F and U+007F |
| Plate dimensions / supported member-axis length | Greater than zero and at most 1,000,000 mm |
| Coordinates / translation components | Absolute value at most 1,000,000,000 mm |
| Material density, when authored in a definition | Greater than zero and at most 1,000,000 kg/m3; absence is unknown |
| Element relationship targets | At most 1024; require valid pinned targets and reject forbidden cycles |
| Derived command footprint entries | At most 4096; fail before publishing if the footprint cannot be bounded |

F05 must enforce input byte/depth/string bounds before unsafe allocations and cannot widen
these limits silently. F06 fixes mesh/topology bounds and tolerance policy separately.
Host limits for stage traversal and immutable assets are additional, not replaced by this table.

Placement is a rigid pose: `translation_mm` (three numbers) and `orientation_xyzw`
(four quaternion numbers). Components of orientation are finite in [-1, 1]; squared norm must
be within 1e-12 of one. F06 must validate and construct a proper rigid transform, normalize that
accepted quaternion and define deterministic antipodal/sign handling for geometry signatures.
The F09 review integrated the rule for the first scope (CR-PLACE-001). It remains a proposed
rule until the owner freezes this contract (section 11); F06 still owns rigid-transform
construction, deterministic sign handling and tolerance policy (G-06). Scale, shear, reflection and
commercial position presets are unsupported in the initial mutation API. Generic foreign
transforms remain read-only and use a separate measurement-support decision.

**Quaternion convention (G-04, adopted; matches CR-PLACE-001 of the first-scope package).**

- Component order is `(x, y, z, w)`: `q = x*i + y*j + z*k + w`, with Hamilton's rule
  `i*j = k`. `[0, 0, 0, 1]` is the identity.
- The rotation is **active** and acts on element-local coordinates. For the normalized
  quaternion `q`, a local point `v` becomes `R(q) * v`, where `R(q)` is the proper rotation
  matrix of `v' = q v q^-1`. For a unit axis `u` and angle `a` the quaternion is
  `(u*sin(a/2), cos(a/2))`, a counter-clockwise turn seen from the tip of `u` in a
  right-handed frame.
- The pose maps a local point to the world as `world = R(q) * local + translation_mm`: rotate
  first, then translate. Rotation is about the local origin. Composition with any parent
  transform is outside this contract.
- The accepted quaternion is normalized (`q / |q|`) before use. The authored components are
  stored as submitted; this contract does not rewrite them.
- `q` and `-q` denote the same rotation and are both accepted. A canonical sign for
  signatures is not defined here; it stays with F06 (and P04).
- The component range and the 1e-12 squared-norm rule above are unchanged. This contract does
  not fix the f64 evaluation order of the squared norm, so no case sits exactly on that limit;
  the cases keep a clear margin on both sides.

Worked example (exact; checked with rational arithmetic): `q = (0, 0, 1/sqrt(2), 1/sqrt(2))`
is a +90 degree turn about +Z and maps the local point `(3000, 0, 0)` to `(0, 3000, 0)`. With
`translation_mm = [100, 200, 300]` the world point is `(100, 3200, 300)`. See
`F04-POS-10` and first-scope case `FS-POSE-A`. With the f64 literal `0.7071067811865476` for both
components, the squared norm differs from one by about 2.2e-16, inside the limit.

**Degenerate member axis (G-03, proposed for owner adoption).** The two `axis_points_mm` points
must give a length greater than zero. Identical points (distance exactly zero) are rejected with
`INVALID_ARGUMENT`; a length above 1,000,000 mm is `LIMIT_EXCEEDED`; a non-finite coordinate is
`NON_FINITE_NUMBER` or `MALFORMED_COMMAND` as for any number. Reasons: a zero axis is a value
that violates the "greater than zero" range of an authored field, exactly like zero plate
thickness (`F04-NEG-04`, `INVALID_ARGUMENT`) and a negative thickness (`F04-NEG-14`).
`UNSUPPORTED_GEOMETRY` means a well-formed shape or transform outside the supported scope
(for example a curved member or a scaled transform); a zero-length straight axis is not a
well-formed supported shape that merely lacks a feature. This keeps every "dimension must be
positive" rejection under one code. This is the only error-code choice changed by the
freeze-candidate amendments. Whether a nonzero but very short axis is acceptable is a minimum-size
policy, not decided here (gap G-06, F06).

## 4. Authored model fields

| Field / USD mapping | Required semantics |
|---------------------|--------------------|
| `bim:id` | Immutable owned element identity |
| `bim:name` | Explicit caller label; no environment-dependent hidden name |
| `bim:elementKind` | `plate`, `beam` or `column` for the first slice; independent of USD model kind |
| `bim:geometryMode` | `parametric` for initial create/update operations; preserve other loaded modes and report unsupported edits |
| `bim:schemaVersion` | Explicit `0.1.0` on owned elements; project/schema package also declare compatibility |
| `bim:createdBy` | Creation actor from trusted context; does not authorize future edits |
| `bim:revision` | Last accepted authored revision, absent on a never-published draft element; workspace generation tracks draft edits separately |
| Plate width / length / thickness | Explicit double `steel:plate:widthMm`, `lengthMm`, `thicknessMm`; no schema fallback can supply missing input |
| Straight member axis | Explicit two `bim:axisPointsMm` points in element-local coordinates |
| Profile / material | `bim:profile` / `bim:material` relationships resolve to pinned immutable definitions |
| Placement | Standard USD Xform operations on the owned element root, in authored content layers |
| Engineering operations | Explicit owned operation records and relationships; initial create has no cuts/holes/welds |

Immutable profile/material definitions carry their full content digest and explicit units/schema
version. A member's profile is required; material is optional. Plate shape does not require a
profile definition. Missing material/density never selects a guessed catalogue grade or density.
Physical material and display shader are separate references and signatures.

Profile lookup by commercial catalogue name, inferred environment defaults, position presets,
curved members, direct-shape authoring and fabrication operations are outside this first slice.
Do not implement their behavior from unapproved source notes. Preserve unknown USD namespaces,
attributes and relationships when editing supported fields. Unsupported schema versions may be
inspected/preserved, but cannot be mutated without an explicit reviewed migration.

## 5. Derived results and signatures

Each quantity has its own record: value (if any), unit, status, input signature and algorithm
version. Status is `current`, `stale`, `unknown` or `unsupported`.
`current` requires a supported computation over matching authored inputs and dependencies.
`stale` may retain a clearly marked previous value; consumers cannot treat it as current.
`unknown`/`unsupported` have no calculated value. A declared external value is a separate record.
No bounding-box volume, approximate mesh or missing density is promoted to exact mass/volume.

| Change | Dependencies and invalidation |
|--------|-------------------------------|
| Rename | Label only; retain shape and engineering quantities |
| Placement | Retain local immutable shape; invalidate world bounds/measurements |
| Plate dimension, axis, profile or supported operation | Invalidate shape and its dependent quantities/features |
| Material/density | Retain shape; invalidate mass and material-dependent appearance |
| Camera / selection | No authored element or engineering-signature mutation |

F06/P04 define canonical numeric encoding and algorithm-version signatures. A blob digest
identifies saved bytes; it is not the signature of normalized engineering inputs.
No arbitrary dimension rounding is allowed to manufacture shared hashes.

For the original synthetic example, 1000 x 500 x 10 mm gives volume 0.005 m3.
If an explicitly authored synthetic material has density 1000 kg/m3, mass is 5 kg.
These are analytic project examples, not catalogue records or a shipped USD fixture.
F07/U02 must provide complete original fixtures and real resources before runtime acceptance.

## 6. Logical mutation envelope

The envelope is shared by the future engine API, CLI and host. It does not fix F05 framing.

| Field | Rule |
|-------|------|
| `contract_version` | Exactly `0.1.0`; unsupported version is diagnostic, not best-effort parsing |
| `project_id`, `workspace_id` | Full IDs selecting an authorized project and owned draft |
| `op_id` | Full UUIDv7 for one atomic logical command |
| `actor_id` | Must match trusted caller context; never a credential |
| `base_revision` | Revision pinned by the selected workspace |
| `expected_generation` | Exact decimal WorkspaceGeneration precondition |
| `command` | Closed command name from the table below |
| `arguments` | Typed command-specific object; unknown envelope/argument fields are rejected |

Draft execution compares `base_revision` with the workspace base, not automatically with the
latest published head. This permits offline drafts. Publication additionally checks the current
accepted head. A changed head requires explicit rebase/conflict handling, never silent draft loss.

Within a workspace, retrying an accepted op_id with identical canonical request content returns
the recorded result without another generation increment. Different content under that op_id
is `OP_ID_REUSE`. Authentication/authorization still apply to a replay. Canonical request content
includes project/workspace/actor/base/generation, command and all arguments; exclude transport
metadata. The byte/number canonicalization profile must be fixed in F05 before implementation.
C01 additionally defines project-wide publication idempotency; this draft rule is not its proof.

## 7. Command surface

| Command / arguments | Atomic behavior | Planned feature owner |
|---------------------|-----------------|-----------------------|
| `CreatePlate`: new_id, name, width_mm, length_mm, thickness_mm, placement, optional material_digest | Validate explicit dimensions and unused ID; author one parametric owned plate | P01 |
| `CreateStraightMember`: new_id, element_kind, name, axis_points_mm, profile_digest, placement, optional material_digest | Validate nonzero supported local axis and pinned profile; beam/column is an explicit kind | P03 |
| `UpdateElement`: target_id, changes | Typed patch of supported name/dimensions/axis/profile_digest/material_digest/placement; no raw USD attribute/path setter | P04/P08 |
| `CopyElement`: source_id, new_id, name | Create an independent owned root with a fresh ID; share immutable definitions/shape until an edit | P05 |
| `DeleteElement`: target_id | Reject unresolved incoming dependencies; no implicit cascade or foreign-source mutation | P08 |

`changes` must be nonempty, every property must be supported by that kind, and all requested
changes form one transaction. `material_digest=null` explicitly clears a material reference;
absence from a patch leaves the material unchanged. Clearing a required profile is invalid.
No caller-supplied `bim:id`, generated path, derived mesh, quantity, actor, revision or numbering
approval can be patched through this API. A diagnostic never applies the valid half of a patch.

The first `CopyElement` takes one source. Internal self/owned operation-record relationships remap
to fresh destination entity IDs; shared external immutable references remain shared. Local
geometry FeatureIds remain scoped to their immutable result and are addressed through the new
ElementId. Relationships
outside its supported copy set cause a diagnostic rather than an incomplete copy. Copying a
foreign source, inventing a density or retaining a live reference to the mutable source is forbidden.
Multi-element/assembly copy and feature-ID remap rules need later reviewed extensions.

Read API `InspectElement` returns a revision/generation-stamped view, ownership, explicit inputs,
relationships and per-quantity status. It never reconstructs missing engineering input silently.
Preview validates/calculates without publishing or advancing a draft generation; it is not an
accepted op_id journal entry. `CommitWorkspace` is a separate host operation under U06/U07;
it publishes a complete USD revision and does not turn one command into a separate saved model.

## 8. Footprints, transactions and errors

The engine derives read/write sets from command semantics and validated dependencies. Caller
claims cannot narrow them. Each entry identifies an owned ID or immutable digest plus a semantic
group: label, shape_inputs, placement, material_ref, relationship, lifecycle or derived signature.

| Operation | Minimum semantic footprint |
|-----------|----------------------------|
| Create | Read ID absence and pinned definitions; write new lifecycle, complete authored fields and relationships |
| Update dimensions/axis/profile | Read complete shape-input group and relevant immutable definitions; write that group and invalidate dependent results |
| Update material | Read geometry-result signature and new material definition; write material_ref and mass/appearance invalidation |
| Update placement | Read local shape and placement; write placement and world-measurement invalidation |
| Copy | Read source lifecycle, full supported authored state, copy-set relationships and definitions; write new lifecycle and remapped owned state |
| Delete | Read lifecycle and complete incoming-relationship index at the expected generation; write lifecycle only if all dependencies permit deletion |

Different property names alone do not prove independence. A later cut/bolt command must declare
its dependencies on related element shape/profile/placement. If an index or required payload is
unavailable, return `DEPENDENCY_UNAVAILABLE`; do not assume no incoming links or a current result.

Validation order: bounded decode/version/types -> trusted context and authorized workspace ->
existing op_id replay -> for new operations, workspace/base/generation -> ownership/ID/relationships ->
complete semantic footprint -> engineering
validation -> apply to isolated draft -> validate resulting composition -> record draft/journal.
All state checks and commit occur under the host's workspace transaction serialization mechanism.
U06 also serializes publication with expected-head verification; plain read-then-rename is not CAS.

An accepted mutation advances generation once and constitutes one future undo unit, regardless
of changed-property count. Rejection leaves authored state, derived state and generation unchanged.
P07 defines durable undo/redo and restored identity/relationships; no fake undo is introduced here.
Cancellation before the apply/commit point discards work. After that point, report the recorded
outcome or `OUTCOME_UNKNOWN`; a timeout is not proof of rollback. U07/F05 define recovery/query.

| Stable code | Meaning |
|-------------|---------|
| `MALFORMED_COMMAND` | Invalid encoding/JSON, duplicate keys or structurally invalid envelope |
| `UNSUPPORTED_VERSION`, `UNSUPPORTED_COMMAND`, `UNSUPPORTED_PROPERTY` | Version/operation/property outside the accepted contract |
| `INVALID_ID`, `DUPLICATE_ID`, `NOT_FOUND` | Invalid full identity, already-owned creation ID, or missing target/definition |
| `INVALID_ARGUMENT`, `NON_FINITE_NUMBER`, `LIMIT_EXCEEDED` | Bad field/type/range, non-finite f64 or bounded-input cap violation |
| `PERMISSION_DENIED`, `FOREIGN_READ_ONLY` | Untrusted/mismatched caller or attempted foreign edit/override |
| `BASE_REVISION_MISMATCH`, `STALE_WORKSPACE`, `OP_ID_REUSE` | Failed revision/generation precondition or changed request under accepted op_id |
| `DANGLING_RELATIONSHIP`, `DEPENDENCY_CONFLICT`, `DEPENDENCY_UNAVAILABLE` | Invalid link, forbidden delete/change, or incomplete validation inputs |
| `UNSUPPORTED_GEOMETRY`, `INTEGRITY_ERROR` | Shape/transform outside scope, or revision/blob bytes inconsistent with their identity |
| `CANCELLED`, `OUTCOME_UNKNOWN`, `BACKEND_UNAVAILABLE` | Cancelled before commit, uncertain committed outcome, or required backend unavailable |

A zero-length member axis returns `INVALID_ARGUMENT` (G-03, proposed for owner adoption; see
[section 3](#3-units-types-and-initial-limits)).

Errors return code, safe message, field/target diagnostics and the observed workspace token when
known. Never include credentials, private paths or client data. Multiple failures follow this
validation order; runtime tests must not rely on whichever untrusted collection entry is visited first.

## 9. F02 gaps resolved for this contract

- No automatic environment/default/catalogue resolution in the first slice. Explicit pinned
  definitions are required; environment resolver scope belongs to later packages.
- Initial member placement is explicit rigid pose plus local axis. Commercial position presets
  and unverified position cells are unsupported, not inferred.
- Quantities use geometric volume and explicit density; vendor gross/net tables and rounding are
  outside this contract. Unknown density remains unknown.
- Foreign-source annotations are separate owned result objects, never edited foreign UDAs.
- One accepted command is one atomic local undo unit; cancellation/rollback do not leave half a patch.
- Numbering/equivalence is not specified by F04. Placement-sensitive fabrication cases, tolerance
  and approved-number invalidation must be resolved by A08/A09 before numbering implementation.

## 10. Acceptance handoff and provenance

[Original command cases](model-command-cases-v0.1.json) define positive and negative acceptance
descriptions. They are not a runnable model, geometry fixture, engine mock or evidence of a
working command dispatcher. No placeholder blob is offered as an actual saved resource.
Case IDs are proposed future tests; acceptance requires real API -> CLI -> USD -> independent reader.
There are 39 logical cases (thirteen positive, twenty-six negative) and six malformed/non-finite
raw JSON cases. `F04-POS-01` to `F04-POS-07`, `F04-NEG-01` to `F04-NEG-18` and `F04-RAW-01` to
`F04-RAW-03` are the original draft cases and keep their IDs and meaning; `F04-POS-08` to
`F04-POS-13`, `F04-NEG-19` to `F04-NEG-26` and `F04-RAW-04` to `F04-RAW-06` were added by the
amendments in [section 12](#12-amendments-log). The member cases that need a profile
(`F04-POS-07`, `F04-POS-10`, `F04-POS-13`, `F04-NEG-20`, `F04-NEG-21`) require the harness to bind an
actual validated original profile digest; they do not provide a fake resource or an executable fixture.

**Test comparison rule (G-05, adopted).** Expected numeric values in the case file are compared
with the rule CR-TOL-001 of [first-scope package 0.1.0](../clean-room/handoff/first-scope-v0.1.md):
a quantity passes when `|actual - expected| <= 1e-12 * |expected|`; a coordinate passes when
`|actual - expected| <= 1e-9 mm + 1e-12 * |expected|`. Identifiers, strings, booleans, integers,
generations and error codes compare exactly. The basis is the binary64 unit roundoff 2^-53
(IEEE Std 754-2019, binary64) with a margin of over three orders
of magnitude, derived in the package. **This is a rule for comparing test results only. It is not a
geometric tolerance policy**: it does not decide whether two points coincide, how short an axis
may be or how a kernel snaps geometry; that is F06 and open gap G-06. The rule is also stated in
the `comparison` field of the case file. The [F07 expectations](../fixtures/expectations.md) use
their own, separate analytical-comparison tolerances for the F07 fixture format; they do not
replace this rule.

**Coverage and cross-references (G-05).** New cases refer to the first-scope cases by ID through
`first_scope_refs` (package version 0.1.0, field `first_scope_package`) where a corresponding
first-scope case exists; the expected codes agree with those cases except for the one G-03 choice.
`F04-POS-12` and `F04-RAW-04` to `F04-RAW-06` are F04-only cases with no first-scope
counterpart and carry no `first_scope_refs`.

| Topic | F04 cases | First-scope cases |
|-------|-----------|-------------------|
| Degenerate (zero-length) axis | `F04-NEG-20` | `FS-NEG-AXIS-ZERO` |
| Axis and plate dimension at the limit, accepted | `F04-POS-08`, `F04-POS-13` | `FS-PLATE-C`, `FS-MEMBER-B` |
| Axis and plate dimension just above the limit, rejected | `F04-NEG-19`, `F04-NEG-21` (coarser: `F04-NEG-05`) | `FS-NEG-PLATE-BOUND`, `FS-NEG-AXIS-BOUND` |
| Translation at the limit, accepted / just above, rejected | `F04-POS-09` / `F04-NEG-26` | `FS-POSE-E` / `FS-NEG-TRANSLATION-BOUND` |
| Invalid quaternion: norm, component range, zero, just outside tolerance | `F04-NEG-22` to `F04-NEG-25` | `FS-NEG-QUAT` |
| Non-finite quaternion or translation component | `F04-RAW-04`, `F04-RAW-05`, `F04-RAW-06` | none (F04-only cases; they follow the parsing pattern of `F04-RAW-01` and `F04-RAW-02`) |
| Rotation convention and rotate-then-translate | `F04-POS-10` | `FS-POSE-A` |
| Quaternion norm slightly off one, accepted and normalized | `F04-POS-11` | `FS-POSE-F` |
| `q` and `-q` | `F04-POS-12` | none (F04-only case) |

Non-finite quaternion and translation components follow the existing raw-JSON rules: a bare
`NaN` is `MALFORMED_COMMAND` and an overflowing literal such as `1e400` is `NON_FINITE_NUMBER`.

U03 must reject duplicate/short IDs and preserve identity through rename. P01/P03 must round-trip
explicit dimensions/local axis/placement. P04 must distinguish shape/mass/world invalidation and
unknown density. P05 must preserve the source through copy/edit/reopen. P08 must reject dangling
delete and partial patches. U06/U07 must prove complete publication, retries and crash recovery.
F05/F06 must add transport/geometry negative cases under these invariants, not weaken them.

Sources: original owner decisions in local architecture-proposal.md sections 1, 3–7, 9–14 and 17;
implementation-plan.md revision 1.1 sections 1–4, 7.1 and 12; local F02 gap analysis; public RFC/IO
references linked above. No private source specification, manual, catalogue, UI label, screenshot
or third-party implementation is published by this document. Constants and command choices above
are proposed original BeamCraft rules, not commercial-product behavior.

Freeze conditions are listed in [section 11](#11-freeze-status). F05/F06 fix their implementation
details within these logical invariants; they are follow-up tasks, not circular prerequisites for
F04 acceptance. F04 review does not permit geometry implementers to inherit source-exposed
contexts or declare unsupported features implemented. The owner permits TOST-1.0 only for official OpenUSD and its
SDK resources; the scoped policy is recorded in [policy PR #7](https://github.com/Apibim-com/BeamCraft/pull/7).
SDK integration still requires provenance, complete notices, native dependency closure and runtime
checks. The remaining dependency, asset and unsafe-code policies continue to apply.

## 11. Freeze status

This contract is a **freeze candidate, not frozen**. Nothing in it is implemented, K0 is not
passed, and no geometry implementer is declared eligible. Freezing requires the owner's explicit
acceptance; no review, task or merge freezes it implicitly.

Adopted by the freeze-candidate amendments (F09 review findings):

| Finding | Resolution | Where |
|---------|------------|-------|
| G-03 zero-length axis code | `INVALID_ARGUMENT`; **proposed for owner adoption** | [section 3](#3-units-types-and-initial-limits), `F04-NEG-20` |
| G-04 quaternion convention | `(x, y, z, w)`, Hamilton, active rotation, `world = R*local + translation`, normalized before use, `q` and `-q` equal | [section 3](#3-units-types-and-initial-limits) |
| G-05 comparison rule and cases | CR-TOL-001 as a test comparison rule only; 14 logical and three raw cases added | [section 10](#10-acceptance-handoff-and-provenance) |

Still open (not resolved by this document):

| Gap | Question | Owner and next step |
|-----|----------|---------------------|
| G-01 | The plate's local frame (which local axes carry width, length and thickness, and the origin) is undefined | Owner: decide, or delegate to F06 with a deadline |
| G-02 | The member section's rotation about the axis, its offset and the end-face definition, and the profile-definition resource format, are undefined | Owner: decide, or delegate to F06 with a deadline |
| G-06 | No minimum nonzero dimension or geometric tolerance policy | F06 owns it |

Until G-01 and G-02 are resolved only frame-independent quantities are admitted: volume, area and
mass for plates, and axis length and volume for members. No case or text here defines local bounds.

Freeze conditions: the owner accepts G-03 to G-05 as written or replaces them; the owner decides
or delegates G-01 and G-02; F06 accepts ownership of G-06; the case file and these amendments
pass review. The `contract_version` stays `0.1.0` until the owner freezes it.

## 12. Amendments log

- 2026-10-14, F04.2: status changed to freeze candidate; stale F09 statements updated to the
  integrated review 0.1.0 and first-scope package 0.1.0 (PR #10).
- 2026-10-14, F04.2: G-04 quaternion convention stated in section 3, consistent with CR-PLACE-001.
- 2026-10-14, F04.2: G-03 zero-length axis mapped to `INVALID_ARGUMENT` (proposed for owner
  adoption); recorded in sections 3 and 8. No new error code was added.
- 2026-10-14, F04.2: G-05 comparison rule adopted (section 10); cases `F04-POS-08` to
  `F04-POS-13`, `F04-NEG-19` to `F04-NEG-26` and `F04-RAW-04` to `F04-RAW-06` added with
  first-scope cross-references. Existing case IDs and meanings are unchanged.
- 2026-10-14, F04.2: sections 11 and 12 added. Caps, identity rules, envelope, command surface
  and the other error-code semantics are unchanged. `contract_version` remains `0.1.0`.
