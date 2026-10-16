# beamcraft-engine

Contributor-original BeamCraft code and documentation, MPL-2.0.

V02 adds the renderer-agnostic **view snapshot and query contract** to the engine. The crate
is pure, bounded and deterministic. It has no UI, wgpu, egui, I/O, clock or third-party
dependency; it depends on `geom`, `model` and `kernel` only. A renderer receives mesh blobs by
digest, instance and triangle tables, ids and bounds, and computes no engineering geometry
(F06 section 9).

## What is implemented

| Area | API | Notes |
|------|-----|-------|
| Pinned snapshot | `ViewSnapshot::build(ViewPin, Vec<SceneElement>, &Limits)` | `ViewPin` is a revision digest and a workspace generation. Elements are ordered by `ElementId`; a duplicate id is rejected. |
| Revision token | `SnapshotToken` (`vs1-` + 64 hex) | Opaque, equality only. Covers the pin and the authoritative content (ids, kinds, placements, mesh digests). Never view settings or selection. |
| ID mapping | `ElementEntry::index`, `ViewSnapshot::{element, element_at}`; `ViewProjection::{instance_of, element_for_instance, element_for_triangle}` | Indices are snapshot-local positions and are never identities. A triangle or instance resolves to an `ElementId` only through a projection of one snapshot. |
| Bounds | `Aabb`; `ViewSnapshot::{element_bounds, engineering_bounds}`; `ViewProjection::bounds` | Computed by the engine from kernel vertices plus the translation placement. Display aid, never a volume. |
| Mesh buffers | `ViewSnapshot::{meshes, mesh, mesh_at}`; `MeshEntry` | Exact BCMESH01 bytes identified by SHA-256, stored once. Every blob is re-inspected (F06 section 6 header, caps, sizes, finite and non-negative-zero coordinates, zero padding, index ranges) and its digest recomputed. |
| Representations | `Representation::{Full, Simplified}` | A part with no operations references one mesh from both. A distinct simplified mesh is flagged not exact; `Representation::feeds_engineering()` is false for it, and `ViewSnapshot::engineering_representation` always returns the full one. |
| View settings | `ViewSettings { hidden_kinds: KindFilter, representation }`; `ViewSnapshot::project` | A read-time projection: it changes no token, no authored or derived data. It exposes the instance/triangle tables, the meshes a view must load (`required_meshes`) and what is hidden. |
| Incremental delta | `diff`, `SnapshotHistory::{push, changes_since}`; `ChangeSet` | Added/changed/removed in id order; `new_meshes` lists only blobs the consumer lacks, so a placement-only edit resends none. An unknown, expired or evicted token is `GEO_STALE_TOKEN` (`STALE_WORKSPACE`) with no partial list. History holds at most 8 snapshots. |
| `ListElements` | `ViewSnapshot::list_elements(after, page_size)` | Id-ordered, cursor is the last id, page size 1 to 1,024. |
| Selection | `Selection::{from_ids, from_instances, from_triangles, instances, rebased}` | Holds ids only; display hints are resolved through the projection and discarded. `rebased` enforces the destination snapshot's selection cap (`VIEW_SELECTION_LIMIT`). Selection is viewport state and never enters a key. |
| Errors | `ViewCode`, `ViewError` | 11 closed codes, `GEO_*` where F06 section 3 names one and `VIEW_*` otherwise, each mapped onto an existing F04 `ErrorCode`. |
| Headless | `beamcraft view-snapshot`, `beamcraft view-changes` | See the CLI README. |

## Limits

All caps are checked before the work they bound, with checked arithmetic and no
input-derived indexing. A caller may lower a limit with `Limits::new`, never raise it.

| Limit | Maximum | Error |
|-------|---------|-------|
| Elements per snapshot | 65,536 | `VIEW_ELEMENT_LIMIT` |
| Mesh blob | 213,024 bytes, 4,096 vertices, 8,192 triangles, 1,024 faces | `GEO_BUFFER_LIMIT`, `GEO_TOPOLOGY_LIMIT` |
| Distinct mesh bytes per snapshot | 64 MiB | `GEO_BUFFER_LIMIT` |
| Triangles per projection | 4,194,304 | `GEO_BUFFER_LIMIT` |
| Selection | 65,536 ids | `VIEW_SELECTION_LIMIT` |
| `ListElements` page | 1,024 | `VIEW_ARGUMENT_INVALID` |
| Retained snapshots | 8 | (older tokens are `GEO_STALE_TOKEN`) |

`GEO_NUMERIC_UNCERTAIN` is a defensive finite check in the bounds computation; typed kernel
inputs cannot reach it through `ViewSnapshot::build`.

## What is not implemented (and is not faked)

- **P01 is absent.** There are no authored elements, commands, dependency keys, quantity
  statuses or ElementId in `beamcraft_model::id`. The builder takes sealed kernel
  `PlateResult`s (the only body the kernel builds) with a caller-supplied `ElementKind` and a
  translation placement. Until P01, `ElementId` lives in `beamcraft_engine::element`; a non-plate
  kind carries a plate body as a stand-in. `synthetic` builds such scenes with the real kernel
  for the CLI and tests, and every command output marks them (`stand_ins`).
- **No operations exist yet** (A03/A04), so a distinct simplified mesh can only be a stand-in
  supplied by the caller. Measuring the load-time gain on the 10 000-part scene is a V01 task.
- **Welds and bolts are not registered kinds** (F10 vocabulary 0.1). The kind filter works on
  the 13 registered kinds and will cover welds and bolts when a vocabulary version adds them.
- **No geometric picking.** F06 `Pick` and `LookupFeature` need kernel feature geometry (G04).
  Selection by display hint (instance or triangle index from a GPU pick) is supported as a hint
  only; anything selecting for engineering use must come from the engine's `Pick`.
- **No keys, statuses or local-space bounds** (P01), no presentation-key preimage (V01/V03),
  no cancellation or deadline polling, no transport schema for deltas (U02/F05), no transform
  other than translation (the first-scope placement).
- **No mesh export from the CLI.** The CLI reports digests, sizes and counts; delivering blob
  bytes is an I/O-edge decision for the UI/host tasks.

## Tests

`cargo test -p beamcraft-engine` runs unit tests (ids, kinds, error table, limits, blob
inspection with every structural failure and cap) and `tests/view_snapshot.rs`, which uses the
real kernel for determinism, bounds, deduplication, settings, tables, paging, deltas, staleness,
selection and a negative test for each error and cap, including the 65,536-element hard cap.
