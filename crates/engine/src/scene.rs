// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! The input of the snapshot builder: elements with sealed kernel results.
//!
//! P01 will derive these from authored model elements. Until then a caller supplies the kernel
//! plate results directly (the only body the kernel builds today). The builder trusts neither the
//! caller's `kind` nor any mesh: it re-validates every blob.

use crate::element::{ElementId, ElementKind};
use beamcraft_geom::Translation3;
use beamcraft_kernel::PlateResult;

/// One element offered to the snapshot builder.
///
/// `full` is the representation with all operations applied; there are no operations in the
/// kernel yet, so `full` is the plain primitive. `simplified` is the optional lightweight
/// stand-in. When it is absent the part has no operations and one mesh serves both
/// representations. A caller-supplied `simplified` is flagged not exact by the snapshot.
#[derive(Debug, Clone, PartialEq)]
pub struct SceneElement {
    pub(crate) id: ElementId,
    pub(crate) kind: ElementKind,
    pub(crate) placement: Translation3,
    pub(crate) full: PlateResult,
    pub(crate) simplified: Option<PlateResult>,
}

impl SceneElement {
    /// An element whose kernel body is `full`, placed by a translation (the only placement of
    /// the first scope). A non-plate `kind` carries the plate body as a stand-in until P01.
    pub const fn new(
        id: ElementId,
        kind: ElementKind,
        placement: Translation3,
        full: PlateResult,
    ) -> Self {
        Self {
            id,
            kind,
            placement,
            full,
            simplified: None,
        }
    }

    /// Adds a distinct simplified representation.
    #[must_use]
    pub fn with_simplified(mut self, simplified: PlateResult) -> Self {
        self.simplified = Some(simplified);
        self
    }

    /// The element identity.
    pub const fn id(&self) -> ElementId {
        self.id
    }

    /// The element kind.
    pub const fn kind(&self) -> ElementKind {
        self.kind
    }
}
