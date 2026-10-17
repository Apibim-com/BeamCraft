// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! F04 section 7 command arguments and their model-level admission.
//!
//! P01 implements `CreatePlate` only. This module holds what is true of the arguments
//! regardless of geometry: identity, label, finiteness, caps and the rigid placement. Contour
//! and thickness *geometry* (floor, winding, supported shapes) is evaluated by the engine with
//! the kernel. No JSON, envelope or canonical-byte form lives here yet: F04 section 6 and F05
//! section 6 (`canonical`) remain planned, and [`CreatePlate::content_fingerprint`] is a
//! deterministic digest of the typed arguments, not RFC 8785 canonical bytes.

use crate::digest::{ContentDigest, Sha256};
use crate::error::ErrorCode;
use crate::id::ElementId;
use crate::limits::{
    CONTOUR_POINT_CAP, COORDINATE_CAP_MM, DIMENSION_CAP_MM, MAX_LABEL_BYTES, MIN_CONTOUR_POINTS,
    MIN_LABEL_BYTES, ORIENTATION_NORM_TOLERANCE, ROLL_CAP_RAD,
};
use beamcraft_geom::canonical_f64_bytes;
use core::fmt;

/// The only contract version this implementation accepts.
pub const CONTRACT_VERSION: &str = "0.1.0";

/// A typed rejection: a stable F04 code, a fixed field token and a fixed safe message. It never
/// carries caller text, paths or credentials.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandError {
    /// Stable F04 code.
    pub code: ErrorCode,
    /// Fixed schema token of the offending field, for example `contour_points_mm`.
    pub field: &'static str,
    /// Fixed ASCII explanation.
    pub message: &'static str,
}

impl CommandError {
    /// A rejection with the given code, field and message.
    pub const fn new(code: ErrorCode, field: &'static str, message: &'static str) -> Self {
        Self {
            code,
            field,
            message,
        }
    }
}

impl fmt::Display for CommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {} ({})", self.code, self.message, self.field)
    }
}

impl core::error::Error for CommandError {}

/// A rigid pose (F04 section 3): `world = R(q) * local + translation_mm`, quaternion in
/// `(x, y, z, w)` order. The components are authored as submitted; the accepted quaternion is
/// normalized only where it is used.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placement {
    /// Translation in millimetres, each component at most 1,000,000,000 in magnitude.
    pub translation_mm: [f64; 3],
    /// Quaternion components `(x, y, z, w)`, each in `[-1, 1]`, squared norm within 1e-6 of one.
    pub orientation_xyzw: [f64; 4],
}

impl Placement {
    /// No translation and the identity rotation.
    pub const IDENTITY: Self = Self {
        translation_mm: [0.0; 3],
        orientation_xyzw: [0.0, 0.0, 0.0, 1.0],
    };

    /// Checks finiteness, the translation cap and the quaternion rules, in that order.
    ///
    /// Non-finite is `NON_FINITE_NUMBER`; a translation above the cap is `LIMIT_EXCEEDED`; a
    /// quaternion component outside `[-1, 1]` or a squared norm farther than 1e-6 from one is
    /// `INVALID_ARGUMENT`.
    pub fn validate(&self) -> Result<(), CommandError> {
        let finite = |values: &[f64], field: &'static str| {
            if values.iter().all(|v| v.is_finite()) {
                Ok(())
            } else {
                Err(CommandError::new(
                    ErrorCode::NonFiniteNumber,
                    field,
                    "value is not finite",
                ))
            }
        };
        finite(&self.translation_mm, "placement.translation_mm")?;
        finite(&self.orientation_xyzw, "placement.orientation_xyzw")?;
        if self
            .translation_mm
            .iter()
            .any(|v| v.abs() > COORDINATE_CAP_MM)
        {
            return Err(CommandError::new(
                ErrorCode::LimitExceeded,
                "placement.translation_mm",
                "translation component exceeds the coordinate cap",
            ));
        }
        let [x, y, z, w] = self.orientation_xyzw;
        if self.orientation_xyzw.iter().any(|v| v.abs() > 1.0) {
            return Err(CommandError::new(
                ErrorCode::InvalidArgument,
                "placement.orientation_xyzw",
                "quaternion component is outside [-1, 1]",
            ));
        }
        let squared = ((x * x + y * y) + z * z) + w * w;
        if (squared - 1.0).abs() > ORIENTATION_NORM_TOLERANCE {
            return Err(CommandError::new(
                ErrorCode::InvalidArgument,
                "placement.orientation_xyzw",
                "quaternion squared norm is not within 1e-6 of one",
            ));
        }
        Ok(())
    }

    /// Whether the accepted rotation is the identity: `x = y = z = 0` (so `w` is `+-1` up to the
    /// accepted norm tolerance). `q` and `-q` are the same rotation.
    pub fn is_identity_rotation(&self) -> bool {
        let [x, y, z, _] = self.orientation_xyzw;
        x == 0.0 && y == 0.0 && z == 0.0
    }
}

