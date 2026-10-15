# F05: private USD-worker protocol, version 0.1.0

## 1. Status and scope

Status: **draft 0.1.0, not frozen; nothing implemented**. Date: 2026-10-14.
This original contract consumes [F04 version 0.1.0](model-and-commands-v0.1.md), specifically
sections 2, 3, 6, 7 and 8. The same contract serves the CLI and native engine host.
[Acceptance descriptions](worker-protocol-cases-v0.1.json) are future tests, not runnable fixtures.

The Rust host owns engineering validation, authorization, transactions and recorded outcomes.
One local worker owns one live USD stage and performs composition and serialization only.
The initial worker is thin Python/pxr, subject to runtime admission. This text integrates no SDK.

Out of scope: implementation, new dependencies, schema properties (U02), mesh layout/topology
(F06), journal/storage mechanisms (U06/U07), geometry algorithms, remote or multi-client
operation, user-facing control servers and a later worker-language decision.

## 2. Process and channels

The CLI embeds/calls the native host client; both host entry points use this protocol.
The host spawns one verified application-packaged worker executable directly, without a shell,
with private inherited anonymous pipes: host-to-worker input, worker-to-host output, and a
separate diagnostic stream. Only these explicitly selected handles are inherited. Stdout carries
only protocol bytes. Diagnostics never share its stream. No listening socket, network service,
network resolver or outbound network access is permitted. A user-facing server is a separate
opt-in, loopback-only, token-authenticated feature outside F05.

Spawn with an allowlisted environment and a host-controlled working directory. Remove inherited
Python import/startup/user-site paths, SDK/plugin discovery overrides and dynamic-library search
overrides; reconstruct required runtime paths solely from verified package entries. Python starts
in isolated mode with user-site and current-directory imports disabled. Do not inherit arbitrary
search-path entries. Plugin/schema registration is restricted to a verified package manifest and
digest before opening any model. Never load incompatible SDK copies in one process.

Use the current user's least privileges, no elevation, a contained writable workspace and no
unrelated inherited handles. Resolve only host-authorized contained local assets; model-authored
paths cannot select executables, import modules, extend plugin paths, execute scripts or authorize
network access. An attempted executable/plugin directive is rejected with IPC_MODEL_EXECUTION,
before opening that composition. Ordinary unknown metadata remains inert and is preserved as
required by F04; it is never evaluated. OS isolation supplements validation on both sides.
No unsafe Rust or C++ FFI boundary is authorized.

After failure, terminate/reap the old child and close all its pipes before restarting.
At most two automatic restarts follow the initial spawn per workspace-open attempt (three
launches total), with fixed delays of 250 ms then 1000 ms. An incompatible handshake or a
malformed peer session gets no automatic restart. Do not automatically resend a mutation.
Each restart requires fresh handshake and verified workspace recovery; retry exhaustion leaves
the backend unavailable until a new explicit open attempt. An open attempt ends on success or
exhaustion; successful handshakes do not reset retries inside the same failed recovery attempt.
The diagnostic reader accepts at most 64 KiB per child lifetime and retains at most 4 KiB of
sanitized text. Exceeding the drain cap closes the failed session with IPC_LIMIT and stops/reaps
the child; there is no unbounded discard loop. Raw diagnostics are not exposed to clients.

## 3. Wire framing and binary attachments

Each direction is an ordered byte stream. Reads/writes may split anywhere. All multi-byte
integers below are unsigned, big-endian. A fixed 16-byte header precedes every frame:

| Offset | Width (bytes) | Field | Draft value |
|--------|---------------|-------|-------------|
| 0 | 4 | magic | ASCII BCWP, hex 42 43 57 50 |
| 4 | 1 | protocol major | 0 |
| 5 | 1 | protocol minor | 1 |
| 6 | 2 | protocol patch | 0 |
| 8 | 1 | frame kind | 1 = control JSON; 2 = binary attachment |
| 9 | 1 | flags | 0; compression/extensions forbidden |
| 10 | 2 | attachment count | 0..8 on control; 0 on binary |
| 12 | 4 | payload length | Excludes this header; rules below |

