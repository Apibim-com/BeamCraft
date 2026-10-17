// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! F04 command application for `CreateStraightMember` (P03): the member counterpart of
//! [`crate::command::create_plate`].
//!
//! A member is a beam or a column with a required pinned rectangle profile and an optional pinned
//! material. The engine never touches USD: it validates the command, resolves the pinned
//! definitions through caller lookups, asks the kernel for the body (the section is centred on
//! the axis, width along the rolled `X_m`, height along `Y_m`; F06 sections 2, 4 and 6) and
//! returns the authored state, the body, the quantities and the declared footprint.
//!
//! First-slice limits, none of which is guessed: only the `rectangle` profile exists, and a
//! placement rotation other than the identity is `UNSUPPORTED_GEOMETRY` (the view contract
//! carries translations only), so a beam is laid out through its own axis points.

use crate::command::{MaterialInfo, MaterialLookup, Quantities, quantities_with_material};
use crate::footprint::Footprint;
use crate::transaction::PriorOp;
use beamcraft_geom::Point3;
use beamcraft_kernel::diagnostic::{Failure, Field};
use beamcraft_kernel::{PlateResult, StraightMember, straight_member};
use beamcraft_model::command::{CommandError, CreateStraightMember, MemberKind, Placement};
use beamcraft_model::definition::RectangleProfileSpec;
use beamcraft_model::digest::ContentDigest;
use beamcraft_model::error::ErrorCode;
use beamcraft_model::id::{ElementId, OpId};

/// Version label of the volume algorithm in a member's [`Quantities`] record.
pub const MEMBER_VOLUME_ALGORITHM: &str = "kernel-p03-straight-member-volume/1;tolerance-policy-3";

/// The content of a resolved immutable profile definition.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProfileInfo {
    /// Section width in millimetres.
    pub width_mm: f64,
    /// Section height in millimetres.
    pub height_mm: f64,
}

/// Resolves a pinned profile digest. A digest with no definition is `NOT_FOUND`; a digest that
/// names a definition of another kind is `INVALID_ARGUMENT`.
pub type ProfileLookup<'a> = &'a dyn Fn(&ContentDigest) -> Result<ProfileInfo, CommandError>;

/// A lookup that knows no definition: any pinned profile is `NOT_FOUND`.
pub fn no_profiles(_: &ContentDigest) -> Result<ProfileInfo, CommandError> {
    Err(CommandError::new(
        ErrorCode::NotFound,
        "profile_digest",
        "profile definition is not in the model",
    ))
}

/// The authored state of one member: exactly the inputs F04 section 4 stores in the model layer.
#[derive(Debug, Clone, PartialEq)]
pub struct AuthoredMember {
    /// Full element identity.
    pub id: ElementId,
    /// Beam or column.
    pub kind: MemberKind,
    /// Label.
    pub name: String,
    /// Element-local axis points `p1`, `p2`.
    pub axis_points_mm: [[f64; 3]; 2],
    /// Pinned profile definition.
    pub profile: ContentDigest,
    /// Explicit roll; `None` is zero.
    pub roll_rad: Option<f64>,
    /// Rigid placement as submitted.
    pub placement: Placement,
    /// Pinned material definition, if any.
    pub material: Option<ContentDigest>,
}

impl AuthoredMember {
    /// The authored fields of an accepted command.
    pub fn from_command(command: &CreateStraightMember) -> Self {
        Self {
            id: command.new_id,
            kind: command.kind,
            name: command.name.clone(),
            axis_points_mm: command.axis_points_mm,
            profile: command.profile_digest,
            roll_rad: command.roll_rad,
            placement: command.placement,
            material: command.material_digest,
        }
    }

    /// The `CreateStraightMember` that would author exactly this state.
    pub fn to_command(&self) -> CreateStraightMember {
        CreateStraightMember {
            new_id: self.id,
            kind: self.kind,
            name: self.name.clone(),
            axis_points_mm: self.axis_points_mm,
            profile_digest: self.profile,
            placement: self.placement,
            roll_rad: self.roll_rad,
            material_digest: self.material,
        }
    }
}

