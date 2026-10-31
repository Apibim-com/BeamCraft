// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! A sealed plate as an operation body.

use crate::ops::body::unit;
use crate::ops::diagnostic::OpFailure;
use crate::ops::{Body, OpField};
use crate::steel::PlateResult;
use crate::trimesh::TriMesh;

impl Body {
    /// Admits a sealed G02 plate with its six local ids and outward normals.
    pub fn from_plate(plate: &PlateResult) -> Result<Self, OpFailure> {
        let mesh = TriMesh::from_blob(plate.mesh().blob())
            .map_err(|e| OpFailure::from_mesh(e, OpField::Body))?;
        let ids = plate.mesh().face_table().iter().map(|s| (*s).to_owned());
        let normals = plate.mesh().face_normals().iter().map(|n| Some(unit(*n)));
        Self::from_mesh(
            mesh,
            ids.collect(),
            normals.collect(),
            Some(plate.tolerance()),
        )
    }
}