A control payload is one UTF-8 JSON object, 1..1,048,576 bytes. The logical F04 mutation
subobject independently obeys F04's 1 MiB cap. No bundling of several logical mutations into
one atomic command bypasses that cap. Oversized commands fail, rather than being fragmented
to evade F04. A control object always contains a buffers array (empty if no attachments).
Its length equals attachment count. Binary bytes are never base64 or numeric arrays in JSON.

A bundle is the control frame followed immediately by its declared binary frames in descriptor
index order. No other frame, even a heartbeat, may interleave within that bundle in that
direction. The opposite direction remains independently readable. Each buffers descriptor has
exactly index, element_type, byte_length, element_count and sha256. Indices are contiguous from
zero; lengths/counts are JSON integers. Draft 0.1.0 supports only element_type "u8": opaque
octets with element_count equal to byte_length. This is transport typing, not a mesh layout.

Each binary payload has this fixed 44-byte prefix followed by exactly byte_length bytes:

| Offset in payload | Width (bytes) | Field |
|-------------------|---------------|-------|
| 0 | 2 | descriptor index |
| 2 | 1 | element type: 1 = u8 |
| 3 | 1 | reserved, zero |
| 4 | 4 | byte_length, 1..16,777,216 |
| 8 | 4 | element_count, equal to byte_length |
| 12 | 32 | raw SHA-256 of following bytes |
| 44 | byte_length | opaque binary bytes |

Common-header payload length must equal 44 + byte_length; the prefix, descriptor and streamed
SHA-256 must agree. JSON digests use F04's full lowercase 64-hex form. SHA-256 detects corrupted
or misassociated buffers and aligns with immutable resource verification; it does not authenticate
a hostile peer or replace structural validation. No compression is supported. An attached buffer
is request-scoped; it cannot become a future resource handle without host verification.

Validate the fixed header before any payload-sized allocation. Validate each descriptor, checked
sum/product, binary prefix and reserved field before allocating its bytes. Reserve capacity
against negotiated bundle and endpoint budgets before allocation. Parsing uses bounded streaming
validation (depth, decoded string sizes and token count) before constructing a bounded object
tree. Use checked integer conversion and arithmetic; do not trust a length to fit a platform
index. If budget cannot be reserved, stop producing/reading another bundle and apply backpressure;
never build an unbounded application queue. Each endpoint owns at most one outgoing and one
incoming bundle at a time; its aggregate retained wire payload/header storage is at most 64 MiB.
Parser/canonicalizer scratch, including the decoded object tree, has a separate 8 MiB ceiling.
Application stage storage is outside
this wire budget and needs its own U01/U02 bounds. Fully consume/release a bundle before taking
the next one. A bundle totals at most 32 MiB, including all headers and prefixes.
The operating system's bounded pipe buffers are not application queues.

Validate the complete bundle, attachment hashes and message schema before dispatch or observable
state change. A malformed bundle changes neither stage nor workspace. Completed earlier
transactions remain completed. Oversized, truncated, garbage, wrong-kind, wrong-flag, undeclared
or mismatched frames terminate the session; there is no scanning for magic or guessed
resynchronization. Never drain an untrusted declared oversized length. EOF between bundles is
a process/channel loss; EOF inside a header/payload is IPC_TRUNCATED. An incomplete bundle that
outlives the assembly deadline is IPC_DEADLINE. A fatal diagnostic is best effort only, written
once if a trustworthy control writer is available within 100 ms; otherwise record it locally.

## 4. Handshake, versions and capabilities

Before stage access, host sends hello, worker returns capabilities, host sends ready and worker
returns ready_ack. No normal request is permitted until ready_ack is validated. All four messages
use control frames without attachments. The handshake completes within 5000 ms of spawn.

hello contains offered_versions (only ["0.1.0"] in this draft), model_contract "0.1.0",
a fresh session_nonce (32 random bytes encoded as 64 lowercase hex digits), expected_schema_digest,
required_features and host_limits. capabilities echoes the nonce, selects protocol_version and
model_contract and reports backend_name, backend_version, sdk_version, python_version (string or
null), schema_package_digest, supported_features and worker_limits. Required feature names are
"private_pipes", "sha256_buffers", "cancel", "outcome_query" and "incremental_events".
Reported versions identify the actual running build; compare them with the verified application's
runtime manifest, not with model claims. This contract pins no SDK/Python release.
Cache keys include backend name/version, SDK version, model contract, protocol version and schema
package digest.