/// A member evaluated by the kernel: the sealed body and its section.
#[derive(Debug, Clone, PartialEq)]
pub struct MemberBody {
    /// Sealed kernel result (mesh, volume, tolerance).
    pub result: PlateResult,
    /// Section width in millimetres.
    pub width_mm: f64,
    /// Section height in millimetres.
    pub height_mm: f64,
}

fn map_kernel(failure: &Failure) -> CommandError {
    failure.diagnostics().next().map_or(
        CommandError::new(
            ErrorCode::UnsupportedGeometry,
            "geometry",
            "geometry evaluation failed",
        ),
        |d| {
            let field = match d.field {
                Field::Axis => "axis_points_mm",
                Field::Section => "profile",
                Field::Roll => "roll_rad",
                _ => "geometry",
            };
            CommandError::new(d.code.f04_code(), field, "geometry evaluation failed")
        },
    )
}

/// Evaluates an authored member with its resolved profile. The profile values are re-admitted
/// because a committed definition is untrusted input.
pub fn evaluate_member(
    authored: &AuthoredMember,
    profile: ProfileInfo,
) -> Result<MemberBody, CommandError> {
    RectangleProfileSpec {
        width_mm: profile.width_mm,
        height_mm: profile.height_mm,
    }
    .validate()?;
    let [p1, p2] = authored.axis_points_mm;
    let result = straight_member(StraightMember {
        start: Point3::new(p1[0], p1[1], p1[2]),
        end: Point3::new(p2[0], p2[1], p2[2]),
        width_mm: profile.width_mm,
        height_mm: profile.height_mm,
        roll_rad: authored.roll_rad,
    })
    .map_err(|failure| map_kernel(&failure))?;
    Ok(MemberBody {
        result,
        width_mm: profile.width_mm,
        height_mm: profile.height_mm,
    })
}

/// The quantity records of a member body with the physical material of the element.
pub fn member_quantities(
    body: &MemberBody,
    material: Option<MaterialInfo>,
) -> Result<Quantities, CommandError> {
    quantities_with_material(&body.result, MEMBER_VOLUME_ALGORITHM, material)
}

/// Everything one accepted `CreateStraightMember` produces for the layers and the host.
#[derive(Debug, Clone, PartialEq)]
pub struct CreateMemberOutcome {
    /// The authored state for the model layer.
    pub authored: AuthoredMember,
    /// The evaluated body for the derived layer.
    pub body: MemberBody,
    /// Quantity records.
    pub quantities: Quantities,
    /// Declared read and write sets.
    pub footprint: Footprint,
    /// Digest of the typed arguments, for replay and reuse decisions.
    pub fingerprint: ContentDigest,
}

/// Applies `CreateStraightMember` to an isolated draft in memory: argument admission, duplicate
/// id, the pinned profile (required) and material (optional) in that order, then geometry.
/// Nothing is published.
pub fn create_straight_member(
    command: &CreateStraightMember,
    existing: &dyn Fn(&ElementId) -> bool,
    profiles: ProfileLookup<'_>,
    materials: MaterialLookup<'_>,
) -> Result<CreateMemberOutcome, CommandError> {
    command.validate_arguments()?;
    if existing(&command.new_id) {
        return Err(CommandError::new(
            ErrorCode::DuplicateId,
            "new_id",
            "element id is already owned",
        ));
    }
    let profile = profiles(&command.profile_digest)?;
    let material = command
        .material_digest
        .as_ref()
        .map(materials)
        .transpose()?;
    let footprint = Footprint::for_create_member(
        command.new_id,
        command.profile_digest,
        command.material_digest,
    )?;
    let authored = AuthoredMember::from_command(command);
    let body = evaluate_member(&authored, profile)?;
    if !command.placement.is_identity_rotation() {
        return Err(CommandError::new(
            ErrorCode::UnsupportedGeometry,
            "placement.orientation_xyzw",
            "rotated placement is not evaluated in this slice",
        ));
    }
    let quantities = member_quantities(&body, material)?;
    Ok(CreateMemberOutcome {
        fingerprint: command.content_fingerprint()?,
        authored,
        body,
        quantities,
        footprint,
    })
}

