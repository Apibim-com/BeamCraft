// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! Pinned read queries (F06 section 9): snapshot, bounds, mesh blobs, deltas and selection.
//!
//! Implemented: `ListElements`, `GetMeshSnapshot` (blobs by digest), `QueryBounds` (element and
//! all, world space), `GetChanges` and the selection API. Not implemented: `LookupFeature` and
//! `Pick`, which need kernel feature geometry (G04), and local-space bounds and keys, which need
//! the dependency keys (planned). Nothing here is a mock of those. [`committed`] (P01) turns the
//! authored plates of a committed revision into the snapshot builder's input.

pub mod bounds;
pub mod bundle;
pub mod changes;
pub mod committed;
pub mod mesh;
pub mod selection;
pub mod snapshot;