host_limits, worker_limits and effective_limits are closed objects with exactly these integer
keys (values below are the draft maxima):

| Key | Maximum |
|-----|---------|
| control_bytes | 1048576 |
| buffer_bytes | 16777216 |
| bundle_bytes | 33554432 |
| endpoint_wire_bytes | 67108864 |
| scratch_bytes | 8388608 |
| attachments | 8 |
| json_depth | 16 |
| json_tokens | 65536 |
| string_bytes | 4096 |
| cursor_bytes | 256 |
| message_bytes | 1024 |
| batch_items | 4096 |
| delivery_pages | 1024 |
| request_timeout_ms | 120000 |

Each value is positive except attachments, which may be zero. Values exceeding these maxima
are incompatible. Fixed widths, ID ranges, concurrency, timing other than the request maximum,
feature-list count and diagnostic limits are not negotiable. A profile too small to encode the
mandatory handshake is incompatible, rather than an invitation to violate its caps.

ready confirms exact selected versions, schema digest and effective_limits (the per-field minimum
of host, worker and the maxima above). ready_ack echoes those fields and nonce. Any mismatch,
missing
required feature or limit outside its allowed range is IPC_CAPABILITY_MISMATCH, with no stage
opened. A smaller supported limit is valid; it may reject a later request. Do not silently fall
back to another schema, SDK or protocol.

The fixed header is the bootstrap envelope too. A recognized magic with unsupported version is
IPC_VERSION: record received and supported version numbers safely and terminate before parsing
the payload. If possible, send one fatal frame in the local supported envelope; this is a
diagnostic, not negotiation success. Bad magic is IPC_FRAME, not a version probe. Exact version
matching is required for this draft, including patch. A future interoperable version requires
explicit review; unknown fields and unknown message types are not ignored. Capabilities never
authorize plugin loading from model data.

## 5. Message model, correlation and order

Every control object has type and buffers. After hello every object also has session_nonce,
which must match. All schemas are closed. The following fields are additional to those common
fields; optional fields are explicitly identified:

| Type / direction | Fields and meaning |
|------------------|-------------------|
| hello / host | Fields in section 4; establishes nonce |
| capabilities / worker | Fields in section 4 |
| ready / host; ready_ack / worker | Confirmations in section 4 |
| request / host | request_id, method, timeout_ms, context (null for OpenStage and permitted for QueryOutcome), body |
| response / worker | request_id, outcome, code (null on success), message, observed_context, body |
| event / worker | event_seq, event_name, context, body |
| cancel / host | target_request_id; no separate request or mutation identity |
| ping / host; pong / worker | ping_id; pong echoes the sole pending ping |
| fatal / either | transport_code, message; terminates session |

request_id is a JSON integer 1..65,535, strictly increasing within the session and never reused,
including completed/rejected requests. It is unrelated to F04 op_id. The host admits at most
one normal request at a time; a second is fatal IPC_BUSY with no second dispatch. A cancel and one
ping may
coexist with that request. There is no application request queue. At ID exhaustion, end the
session after resolving the pending outcome and reopen; never wrap. Retain a high-water mark,
the current ID and the last completed ID, not an unbounded history. A reused or decreasing ID
is IPC_DUPLICATE_ID and fatal. A response for an ID other than the sole pending request,
including a second terminal response, is IPC_UNKNOWN_ID and fatal. Response order therefore
matches admitted request order. One terminal response per admitted request is required.

cancel can target the current request or the last completed request (the latter is a race and
a no-op; it cannot retract its response). Other IDs cause IPC_UNKNOWN_ID and session closure.
At most one cancel is sent per request; duplicates are IPC_DUPLICATE_ID. A late pong after a
matched pong or a mismatched ping_id is IPC_UNKNOWN_ID. ping_id is an increasing integer in
1..65,535; rotate the idle session before exhaustion. Events use a separate increasing event_seq
in the same range; gaps/reuse are IPC_SESSION and require resnapshot in a new session.

