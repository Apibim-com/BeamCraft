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

No root revision transaction, directory crash durability, garbage collection or worker is
implemented here. U06/U07 own revision publication and recovery; the CLI command is U04.

SHA-256 is original code implementing FIPS PUB 180-4 (August 2015), sections 4.1.2, 4.2.2,
5 and 6.2: https://doi.org/10.6028/NIST.FIPS.180-4. NIST examples and .NET-derived boundary
vectors test it independently. The older fixture-tool hasher remains separate and untouched.
