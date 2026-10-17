// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! `UpdateElement` for the supported fields (F04 section 7, P04): a typed patch of the label, the
//! material reference and a plate's thickness.
//!
//! The engine applies the patch to the authored state, re-evaluates what depends on it and says
//! which groups changed, so the adapter writes only those:
//!
//! | Field | Rebuilds the body | Rebuilds the mass | Derived layer |
//! |-------|-------------------|-------------------|---------------|
//! | `name` | no | no | untouched |
//! | `material` | no | yes | quantity record only (same mesh bytes) |
//! | `thickness_mm` | yes | yes | mesh and quantity record |
//!
//! Fields outside this list (contour, axis, roll, profile, placement) are
//! `UNSUPPORTED_PROPERTY`: they belong to later tasks.

use crate::command::AuthoredPlate;
use crate::member::AuthoredMember;
use beamcraft_model::command::{CommandError, ElementPatch};
use beamcraft_model::error::ErrorCode;

/// Which groups of an element a patch changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UpdateEffect {
    /// The label changes.
    pub name: bool,
    /// The material reference changes (including its removal).
    pub material: bool,
    /// A shape input changes, so the body is rebuilt.
    pub shape: bool,
}

/// The effect of a patch on the element it targets, ignoring whether a value really differs:
/// an update that sets the current value is still that update.
pub const fn effect(patch: &ElementPatch) -> UpdateEffect {
    UpdateEffect {
        name: patch.name.is_some(),
        material: patch.material.is_some(),
        shape: patch.thickness_mm.is_some(),
    }
}

/// Applies a patch to an authored plate.
pub fn patch_plate(
    plate: &AuthoredPlate,
    patch: &ElementPatch,
) -> Result<AuthoredPlate, CommandError> {
    let mut next = plate.clone();
    if let Some(name) = &patch.name {
        next.name.clone_from(name);
    }
    if let Some(material) = patch.material {
        next.material = material;
    }
    if let Some(thickness) = patch.thickness_mm {
        next.thickness_mm = thickness;
    }
    Ok(next)
}

/// Applies a patch to an authored member. A member has no thickness: that field is
/// `UNSUPPORTED_PROPERTY`.
pub fn patch_member(
    member: &AuthoredMember,
    patch: &ElementPatch,
) -> Result<AuthoredMember, CommandError> {
    if patch.thickness_mm.is_some() {
        return Err(CommandError::new(
            ErrorCode::UnsupportedProperty,
            "thickness_mm",
            "a member has no thickness",
        ));
    }
    let mut next = member.clone();
    if let Some(name) = &patch.name {
        next.name.clone_from(name);
    }
    if let Some(material) = patch.material {
        next.material = material;
    }
    Ok(next)
}

#[cfg(test)]
mod tests {
    use super::*;
    use beamcraft_model::command::{MemberKind, Placement};
    use beamcraft_model::digest::Sha256;
    use beamcraft_model::id::ElementId;

    fn plate() -> AuthoredPlate {
        AuthoredPlate {
            id: ElementId::parse("019a7bf0-0000-7000-8000-000000000005").unwrap(),
            name: "Plate".into(),
            contour_points_mm: vec![
                [0.0, 0.0, 0.0],
                [1000.0, 0.0, 0.0],
                [1000.0, 500.0, 0.0],
                [0.0, 500.0, 0.0],
            ],
            thickness_mm: 10.0,
            placement: Placement::IDENTITY,
            material: None,
            holes_mm: Vec::new(),
        }
    }

    #[test]
    fn the_effect_names_the_groups_a_patch_touches() {
        let rename = ElementPatch {
            name: Some("B".into()),
            ..ElementPatch::default()
        };
        assert_eq!(
            effect(&rename),
            UpdateEffect {
                name: true,
                material: false,
                shape: false
            }
        );
        let clear = ElementPatch {
            material: Some(None),
            ..ElementPatch::default()
        };
        assert!(effect(&clear).material && !effect(&clear).shape);
    }

    #[test]
    fn a_patch_changes_only_its_fields_and_distinguishes_clearing_the_material() {
        let steel = Sha256::digest(b"steel").unwrap();
        let set = patch_plate(
            &plate(),
            &ElementPatch {
                material: Some(Some(steel)),
                thickness_mm: Some(12.0),
                ..ElementPatch::default()
            },
        )
        .unwrap();
        assert_eq!(set.material, Some(steel));
        assert_eq!(set.thickness_mm, 12.0);
        assert_eq!(set.name, "Plate");
        let cleared = patch_plate(
            &set,
            &ElementPatch {
                material: Some(None),
                ..ElementPatch::default()
            },
        )
        .unwrap();
        assert_eq!(cleared.material, None);
        assert_eq!(cleared.thickness_mm, 12.0);
        // Absence leaves the material alone.
        let same = patch_plate(
            &set,
            &ElementPatch {
                name: Some("N".into()),
                ..ElementPatch::default()
            },
        )
        .unwrap();
        assert_eq!(same.material, Some(steel));
    }

    #[test]
    fn a_member_has_no_thickness() {
        let profile = Sha256::digest(b"profile").unwrap();
        let member = AuthoredMember {
            id: ElementId::parse("019a7bf0-0000-7000-8000-000000000006").unwrap(),
            kind: MemberKind::Beam,
            name: "Beam".into(),
            axis_points_mm: [[0.0; 3], [1000.0, 0.0, 0.0]],
            profile,
            roll_rad: None,
            placement: Placement::IDENTITY,
            material: None,
        };
        assert_eq!(
            patch_member(
                &member,
                &ElementPatch {
                    thickness_mm: Some(3.0),
                    ..ElementPatch::default()
                }
            )
            .unwrap_err()
            .code,
            ErrorCode::UnsupportedProperty
        );
        let renamed = patch_member(
            &member,
            &ElementPatch {
                name: Some("Beam 2".into()),
                ..ElementPatch::default()
            },
        )
        .unwrap();
        assert_eq!(renamed.name, "Beam 2");
        assert_eq!(renamed.profile, profile);
    }
}