/// F04 section 7 `CreatePlate`. `material_digest` pins an existing immutable material definition;
/// whether the digest resolves (`NOT_FOUND`) is the engine's decision, not an argument check.
#[derive(Debug, Clone, PartialEq)]
pub struct CreatePlate {
    /// The new element's full identity; creation also checks that it is unused.
    pub new_id: ElementId,
    /// Label, 1 to 256 UTF-8 bytes without control characters. Not an identity.
    pub name: String,
    /// Element-local contour, 3 to 1024 points, closure implicit.
    pub contour_points_mm: Vec<[f64; 3]>,
    /// Extrusion thickness in millimetres.
    pub thickness_mm: f64,
    /// Rigid placement.
    pub placement: Placement,
    /// Optional pinned material definition.
    pub material_digest: Option<ContentDigest>,
}

/// Whether `name` is a valid element label (F04 section 3, `DV-BASE-003`).
pub fn label_is_valid(name: &str) -> bool {
    (MIN_LABEL_BYTES..=MAX_LABEL_BYTES).contains(&name.len())
        && !name.chars().any(|c| c <= '\u{1f}' || c == '\u{7f}')
}

/// The explicit member kind of F04 section 7; it is independent of the USD model kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemberKind {
    /// A straight beam.
    Beam,
    /// A straight column.
    Column,
}

impl MemberKind {
    /// The `bim:elementKind` token.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Beam => "beam",
            Self::Column => "column",
        }
    }
}

/// F04 section 7 `CreateStraightMember`: a straight member with a pinned profile (required) and
/// an optional pinned material. Whether the digests resolve (`NOT_FOUND`) and the shape (axis
/// length, supported section) are the engine's decisions, not argument checks.
#[derive(Debug, Clone, PartialEq)]
pub struct CreateStraightMember {
    /// The new element's full identity.
    pub new_id: ElementId,
    /// Beam or column.
    pub kind: MemberKind,
    /// Label, 1 to 256 UTF-8 bytes without control characters.
    pub name: String,
    /// The two element-local axis points `p1` and `p2`.
    pub axis_points_mm: [[f64; 3]; 2],
    /// Pinned profile definition.
    pub profile_digest: ContentDigest,
    /// Rigid placement.
    pub placement: Placement,
    /// Explicit roll about the member axis; `None` is zero.
    pub roll_rad: Option<f64>,
    /// Optional pinned material definition.
    pub material_digest: Option<ContentDigest>,
}

impl CreateStraightMember {
    /// Admits what does not depend on geometry: label, finiteness, the coordinate and roll caps
    /// and the placement. A non-finite number is `NON_FINITE_NUMBER`; a coordinate above
    /// 1,000,000,000 mm or a roll above 2 pi is `LIMIT_EXCEEDED`.
    pub fn validate_arguments(&self) -> Result<(), CommandError> {
        if !label_is_valid(&self.name) {
            return Err(CommandError::new(
                ErrorCode::InvalidArgument,
                "name",
                "label must be 1 to 256 bytes without control characters",
            ));
        }
        let roll = self.roll_rad.unwrap_or(0.0);
        if !roll.is_finite() || self.axis_points_mm.iter().flatten().any(|v| !v.is_finite()) {
            return Err(CommandError::new(
                ErrorCode::NonFiniteNumber,
                "axis_points_mm",
                "value is not finite",
            ));
        }
        if self
            .axis_points_mm
            .iter()
            .flatten()
            .any(|v| v.abs() > COORDINATE_CAP_MM)
        {
            return Err(CommandError::new(
                ErrorCode::LimitExceeded,
                "axis_points_mm",
                "coordinate exceeds the coordinate cap",
            ));
        }
        if roll.abs() > ROLL_CAP_RAD {
            return Err(CommandError::new(
                ErrorCode::LimitExceeded,
                "roll_rad",
                "roll exceeds 2 pi",
            ));
        }
        self.placement.validate()
    }

