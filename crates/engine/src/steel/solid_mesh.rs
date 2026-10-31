// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! The meshes of the steel bodies: a sealed box (a plate or a member), a member of a steel
//! section, and a ruled loft, as [`SolidMesh`] and [`SceneMesh`].

use crate::scene::SceneMesh;
use crate::solid::SolidMesh;
use beamcraft_kernel::steel::PlateResult;
use beamcraft_kernel::steel::section::SectionResult;
use std::sync::Arc;

impl SolidMesh {
    /// The mesh of a sealed box.
    pub fn from_box(result: &PlateResult) -> Self {
        let mesh = result.mesh();
        Self {
            vertices: mesh.vertices().iter().map(|p| [p.x, p.y, p.z]).collect(),
            triangles: mesh.triangles().to_vec(),
            face_table: mesh
                .face_table()
                .iter()
                .map(|id| (*id).to_owned())
                .collect(),
            blob: mesh.blob().to_vec(),
            digest: mesh.mesh_digest(),
            volume_mm3: result.volume_mm3(),
        }
    }

    /// The mesh of a straight member of an A07.1 section; the volume is the analytic
    /// `net_area * axis_length` of the extruded outline.
    pub fn from_section(result: &SectionResult) -> Self {
        let mesh = result.body().mesh();
        Self {
            vertices: mesh.vertices().to_vec(),
            triangles: mesh.triangles().to_vec(),
            face_table: result.body().face_table().to_vec(),
            blob: result.blob().to_vec(),
            digest: result.mesh_digest(),
            volume_mm3: result.volume_mm3(),
        }
    }

    /// The mesh of a G13 ruled loft (an A11 lofted plate); the volume is the midpoint of the
    /// enclosure of the certified signed-volume sum.
    pub fn from_loft(result: &beamcraft_kernel::steel::ruled::LoftResult) -> Self {
        Self {
            vertices: result.body().mesh().vertices().to_vec(),
            triangles: result.body().mesh().triangles().to_vec(),
            face_table: result.body().face_table().to_vec(),
            blob: result.blob().to_vec(),
            digest: result.mesh_digest(),
            volume_mm3: result.volume_mm3().midpoint(),
        }
    }
}

impl From<&PlateResult> for SceneMesh {
    fn from(result: &PlateResult) -> Self {
        let mesh = result.mesh();
        Self {
            blob: Arc::from(mesh.blob().as_slice()),
            digest: mesh.mesh_digest(),
            vertices: Arc::from(mesh.vertices().as_slice()),
            face_table: Arc::from(
                mesh.face_table()
                    .iter()
                    .map(|id| (*id).to_owned())
                    .collect::<Vec<_>>(),
            ),
        }
    }
}

impl From<PlateResult> for SceneMesh {
    fn from(result: PlateResult) -> Self {
        Self::from(&result)
    }
}
