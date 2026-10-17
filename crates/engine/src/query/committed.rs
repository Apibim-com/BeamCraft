// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! Read side of P01: from the authored plates of a committed revision to the view contract.
//!
//! The adapter that reads USD hands over [`AuthoredPlate`] records (typed, decoded from the
//! committed model layer). This module re-evaluates each one with the same kernel path that
//! `CreatePlate` used, so a viewer receives genuine kernel meshes, quantities and a pinned
//! [`crate::ViewSnapshot`] built from committed data, never from a mock.

use crate::command::{AuthoredPlate, PlateBody, Quantities, evaluate_plate};
use crate::element::ElementKind;
use crate::scene::SceneElement;
use beamcraft_geom::Translation3;
use beamcraft_model::command::{CommandError, label_is_valid};
use beamcraft_model::error::ErrorCode;
use beamcraft_model::id::ElementId;
use std::collections::BTreeSet;

/// A committed plate with its re-evaluated body.
#[derive(Debug, Clone, PartialEq)]
pub struct PlateRecord {
    /// The authored state read from the committed model layer.
    pub authored: AuthoredPlate,
    /// The body the kernel evaluates from it.
    pub body: PlateBody,
    /// The quantity records of that body.
    pub quantities: Quantities,
}

/// Re-evaluates committed authored plates, ordered by element id.
///
/// An element id listed twice is `DUPLICATE_ID`. A plate that no longer evaluates (a model edited
/// outside BeamCraft, for example) is rejected with the evaluation's own code and no partial
/// list. The label rule is re-checked because the layer is untrusted input.
pub fn evaluate_committed(plates: &[AuthoredPlate]) -> Result<Vec<PlateRecord>, CommandError> {
    let mut seen = BTreeSet::new();
    let mut records = Vec::with_capacity(plates.len().min(crate::limits::MAX_ELEMENTS));
    for authored in plates {
        if !seen.insert(authored.id) {
            return Err(CommandError::new(
                ErrorCode::DuplicateId,
                "id",
                "element id appears twice",
            ));
        }
        if !label_is_valid(&authored.name) {
            return Err(CommandError::new(
                ErrorCode::InvalidArgument,
                "name",
                "committed label is not valid",
            ));
        }
        authored.placement.validate()?;
        let body = evaluate_plate(authored)?;
        let quantities = body.quantities();
        records.push(PlateRecord {
            authored: authored.clone(),
            body,
            quantities,
        });
    }
    records.sort_by_key(|r| r.authored.id);
    Ok(records)
}

/// The snapshot-builder input for committed plates. Each placement must be a pure translation
/// (identity rotation); another rotation is `UNSUPPORTED_GEOMETRY`, because the view contract
/// carries translations only.
pub fn scene_elements(records: &[PlateRecord]) -> Result<Vec<SceneElement>, CommandError> {
    records
        .iter()
        .map(|r| {
            if !r.authored.placement.is_identity_rotation() {
                return Err(CommandError::new(
                    ErrorCode::UnsupportedGeometry,
                    "placement.orientation_xyzw",
                    "rotated placement is not evaluated in this slice",
                ));
            }
            let [x, y, z] = r.authored.placement.translation_mm;
            let placement = Translation3::new(x, y, z).map_err(|_| {
                CommandError::new(
                    ErrorCode::NonFiniteNumber,
                    "placement.translation_mm",
                    "value is not finite",
                )
            })?;
            Ok(SceneElement::new(
                r.authored.id,
                ElementKind::Plate,
                placement,
                r.body.result.clone(),
            ))
        })
        .collect()
}

/// The ids of committed plates, for the `DUPLICATE_ID` check of a new command.
pub fn ids(records: &[PlateRecord]) -> BTreeSet<ElementId> {
    records.iter().map(|r| r.authored.id).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use beamcraft_model::command::Placement;

    fn plate(n: u8, width: f64, x: f64) -> AuthoredPlate {
        AuthoredPlate {
            id: ElementId::parse(&format!("019a7bf0-0000-7000-8000-0000000000{n:02x}")).unwrap(),
            name: format!("Plate {n}"),
            contour_points_mm: vec![
                [0.0, 0.0, 0.0],
                [width, 0.0, 0.0],
                [width, 500.0, 0.0],
                [0.0, 500.0, 0.0],
            ],
            thickness_mm: 10.0,
            placement: Placement {
                translation_mm: [x, 0.0, 0.0],
                ..Placement::IDENTITY
            },
        }
    }

    #[test]
    fn committed_plates_are_evaluated_sorted_and_become_a_scene() {
        let records =
            evaluate_committed(&[plate(9, 800.0, 0.0), plate(5, 1000.0, 2000.0)]).unwrap();
        assert_eq!(records.len(), 2);
        assert!(records[0].authored.id < records[1].authored.id);
        assert_eq!(records[0].quantities.volume_mm3, 5_000_000.0);
        assert_eq!(records[1].quantities.volume_mm3, 4_000_000.0);
        let scene = scene_elements(&records).unwrap();
        assert_eq!(scene.len(), 2);
        assert_eq!(scene[0].id(), records[0].authored.id);
        assert_eq!(ids(&records).len(), 2);
    }

    #[test]
    fn duplicates_bad_labels_and_unsupported_rotation_are_rejected_without_a_partial_list() {
        assert_eq!(
            evaluate_committed(&[plate(5, 1000.0, 0.0), plate(5, 800.0, 0.0)])
                .unwrap_err()
                .code,
            ErrorCode::DuplicateId
        );
        let mut bad = plate(5, 1000.0, 0.0);
        bad.name = "line\nbreak".into();
        assert_eq!(
            evaluate_committed(&[bad]).unwrap_err().code,
            ErrorCode::InvalidArgument
        );
        let mut rotated = plate(5, 1000.0, 0.0);
        let s = core::f64::consts::FRAC_1_SQRT_2;
        rotated.placement.orientation_xyzw = [0.0, 0.0, s, s];
        let records = evaluate_committed(&[rotated]).unwrap();
        assert_eq!(
            scene_elements(&records).unwrap_err().code,
            ErrorCode::UnsupportedGeometry
        );
        let mut broken = plate(5, 1000.0, 0.0);
        broken.thickness_mm = -1.0;
        assert!(evaluate_committed(&[broken]).is_err());
    }
}
