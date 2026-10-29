// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! Bounded interactive contour drafts. Only the engine decides geometric admission.

use crate::tools::FormError;
use beamcraft_engine::work_plane::WorkPlane;
use beamcraft_model::limits::{CONTOUR_POINT_CAP, COORDINATE_CAP_MM};

/// Unpublished world points of one plate contour, in the picked order.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ContourDraft {
    points: Vec<[f64; 3]>,
}

impl ContourDraft {
    /// World vertices for the draft overlay.
    pub fn points(&self) -> &[[f64; 3]] {
        &self.points
    }

    /// Add one finite point without allocating beyond the engine's contour budget.
    pub fn push(&mut self, point: [f64; 3]) -> Result<(), FormError> {
        if self.points.len() >= CONTOUR_POINT_CAP {
            return Err(FormError {
                field: "Contour",
                message: "The contour has reached its vertex limit.".into(),
            });
        }
        if point
            .iter()
            .any(|p| !p.is_finite() || p.abs() > COORDINATE_CAP_MM)
        {
            return Err(FormError {
                field: "Point",
                message: "Choose a finite point inside the model limits.".into(),
            });
        }
        self.points.push(point);
        Ok(())
    }

    /// Undo the last draft vertex without touching model history.
    pub fn back(&mut self) {
        self.points.pop();
    }

    /// Explicitly change the picked contour's direction without changing its vertex positions.
    pub fn reverse(&mut self) {
        self.points.reverse();
    }

    /// Discard an unfinished contour; this runs no command.
    pub fn clear(&mut self) {
        self.points.clear();
    }

    /// Convert the draft through the engine frame for the existing local-contour CLI.
    /// Preserve winding, topology and coordinates for engine validation; repair nothing.
    pub fn contour(&self, plane: Option<WorkPlane>) -> Result<String, FormError> {
        if self.points.len() < 3 {
            return Err(FormError {
                field: "Contour",
                message: "Pick at least three vertices.".into(),
            });
        }
        self.points
            .iter()
            .map(|point| {
                let [x, y, z] = match plane {
                    Some(p) => p.to_local(*point).map_err(|e| FormError {
                        field: "Work plane",
                        message: e.message.into(),
                    })?,
                    None => *point,
                };
                if z.abs() > 1e-6 {
                    return Err(FormError {
                        field: "Point",
                        message: "All vertices must lie on the active work plane.".into(),
                    });
                }
                Ok(format!("{x},{y}"))
            })
            .collect::<Result<Vec<_>, _>>()
            .map(|points| points.join(";"))
    }
}