Transport methods are OpenStage, ReadSnapshot, PrepareChangeBatch, InstallPrepared,
DiscardPrepared, QueryOutcome and CloseStage.
These are composition/client operations, not additional F04 engineering commands.
OpenStage.body identifies authorized project/workspace IDs, base_revision and generation with
verified content digests; no arbitrary caller path or resolver executable is allowed.
It returns a verified stage context. ReadSnapshot returns a bounded page over that exact context,
with an opaque continuation cursor (at most 256 UTF-8 bytes); a page never implies a full stage.
PrepareChangeBatch.body carries the unchanged F04 mutation as logical_command and a host-validated
bounded composition change batch. Only F04 section 7 commands are eligible; a batch cannot
introduce a raw caller-controlled USD setter. U02/F08 must fix closed schema-specific batch and
snapshot item fields before implementation, within these limits. This draft does not invent
schema properties. The transport body shapes are closed: OpenStage has project_id, workspace_id,
base_revision, workspace_generation and manifest_digest; ReadSnapshot has cursor (null or string);
PrepareChangeBatch has logical_command and changes (an array of bounded schema-specific items).
A successful prepare returns candidate_token (the same representation as stage_token) and a
bounded candidate result, without advancing generation or changing the live stage token.
At most one candidate exists. InstallPrepared and DiscardPrepared each have body containing
only candidate_token and op_id; no attachments. InstallPrepared is sent only after host outcome
recording, while DiscardPrepared is sent only before that decision. Neither command creates a
second logical acceptance. All three use the still-pinned live context; stale candidate tokens
are IPC_STALE_STAGE. A new prepare while a candidate exists is fatal IPC_BUSY.
QueryOutcome.body contains project_id, workspace_id and op_id and requests the host-coordinated
recorded outcome. The host consults the durable ledger first; any worker query is only stage
status evidence and cannot prove durability. QueryOutcome may run without an open stage;
a missing worker record remains OUTCOME_UNKNOWN. CloseStage.body is empty; it releases the
stage only after resolving pending mutation status.

response.outcome is one of stage_opened, snapshot_page, prepared, stage_installed, stage_discarded,
recorded_result, stage_closed, rejected, cancelled or outcome_unknown. Success codes are null;
rejected/cancelled/outcome_unknown carry their F04 code. observed_context is null when unknown.
Only a host-verified recorded_result may describe accepted_draft; a prepared or stage_installed
response alone is not engineering acceptance. Error bodies contain only optional transport_code;
success bodies are method-specific results under U02/F08 and cannot acquire arbitrary fields.

Events are "changed" (bounded incremental change/result page) or "resnapshot_required"
(invalidation only). Responses and events carry pinned contexts and never stream a whole model
each frame. Event notification is ordered after the response for its causal request. At most
one event is queued; coalesce excess changes to resnapshot_required, never silently drop them.
Clients stop applying deltas after invalidation until ReadSnapshot completes. At most 1024 pages
or events form one snapshot/change delivery, and at most 4096 items appear per page/batch.
Larger delivery fails with IPC_LIMIT before dispatch or requests a fresh bounded query.
Snapshot cursors expire on context change; no unbounded traversal/retry loop is authorized.

## 6. JSON control and canonical replay bytes

