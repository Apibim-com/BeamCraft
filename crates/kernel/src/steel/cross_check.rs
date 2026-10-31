// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! The independent mesh cross-check of a sealed plate (G04 section 2 and 3.3).

use crate::enclosure::Enclosure;
use crate::measure::{QuantityStatus, measure};
use crate::steel::PlateResult;
use crate::trimesh::TriMesh;
use crate::verdict::{MeshError, ShellStatus};

/// Independent mesh cross-check of a sealed G02 plate (G04 section 2 and 3.3).
#[derive(Debug, Clone, PartialEq)]
pub struct PlateCrossCheck {
    /// Shell verdict of the plate's own BCMESH01 blob.
    pub shell: ShellStatus,
    /// Analytic volume `(width*length)*thickness` in mm3.
    pub analytic_volume_mm3: f64,
    /// Status of the analytic volume: `current`, as the builder sealed it.
    pub analytic_status: QuantityStatus,
    /// Analytic enclosure `[V - r, V + r]` with `r = g(3)/(1-g(3)) * V`, rounded outward.
    pub analytic_enclosure_mm3: Enclosure,
    /// Exact-sum mesh enclosure of the blob's volume, when the shell certified.
    pub mesh_volume_mm3: Option<Enclosure>,
    /// True when the shell certified and the two enclosures overlap.
    pub agrees: bool,
}

/// Re-reads a plate's blob through the generic validator and compares volumes.
///
/// The analytic bound is G04 section 3.2's `g(3)` for `((a*b)*c)`, relative to the
/// ideal value; the mesh enclosure comes from exact integer sums. The two must
/// overlap. This is a consistency gate, not a replacement for the builder.
pub fn cross_check_plate(plate: &PlateResult) -> Result<PlateCrossCheck, MeshError> {
    let mesh = TriMesh::from_blob(plate.mesh().blob())?;
    let analysis = crate::topology::analyze_with_tolerance(
        &mesh,
        plate.tolerance(),
        crate::topology::Limits::DEFAULT,
        |_| false,
    )?;
    let measurement = measure(&mesh, &analysis)?;
    let v = plate.volume_mm3();
    let u = 1.110_223_024_625_156_5e-16_f64;
    let g3 = (3.0 * u / (1.0 - 3.0 * u)).next_up();
    let radius = ((g3 / (1.0 - g3)) * v.abs()).next_up();
    let analytic = Enclosure {
        lo: (v - radius).next_down(),
        hi: (v + radius).next_up(),
    };
    let mesh_volume = measurement.volume_mm3();
    Ok(PlateCrossCheck {
        shell: analysis.status(),
        analytic_volume_mm3: v,
        analytic_status: QuantityStatus::Current,
        analytic_enclosure_mm3: analytic,
        mesh_volume_mm3: mesh_volume,
        agrees: mesh_volume.is_some_and(|m| m.overlaps(analytic)),
    })
}