/// One accepted member command, bound to its operation.
#[derive(Debug, Clone, PartialEq)]
pub struct MemberTransaction {
    /// The operation id bound to this application.
    pub op_id: OpId,
    /// Authored state, body, quantities, footprint and fingerprint.
    pub outcome: CreateMemberOutcome,
}

/// The decision of [`admit_create_member`].
#[derive(Debug, Clone, PartialEq)]
pub enum MemberAdmission {
    /// A new operation: apply it.
    Apply(Box<MemberTransaction>),
    /// The same operation with identical content was already accepted: change nothing.
    Replay,
}

/// Admits one `CreateStraightMember`: an already accepted `op_id` is decided first (identical
/// content is a replay, different content `OP_ID_REUSE`), a new one is validated and evaluated.
pub fn admit_create_member(
    op_id: OpId,
    command: &CreateStraightMember,
    prior: PriorOp,
    existing: &dyn Fn(&ElementId) -> bool,
    profiles: ProfileLookup<'_>,
    materials: MaterialLookup<'_>,
) -> Result<MemberAdmission, CommandError> {
    match prior {
        PriorOp::Accepted { fingerprint } => {
            if command.content_fingerprint()? == fingerprint {
                Ok(MemberAdmission::Replay)
            } else {
                Err(CommandError::new(
                    ErrorCode::OpIdReuse,
                    "op_id",
                    "operation id was accepted with different content",
                ))
            }
        }
        PriorOp::Unseen => Ok(MemberAdmission::Apply(Box::new(MemberTransaction {
            op_id,
            outcome: create_straight_member(command, existing, profiles, materials)?,
        }))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::no_materials;
    use crate::footprint::{Group, Target};
    use beamcraft_model::digest::Sha256;

    fn digest(text: &[u8]) -> ContentDigest {
        Sha256::digest(text).unwrap()
    }

    fn command(profile: ContentDigest) -> CreateStraightMember {
        CreateStraightMember {
            new_id: ElementId::parse("019a7bf0-0000-7000-8000-000000000007").unwrap(),
            kind: MemberKind::Beam,
            name: "Synthetic beam".into(),
            axis_points_mm: [[0.0; 3], [6000.0, 0.0, 0.0]],
            profile_digest: profile,
            placement: Placement::IDENTITY,
            roll_rad: None,
            material_digest: None,
        }
    }

    fn none(_: &ElementId) -> bool {
        false
    }

    fn rectangle(
        digest: &ContentDigest,
        known: ContentDigest,
    ) -> Result<ProfileInfo, CommandError> {
        if *digest == known {
            Ok(ProfileInfo {
                width_mm: 200.0,
                height_mm: 300.0,
            })
        } else {
            no_profiles(digest)
        }
    }

    #[test]
    fn a_beam_gets_its_volume_from_the_pinned_section_and_the_axis() {
        let profile = digest(b"rectangle 200x300");
        let lookup = |d: &ContentDigest| rectangle(d, profile);
        let out = create_straight_member(&command(profile), &none, &lookup, &no_materials).unwrap();
        // 200 x 300 mm section along a 6000 mm axis: 360,000,000 mm3 = 0.36 m3.
        assert_eq!(out.quantities.volume_mm3, 360_000_000.0);
        assert!((out.quantities.volume_m3 - 0.36).abs() <= 1e-12);
        assert_eq!(
            out.quantities.mass_status,
            crate::command::QuantityStatus::Unknown
        );
        assert_eq!(out.quantities.algorithm, MEMBER_VOLUME_ALGORITHM);
        assert_eq!(out.authored.to_command(), command(profile));
        assert_eq!(out.body.width_mm, 200.0);
        assert!(out.footprint.reads().iter().any(|e| {
            e.target == Target::Definition(profile) && e.group == Group::Relationship
        }));
    }

    #[test]
    fn a_pinned_material_with_density_gives_the_mass() {
        let profile = digest(b"rectangle 200x300");
        let steel = digest(b"steel");
        let profiles = |d: &ContentDigest| rectangle(d, profile);
        let materials = |d: &ContentDigest| {
            if *d == steel {
                Ok(MaterialInfo {
                    density_kg_m3: Some(7850.0),
                })
            } else {
                no_materials(d)
            }
        };
        let mut c = command(profile);
        c.material_digest = Some(steel);
        let out = create_straight_member(&c, &none, &profiles, &materials).unwrap();
        // 0.36 m3 * 7850 kg/m3 = 2826 kg.
        assert!((out.quantities.mass_kg.unwrap() - 2826.0).abs() <= 1e-9);
    }

    #[test]
    fn the_profile_is_required_and_resolved_before_geometry() {
        let profile = digest(b"rectangle 200x300");
        let lookup = |d: &ContentDigest| rectangle(d, profile);
        let mut c = command(digest(b"absent"));
        let code = |c: &CreateStraightMember| {
            create_straight_member(c, &none, &lookup, &no_materials)
                .unwrap_err()
                .code
        };
        assert_eq!(code(&c), ErrorCode::NotFound);
        c.axis_points_mm[1] = [0.0; 3];
        assert_eq!(code(&c), ErrorCode::NotFound);
        c.profile_digest = profile;
        assert_eq!(code(&c), ErrorCode::InvalidArgument); // zero axis, F04 section 3 (G-03)
        let mut c = command(profile);
        c.material_digest = Some(digest(b"absent material"));
        assert_eq!(code(&c), ErrorCode::NotFound);
    }

    #[test]
    fn rotation_ids_and_hostile_profiles_are_rejected_without_a_panic() {
        let profile = digest(b"rectangle 200x300");
        let lookup = |d: &ContentDigest| rectangle(d, profile);
        let mut c = command(profile);
        let s = core::f64::consts::FRAC_1_SQRT_2;
        c.placement.orientation_xyzw = [0.0, 0.0, s, s];
        assert_eq!(
            create_straight_member(&c, &none, &lookup, &no_materials)
                .unwrap_err()
                .code,
            ErrorCode::UnsupportedGeometry
        );
        let c = command(profile);
        let taken = |id: &ElementId| *id == c.new_id;
        assert_eq!(
            create_straight_member(&c, &taken, &lookup, &no_materials)
                .unwrap_err()
                .code,
            ErrorCode::DuplicateId
        );
        // A committed definition is untrusted: a profile outside the F04 caps never evaluates.
        for (w, h, expected) in [
            (0.0, 300.0, ErrorCode::InvalidArgument),
            (f64::NAN, 300.0, ErrorCode::NonFiniteNumber),
            (200.0, 2e6, ErrorCode::LimitExceeded),
        ] {
            let authored = AuthoredMember::from_command(&command(profile));
            let error = evaluate_member(
                &authored,
                ProfileInfo {
                    width_mm: w,
                    height_mm: h,
                },
            )
            .unwrap_err();
            assert_eq!(error.code, expected, "{w} x {h}");
        }
    }

    #[test]
    fn replay_and_reuse_follow_the_ledger() {
        let profile = digest(b"rectangle 200x300");
        let lookup = |d: &ContentDigest| rectangle(d, profile);
        let op = OpId::parse("019a7bf0-0000-7000-8000-000000000004").unwrap();
        let accepted = PriorOp::Accepted {
            fingerprint: command(profile).content_fingerprint().unwrap(),
        };
        assert_eq!(
            admit_create_member(
                op,
                &command(profile),
                accepted,
                &none,
                &lookup,
                &no_materials
            )
            .unwrap(),
            MemberAdmission::Replay
        );
        let mut changed = command(profile);
        changed.axis_points_mm[1][0] = 5000.0;
        assert_eq!(
            admit_create_member(op, &changed, accepted, &none, &lookup, &no_materials)
                .unwrap_err()
                .code,
            ErrorCode::OpIdReuse
        );
        assert!(matches!(
            admit_create_member(
                op,
                &command(profile),
                PriorOp::Unseen,
                &none,
                &lookup,
                &no_materials
            )
            .unwrap(),
            MemberAdmission::Apply(_)
        ));
    }
}