    /// SHA-256 of a versioned, length-prefixed binary encoding of the validated arguments, with
    /// signed zero normalized; like [`CreatePlate::content_fingerprint`] it is not the F05
    /// canonical form.
    pub fn content_fingerprint(&self) -> Result<ContentDigest, CommandError> {
        self.validate_arguments()?;
        let bad = || {
            CommandError::new(
                ErrorCode::NonFiniteNumber,
                "axis_points_mm",
                "value cannot be encoded",
            )
        };
        let mut bytes = Vec::with_capacity(256);
        bytes.extend_from_slice(b"BeamCraft/P03/CreateStraightMember/v1\0");
        bytes.extend_from_slice(self.new_id.as_bytes());
        bytes.push(match self.kind {
            MemberKind::Beam => 0,
            MemberKind::Column => 1,
        });
        let name = self.name.as_bytes();
        bytes.extend_from_slice(&(name.len() as u64).to_le_bytes());
        bytes.extend_from_slice(name);
        bytes.extend_from_slice(self.profile_digest.as_bytes());
        match &self.material_digest {
            Some(digest) => {
                bytes.push(1);
                bytes.extend_from_slice(digest.as_bytes());
            }
            None => bytes.push(0),
        }
        match self.roll_rad {
            Some(roll) => {
                bytes.push(1);
                bytes.extend_from_slice(&canonical_f64_bytes(roll).map_err(|_| bad())?);
            }
            None => bytes.push(0),
        }
        let mut scalars = self
            .axis_points_mm
            .iter()
            .flatten()
            .copied()
            .chain(self.placement.translation_mm)
            .chain(self.placement.orientation_xyzw);
        scalars.try_for_each(|v| {
            bytes.extend_from_slice(&canonical_f64_bytes(v).map_err(|_| bad())?);
            Ok::<(), CommandError>(())
        })?;
        Sha256::digest(&bytes).map_err(|_| {
            CommandError::new(ErrorCode::LimitExceeded, "content", "content is too large")
        })
    }
}

impl CreatePlate {
    /// Admits what does not depend on geometry: label, contour count, finiteness, coordinate and
    /// dimension caps, then the placement. The count is checked before any traversal.
    ///
    /// A count below 3 is `INVALID_ARGUMENT`, above 1024 `LIMIT_EXCEEDED`; a non-finite number is
    /// `NON_FINITE_NUMBER`; a coordinate above 1,000,000,000 mm or a thickness above
    /// 1,000,000 mm is `LIMIT_EXCEEDED`. Floor, winding, planarity and supported shape are the
    /// engine's.
    pub fn validate_arguments(&self) -> Result<(), CommandError> {
        if !label_is_valid(&self.name) {
            return Err(CommandError::new(
                ErrorCode::InvalidArgument,
                "name",
                "label must be 1 to 256 bytes without control characters",
            ));
        }
        let count = self.contour_points_mm.len();
        if count > CONTOUR_POINT_CAP {
            return Err(CommandError::new(
                ErrorCode::LimitExceeded,
                "contour_points_mm",
                "contour has more than 1024 points",
            ));
        }
        if count < MIN_CONTOUR_POINTS {
            return Err(CommandError::new(
                ErrorCode::InvalidArgument,
                "contour_points_mm",
                "contour needs at least 3 points",
            ));
        }
        if !self.thickness_mm.is_finite()
            || self
                .contour_points_mm
                .iter()
                .flatten()
                .any(|v| !v.is_finite())
        {
            return Err(CommandError::new(
                ErrorCode::NonFiniteNumber,
                "contour_points_mm",
                "value is not finite",
            ));
        }
        if self
            .contour_points_mm
            .iter()
            .flatten()
            .any(|v| v.abs() > COORDINATE_CAP_MM)
        {
            return Err(CommandError::new(
                ErrorCode::LimitExceeded,
                "contour_points_mm",
                "coordinate exceeds the coordinate cap",
            ));
        }
        if self.thickness_mm > DIMENSION_CAP_MM {
            return Err(CommandError::new(
                ErrorCode::LimitExceeded,
                "thickness_mm",
                "thickness exceeds the dimension cap",
            ));
        }
        self.placement.validate()?;
        Ok(())
    }

