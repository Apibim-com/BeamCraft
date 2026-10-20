# UPI: USD process integration addendum, version 0.1.0

Status: **draft 0.1.0; documentation only; nothing implemented**. Date: 2026-10-16.
Task: DR. Base: `bca89e23671470ba0633497b82d38a83fb5cc11d`.

This addendum records four owner decisions of 2026-10-16 and turns six owner-reviewed
integration points into numbered requirements (UPI-nnn) with owner tasks and testable
acceptance. It amends no frozen contract. Changes that F04, F05, F06 or F10 would need are
listed only in the PR body, as proposals. Where this text says "the USD process" it means the
separate process that hosts the pinned Rust layer engine (F11 variant C, `openusd` behind the
`beamcraft-layer-engine` wrapper). "The application" means the desktop process that holds the
session model, the engine and the in-process kernel.

Contract references: [F04](../contracts/model-and-commands-v0.1.md),
[F05](../contracts/worker-protocol-v0.1.md), [F06](../contracts/geometry-contract-v0.1.md),
[F08](../contracts/module-boundaries-v0.1.md),
[F10](../contracts/domain-vocabulary-v0.1.md),
[F11 decision](layer-engine-decision-v0.1.md),
[F11 wrapper requirements](layer-engine-wrapper-requirements-v0.1.md) (LEW-nn),
[U01 placement record](u01-placement-containment.md).

## 1. Owner decisions recorded (2026-10-16)

| ID | Decision |
|----|----------|
| OD-1 | The USD layer engine runs in a **separate process**; the F05 transport applies. Parsing untrusted USD is the isolated part. The application keeps the session model; the in-process `beamcraft-kernel` builds meshes. Large derived data (meshes, assets) are shared as immutable content-addressed files read directly by digest, not streamed through IPC. IPC carries control messages, bounded pages of authored properties and bounded responses. Binary attachments are separate from JSON controls (F05 section 3). |
| OD-2 | Holes and cuts are our **own implementation**. Through holes in steel parts are **circular or oval (obround) only**. Other shapes (square, rectangular, polygonal, arbitrary) are explicitly unsupported; users cut such shapes by other means. Third-party kernels (truck, solidcraft) are not used for them. |
| OD-3 | Member roll uses `f64::sin` and `f64::cos`. The open question from G01 is closed. The isolated function is kept. |
| OD-4 | Evidence policy for spikes and tasks: the repository holds the report, the decision and a minimal reproducible harness. Bulky raw outputs, logs and large generated files stay local (archived outside the repo) and are referenced by command or hash, never committed. |

