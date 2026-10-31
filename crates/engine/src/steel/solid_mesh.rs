// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! The meshes of the steel bodies: a member of a steel section, a ruled loft and a sweep, as
//! [`SolidMesh`].

use crate::solid::SolidMesh;
use beamcraft_kernel::steel::section::SectionResult;
use beamcraft_kernel::steel::sweep::SweepResult;

impl SolidMesh {
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

    /// The mesh of a G12 sweep (a P12 polybeam); the volume is the midpoint of the enclosure of
    /// the certified signed-volume sum, which the seal checked against the analytic truncated
    /// prism value.
    pub fn from_sweep(result: &SweepResult) -> Self {
        let mesh = result.body().mesh();
        Self {
            vertices: mesh.vertices().to_vec(),
            triangles: mesh.triangles().to_vec(),
            face_table: result.body().face_table().to_vec(),
            blob: result.blob().to_vec(),
            digest: result.mesh_digest(),
            volume_mm3: result.volume_mm3().midpoint(),
        }
    }
}
