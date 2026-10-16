# U06 commit of a full revision

Date: 2026-10-16. Status: **experimental engine API plus headless commands; the byte, manifest
and root layer of U06 is implemented, content verification is injected (not wired into the CLI)
and U07 recovery is not implemented.** Branch `feat/u06-revision-commit`, stacked on
`feat/u03-uuid-relationship-index` (PR #53); retarget to `master` after #53 merges.

Plan row: U06 / M, owner Storage, depends on U02, U03, U05. Result: immutable generations of
model/derived and an atomic switch of the root. Acceptance: the root reads only complete
generations; the expected base is verified; there is no partial Save of a set of files.

## What is implemented

`beamcraft_host::revision` (pure `std`, no new dependency, no `unsafe`, bounded):

| Piece | Behaviour |
|---|---|
| `Manifest` | Closed canonical text: project, workspace, revision UUIDv7, op UUIDv7, sequence, workspace generation, base (revision + manifest digest), schema identity digest, root layer, optional index record (U03 structure digest, prim count), 1 to 64 layer entries `role name sha256 size` strictly ascending by name. Strict parser; a parsed manifest is re-encoded and must equal the input. Stored as an immutable U05 blob; its SHA-256 is the revision's content identity. |
| `RevisionDraft` | What the caller decides. Layer order is irrelevant: entries are sorted, duplicates are `DUPLICATE_ID`, at least one authored and one derived layer, the root layer must be an entry. 20 reordered drafts give byte-identical manifests. |
| Root | Append-only head log `revisions/<sequence, 20 digits>`. Record `N` is a complete, synced file installed by a **non-replacing hard link**; the root is the highest record. A reader sees revision `N-1` or `N`, never part of a set. |
| `RevisionStore::commit_with` | Validate draft, read and verify the root, op-id replay, expected base (`BASE_REVISION_MISMATCH`), project, verify every referenced blob (presence, full digest, size) and run the optional `ContentVerifier`, store the manifest blob, write and sync the record, link it. Everything before the link leaves the root unchanged. |
| Idempotency | Same op id with identical content returns the recorded revision (`Replayed`), also after the root moved on (bounded look-back of 64 records). Same op id, other content: `OP_ID_REUSE`. Same revision id, other op: `INTEGRITY_ERROR`. |
| Compare-and-swap | The link is the CAS. Two committers on one base cannot both win; the loser re-resolves to a replay or `BASE_REVISION_MISMATCH`. There is no lock file a crash could leave behind (100 two-writer repetitions pass). |
| `RevisionStore::open` | Verifies the log shape (no gap, no unknown name, no torn record), record, manifest, base link and every referenced blob, and returns typed errors for garbage-collected, corrupt, resized or torn state. |
| `CommitHook` | `Checked`, `BlobsVerified`, `ManifestStored`, `RecordStaged` (abort leaves the root unchanged: `Aborted`, F04 `CANCELLED`) and `Switched` (failure there is `OUTCOME_UNKNOWN`, never success or rollback). This is the seam U07 uses for intent records and fault injection. |
| `ContentVerifier` | Injected content gate that receives the verified layer bytes and returns U03 evidence or a `ContentRejection` carrying the layer engine's own F04 code (`DUPLICATE_ID`, `INVALID_ID`, `DANGLING_RELATIONSHIP`, `FOREIGN_READ_ONLY`, ...). The host checks that the evidence is complete (`DEPENDENCY_UNAVAILABLE`) and belongs to the draft's project, workspace, generation and base (`STALE_WORKSPACE`), then records it in the manifest. |

CLI (`beamcraft`): `revision-commit` and `revision-open`, machine-readable JSON, exit codes of the
existing table. `revision-commit` takes already stored blobs (`blob-put`) and does not parse USD.

## Decisions to review

1. **Revision identity (resolves the U04 note).** U01 pins a revision as a content digest while F04
   defines `RevisionId` as a UUIDv7. U06 keeps both: the revision id is a UUIDv7 inside the
   manifest, and the digest of the manifest blob is what U01 carries as `Identity::revision`.
   `ExpectedBase` is the UUIDv7; the manifest records the base's id and manifest digest, so the
   chain is checked both ways. The U01 generation of a reopened revision is the manifest's
   `workspace_generation`.
2. **Head log instead of rename-over-pointer.** The plan text suggests temp + sync + rename. A
   rename replaces blindly, so it needs a lock to be a compare-and-swap (F04 section 8: plain
   read-then-rename is not CAS), and a lock file left by a crash blocks writers until recovery
   exists. A non-replacing hard link of a complete synced file is atomic, cannot overwrite, is
   the CAS itself and leaves nothing blocking. It uses the same hard-link primitive and
   filesystem requirement as U05. It also leaves a verifiable per-revision history.
3. **The host does not link the layer engine.** The module table gives the host no edge to
   `beamcraft-layer-engine` and the owner decision puts USD parsing in a separate process.
   Content checks are therefore a trait. The tests implement it with the real U01/U02/U03
   engine; a product wiring is open (below).
4. A commit without a verifier records `index none` in the manifest. The index is never accepted
   as a caller claim.

## Guarantees and non-guarantees

* Promised: complete-or-absent root, expected-base and project check, all blobs verified before
  the switch, no overwrite, typed refusal of partial/garbage-collected/corrupt/torn state,
  deterministic bytes.
* Not promised: **directory-entry crash durability** (the temporary record is synced, the directory
  is not, so a reported commit can vanish after power loss; what remains is still a complete
  previous root); removal of `.tmp-*` files or unreferenced blobs after a crash; garbage
  collection; protection from a hostile local writer to the store directory (same limitation as
  U05).
* Not implemented: **U07** journal, intent record, crash recovery and fault-injected recovery
  proof; personal workspaces; a durable op-id ledger beyond the head log; multi-client
  collaboration; a CLI path that runs the layer-engine verifier (needs an F05 profile with an
  index/verify message; the frozen U04 profile has none). The hook is the U07 seam, and the
  replay look-back of 64 records is the head log's own, not a ledger.
* The worker protocol is unchanged: no mutation schema was added and the worker cannot publish a
  root; only the host does.

## Provenance and limits

Original code and fixtures; formats derived from F04 sections 2, 6, 8, F05 section 7 and the
plan. No asset, no third-party dependency, no Tekla/Trimble material. Caps: 64 entries, 128-byte
names, 64 KiB manifest, 256 MiB declared layer bytes, 64 MiB in memory for a verifier, 1,000,000
revisions, 512-byte records. Linux/macOS, wasm32 and `cargo deny` were not run (not installed
here).