Apply the [RFC 8259](https://www.rfc-editor.org/rfc/rfc8259.html) grammar with F04 section 3's
stricter rules. Reject duplicate decoded object keys, invalid UTF-8, BOM, lone surrogate escapes,
trailing data and non-JSON literals. Reject excessive nesting/strings/tokens while scanning.
NaN/Infinity literals are IPC_JSON / MALFORMED_COMMAND; syntactically valid numbers overflowing
f64 or underflowing a nonzero value to zero produce NON_FINITE_NUMBER. Number-typed fields
reject booleans and numeric strings with INVALID_ARGUMENT. WorkspaceGeneration remains F04's
exception: a canonical unsigned decimal string in 0..18,446,744,073,709,551,615, never f64.
Transport integer fields must have integer JSON tokens (no fraction/exponent), within their
field bounds, and must be checked before conversion. Depth counts the root object as one;
an embedded F04 command must fit both its own F04 bounds and the enclosing control bounds.

F04 section 6 replay comparison uses [RFC 8785 sections 3.1 and
3.2](https://www.rfc-editor.org/rfc/rfc8785.html#section-3):
parse valid engineering numbers to nearest IEEE binary64 (ties to even), reject F04 overflow/
underflow first, then serialize with JCS's binary64 number rules. Negative zero becomes 0.
Use JCS property ordering by UTF-16 code units, escaping and UTF-8 output without whitespace.
Strings retain their decoded Unicode sequence without normalization; arrays retain order.
Python's default JSON serializer is not a substitute for this profile.

Canonicalize exactly the complete validated F04 envelope: contract_version, project_id,
workspace_id, op_id, actor_id, base_revision, expected_generation, command and arguments.
Include all explicit argument values; absent and null differ. Do not fill defaults, round
dimensions, normalize placements or perform geometry-specific transformations for replay.
Exclude request_id, session_nonce, deadlines, stage tokens, attachment descriptors, composition
batches and all other transport metadata. The identical op_id itself remains in these bytes.
Compare canonical bytes (a digest may index them but cannot replace collision-safe equality).
Thus object order, insignificant whitespace, escaped equivalents and 1 versus 1.0 versus 1e0
compare equal; a different argument under the same op_id is OP_ID_REUSE.
The host owns the replay ledger; the worker cannot declare a new acceptance on a transport retry.
Authentication/authorization precede replay; recorded replay precedes fresh base/generation
checks as F04 requires. This does not define geometry signatures, tolerances or publication
idempotency. Canonical output and scratch remain bounded; cap failure is LIMIT_EXCEEDED.

## 7. Revision and session consistency

context contains exactly project_id, workspace_id, base_revision, workspace_generation and
stage_token. F04 identities/digests/generations retain their full representations.
stage_token is an opaque worker-generated 32-byte random value encoded as 64 lowercase hex
digits, unique for each stage incarnation and every successful stage-changing operation.
The session nonce identifies the process conversation; a stage token identifies a composed view.
It grants no authority. OpenStage and successful changes return the new context.

For a new operation, the host serializes workspace checks and commit. The worker checks nonce,
stage token and the complete pinned revision/generation tuple before composition. Stale stage
token produces IPC_STALE_STAGE / STALE_WORKSPACE without changes. A matching stage token with
wrong base returns BASE_REVISION_MISMATCH; a generation mismatch returns STALE_WORKSPACE.
Mutations also carry F04 base_revision and expected_generation inside logical_command; they
must match context for a new operation. The host resolves authorized recorded op_id replays
before dispatching to a stale worker; never let stage checks block F04's accepted replay rule.
Compare with the selected workspace base, not automatically the latest published head.
Publication's expected-head check remains mandatory under U06.

PrepareChangeBatch stages and validates a complete isolated candidate; its result is provisional.
The host validates that result and records its serialized transaction decision before sending
InstallPrepared. Installation switches the worker live view to that already accepted generation
and rotates stage_token. The worker cannot increment the host generation, publish a root or record
engineering acceptance independently. A reported accepted result requires host recording and
exactly one generation increment; worker acknowledgement alone is insufficient. Check generation
increment for u64 overflow before acceptance; exhaustion is LIMIT_EXCEEDED.

Before the host decision, cancellation uses DiscardPrepared if preparation already completed;
otherwise cancel targets the active preparation request. After the host decision, do not discard
or compensate automatically: resolve the recorded outcome, and reconstruct a lost projection.
U06/U07 must implement durable candidate/result validation, recording and recovery without
exposing a half-accepted stage. No next engineering mutation or incremental event is released
until that decision and projection are resolved. Failure discards an unaccepted candidate or
reopens the worker projection from verified recorded workspace state.

Restart invalidates all stage tokens, cursors, pending correlations and events. New pipes and
nonce are mandatory. OpenStage reconstructs only a verified complete workspace revision/
generation after recovery; do not assume generation zero or erase unpublished drafts.
Late old-session bytes cannot update a new view. Missing/uncertain recovery keeps the workspace
blocked for new mutations with OUTCOME_UNKNOWN; cached geometry is never evidence of acceptance.

## 8. Deadlines, liveness, cancellation and failure

timeout_ms is 1..120,000, default 30,000 when supplied by the host client; the wire field is
required. The host starts a monotonic deadline when admitting the request, covering sending,
queue-free dispatch and receiving the response. The enclosing logical mutation also has one
host deadline across prepare, decision and install; separate calls cannot reset or extend it. The
worker starts its local budget at complete
bundle validation; the earlier host deadline always controls. Partial bundle assembly has an
independent 10,000 ms limit from first header byte to final attachment byte. There is no timeout
extension from trickled bytes. Read/write/budget waits honor these deadlines. Idle header waits
are bounded by liveness. No timed-out bundle is dispatched.

Host sends at most one ping every 5000 ms when no ping is pending; any pong must arrive within
15,000 ms of its ping. Pongs show a responsive IPC reader, not completion or rollback.
Request deadlines still detect a hung composition loop even if pongs continue.
A bundle must finish before another outgoing frame, so liveness handling must account for the
10,000 ms bundle deadline without extending the request deadline. Fatal close permits at most
1000 ms for child exit, then terminates it and waits at most 1000 ms for reaping; failure to
confirm exit blocks replacement rather than creating competing workers.

A valid cancel is advisory until a terminal result. Before the serialized apply/commit decision
point, discard candidate state and return CANCELLED with unchanged authored/derived state and
generation. Receipt of cancel is not acknowledgement of rollback. After that point, return the
recorded outcome if provable, otherwise OUTCOME_UNKNOWN. Never fabricate CANCELLED for work
already applied or recorded. A late cancel cannot change a recorded result.

On deadline, initiate cancel if still possible, but wait no longer than 1000 ms beyond the host
deadline for a conclusive terminal result; then close the session and stop/reap the child.
IPC_DEADLINE describes the transport cause. A host may report CANCELLED only with proven
pre-commit discard, or a recorded accepted/rejected result if verified. Otherwise any dispatched
mutation whose acceptance may have occurred is OUTCOME_UNKNOWN. A timeout is never proof of
rollback. Unsent requests and read-only calls map loss/hang to BACKEND_UNAVAILABLE.

Worker exit, broken pipe or hang produces IPC_BACKEND_LOST (or the more specific framing/
deadline cause). Required backend service is BACKEND_UNAVAILABLE; an uncertain in-flight
mutation additionally remains OUTCOME_UNKNOWN until U07/QueryOutcome resolves it.
Do not return success, decrement generation, silently resend or create a fresh op_id to hide
uncertainty. QueryOutcome returns an actual recorded result or OUTCOME_UNKNOWN, never guessed
not-applied. U07 may prove absence only from its verified recovery protocol. Limit automatic
outcome queries to one after recovery; further queries require an explicit client request.

## 9. Errors and safe diagnostics

Transport codes have an IPC_ prefix; they are distinct from F04 stable codes.
response.code is an F04 code or null for success; its body may include transport_code.
fatal uses transport_code without asserting a mutation result. Safe messages use fixed
English descriptions and bounded field names, not echoed input, private paths, credentials,
client labels, file contents or raw exceptions. A message is at most 1024 UTF-8 bytes.
Observed context is returned only when known and authorized; diagnostic target data follows F04.

| Transport code | Condition | F04 mapping when a command outcome is reportable |
|----------------|-----------|-------------------------------------------------|
| IPC_VERSION | Unsupported header/negotiated version | UNSUPPORTED_VERSION |
| IPC_FRAME | Bad magic/kind/flags/reserved fields or structural framing | MALFORMED_COMMAND |
| IPC_LIMIT | Byte/count/depth/queue budget exceeded | LIMIT_EXCEEDED |
| IPC_TRUNCATED | EOF within a frame/bundle | BACKEND_UNAVAILABLE |
| IPC_BUFFER | Undeclared buffer, length/type/index/count/hash disagreement | INTEGRITY_ERROR |
| IPC_JSON | Invalid encoding, JSON grammar or duplicate decoded keys | MALFORMED_COMMAND |
| IPC_MESSAGE | Unknown type, method or closed control-schema field | UNSUPPORTED_COMMAND |
| IPC_DUPLICATE_ID | Reused/decreasing correlation ID or repeated cancel | MALFORMED_COMMAND |
| IPC_UNKNOWN_ID | Uncorrelated response/cancel/pong | MALFORMED_COMMAND |
| IPC_BUSY | More than one outstanding normal request | LIMIT_EXCEEDED |
| IPC_CAPABILITY_MISMATCH | Missing feature, unexpected runtime/schema or limits | UNSUPPORTED_VERSION |
| IPC_SESSION | Wrong nonce or invalid event sequence | STALE_WORKSPACE |
| IPC_STALE_STAGE | Wrong stage incarnation token | STALE_WORKSPACE |
| IPC_MODEL_EXECUTION | Model tries to request scripts/plugin loading | PERMISSION_DENIED |
| IPC_DEADLINE | Assembly/request deadline or liveness expiry | BACKEND_UNAVAILABLE |
| IPC_BACKEND_LOST | Process exit, broken pipe or unresponsive backend | BACKEND_UNAVAILABLE |

Framing, JSON, capability, session and correlation errors are fatal, except IPC_STALE_STAGE is a
request rejection. IPC_BUSY closes the session without trying to
correlate a second response while the first request is pending. IPC_LIMIT is fatal during
decode/framing, but a complete valid request exceeding an application batch cap is rejected
without closing the session. IPC_MODEL_EXECUTION rejects the open/change without running it.
A valid F04 envelope with a semantic/type error returns its F04 code and keeps the session:
for example INVALID_ARGUMENT, NON_FINITE_NUMBER, OP_ID_REUSE or BASE_REVISION_MISMATCH.
UNSUPPORTED_VERSION for the F04 envelope is distinct from an unsupported wire version.
Section 8's uncertainty override takes precedence over this table for any previously dispatched
mutation; a transport diagnostic never proves rollback of earlier work.

## 10. Initial numeric limits

All values are provisional operating caps for draft interoperability, pending measurement.
Both peers enforce the negotiated minimum; negotiation cannot widen any F04 cap.
Fixed representation values are not negotiable. MiB = 1,048,576 bytes; KiB = 1024 bytes.

| Item | Bound / value |
|------|---------------|
| Wire version | 0.1.0 exactly; header version widths 1/1/2 bytes |
| Common header; control/binary kinds; flags/reserved | 16 bytes; kinds 1/2; flags/reserved 0 |
| Payload length width; attachment count width | u32 (4 bytes); u16 (2 bytes) |
| Control payload | 1..1 MiB |
| F04 logical command; nesting | At most 1 MiB; at most 16 containers including root |
| All control nesting; tokens (keys and values/containers each count once) | At most 16; at most 65,536 |
| Decoded string/key; cursor; safe message | At most 4096, 256, 1024 UTF-8 bytes respectively |
| F04 labels; relationship targets; footprint entries | 1..256 bytes; at most 1024; at most 4096 |
| WorkspaceGeneration | Decimal string representing u64, 0..18,446,744,073,709,551,615 |
| Nonce; stage token; SHA-256 | 32 bytes each, 64 lowercase hex digits in JSON |
| Binary descriptor prefix; u8 type | 44 bytes; type 1 with one byte per element |
| Attachments per bundle | 0..8; contiguous indices 0..7 |
| Binary data bytes/elements per attachment | 1..16 MiB |
| Entire bundle including headers | At most 32 MiB |
| Endpoint retained wire bytes; parse/canonical scratch | At most 64 MiB; at most 8 MiB separately |
| Simultaneous normal requests; pending cancels/pings; candidates | 1; at most 1 each; at most 1 |
| Incoming/outgoing bundles; queued events | At most 1 each; at most 1 event |
| Request/ping/event IDs | Integers 1..65,535; no wrap |
| Batch/page items; pages/events per delivery | At most 4096; at most 1024 |
| Handshake offered versions; required/supported feature entries | 1; at most 16 each |
| Negotiable limit fields | Exactly 14 keys listed in section 4; no extra fields |
| Handshake timeout; bundle assembly timeout | 5000 ms; 10,000 ms |
| Request timeout range/default | 1..120,000 ms / 30,000 ms |
| Ping interval; pong allowance | 5000 ms; 15,000 ms |
| Post-deadline cancellation grace | At most 1000 ms |
| Fatal diagnostic write; exit grace; forced reaping wait | At most 100 ms; 1000 ms; 1000 ms |
| Automatic restarts; delays | At most 2 after initial launch; 250 ms then 1000 ms |
| Automatic recovery outcome queries | At most 1; never automatic mutation retries |
| Diagnostic drain/retention | At most 64 KiB per child; 4 KiB sanitized retained |

Every collection/string in a control message is also bounded by its byte/token/depth budget.
Capability feature lists cannot trigger plugin discovery. Negotiable caps use the exact section 4
names. Fixed timing/concurrency rules and F04 type
representations remain unchanged. The default host request budget is the smaller of 30,000 ms
and negotiated request_timeout_ms.
F04's engineering input limits still apply unchanged. Geometry mesh layout, topology and
accuracy bounds belong to F06; opaque byte limits do not grant permission for any topology.
All work iterates over bounded frames/items/pages or stops at a deadline. Session service may
idle only under bounded liveness checks; there is no unbounded recursion, queue or retry policy.

## 11. Acceptance handoff

| Future task | Required verification |
|-------------|-----------------------|
| F08 | Keep protocol/domain/native-host boundaries independent; bounded decoders on both sides, malformed input isolation and canonical-byte comparison |
| U01 | Spawn/sanitize/handshake, no socket, package-pinned plugin loading, open/read, stream splits, bounds, liveness and process loss |
| U04 | CLI and native client use the same contract, correlation, deadlines/cancel, snapshots and safe diagnostics |
| U02 | Closed composition/snapshot bodies, verified schema digest, inert model metadata and schema/package mismatch rejection |
| U06 | Serialized revision/generation checks, candidate composition, host acceptance point, exactly-once draft replay and complete publication |
| U07 | Crash/restart token invalidation, preserved drafts, recorded outcomes and uncertainty resolution |

Descriptions alone pass no runtime gate. Future acceptance needs synthetic original inputs,
real pipes/processes and API -> CLI -> USD -> independent-reader checks. A skipped or unavailable
check is **not run**, never passed. Malformed framing must be tested before allocation and before
stage dispatch, including every boundary split, maximum and maximum-plus-one.

## 12. Open questions and unresolved decisions

- Final numeric caps and timing values need measurement; values here are explicit draft defaults,
  not performance claims. No peer may silently raise them before review.
- Whether a later C++ worker preserves this framing is unresolved; no C++ FFI is authorized.
- SHA-256 cost for large buffers needs measurement; draft hashing is mandatory until reviewed.
- F06 must define mesh layouts/topology and any later element-type negotiation.
- U02/F08 must publish the schema-specific change/snapshot item fields and opaque cursor encoding
  before implementing those payloads. Transport envelopes, method names and limit keys are fixed
  here; unknown names remain rejected in this draft.
- U06/U07 must prove coordination between provisional stage results and host-recorded acceptance,
  including post-apply loss. This contract fixes outcomes, not journal/storage mechanics.
- F04 remains a draft. Its geometry/frame/quaternion/tolerance review does not change this
  transport profile. No conflict with the consumed F04 sections was identified. A later F04
  change that conflicts with this profile requires explicit joint review; do not weaken F04.

## 13. Provenance

Contributor-original BeamCraft contract and synthetic acceptance descriptions, licensed MPL-2.0.
Protocol layout, limits and process rules are original project decisions from the task card and
the repository [architecture](../architecture.md), [development guide](../development.md) and
F04 draft. No commercial-product material, catalogue data, third-party kernel code/tests or
copied implementation is used. No runtime asset is added.

Public specification references (rules are referenced, not copied implementations):

- [RFC 8259 sections 4, 6, 8 and 9](https://www.rfc-editor.org/rfc/rfc8259.html):
  JSON syntax and interoperability; F04/F05 impose stricter bounded decoding.
- [RFC 8785 sections 3.1 and 3.2](https://www.rfc-editor.org/rfc/rfc8785.html#section-3):
  JCS canonical bytes and binary64 serialization for replay only.
- [RFC 6234 sections 4–6](https://www.rfc-editor.org/rfc/rfc6234.html#section-4):
  SHA-256 algorithm specification for attachment integrity; no source code is incorporated.

The optional candidate-runtime audit was not needed and is not represented as integrated.