    /// SHA-256 of a versioned, length-prefixed binary encoding of the validated arguments, with
    /// signed zero normalized. Two commands with the same fingerprint have identical content;
    /// it is *not* the F05 canonical form and must not be sent over a wire.
    ///
    /// Fails (`LIMIT_EXCEEDED`, `NON_FINITE_NUMBER`) on arguments that do not validate, so a
    /// fingerprint exists only for admissible content.
    pub fn content_fingerprint(&self) -> Result<ContentDigest, CommandError> {
        self.validate_arguments()?;
        let bad = || {
            CommandError::new(
                ErrorCode::NonFiniteNumber,
                "contour_points_mm",
                "value cannot be encoded",
            )
        };
        let mut bytes = Vec::with_capacity(64 + self.contour_points_mm.len() * 24);
        bytes.extend_from_slice(b"BeamCraft/P02/CreatePlate/v2\0");
        bytes.extend_from_slice(self.new_id.as_bytes());
        let name = self.name.as_bytes();
        bytes.extend_from_slice(&(name.len() as u64).to_le_bytes());
        bytes.extend_from_slice(name);
        bytes.extend_from_slice(&(self.contour_points_mm.len() as u64).to_le_bytes());
        let mut scalars = self
            .contour_points_mm
            .iter()
            .flatten()
            .copied()
            .chain([self.thickness_mm])
            .chain(self.placement.translation_mm)
            .chain(self.placement.orientation_xyzw);
        scalars.try_for_each(|v| {
            bytes.extend_from_slice(&canonical_f64_bytes(v).map_err(|_| bad())?);
            Ok::<(), CommandError>(())
        })?;
        match &self.material_digest {
            Some(digest) => {
                bytes.push(1);
                bytes.extend_from_slice(digest.as_bytes());
            }
            None => bytes.push(0),
        }
        Sha256::digest(&bytes).map_err(|_| {
            CommandError::new(ErrorCode::LimitExceeded, "content", "content is too large")
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plate() -> CreatePlate {
        CreatePlate {
            new_id: ElementId::parse("019a7bf0-0000-7000-8000-000000000005").unwrap(),
            name: "Synthetic plate".into(),
            contour_points_mm: vec![
                [0.0, 0.0, 0.0],
                [1000.0, 0.0, 0.0],
                [1000.0, 500.0, 0.0],
                [0.0, 500.0, 0.0],
            ],
            thickness_mm: 10.0,
            placement: Placement::IDENTITY,
            material_digest: None,
        }
    }

    fn code(plate: &CreatePlate) -> ErrorCode {
        plate.validate_arguments().unwrap_err().code
    }

    #[test]
    fn a_rectangle_is_admitted() {
        assert!(plate().validate_arguments().is_ok());
    }

    #[test]
    fn labels_follow_the_f04_rule() {
        for (name, ok) in [
            ("a", true),
            ("x".repeat(256).as_str(), true),
            ("x".repeat(257).as_str(), false),
            ("", false),
            ("line\nbreak", false),
            ("nul\0", false),
            ("del\u{7f}", false),
            ("c1\u{85}", true),
            ("é".repeat(128).as_str(), true),
            ("é".repeat(129).as_str(), false),
        ] {
            let mut p = plate();
            p.name = name.into();
            assert_eq!(p.validate_arguments().is_ok(), ok, "{name:?}");
        }
    }

    #[test]
    fn contour_count_finiteness_and_caps_have_their_codes() {
        let mut p = plate();
        p.contour_points_mm.truncate(2);
        assert_eq!(code(&p), ErrorCode::InvalidArgument); // F04-NEG-31
        let mut p = plate();
        p.contour_points_mm = vec![[0.0; 3]; 1025];
        assert_eq!(code(&p), ErrorCode::LimitExceeded);
        let mut p = plate();
        p.contour_points_mm[1][0] = f64::NAN;
        assert_eq!(code(&p), ErrorCode::NonFiniteNumber);
        let mut p = plate();
        p.contour_points_mm[2][1] = f64::NEG_INFINITY;
        assert_eq!(code(&p), ErrorCode::NonFiniteNumber);
        let mut p = plate();
        p.thickness_mm = f64::INFINITY;
        assert_eq!(code(&p), ErrorCode::NonFiniteNumber);
        let mut p = plate();
        p.contour_points_mm[2][1] = 1_000_000_000.5;
        assert_eq!(code(&p), ErrorCode::LimitExceeded);
        let mut p = plate();
        p.contour_points_mm[2][1] = 1_000_000_000.0;
        assert!(p.validate_arguments().is_ok());
        let mut p = plate();
        p.thickness_mm = 1_000_000.5;
        assert_eq!(code(&p), ErrorCode::LimitExceeded);
    }

    #[test]
    fn placement_rules_match_the_f04_cases() {
        let q = |x: f64, y: f64, z: f64, w: f64| {
            let mut p = plate();
            p.placement.orientation_xyzw = [x, y, z, w];
            p.validate_arguments()
        };
        assert!(q(0.0, 0.0, 0.0, 1.0).is_ok());
        assert!(q(0.0, 0.0, 0.0, -1.0).is_ok()); // F04-POS-12
        assert!(q(0.0, 0.0, 0.0, 0.99999995).is_ok()); // F04-POS-11
        let s = core::f64::consts::FRAC_1_SQRT_2;
        assert!(q(0.0, 0.0, s, s).is_ok());
        for (x, y, z, w) in [
            (0.0, 0.0, 0.0, 0.5),      // F04-NEG-22
            (0.0, 0.0, 0.0, 1.5),      // F04-NEG-23
            (0.0, 0.0, 0.0, 0.0),      // F04-NEG-24
            (0.0, 0.0, 0.0, 0.999999), // F04-NEG-25
        ] {
            assert_eq!(q(x, y, z, w).unwrap_err().code, ErrorCode::InvalidArgument);
        }
        assert_eq!(
            q(0.0, f64::NAN, 0.0, 1.0).unwrap_err().code,
            ErrorCode::NonFiniteNumber
        );
        let mut p = plate();
        p.placement.translation_mm = [1_000_000_000.0, 0.0, 0.0]; // F04-POS-09
        assert!(p.validate_arguments().is_ok());
        p.placement.translation_mm = [0.0, -1_000_000_001.0, 0.0];
        assert_eq!(code(&p), ErrorCode::LimitExceeded);
        p.placement.translation_mm = [0.0, 0.0, f64::INFINITY];
        assert_eq!(code(&p), ErrorCode::NonFiniteNumber);
    }

    #[test]
    fn identity_rotation_ignores_the_sign_of_w() {
        let mut p = Placement::IDENTITY;
        assert!(p.is_identity_rotation());
        p.orientation_xyzw = [0.0, 0.0, 0.0, -1.0];
        assert!(p.is_identity_rotation());
        let s = core::f64::consts::FRAC_1_SQRT_2;
        p.orientation_xyzw = [0.0, 0.0, s, s];
        assert!(!p.is_identity_rotation());
    }

    #[test]
    fn the_fingerprint_tracks_content_and_ignores_signed_zero() {
        let a = plate().content_fingerprint().unwrap();
        assert_eq!(a, plate().content_fingerprint().unwrap());
        let mut zero = plate();
        zero.contour_points_mm[0][0] = -0.0;
        assert_eq!(zero.content_fingerprint().unwrap(), a);
        for mutate in [
            |p: &mut CreatePlate| p.thickness_mm = 10.5,
            |p: &mut CreatePlate| p.name = "Other".into(),
            |p: &mut CreatePlate| p.contour_points_mm[1][0] = 750.0,
            |p: &mut CreatePlate| p.placement.translation_mm[2] = 1.0,
            |p: &mut CreatePlate| {
                p.new_id = ElementId::parse("019a7bf0-0000-7000-8000-000000000006").unwrap();
            },
        ] {
            let mut p = plate();
            mutate(&mut p);
            assert_ne!(p.content_fingerprint().unwrap(), a);
        }
        let mut bad = plate();
        bad.thickness_mm = f64::NAN;
        assert!(bad.content_fingerprint().is_err());
    }

    #[test]
    fn a_material_reference_is_admitted_and_part_of_the_fingerprint() {
        let digest = |text: &[u8]| crate::digest::Sha256::digest(text).unwrap();
        let none = plate().content_fingerprint().unwrap();
        let mut a = plate();
        a.material_digest = Some(digest(b"a"));
        assert!(a.validate_arguments().is_ok());
        let mut b = plate();
        b.material_digest = Some(digest(b"b"));
        let fingerprints = [
            none,
            a.content_fingerprint().unwrap(),
            b.content_fingerprint().unwrap(),
        ];
        assert_ne!(fingerprints[0], fingerprints[1]);
        assert_ne!(fingerprints[1], fingerprints[2]);
        assert_ne!(fingerprints[0], fingerprints[2]);
    }

    fn member() -> CreateStraightMember {
        let digest = |text: &[u8]| crate::digest::Sha256::digest(text).unwrap();
        CreateStraightMember {
            new_id: ElementId::parse("019a7bf0-0000-7000-8000-000000000007").unwrap(),
            kind: MemberKind::Beam,
            name: "Synthetic beam".into(),
            axis_points_mm: [[0.0; 3], [6000.0, 0.0, 0.0]],
            profile_digest: digest(b"profile"),
            placement: Placement::IDENTITY,
            roll_rad: None,
            material_digest: None,
        }
    }

    #[test]
    fn a_member_has_the_f04_argument_rules() {
        assert!(member().validate_arguments().is_ok());
        let code = |m: CreateStraightMember| m.validate_arguments().unwrap_err().code;
        let mut m = member();
        m.name = "line\nbreak".into();
        assert_eq!(code(m), ErrorCode::InvalidArgument);
        let mut m = member();
        m.axis_points_mm[1][2] = f64::NAN;
        assert_eq!(code(m), ErrorCode::NonFiniteNumber);
        let mut m = member();
        m.roll_rad = Some(f64::INFINITY);
        assert_eq!(code(m), ErrorCode::NonFiniteNumber);
        let mut m = member();
        m.axis_points_mm[0][0] = 1_000_000_000.5;
        assert_eq!(code(m), ErrorCode::LimitExceeded);
        let mut m = member();
        m.roll_rad = Some(core::f64::consts::TAU);
        assert!(m.validate_arguments().is_ok());
        m.roll_rad = Some(6.3);
        assert_eq!(code(m), ErrorCode::LimitExceeded);
        let mut m = member();
        m.placement.orientation_xyzw = [0.0, 0.0, 0.0, 0.5];
        assert_eq!(code(m), ErrorCode::InvalidArgument);
    }

    #[test]
    fn the_member_fingerprint_tracks_every_input() {
        let base = member().content_fingerprint().unwrap();
        let digest = |text: &[u8]| crate::digest::Sha256::digest(text).unwrap();
        for mutate in [
            |m: &mut CreateStraightMember| m.kind = MemberKind::Column,
            |m: &mut CreateStraightMember| m.name = "Other".into(),
            |m: &mut CreateStraightMember| m.axis_points_mm[1][0] = 6001.0,
            |m: &mut CreateStraightMember| m.roll_rad = Some(0.5),
            |m: &mut CreateStraightMember| m.placement.translation_mm[1] = 1.0,
        ] {
            let mut m = member();
            mutate(&mut m);
            assert_ne!(m.content_fingerprint().unwrap(), base);
        }
        let mut m = member();
        m.profile_digest = digest(b"other profile");
        assert_ne!(m.content_fingerprint().unwrap(), base);
        let mut m = member();
        m.material_digest = Some(digest(b"material"));
        assert_ne!(m.content_fingerprint().unwrap(), base);
        // Signed zero and an explicit zero roll are not the same authored input, but -0.0 is 0.0.
        let mut m = member();
        m.axis_points_mm[0][0] = -0.0;
        assert_eq!(m.content_fingerprint().unwrap(), base);
    }
}