OD-1 selects the topology that F11 section 9 left open ("Execution question and F05
applicability"). The in-process alternative is not selected; its semantic mapping in the F11
cards is retained only as history.

## 2. Gap check against the frozen contracts and F11/U01

Verdicts: **covered**, **partly**, **not found**. Quotes are exact; "not found" means a search of
the listed documents for the concept returned nothing.

| # | Point | Verdict | What exists (document, section, clause) | What is missing |
|---|-------|---------|------------------------------------------|-----------------|
| 1 | Property provenance | partly | F11 s.9 U01 outputs: "bounded typed snapshot pages with composition provenance". U01 record: results contain "all contributing prim sites with source digests and host-selected ownership". LEW-01: "require an owned edit target". F04 s.1: "The engine derives ownership from validated composition provenance". F05 s.5 defers: "U02/F08 must fix closed schema-specific batch and snapshot item fields". | Not found: a per-property `layer_id`, an `editable` flag, strength of the winning opinion, or the layer a delta must write into, delivered to the application. Provenance is per prim site, not per property. |
| 2 | Delta knows its base | partly | F04 s.6: `base_revision` is the "Revision pinned by the selected workspace". F05 s.7: "Mutations also carry F04 base_revision and expected_generation inside logical_command"; "A matching stage token with wrong base returns BASE_REVISION_MISMATCH". F04 s.8: "U06 also serializes publication with expected-head verification; plain read-then-rename is not CAS." F05 s.7: the worker "cannot ... publish a root". | Not found: a rule that the USD process applies a delta only when the current root equals `base_revision` at apply time; one delta format named for save, commit and a future live operation (F05 s.1 puts "remote or multi-client operation" out of scope). |
| 3 | Responses are untrusted | partly | F05 s.3: "Parsing uses bounded streaming validation (depth, decoded string sizes and token count)"; attachment SHA-256 is checked, but "does not authenticate a hostile peer or replace structural validation". F05 s.5/s.10: at most 4096 items per page. F05 s.9: "The host validates worker diagnostics before forwarding them". F06 s.6: validator checks counts against caps and rejects with `GEO_BLOB_INVALID`. Architecture: "Retrieval, presence and verification check the complete SHA-256" (U05). | Not found: an application-side validation rule for snapshot pages as hostile output (element count per stage, path length, allowed values); path length appears only as the generic `string_bytes` 4096. Not found: verifying, on read, the digest of a blob that the USD process reports from a file. |
| 4 | Geometry cache key by inputs | partly | F06 s.7: shape key is a SHA-256 of ordered inputs ending in "four u16 big-endian: `algorithm`, `tolerance_policy`, `feature_id_rule`, `mesh_layout`". F06 s.6: "The digest identifies bytes; it is not the shape key". F06 s.7 matrix: "Algorithm or tolerance-policy version change" invalidates shape and world keys. | Not found: a rule that a geometry bug fix must change the version in the key (`algorithm` is a contract constant, bumped only by amendment). Not found: cuts in the key (architecture "Part representations": "F04/F06 0.1.0 have no operations"). |
| 5 | Unknown data in USD | partly | F04 s.4: "Preserve unknown USD namespaces, attributes and relationships when editing supported fields." F10 s.4.4: unknown content "survives with its meaning unchanged" (DV-NS-002). F05 s.2: unknown metadata "is preserved as required by F04". LEW-05: "retain unknown opinions, metadata, relationships and layer structure". Architecture: "Reference updates ... preserve the original source bytes." | Not found: the mechanism rule "saving writes only the delta and never rebuilds a layer from the application's model". LEW-05 "Save only validated owned authored layers from an isolated candidate" does not forbid whole-layer serialization. |
| 6 | USD process crash | partly | F05 s.2: at most two automatic restarts, delays 250 ms then 1000 ms; "Do not automatically resend a mutation." F05 s.7: "Restart invalidates all stage tokens, cursors, pending correlations and events"; "Missing/uncertain recovery keeps the workspace blocked for new mutations with OUTCOME_UNKNOWN". F05 s.8: loss gives `BACKEND_UNAVAILABLE`, uncertain mutation stays `OUTCOME_UNKNOWN`. Architecture: personal workspaces "preserve unpublished authored edits"; "Recovery must expose either the previous or the new complete revision." | Not found: what the user sees during open or save failure; the statement that the unsent delta stays in the application; the retry rule for an interrupted open. |

Contradictions found with OD-1 to OD-3 (handled in section 8): F11 decision sections 1 and 9
call the topology open; F08 layer-engine row says "pending owner review"; wrapper requirements
and the architecture also call the topology open (outside this task's allowlist). The roll
question appeared as "OPEN owner decision" in the G01 math-core PR description (PR #38) and as an unresolved math
choice in `g01-math-crates-evaluation-v0.1.md` and `tolerance-comparison-v0.1.md` (also outside
the allowlist). No document in this task's allowlist names a hole shape other than circular or
oval; `kernel-design-v0.1.md` lists holes and cuts only as `unsupported` and needs no edit.

## 3. UI data path (state description)

```
 files on disk --> USD process (parse, compose, commit)        untrusted input
                       |  control messages, bounded property pages,
                       |  digests of derived blobs (F05 pipes)
                       v
 APPLICATION   validates every page (UPI-008) --> session model
   session model --> engine (F04 commands) --> kernel (in process) --> meshes
   meshes + derived blobs: content-addressed store, read by digest
   viewport / picking / measurements: session model + kernel only
```

1. **Open.** The application starts the USD process (F05 section 2) and sends `OpenStage`, then
   `ReadSnapshot` pages. Each page is validated (UPI-008) before it changes the session model.
   A page that fails validation closes the session; no partial model is shown.
2. **Session model.** The application owns it: elements, parameters, relationships, per-property
   provenance (UPI-006), pinned context. It is rebuilt from validated pages only.
3. **Geometry.** The in-process kernel builds meshes from session parameters. A mesh is cached
   under its input key (UPI-009). The USD process never draws, picks or measures.
4. **Derived blobs.** Meshes and assets that already exist as files are immutable and
   content-addressed. The application reads them from the U05 store by digest and verifies the
   full SHA-256 on read (UPI-008). IPC pages carry the digest, not the bytes.
5. **Edit.** UI and CLI call the engine API. The engine validates and updates the session model
   and records a delta (UPI-007). Nothing is sent to the USD process on each edit.
6. **Save and commit.** The delta goes to the USD process through `PrepareChangeBatch`,
   the host decision and `InstallPrepared` (F05 section 7). Root publication stays with the host
   (U06). The process writes only the delta (UPI-010).
7. **Loss.** If the process dies, section 6 applies. The session model and viewport keep
   working from application memory; only open, save and commit wait for the process.

## 4. Requirements: owner decisions

### UPI-001 Separate process topology (OD-1)

**Normative text.** The wrapper's parser and stage run only in a separate USD process started by
the application's host layer under F05 section 2 (private pipes, allowlisted environment, no
socket, restart limits). The application process must contain no call path that parses USD
bytes. The pinned `openusd` dependency is linked only into the USD process executable. The
package placement of that executable requires F08 review; this record decides the topology, not
the crate layout. F05 stays the transport baseline; the F11 in-process semantic mapping is not
used.

**Binds:** U01, U04; F08 placement review. **Acceptance.**

- Positive: open an original USDA and USDC fixture through real pipes; the application process
  shows no `openusd` code in its dependency tree (`cargo tree -p <app crate> -i openusd` finds
  nothing).
- Negative: a hostile fixture that makes the USD process abort, hang or exceed its cap leaves
  the application alive, with `BACKEND_UNAVAILABLE` and the session model intact.
- Negative: no listening socket and no network access from either process (F05 section 2).

### UPI-002 Data path (OD-1)

**Normative text.** Section 3 is normative. In particular: (a) the session model lives in the
application; (b) meshes are built by the in-process kernel; (c) derived blobs travel as
content-addressed files referenced by digest, not as IPC attachments; (d) the USD process is
consulted only to open, parse, compose and commit; (e) IPC carries control messages, bounded
pages of authored properties and bounded responses, and any binary attachment remains a separate
F05 frame, never embedded in JSON.

**Binds:** U01, U02, U04, V02, P-tasks. **Acceptance.**

- Positive: opening a 10,000-element synthetic project moves no mesh bytes through IPC; the
  pages hold digests and parameters only (assert attachment count 0 on snapshot responses that
  reference derived blobs).
- Positive: orbit, pick and measure work while the USD process is stopped.
- Negative: a page that embeds blob bytes in JSON, or an attachment on a method that must not
  have one, is rejected (`IPC_BUFFER` or `IPC_MESSAGE`).
- Negative: no viewport or pick code imports the layer-engine or protocol client (F08 boundary
  check).

### UPI-003 Holes and cuts: own implementation, circular and oval only (OD-2)

**Normative text.** A through hole in a plate or member web or flange is a circle (centre,
diameter) or an obround (centre, overall length, width, orientation in the part plane, with
length greater than width). Any other contour (square, rectangle, polygon, arbitrary closed
curve) is `UNSUPPORTED_GEOMETRY`, with no partial geometry and no authored operation. Hole
and cut geometry is implemented in `beamcraft-kernel` under the F06 numeric rules. truck,
solidcraft and other third-party kernels are not used for holes or cuts. This decision covers
through holes only; blind holes, slots in free edges and other cut kinds still need their own
contracts (A03/A04) and are not authorized here. F04/F06 0.1.0 define no operations; the
vocabulary and key fields are a proposal in the PR body.

**Binds:** A03, A04, G-tasks (kernel), P-tasks, U02 (schema). **Acceptance.**

- Positive: circle and obround holes build; the full-representation volume equals the part
  volume minus the analytic hole volume within the F06 tolerance; the mesh passes the F06
  closed-shell validator.
- Negative: square, rectangle, triangle and a 64-vertex polygon each return
  `UNSUPPORTED_GEOMETRY`; the authored element is unchanged.
- Negative: obround with length not greater than width, zero or negative size, or non-finite
  input is `INVALID_ARGUMENT` or `NON_FINITE_NUMBER` per F04 section 3.
- Negative: `Cargo.lock` and `cargo tree` contain neither truck nor solidcraft packages.

### UPI-004 Member roll uses `f64::sin` and `f64::cos` (OD-3)

**Normative text.** Nonzero member roll uses one private function (`roll_sin_cos`) that calls
`f64::sin` and `f64::cos` separately. No other trigonometric call exists in non-test code of
the kernel. Omitted or zero roll bypasses trigonometry and is exact. No trigonometry backend,
crate or cross-target byte identity is claimed; the platform math library may differ in the last
bits between targets. Consequences are accepted: nonzero-roll meshes may hash differently on
different platforms, which only yields different content-addressed blobs and cache entries, never
a wrong key (UPI-009). Tests compare rolled results with an absolute tolerance, not by bits. This
closes the G01 open question; the function stays isolated so a later change is one edit.

**Binds:** G-tasks (G01 follow-up and kernel builders). **Acceptance.**

- Positive: roll `0` and omitted roll give bit-identical output and call no trigonometry.
- Positive: roll `pi/2` and `pi` give axes within `1e-12` of exact values; roll `2*pi` (the F04
  cap) is accepted.
- Negative: a source scan test finds no `sin(`, `cos(` or `sin_cos(` in non-test kernel code
  outside `roll_sin_cos`.
- Negative: roll above the F04 cap is `LIMIT_EXCEEDED`; non-finite roll is `NON_FINITE_NUMBER`.

### UPI-005 Evidence policy (OD-4)

**Normative text.** A spike or task commits its report, its decision and a minimal reproducible
harness. Bulky raw output, logs and large generated files stay local (archived outside the
repository). The report names each omitted artifact by producing command and SHA-256, so a
reviewer can regenerate or compare it. Committed fixtures remain small, original and covered by
`ATTRIBUTION.md` rules.

**Binds:** every spike and verification task (N03, U01-U04 evidence, G-tasks). **Acceptance.**

- Positive: a report lists for every omitted artifact the command, tool versions and full
  SHA-256; the harness reproduces a digest on the recorded machine.
- Negative: a report that cites "see local log" without command and hash fails review.
- Negative: a PR adding generated output larger than a few hundred KiB (proposed review
  threshold, not a hard rule) must justify it or move it out of the repository.

## 5. Requirements: the six integration points

### UPI-006 Property provenance (point 1)

**Normative text.** Every property in a snapshot page carries a closed provenance record:

- `source.layer_id`: the manifest layer identifier of the winning opinion (1..256 UTF-8 bytes;
  exact form fixed by U02/F08);
- `source.ownership`: `own` or `foreign`;
- `source.strength`: unsigned rank of the winning opinion among contributors, 0 strongest;
- `editable`: boolean;
- `write_layer_id`: the owned layer a delta for this property must write into, or null.

`editable` is true only if every contributing site of the prim is owned (LEW-01), the element's
discipline owns it (F10 section 4), and the host-selected owned-layer set contains
`write_layer_id`. Otherwise `editable` is false and `write_layer_id` is null. Missing provenance
never defaults to editable. The application recomputes ownership from `layer_id` and its own
owned-layer set; it does not trust the flag. A delta item whose layer differs from the assigned
`write_layer_id` is refused. Discipline isolation is therefore enforced by the application and
the USD process, not only hidden in the UI, and merge has per-layer opinions to work on.

**Binds:** U01 (pages), U02 (closed fields), U03 (index keeps provenance), U04, V02 (read-only
display). **Acceptance.**

- Positive: an owned plate property reports `own`, `editable` true and a `write_layer_id` in
  the owned set; a foreign reference property reports `foreign`, `editable` false, null write
  layer.
- Positive: a stronger owned override of a foreign prim reports `editable` false (LEW-01).
- Negative: a page that marks a property from a foreign `layer_id` as `editable` is rejected as
  `IPC_MESSAGE`; the session model is not updated.
- Negative: a delta to a foreign property returns `FOREIGN_READ_ONLY`; to an owned property with
  a different layer than assigned returns `PERMISSION_DENIED`; both leave all layer digests and
  generation unchanged.
- Negative: unavailable provenance yields `editable` false and `DEPENDENCY_UNAVAILABLE` on edit.

### UPI-007 Delta knows its base; compare-and-swap (point 2)

**Normative text.** One delta format serves save, commit and a future live operation. A delta is
the F05 `PrepareChangeBatch` pair: the F04 `logical_command` (carrying `op_id`, `base_revision`
and `expected_generation`) and the `changes` items, each naming its `target_layer_id` (UPI-006).
The USD process applies a delta only if the current root equals `base_revision`. On mismatch it
returns `BASE_REVISION_MISMATCH` with the observed context and a conflict diagnostic, and writes
nothing; it never overwrites and never rebases (semantic rebase belongs to the C-tasks). The
check and the apply form one serialized step. The host's expected-head check at root swap
(F04 section 8, U06) stays the final authority, so a race after the process check is still
caught. A later live operation reuses the format; it adds no second delta shape. A repeated
`op_id` follows F04 replay rules.

**Binds:** U02 (item schema), U04, U06, U07; C-tasks (live). **Acceptance.**

- Positive: a delta whose base equals the current root is applied once; generation increases
  exactly once after host acceptance.
- Negative: a delta with a stale base returns `BASE_REVISION_MISMATCH`; all layer bytes and
  digests are identical before and after.
- Negative: two clients submit different deltas on the same base (repeat 100 times with random
  interleavings): exactly one is accepted, the other is a conflict; no lost update.
- Negative: a delta without `base_revision` is `MALFORMED_COMMAND` or `INVALID_ARGUMENT`.
- Positive: save, commit and the test live stub use the same encoder; a schema test shows one
  closed delta schema and no second variant.

### UPI-008 Untrusted responses and blob verification (point 3)

**Normative text.** Every response of the USD process is untrusted. The application validates,
before use and within F05 bounds: item count per page (at most the negotiated `batch_items`),
total elements per stage (at most the reviewed stage budget), prim path length (proposed at most
1024 bytes and 32 segments), property and layer names (at most 256 bytes), JSON nesting and
token counts (F05 section 4), UUIDs (full canonical form), digests (64 lowercase hex), enum
values (closed sets), numbers (finite and within F04 section 3 caps), and cursor size. Any
violation closes the session as `IPC_LIMIT`, `IPC_MESSAGE` or `IPC_BUFFER`, and no part of the
page enters the session model. For an asset blob that came from a file and not from the kernel,
the application reads it by digest from the U05 store, verifies the complete SHA-256 on read,
and then runs the F06 structural validator for meshes. A digest or validator failure is
`INTEGRITY_ERROR` or `GEO_BLOB_INVALID` for that element only; the bytes are never drawn.
Blobs built by the in-process kernel are the application's own and need no re-hash.

**Binds:** U01, U02, U04, V02; G-tasks (blob validator reuse). **Acceptance.**

- Positive: pages at exactly each cap are accepted (page items, path length, depth).
- Negative: each cap plus one is rejected with the stated code, before allocation where the
  F05 rules require it, and the session model is unchanged.
- Negative: a blob file whose bytes were altered after publication fails digest verification
  and is not drawn; other elements still display.
- Negative: a blob with a correct digest but an invalid mesh layout (wrong magic, open shell)
  is rejected by the F06 validator.
- Negative: a stale or foreign cursor is rejected (`STALE_WORKSPACE`).

### UPI-009 Geometry cache key by inputs (point 4)

**Normative text.** A geometry cache entry is addressed by `key = sha256(canonical element
parameters + cuts + kernel version)`, computed from inputs before any build; a result hash
is never a key. For the first scope this is the F06 section 7 shape key: parameters are its
ordered preimage fields, `cuts` is the canonical ordered list of operations (empty until
operations exist; encoded with the F06 section 7 numeric rules), and the kernel version is the
`algorithm` value (with `tolerance_policy`, `feature_id_rule` and `mesh_layout`). The kernel
version is mandatory: a key function that omits it cannot be constructed. Any change that can
alter output bytes for a valid input, including a geometry bug fix, must raise the version,
so that old meshes miss. Placement, label, appearance and camera stay outside the key
(F06 section 7 matrix). A cache hit is verified against the digest stored with the entry
(UPI-008).

**Binds:** G-tasks (keys, golden digests), P-tasks (invalidation on update), U02 (derived blob
references carry the key), V02 (viewport cache). **Acceptance.**

- Positive: equal inputs give an equal key; rename, camera and copy do not change it.
- Positive: raising the kernel version changes the key; an old cached mesh is not served and a
  rebuild runs.
- Negative: a golden test that pins mesh digests for a fixed input set fails when output bytes
  change without a version raise.
- Negative: no code path uses a mesh digest as a cache key; the key type requires a version
  argument at compile time.
- Positive (once operations exist): a hole added or moved changes the key.

### UPI-010 Unknown data and delta-only writes (point 5)

**Normative text.** Saving writes only the delta. The USD process opens the target owned
layer, applies the typed items as authored-opinion edits and writes the layer back; it never
builds a layer from the application's model, and the wrapper exposes no API that takes a full
element list to serialize. Foreign layers are never rewritten: their bytes and digests stay
identical. Foreign prims, custom attributes, unknown namespaces, metadata and relationships
in any layer are preserved as they are (in owned layers: with unchanged authored values and
meaning, F10 DV-NS-002). A delta that targets an unknown or unsupported property is refused
(`UNSUPPORTED_PROPERTY`); an unsupported schema version is inspected and preserved, not
mutated (F04 section 4).

**Binds:** U02, U04, U06 (published revision keeps foreign bytes). **Acceptance.**

- Positive: a fixture holds a foreign reference, a custom attribute in a non-BeamCraft
  namespace, layer metadata and an unknown relationship; apply a label delta; reopen with an
  independent reader; every unknown item equals its original and the foreign layer digest is
  unchanged.
- Positive: 20 saves with fixed IDs and permuted order produce identical owned bytes (LEW-05).
- Negative: a delta on an unknown attribute returns `UNSUPPORTED_PROPERTY` and writes nothing.
- Negative: an API-surface test finds no public function that writes a layer from an element
  list.

### UPI-011 USD process crash during open or save (point 6)

**Normative text.** Section 6 is the normative state description. In short: the application never
loses an unsent or unresolved delta; the file is protected by the atomic root swap; automatic
restarts and retries follow F05; a mutation is never resent silently.

**Binds:** U01, U04, U06, U07, V02. **Acceptance:** crash injection at each step of section 6
produces the stated user-visible state and no half-written revision.

## 6. Crash behaviour as a state description

The application tracks the USD process with these states; transitions follow F05 sections 2, 7
and 8.

| State | Meaning | On process loss |
|-------|---------|-----------------|
| Idle | No process; session model may exist | none |
| Starting | Spawn and handshake (5000 ms) | restart if attempts remain; else Unavailable |
| Opening | `OpenStage` and `ReadSnapshot` running | discard partial pages; restart; repeat the whole open automatically |
| Ready | Stage pinned; reads served | restart; reopen from verified state; session model stays shown |
| Applying | Delta sent; decision or install pending | restart; query the host ledger first; delta stays pending |
| Unavailable | Retries exhausted | stay; user may start a new explicit open |

1. **Open (read-only).** The process dies during spawn, `OpenStage` or any page. The partial
   pages are dropped; nothing partial is shown. The application restarts the process (at most
   two restarts, 250 ms then 1000 ms) and repeats the open, which is safe because it changes
   nothing. If a previous session model exists it remains visible and editable in memory. After
   exhaustion the user sees "cannot open: the file reader stopped" with `BACKEND_UNAVAILABLE`, a
   Retry action (a new explicit open attempt) and the file left untouched. A file that crashes
   the reader every time ends here; the application stays alive (UPI-001).
2. **Save or commit.** The delta is created in the application and kept there until its outcome
   is resolved; it is never dropped, rewritten or given a fresh `op_id`. If the process dies
   before or during `PrepareChangeBatch`, nothing is accepted: the application restarts, reopens
   from the verified recorded state, and shows "save did not finish; your changes are kept".
   The user, not the application, chooses Retry; Retry resends the same delta with the same
   `op_id` (idempotent under F04 replay). No automatic mutation resend (F05 section 2).
3. **Loss after the host decision.** The application asks the host ledger first (it works
   without the process). Recorded accepted: the delta is marked applied and the projection is
   rebuilt from the recorded state. Recorded rejected: the rejection is shown and the delta
   stays for the user. Unknown: the workspace is blocked for new mutations with
   `OUTCOME_UNKNOWN`, the user sees "save status unknown", and the delta stays pending.
4. **File safety.** A crash before the root swap leaves the previous complete revision; after
   the swap, the new complete one; never a mix (architecture "Project state and revisions").
   Orphan temporary files may remain (U05) and are never referenced. Foreign layers are never
   opened for writing (UPI-010).
5. **Durability.** The pending delta lives in application memory. From U07 it is also kept in
   the personal-workspace journal, which survives an application restart.

**Acceptance.** With a fault-injection hook, kill the process at each of: during spawn; during
`OpenStage`; during page N of `ReadSnapshot`; after `PrepareChangeBatch` and before the host
decision; after the host decision and before `InstallPrepared`; during `InstallPrepared`; during
the root swap. For every point assert: application alive; the delta still held; no half-written
file (all revisions in the store verify by digest); the user-visible state above; no automatic
resend; and the retry count never exceeds three launches.

## 7. Added acceptance for task cards

The cards are in the plan, not files. These rows are added to them; existing criteria stay.

| Task | Added acceptance criteria |
|------|---------------------------|
| U02 | Closed snapshot item schema carries `source.layer_id`, `source.ownership`, `source.strength`, `editable`, `write_layer_id` (UPI-006). Closed change item carries `target_layer_id`; one delta schema serves save, commit and live (UPI-007). Saves write only the delta; foreign layer digests stay identical; fixture with foreign prim, custom attribute, layer metadata and unknown relationship survives (UPI-010). Derived blob references carry the digest and the input cache key (UPI-009). |
| U03 | Index entries keep provenance and the `editable`/`write_layer_id` verdict; a stale or partial index never yields `editable` true (UPI-006). Index is rebuilt only from validated pages (UPI-008). |
| U04 | CLI and host API use the one delta encoder (UPI-007). Primary path has no in-process parser: `openusd` absent from the CLI dependency tree (UPI-001). Pages are validated as untrusted at every cap and cap plus one (UPI-008). Messages and exit status for `BACKEND_UNAVAILABLE` and `OUTCOME_UNKNOWN`; a `retry` that reuses the same `op_id` (UPI-011). |
| U06 | Stale-base delta returns `BASE_REVISION_MISMATCH` with bytes unchanged; two-writer race on one base: one accepted, one conflict, 100 repetitions (UPI-007). Crash injection around the root swap leaves the previous or the new complete revision (UPI-011). Published revision keeps foreign layer bytes identical (UPI-010). |
| V02 | Viewport and pick run with the USD process stopped (UPI-002). Read-only state is shown from `editable`, but enforcement stays in the engine and USD process (UPI-006). Mesh cache uses the input key with the kernel version; version bump misses (UPI-009). Blob read failures show per element and do not stop the view (UPI-008). Process-loss and save-status states of section 6 are visible (UPI-011). |

## 8. Statements elsewhere that need follow-up

Changed in this task: `layer-engine-decision-v0.1.md` (topology decided) and the layer-engine
rows of `module-boundaries-v0.1.md`. Outside this task's allowlist and left unchanged, to be
proposed in the PR body: topology statements in `architecture.md`,
`layer-engine-wrapper-requirements-v0.1.md`, `u01-placement-containment.md` and ROADMAP.md;
roll wording in the G01 PR description (PR #38), `g01-math-crates-evaluation-v0.1.md` and
`tolerance-comparison-v0.1.md`. Frozen-contract amendments (F04, F05, F06, F10) are in the PR
body only.

## 9. Provenance and limits

Contributor-original documentation, MPL-2.0, based on the owner's chat decisions of 2026-10-16,
the repository contracts and designs cited in section 2, and no external source. No Tekla or
Trimble material, no third-party code, no assets. Numeric proposals (path length 1024 bytes and
32 segments, name length 256 bytes, the evidence-size review threshold) are draft values for
U01/U02 review and never widen an F04, F05 or F06 maximum. Section 6 treats an explicit user
Retry of the same `op_id` as permitted and an automatic resend as forbidden; that reading of
F05 section 2 awaits owner confirmation. Nothing here is implemented or tested.
