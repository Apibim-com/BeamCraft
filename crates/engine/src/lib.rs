// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! The BeamCraft engine API. V02 adds the renderer-agnostic view snapshot and query contract.
//!
//! The crate holds pure, bounded, deterministic data and no UI, wgpu, egui, I/O or clock. A
//! renderer receives meshes by digest, instance and triangle tables, ids and bounds; it computes
//! no engineering geometry. P01 (command application, authored elements, dependency keys) is not
//! implemented: the builder takes sealed kernel plate results directly, see [`scene`].

pub mod element;
pub mod error;
pub mod limits;
pub mod presentation;
pub mod query;
pub mod scene;
pub mod synthetic;

pub use element::{ElementId, ElementKind, KindFilter};
pub use error::{ViewCode, ViewError};
pub use limits::Limits;
pub use presentation::{ProjectedElement, Representation, ViewProjection, ViewSettings};
pub use query::bounds::Aabb;
pub use query::changes::{Change, ChangeKind, ChangeSet, ChangedFields, SnapshotHistory, diff};
pub use query::mesh::MeshEntry;
pub use query::selection::Selection;
pub use query::snapshot::{
    ElementEntry, ElementPage, RepresentationRef, SnapshotToken, ViewPin, ViewSnapshot,
};
pub use scene::SceneElement;
