# BeamCraft native host

U05 implements `beamcraft_host::storage` using the standard library and the portable
`beamcraft_model::digest` module. No third-party dependency is used.

Public API:

- `BlobStore::new(root)` / `with_limit(root, cap)` bind one existing absolute directory.
- `publish(&[u8])` returns `(ContentDigest, Published::{Created, AlreadyPresent})`.
- `read(&ContentDigest)` returns completely verified bytes.
- `contains(&ContentDigest)` verifies presence; corruption is an error.
- `verify(&ContentDigest)` verifies and discards the returned capped buffer.
- `StorageError::code()` maps safe, typed failures to F04 `ErrorCode`.
- `MAX_BLOB_BYTES` is 33,554,432 (F05 0.1.0 sections 3/4 bundle budget).

The root argument is host configuration, not a resource identifier. No resource API accepts
paths or URIs. Files use `blobs/<two lowercase hex digits>/<64 lowercase hex digits>`.

Publication flushes and syncs a temporary file inside the store before a non-replacing hard
link. There is no replacing rename fallback. Filesystems lacking hard links return an error.
Existing files are never rewritten; all existing bytes are compared, even after digest
verification. Cryptographic addressing does not prove collisions impossible. Temporary cleanup
is attempted on errors; a crash can leave a temporary file. Cleanup or verification errors after
installation can report failure with the blob already installed; retry is safe.

The authorized root and its namespace must be protected against hostile concurrent filesystem
mutation. Static links/reparse points, non-regular blobs and containment violations are rejected,
but portable safe std lacks directory-relative no-follow traversal to close directory replacement
races. Windows blob opens additionally use `FILE_FLAG_OPEN_REPARSE_POINT`. A writer allowed to
change stored bytes can still change them after a verified read; readers receive their verified
snapshot. Neither this API nor SHA-256 authenticates a hostile writer.

No directory crash durability, garbage collection or worker is implemented in `storage`;
revision publication is the separate `revision` module below and recovery is U07; the CLI
byte commands are U04.

SHA-256 is original code implementing FIPS PUB 180-4 (August 2015), sections 4.1.2, 4.2.2,
5 and 6.2: https://doi.org/10.6028/NIST.FIPS.180-4. NIST examples and .NET-derived boundary
vectors test it independently. The older fixture-tool hasher remains separate and untouched.

## U06 revision commit

`beamcraft_host::revision` commits and reads one complete revision; see the
[design record](../../docs/design/u06-revision-commit.md). A revision is a canonical
`Manifest` blob (identities, base, schema digest, optional verified U03 index record, sorted
layer entries with full digests and sizes) stored through `BlobStore`. The root is an
append-only head log `revisions/<20-digit sequence>`: each record is a complete synced file
installed by a non-replacing hard link, so the root switches in one step and the link is also
the compare-and-swap for the expected base.

- `RevisionStore::commit(&draft, &expected)` / `commit_with(.., verifier, hook)`.
- `RevisionStore::open()` verifies record, manifest, base link and every layer blob and
  refuses missing, corrupt, resized or torn state with `RevisionError` (`code()` maps to F04).
- `ContentVerifier` is the injected content gate (layer engine, U03 evidence); `CommitHook`
  marks `Checked`, `BlobsVerified`, `ManifestStored`, `RecordStaged`, `Switched`: abort before
  `Switched` leaves the root unchanged, a failure at `Switched` is `OutcomeUnknown`. U07 uses it.
- `beamcraft revision-commit` / `revision-open` are the headless commands.

Not promised: directory-entry crash durability, cleanup of `.tmp-*` files or unreferenced blobs
after a crash, garbage collection, protection from a hostile writer to the store directory.
Not implemented: U07 journal/recovery, personal workspaces, collaboration, a CLI verifier.

## U04 experimental process client

`dispatch::Dispatcher` is shared by CLI and native callers; constructing one starts nothing.
`AuthorizedRead::authorize` requires a protected U05 store, an independently selected full
identity and an explicit owned-layer grant. It verifies manifest/asset bytes and publishes a
small host grant. `BlobStore::read_capped` tightens a single read before reservation.
Manifest/grant reads are capped at 1 MiB; individual layer reads at 64 KiB, with U01's
8-layer/256 KiB aggregate profile. These are byte-storage operations, not U06 publication.

`worker::Session` verifies the capped executable against trusted package configuration,
clears its environment, opens private stdio pipes and validates the four-message handshake.
Only the Windows OS `SystemRoot` is retained; no shell, Python or SDK process is invoked.
One serialized request, bounded pipe channels, independent assembly/request deadlines,
in-flight ping/pong, cancellation and capped diagnostic drain protect the host boundary.
Raw diagnostics are discarded. Stop confirms child exit and pipe-task completion before a
replacement; unconfirmed reaping blocks replacement. Hashing checks the host deadline by chunk.

Only Open/Read/Close and an explicitly uncertain host QueryOutcome are registered. Errors
carry the host-known workspace identity. Read pages are checked for closed fields, counts,
identity/path consistency, value shapes/numbers, depth, source digest/ownership, duplicate
paths and complete delivery counts. A read failure can reconstruct the exact authorized
input at its nonzero generation, with at most two delayed restarts. Continuations from a
lost incarnation are stale. No mutation intent is sent, erased, retried or accepted.
An incarnation-specific supervision handle can force child loss while a request is pending.

This does not close F05 or LEW-03: OS memory/work containment, guaranteed random-byte tokens,
idle keepalive, complete negotiated profiles, mutation/event schemas, canonical replay and
durable outcomes remain open. The standard-library token source is explicitly experimental.
The protected filesystem/package namespaces remain caller prerequisites. See the
[review and prepared draft PR body](../../docs/design/u04-cli-host.md).
