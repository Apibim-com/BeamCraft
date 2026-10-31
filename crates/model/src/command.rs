// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! F04 section 7 command arguments and their model-level admission.
//!
//! P01 implements `CreatePlate` only. This module holds what is true of the arguments
//! regardless of geometry: identity, label, finiteness, caps and the rigid placement. Contour
//! and thickness *geometry* (floor, winding, supported shapes) is evaluated by the engine with
//! the kernel. No JSON, envelope or canonical-byte form lives here yet: F04 sections 3 and 6
//! (`canonical`) remain planned, and [`CreatePlate::content_fingerprint`] is a
//! deterministic digest of the typed arguments, not RFC 8785 canonical bytes.

pub use crate::construction::CreateConstructionObject;
use crate::digest::{ContentDigest, Sha256};
use crate::error::ErrorCode;
pub use crate::grid::CreateGrid;
pub use crate::grid_update::UpdateGrid;
use crate::id::ElementId;
use crate::limits::{
    CONTOUR_POINT_CAP, COORDINATE_CAP_MM, DIMENSION_CAP_MM, MAX_ARRAY_COPIES, MAX_LABEL_BYTES,
    MAX_MEMBER_PATH_POINTS, MIN_CONTOUR_POINTS, MIN_LABEL_BYTES, ORIENTATION_NORM_TOLERANCE,
    ROLL_CAP_RAD,
};
use crate::steel::command::{CornerTreatment, PlateCorner, PlateSide};
use beamcraft_geom::canonical_f64_bytes;
use core::fmt;

/// The only contract version this implementation accepts.
pub const CONTRACT_VERSION: &str = "0.1.0";

/// A typed rejection: a stable F04 code, a fixed field token and a fixed safe message. It never
/// carries caller text, paths or credentials.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandError {
    /// F10 rule for domain admission failures, absent for other errors.
    pub rule_id: Option<&'static str>,
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
            rule_id: None,
        }
    }
    /// Retains the exact F10 rule through engine and CLI admission.
    pub const fn with_rule(mut self, rule_id: &'static str) -> Self {
        self.rule_id = Some(rule_id);
        self
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
                Err(
                    CommandError::new(ErrorCode::NonFiniteNumber, field, "value is not finite")
                        .with_rule("DV-ATTR-003"),
                )
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
            )
            .with_rule("DV-LIM-001"));
        }
        Self::validate_orientation(self.orientation_xyzw)
    }

    /// Checks a quaternion alone: non-finite is `NON_FINITE_NUMBER`; a component outside
    /// `[-1, 1]` or a squared norm farther than 1e-6 from one is `INVALID_ARGUMENT`.
    pub fn validate_orientation(xyzw: [f64; 4]) -> Result<(), CommandError> {
        if xyzw.iter().any(|v| !v.is_finite()) {
            return Err(CommandError::new(
                ErrorCode::NonFiniteNumber,
                "placement.orientation_xyzw",
                "value is not finite",
            )
            .with_rule("DV-ATTR-003"));
        }
        if xyzw.iter().any(|v| v.abs() > 1.0) {
            return Err(CommandError::new(
                ErrorCode::InvalidArgument,
                "placement.orientation_xyzw",
                "quaternion component is outside [-1, 1]",
            )
            .with_rule("DV-LIM-003"));
        }
        let [x, y, z, w] = xyzw;
        let squared = ((x * x + y * y) + z * z) + w * w;
        if (squared - 1.0).abs() > ORIENTATION_NORM_TOLERANCE {
            return Err(CommandError::new(
                ErrorCode::InvalidArgument,
                "placement.orientation_xyzw",
                "quaternion squared norm is not within 1e-6 of one",
            )
            .with_rule("DV-LIM-003"));
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

/// Whether `name` is a valid element label (F04 section 3, `DV-BASE-003`).
pub fn label_is_valid(name: &str) -> bool {
    (MIN_LABEL_BYTES..=MAX_LABEL_BYTES).contains(&name.len())
        && !name.chars().any(|c| c <= '\u{1f}' || c == '\u{7f}')
}

/// T03: a relative rotation of the whole element about a world axis through `pivot_mm` by
/// `angle_rad`, composed with the element's current placement. The local shape is untouched: the
/// rotation is absorbed by the placement alone, so a retried command recomputes the same absolute
/// placement from the head state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ElementRotate {
    /// A point on the rotation axis, in world millimetres.
    pub pivot_mm: [f64; 3],
    /// Direction of the rotation axis; need not be unit or axis-aligned, must be nonzero.
    pub axis: [f64; 3],
    /// Rotation angle in radians, right-handed about `axis`.
    pub angle_rad: f64,
}

impl ElementRotate {
    /// Admits the request: finite numbers (`NON_FINITE_NUMBER`), a nonzero axis
    /// (`INVALID_ARGUMENT`) and an angle within `TAU` (`LIMIT_EXCEEDED`). Whether the rotated
    /// placement stays in range is the engine's decision.
    pub fn validate(&self) -> Result<(), CommandError> {
        if self
            .pivot_mm
            .iter()
            .chain(self.axis.iter())
            .any(|v| !v.is_finite())
            || !self.angle_rad.is_finite()
        {
            return Err(CommandError::new(
                ErrorCode::NonFiniteNumber,
                "rotate",
                "value is not finite",
            )
            .with_rule("DV-ATTR-003"));
        }
        if self.axis.iter().all(|v| *v == 0.0) {
            return Err(CommandError::new(
                ErrorCode::InvalidArgument,
                "rotate",
                "the rotation axis is the zero vector",
            ));
        }
        if self.angle_rad.abs() > ROLL_CAP_RAD {
            return Err(CommandError::new(
                ErrorCode::LimitExceeded,
                "rotate",
                "the angle exceeds the rotation cap",
            )
            .with_rule("DV-LIM-001"));
        }
        Ok(())
    }
}

/// T03: a relative mirror of the whole element about the world plane through `point_mm` with
/// unit-independent `normal`. The engine re-encodes the element through a symmetry of its local
/// body, so the mirrored part is the same authored shape at a mirrored placement.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ElementMirror {
    /// A point on the mirror plane, in world millimetres.
    pub point_mm: [f64; 3],
    /// Normal of the mirror plane; need not be unit, must be nonzero.
    pub normal: [f64; 3],
}

impl ElementMirror {
    /// Admits the request: finite numbers (`NON_FINITE_NUMBER`) and a nonzero normal
    /// (`INVALID_ARGUMENT`).
    pub fn validate(&self) -> Result<(), CommandError> {
        if self
            .point_mm
            .iter()
            .chain(self.normal.iter())
            .any(|v| !v.is_finite())
        {
            return Err(CommandError::new(
                ErrorCode::NonFiniteNumber,
                "mirror",
                "value is not finite",
            )
            .with_rule("DV-ATTR-003"));
        }
        if self.normal.iter().all(|v| *v == 0.0) {
            return Err(CommandError::new(
                ErrorCode::InvalidArgument,
                "mirror",
                "the mirror normal is the zero vector",
            ));
        }
        Ok(())
    }
}

/// A typed patch of supported authored fields (F04 section 7 `UpdateElement`). A field that is
/// `None` is left unchanged. `material` distinguishes no change (`None`), clearing the reference
/// (`Some(None)`) and pinning a definition (`Some(Some(digest))`). There is no raw USD setter.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ElementPatch {
    /// Complete replacement of a plate's sparse corner treatment list; empty clears it.
    pub corners: Option<Vec<PlateCorner>>,
    /// Complete replacement of a plate's authored contour (A12.3): every point, in order. A
    /// member has no contour. Whether the new contour is a valid plate shape is the engine's
    /// decision; the patch carries it whole, like a creation.
    pub contour_points_mm: Option<Vec<[f64; 3]>>,
    /// New label.
    pub name: Option<String>,
    /// New authored integer part class (P15).
    pub class: Option<i32>,
    /// New material reference, or its explicit removal.
    pub material: Option<Option<ContentDigest>>,
    /// New plate thickness in millimetres (a plate only).
    pub thickness_mm: Option<f64>,
    /// New thickness side of a plate (A12.1); `Some` names the side to lay the thickness off
    /// from the contour plane. A plate always has a side (absent means `top`), so there is no
    /// clearing.
    pub thickness_side: Option<PlateSide>,
    /// New placement translation in millimetres, absolute; the rotation is kept (a move).
    pub translation_mm: Option<[f64; 3]>,
    /// New placement rotation, absolute. Replaces the stored quaternion (the identity resets a
    /// part to the unrotated pose); the translation is kept. The quaternion follows the `Placement` rules and is validated.
    pub orientation_xyzw: Option<[f64; 4]>,
    /// D2: new level assignment of a plate, beam or column, or its explicit removal
    /// (`Some(None)`). Whether the level exists is the engine's decision.
    pub level: Option<Option<ElementId>>,
    /// P12: the complete replacement of a member's path points, two for a straight member and
    /// at least three for a polybeam. The whole path is replaced; there is no per-point setter.
    pub axis_points_mm: Option<Vec<[f64; 3]>>,
    /// T03: rotate the element about a world axis, relative to its current placement. Mutually
    /// exclusive with `translation_mm`, `orientation_xyzw` and `mirror`.
    pub rotate: Option<ElementRotate>,
    /// T03: mirror the element about a world plane, relative to its current state. Mutually
    /// exclusive with `translation_mm`, `orientation_xyzw` and `rotate`.
    pub mirror: Option<ElementMirror>,
    /// T03: translate the element by this world offset, relative to its current placement (the
    /// two-point move form is its `to - from`). Mutually exclusive with `translation_mm`,
    /// `rotate` and `mirror`.
    pub move_mm: Option<[f64; 3]>,
    /// A member's new pinned profile definition (replaces the standing one; the body is rebuilt
    /// from it). A plate has no profile. Whether the digest names a profile definition is the
    /// engine's decision.
    pub profile: Option<ContentDigest>,
}

/// F04 section 7 `UpdateElement`: the target element and a typed patch.
#[derive(Debug, Clone, PartialEq)]
pub struct UpdateElement {
    /// The element to change.
    pub target_id: ElementId,
    /// The patch.
    pub patch: ElementPatch,
}

impl UpdateElement {
    /// Admits the patch: it must change something, the label follows the F04 rule, the
    /// thickness is finite (`NON_FINITE_NUMBER`) and at most 1,000,000 mm (`LIMIT_EXCEEDED`), and
    /// the translation is finite and within the coordinate cap. That a field applies to the
    /// target's kind and the geometry are the engine's decisions.
    pub fn validate_arguments(&self) -> Result<(), CommandError> {
        let patch = &self.patch;
        if let Some(corners) = &patch.corners {
            if corners.len() > CONTOUR_POINT_CAP {
                return Err(CommandError::new(
                    ErrorCode::LimitExceeded,
                    "corner",
                    "too many corner treatments",
                ));
            }
            let mut seen = Vec::new();
            for corner in corners {
                corner.validate()?;
                if corner.vertex >= CONTOUR_POINT_CAP || seen.contains(&corner.vertex) {
                    return Err(CommandError::new(
                        ErrorCode::InvalidArgument,
                        "corner",
                        "duplicate or out-of-range corner vertex",
                    ));
                }
                seen.push(corner.vertex);
            }
        }
        if let Some(points) = &patch.contour_points_mm {
            let count = points.len();
            if count > CONTOUR_POINT_CAP {
                return Err(CommandError::new(
                    ErrorCode::LimitExceeded,
                    "contour_points_mm",
                    "contour has more than 1024 points",
                )
                .with_rule("DV-LIM-001"));
            }
            if count < MIN_CONTOUR_POINTS {
                return Err(CommandError::new(
                    ErrorCode::InvalidArgument,
                    "contour_points_mm",
                    "contour needs at least 3 points",
                ));
            }
            if points.iter().flatten().any(|v| !v.is_finite()) {
                return Err(CommandError::new(
                    ErrorCode::NonFiniteNumber,
                    "contour_points_mm",
                    "value is not finite",
                )
                .with_rule("DV-ATTR-003"));
            }
            if points.iter().flatten().any(|v| v.abs() > COORDINATE_CAP_MM) {
                return Err(CommandError::new(
                    ErrorCode::LimitExceeded,
                    "contour_points_mm",
                    "coordinate exceeds the coordinate cap",
                )
                .with_rule("DV-LIM-001"));
            }
        }
        if patch.corners.is_none()
            && patch.contour_points_mm.is_none()
            && patch.class.is_none()
            && patch.name.is_none()
            && patch.material.is_none()
            && patch.thickness_mm.is_none()
            && patch.thickness_side.is_none()
            && patch.translation_mm.is_none()
            && patch.orientation_xyzw.is_none()
            && patch.level.is_none()
            && patch.axis_points_mm.is_none()
            && patch.rotate.is_none()
            && patch.mirror.is_none()
            && patch.move_mm.is_none()
            && patch.profile.is_none()
        {
            return Err(CommandError::new(
                ErrorCode::InvalidArgument,
                "changes",
                "the patch changes nothing",
            ));
        }
        // A relative transform composes with the current placement, so naming an absolute
        // placement field in the same patch would be ambiguous.
        if patch.rotate.is_some() || patch.mirror.is_some() || patch.move_mm.is_some() {
            if patch.translation_mm.is_some() || patch.orientation_xyzw.is_some() {
                return Err(CommandError::new(
                    ErrorCode::InvalidArgument,
                    "changes",
                    "a relative transform takes no absolute placement fields",
                ));
            }
            let named = patch.rotate.is_some() as u8
                + patch.mirror.is_some() as u8
                + patch.move_mm.is_some() as u8;
            if named > 1 {
                return Err(CommandError::new(
                    ErrorCode::InvalidArgument,
                    "changes",
                    "relative transforms are separate patches; compose them with two updates",
                ));
            }
            if let Some(rotate) = &patch.rotate {
                rotate.validate()?;
            }
            if let Some(mirror) = &patch.mirror {
                mirror.validate()?;
            }
            if let Some(move_mm) = patch.move_mm
                && move_mm.iter().any(|v| !v.is_finite())
            {
                return Err(CommandError::new(
                    ErrorCode::NonFiniteNumber,
                    "move_mm",
                    "value is not finite",
                )
                .with_rule("DV-ATTR-003"));
            }
            // A mirror re-encodes the member through its path symmetry: replacing the path in
            // the same patch would mirror through one path and re-encode through another.
            if patch.mirror.is_some() && patch.axis_points_mm.is_some() {
                return Err(CommandError::new(
                    ErrorCode::InvalidArgument,
                    "changes",
                    "a mirror takes no path replacement in the same patch",
                ));
            }
        }
        if let Some(translation) = patch.translation_mm {
            if translation.iter().any(|v| !v.is_finite()) {
                return Err(CommandError::new(
                    ErrorCode::NonFiniteNumber,
                    "translation_mm",
                    "value is not finite",
                )
                .with_rule("DV-ATTR-003"));
            }
            if translation.iter().any(|v| v.abs() > COORDINATE_CAP_MM) {
                return Err(CommandError::new(
                    ErrorCode::LimitExceeded,
                    "translation_mm",
                    "translation component exceeds the coordinate cap",
                )
                .with_rule("DV-LIM-001"));
            }
        }
        if let Some(xyzw) = patch.orientation_xyzw {
            Placement::validate_orientation(xyzw)?;
        }
        if patch.name.as_deref().is_some_and(|n| !label_is_valid(n)) {
            return Err(CommandError::new(
                ErrorCode::InvalidArgument,
                "name",
                "label must be 1 to 256 bytes without control characters",
            )
            .with_rule("DV-BASE-003"));
        }
        if let Some(t) = patch.thickness_mm {
            if !t.is_finite() {
                return Err(CommandError::new(
                    ErrorCode::NonFiniteNumber,
                    "thickness_mm",
                    "value is not finite",
                )
                .with_rule("DV-ATTR-003"));
            }
            if t > DIMENSION_CAP_MM {
                return Err(CommandError::new(
                    ErrorCode::LimitExceeded,
                    "thickness_mm",
                    "thickness exceeds the dimension cap",
                )
                .with_rule("DV-LIM-001"));
            }
        }
        if let Some(points) = &patch.axis_points_mm {
            if points.len() > MAX_MEMBER_PATH_POINTS {
                return Err(CommandError::new(
                    ErrorCode::LimitExceeded,
                    "axis_points_mm",
                    "path has more than 1024 points",
                )
                .with_rule("DV-LIM-001"));
            }
            if points.iter().flatten().any(|v| !v.is_finite()) {
                return Err(CommandError::new(
                    ErrorCode::NonFiniteNumber,
                    "axis_points_mm",
                    "value is not finite",
                )
                .with_rule("DV-ATTR-003"));
            }
            if points.iter().flatten().any(|v| v.abs() > COORDINATE_CAP_MM) {
                return Err(CommandError::new(
                    ErrorCode::LimitExceeded,
                    "axis_points_mm",
                    "coordinate exceeds the coordinate cap",
                )
                .with_rule("DV-LIM-001"));
            }
        }
        Ok(())
    }

    /// SHA-256 of a versioned, length-prefixed encoding of the validated request.
    pub fn content_fingerprint(&self) -> Result<ContentDigest, CommandError> {
        self.validate_arguments()?;
        let mut bytes = Vec::with_capacity(128);
        bytes.extend_from_slice(b"BeamCraft/P04/UpdateElement/v2\0");
        bytes.extend_from_slice(self.target_id.as_bytes());
        match &self.patch.name {
            Some(name) => {
                bytes.push(1);
                bytes.extend_from_slice(&(name.len() as u64).to_le_bytes());
                bytes.extend_from_slice(name.as_bytes());
            }
            None => bytes.push(0),
        }
        match self.patch.class {
            Some(class) => {
                bytes.push(1);
                bytes.extend_from_slice(&class.to_le_bytes());
            }
            None => bytes.push(0),
        }
        match &self.patch.material {
            None => bytes.push(0),
            Some(None) => bytes.push(1),
            Some(Some(digest)) => {
                bytes.push(2);
                bytes.extend_from_slice(digest.as_bytes());
            }
        }
        match self.patch.thickness_mm {
            Some(t) => {
                bytes.push(1);
                bytes.extend_from_slice(&canonical_f64_bytes(t).map_err(|_| {
                    CommandError::new(
                        ErrorCode::NonFiniteNumber,
                        "thickness_mm",
                        "value cannot be encoded",
                    )
                    .with_rule("DV-ATTR-003")
                })?);
            }
            None => bytes.push(0),
        }
        // Appended only when present, so the fingerprints of earlier patches are unchanged.
        if let Some(translation) = self.patch.translation_mm {
            bytes.push(2);
            for v in translation {
                bytes.extend_from_slice(&canonical_f64_bytes(v).map_err(|_| {
                    CommandError::new(
                        ErrorCode::NonFiniteNumber,
                        "translation_mm",
                        "value cannot be encoded",
                    )
                    .with_rule("DV-ATTR-003")
                })?);
            }
        }
        // Appended only when present, so the fingerprints of earlier patches are unchanged.
        match self.patch.level {
            None => {}
            Some(None) => bytes.extend_from_slice(&[3, 0]),
            Some(Some(level)) => {
                bytes.extend_from_slice(&[3, 1]);
                bytes.extend_from_slice(level.as_bytes());
            }
        }
        // Appended only when present, so the fingerprints of earlier patches are unchanged.
        if let Some(xyzw) = self.patch.orientation_xyzw {
            bytes.extend_from_slice(&[4, 1]);
            for v in xyzw {
                bytes.extend_from_slice(&canonical_f64_bytes(v).map_err(|_| {
                    CommandError::new(
                        ErrorCode::NonFiniteNumber,
                        "placement.orientation_xyzw",
                        "value cannot be encoded",
                    )
                    .with_rule("DV-ATTR-003")
                })?);
            }
        }
        // Appended only when present, so the fingerprints of earlier patches are unchanged.
        if let Some(side) = self.patch.thickness_side {
            bytes.push(5);
            bytes.push(match side {
                PlateSide::Top => 1,
                PlateSide::Middle => 2,
                PlateSide::Bottom => 3,
            });
        }
        // Appended only when present, so the fingerprints of earlier patches are unchanged.
        if let Some(profile) = &self.patch.profile {
            bytes.extend_from_slice(&[7, 1]);
            bytes.extend_from_slice(profile.as_bytes());
        }
        // Appended only when present, so the fingerprints of earlier patches are unchanged.
        if let Some(points) = &self.patch.axis_points_mm {
            bytes.extend_from_slice(&[6, 1]);
            bytes.extend_from_slice(&(points.len() as u64).to_le_bytes());
            for v in points.iter().flatten().copied() {
                bytes.extend_from_slice(&canonical_f64_bytes(v).map_err(|_| {
                    CommandError::new(
                        ErrorCode::NonFiniteNumber,
                        "axis_points_mm",
                        "value cannot be encoded",
                    )
                    .with_rule("DV-ATTR-003")
                })?);
            }
        }
        if let Some(corners) = &self.patch.corners {
            bytes.extend_from_slice(b"A12.3/corner-patch/v1\0");
            bytes.extend_from_slice(&(corners.len() as u64).to_le_bytes());
            for corner in corners {
                bytes.extend_from_slice(&(corner.vertex as u64).to_le_bytes());
                bytes.push(match corner.treatment {
                    CornerTreatment::Chamfer { .. } => 1,
                    CornerTreatment::Round { .. } => 2,
                });
                let values = match corner.treatment {
                    CornerTreatment::Chamfer { d1_mm, d2_mm } => [d1_mm, d2_mm],
                    CornerTreatment::Round { radius_mm } => [radius_mm, radius_mm],
                };
                for v in values {
                    bytes.extend_from_slice(&v.to_le_bytes());
                }
            }
        }
        // Appended only when present, so the fingerprints of earlier patches are unchanged.
        if let Some(points) = &self.patch.contour_points_mm {
            bytes.extend_from_slice(b"A12.3/contour-patch/v1\0");
            bytes.extend_from_slice(&(points.len() as u64).to_le_bytes());
            for v in points.iter().flatten().copied() {
                bytes.extend_from_slice(&canonical_f64_bytes(v).map_err(|_| {
                    CommandError::new(
                        ErrorCode::NonFiniteNumber,
                        "contour_points_mm",
                        "value cannot be encoded",
                    )
                    .with_rule("DV-ATTR-003")
                })?);
            }
        }
        // Appended only when present, so the fingerprints of earlier patches are unchanged.
        if let Some(rotate) = &self.patch.rotate {
            bytes.extend_from_slice(b"T03/rotate/v1\0");
            for v in rotate.pivot_mm.into_iter().chain(rotate.axis) {
                bytes.extend_from_slice(&canonical_f64_bytes(v).map_err(|_| {
                    CommandError::new(
                        ErrorCode::NonFiniteNumber,
                        "rotate",
                        "value cannot be encoded",
                    )
                    .with_rule("DV-ATTR-003")
                })?);
            }
            bytes.extend_from_slice(&canonical_f64_bytes(rotate.angle_rad).map_err(|_| {
                CommandError::new(
                    ErrorCode::NonFiniteNumber,
                    "rotate",
                    "value cannot be encoded",
                )
                .with_rule("DV-ATTR-003")
            })?);
        }
        // Appended only when present, so the fingerprints of earlier patches are unchanged.
        if let Some(mirror) = &self.patch.mirror {
            bytes.extend_from_slice(b"T03/mirror/v1\0");
            for v in mirror.point_mm.into_iter().chain(mirror.normal) {
                bytes.extend_from_slice(&canonical_f64_bytes(v).map_err(|_| {
                    CommandError::new(
                        ErrorCode::NonFiniteNumber,
                        "mirror",
                        "value cannot be encoded",
                    )
                    .with_rule("DV-ATTR-003")
                })?);
            }
        }
        // Appended only when present, so the fingerprints of earlier patches are unchanged.
        if let Some(move_mm) = self.patch.move_mm {
            bytes.extend_from_slice(b"T03/move/v1\0");
            for v in move_mm {
                bytes.extend_from_slice(&canonical_f64_bytes(v).map_err(|_| {
                    CommandError::new(
                        ErrorCode::NonFiniteNumber,
                        "move_mm",
                        "value cannot be encoded",
                    )
                    .with_rule("DV-ATTR-003")
                })?);
            }
        }
        Sha256::digest(&bytes).map_err(|_| {
            CommandError::new(ErrorCode::LimitExceeded, "content", "content is too large")
                .with_rule("DV-LIM-001")
        })
    }
}

/// F04 section 7 `DeleteElement` (P08): removes one owned element. It is refused while another
/// element still links to it (`DEPENDENCY_CONFLICT`); there is no cascade.
#[derive(Debug, Clone, PartialEq)]
pub struct DeleteElement {
    /// The element to delete.
    pub target_id: ElementId,
}

impl DeleteElement {
    /// SHA-256 of a versioned encoding of the request. Whether the element exists and whether
    /// something links to it are the engine's and the layer engine's decisions.
    pub fn content_fingerprint(&self) -> Result<ContentDigest, CommandError> {
        let mut bytes = Vec::with_capacity(64);
        bytes.extend_from_slice(b"BeamCraft/P08/DeleteElement/v1\0");
        bytes.extend_from_slice(self.target_id.as_bytes());
        Sha256::digest(&bytes).map_err(|_| {
            CommandError::new(ErrorCode::LimitExceeded, "content", "content is too large")
                .with_rule("DV-LIM-001")
        })
    }
}

/// F04 section 7 `CopyElement` (P05): a new element with a fresh identity and the same authored
/// inputs as the source, optionally renamed and translated. The source is only read.
#[derive(Debug, Clone, PartialEq)]
pub struct CopyElement {
    /// The element to copy.
    pub source_id: ElementId,
    /// Identity of the copy.
    pub new_id: ElementId,
    /// Label of the copy; the source label when `None`.
    pub name: Option<String>,
    /// Translation added to the source placement, in millimetres.
    pub offset_mm: [f64; 3],
}

impl CopyElement {
    /// Admits the request: the identities differ (`DUPLICATE_ID`), the label follows the F04 rule,
    /// the offset is finite (`NON_FINITE_NUMBER`) and within the coordinate cap
    /// (`LIMIT_EXCEEDED`). That the source exists and the resulting placement stays in range are
    /// the engine's decisions.
    pub fn validate_arguments(&self) -> Result<(), CommandError> {
        if self.source_id == self.new_id {
            return Err(CommandError::new(
                ErrorCode::DuplicateId,
                "new_id",
                "the copy identity equals the source",
            )
            .with_rule("DV-BASE-002"));
        }
        if self.name.as_deref().is_some_and(|n| !label_is_valid(n)) {
            return Err(CommandError::new(
                ErrorCode::InvalidArgument,
                "name",
                "label must be 1 to 256 bytes without control characters",
            )
            .with_rule("DV-BASE-003"));
        }
        if self.offset_mm.iter().any(|v| !v.is_finite()) {
            return Err(CommandError::new(
                ErrorCode::NonFiniteNumber,
                "offset_mm",
                "value is not finite",
            )
            .with_rule("DV-ATTR-003"));
        }
        if self.offset_mm.iter().any(|v| v.abs() > COORDINATE_CAP_MM) {
            return Err(CommandError::new(
                ErrorCode::LimitExceeded,
                "offset_mm",
                "offset component exceeds the coordinate cap",
            )
            .with_rule("DV-LIM-001"));
        }
        Ok(())
    }

    /// SHA-256 of a versioned, length-prefixed encoding of the validated request.
    pub fn content_fingerprint(&self) -> Result<ContentDigest, CommandError> {
        self.validate_arguments()?;
        let mut bytes = Vec::with_capacity(128);
        bytes.extend_from_slice(b"BeamCraft/P05/CopyElement/v1\0");
        bytes.extend_from_slice(self.source_id.as_bytes());
        bytes.extend_from_slice(self.new_id.as_bytes());
        match &self.name {
            Some(name) => {
                bytes.push(1);
                bytes.extend_from_slice(&(name.len() as u64).to_le_bytes());
                bytes.extend_from_slice(name.as_bytes());
            }
            None => bytes.push(0),
        }
        for v in self.offset_mm {
            bytes.extend_from_slice(&canonical_f64_bytes(v).map_err(|_| {
                CommandError::new(
                    ErrorCode::NonFiniteNumber,
                    "offset_mm",
                    "value cannot be encoded",
                )
                .with_rule("DV-ATTR-003")
            })?);
        }
        Sha256::digest(&bytes).map_err(|_| {
            CommandError::new(ErrorCode::LimitExceeded, "content", "content is too large")
                .with_rule("DV-LIM-001")
        })
    }
}

/// T03: the placement rule of one array copy. The source stays as it is; the copy at index
/// `i` (counting from one) takes the step `i` times.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ArrayStep {
    /// A linear array: copy `i` is the source translated by `direction * (spacing_mm * i)`.
    Linear {
        /// Direction of the array; need not be unit, must be nonzero.
        direction: [f64; 3],
        /// Distance between neighbouring copies along `direction`, in millimetres.
        spacing_mm: f64,
    },
    /// A polar array: copy `i` is the source rotated by `angle_rad * i` about the world axis
    /// through `point_mm` along `axis`.
    Polar {
        /// A point on the array axis, in world millimetres.
        point_mm: [f64; 3],
        /// Direction of the array axis; need not be unit, must be nonzero.
        axis: [f64; 3],
        /// Angle between neighbouring copies in radians, right-handed about `axis`.
        angle_rad: f64,
    },
}

/// F04 `ArrayElements` (T03): independent copies of one source part, created in one revision so
/// one undo step takes them all back. Each copy is a `CopyElement` duplicate with a fresh
/// identity and freshly minted operation records; the source is only read.
#[derive(Debug, Clone, PartialEq)]
pub struct ArrayElements {
    /// The element to copy.
    pub source_id: ElementId,
    /// Label of every copy; the source label when `None`.
    pub name: Option<String>,
    /// Identities of the copies, in array order. The length is the copy count.
    pub new_ids: Vec<ElementId>,
    /// The placement rule between neighbouring copies.
    pub step: ArrayStep,
}

impl ArrayElements {
    /// Admits the request: one to [`MAX_ARRAY_COPIES`] copies (`LIMIT_EXCEEDED`), distinct
    /// identities that differ from the source (`DUPLICATE_ID`), a valid label, a nonzero
    /// direction or axis, a positive spacing and finite angles (`NON_FINITE_NUMBER`,
    /// `INVALID_ARGUMENT`, `LIMIT_EXCEEDED`). That the source exists, the placements stay in
    /// range and the identities are unused are the engine's decisions.
    pub fn validate_arguments(&self) -> Result<(), CommandError> {
        if self.new_ids.is_empty() || self.new_ids.len() > MAX_ARRAY_COPIES {
            return Err(CommandError::new(
                ErrorCode::LimitExceeded,
                "new_ids",
                "an array has one to one hundred copies",
            )
            .with_rule("DV-LIM-001"));
        }
        if self.new_ids.contains(&self.source_id) {
            return Err(CommandError::new(
                ErrorCode::DuplicateId,
                "new_ids",
                "a copy identity equals the source",
            )
            .with_rule("DV-BASE-002"));
        }
        let mut seen = Vec::new();
        for id in &self.new_ids {
            if seen.contains(id) {
                return Err(CommandError::new(
                    ErrorCode::DuplicateId,
                    "new_ids",
                    "a copy identity repeats in the array",
                )
                .with_rule("DV-BASE-002"));
            }
            seen.push(*id);
        }
        if self.name.as_deref().is_some_and(|n| !label_is_valid(n)) {
            return Err(CommandError::new(
                ErrorCode::InvalidArgument,
                "name",
                "label must be 1 to 256 bytes without control characters",
            )
            .with_rule("DV-BASE-003"));
        }
        let zero = |values: &[f64]| values.iter().all(|v| *v == 0.0);
        match self.step {
            ArrayStep::Linear {
                direction,
                spacing_mm,
            } => {
                if direction.iter().any(|v| !v.is_finite()) || !spacing_mm.is_finite() {
                    return Err(CommandError::new(
                        ErrorCode::NonFiniteNumber,
                        "step",
                        "value is not finite",
                    )
                    .with_rule("DV-ATTR-003"));
                }
                if zero(&direction) {
                    return Err(CommandError::new(
                        ErrorCode::InvalidArgument,
                        "step",
                        "the array direction is the zero vector",
                    ));
                }
                if spacing_mm <= 0.0 || spacing_mm > COORDINATE_CAP_MM {
                    return Err(CommandError::new(
                        ErrorCode::LimitExceeded,
                        "step",
                        "the spacing must be positive and within the coordinate cap",
                    )
                    .with_rule("DV-LIM-001"));
                }
            }
            ArrayStep::Polar {
                point_mm,
                axis,
                angle_rad,
            } => {
                if point_mm.iter().chain(axis.iter()).any(|v| !v.is_finite())
                    || !angle_rad.is_finite()
                {
                    return Err(CommandError::new(
                        ErrorCode::NonFiniteNumber,
                        "step",
                        "value is not finite",
                    )
                    .with_rule("DV-ATTR-003"));
                }
                if zero(&axis) {
                    return Err(CommandError::new(
                        ErrorCode::InvalidArgument,
                        "step",
                        "the array axis is the zero vector",
                    ));
                }
                if angle_rad == 0.0 || angle_rad.abs() > ROLL_CAP_RAD {
                    return Err(CommandError::new(
                        ErrorCode::LimitExceeded,
                        "step",
                        "the angle must be nonzero and within the rotation cap",
                    )
                    .with_rule("DV-LIM-001"));
                }
            }
        }
        Ok(())
    }

    /// SHA-256 of a versioned, length-prefixed encoding of the validated request.
    pub fn content_fingerprint(&self) -> Result<ContentDigest, CommandError> {
        self.validate_arguments()?;
        let mut bytes = Vec::with_capacity(128);
        bytes.extend_from_slice(b"BeamCraft/T03/ArrayElements/v1\0");
        bytes.extend_from_slice(self.source_id.as_bytes());
        match &self.name {
            Some(name) => {
                bytes.push(1);
                bytes.extend_from_slice(&(name.len() as u64).to_le_bytes());
                bytes.extend_from_slice(name.as_bytes());
            }
            None => bytes.push(0),
        }
        bytes.extend_from_slice(&(self.new_ids.len() as u64).to_le_bytes());
        for id in &self.new_ids {
            bytes.extend_from_slice(id.as_bytes());
        }
        let push_vec = |bytes: &mut Vec<u8>, values: [f64; 3]| {
            for v in values {
                bytes.extend_from_slice(&canonical_f64_bytes(v).map_err(|_| {
                    CommandError::new(
                        ErrorCode::NonFiniteNumber,
                        "step",
                        "value cannot be encoded",
                    )
                    .with_rule("DV-ATTR-003")
                })?);
            }
            Ok(())
        };
        match self.step {
            ArrayStep::Linear {
                direction,
                spacing_mm,
            } => {
                bytes.push(1);
                push_vec(&mut bytes, direction)?;
                bytes.extend_from_slice(&canonical_f64_bytes(spacing_mm).map_err(|_| {
                    CommandError::new(
                        ErrorCode::NonFiniteNumber,
                        "step",
                        "value cannot be encoded",
                    )
                    .with_rule("DV-ATTR-003")
                })?);
            }
            ArrayStep::Polar {
                point_mm,
                axis,
                angle_rad,
            } => {
                bytes.push(2);
                push_vec(&mut bytes, point_mm)?;
                push_vec(&mut bytes, axis)?;
                bytes.extend_from_slice(&canonical_f64_bytes(angle_rad).map_err(|_| {
                    CommandError::new(
                        ErrorCode::NonFiniteNumber,
                        "step",
                        "value cannot be encoded",
                    )
                    .with_rule("DV-ATTR-003")
                })?);
            }
        }
        Sha256::digest(&bytes).map_err(|_| {
            CommandError::new(ErrorCode::LimitExceeded, "content", "content is too large")
                .with_rule("DV-LIM-001")
        })
    }
}

/// A01 `CreateAssembly`: a new `assembly` element (domain vocabulary section 2.3), a grouping
/// target of `part-of` with no shape, material or derived body of its own. It carries the common
/// base only; its placement is the identity in this slice.
#[derive(Debug, Clone, PartialEq)]
pub struct CreateAssembly {
    /// Identity of the new assembly.
    pub new_id: ElementId,
    /// Explicit user label.
    pub name: String,
}

impl CreateAssembly {
    /// Admits the request: the label follows the F04 rule (`DV-BASE-003`). Whether the identity
    /// is free (`DUPLICATE_ID`) is the engine's decision.
    pub fn validate_arguments(&self) -> Result<(), CommandError> {
        if !label_is_valid(&self.name) {
            return Err(CommandError::new(
                ErrorCode::InvalidArgument,
                "name",
                "label must be 1 to 256 bytes without control characters",
            )
            .with_rule("DV-BASE-003"));
        }
        Ok(())
    }

    /// SHA-256 of a versioned, length-prefixed encoding of the validated request.
    pub fn content_fingerprint(&self) -> Result<ContentDigest, CommandError> {
        self.validate_arguments()?;
        let mut bytes = Vec::with_capacity(64 + self.name.len());
        bytes.extend_from_slice(b"BeamCraft/A01/CreateAssembly/v1\0");
        bytes.extend_from_slice(self.new_id.as_bytes());
        bytes.extend_from_slice(&(self.name.len() as u64).to_le_bytes());
        bytes.extend_from_slice(self.name.as_bytes());
        Sha256::digest(&bytes).map_err(|_| {
            CommandError::new(ErrorCode::LimitExceeded, "content", "content is too large")
                .with_rule("DV-LIM-001")
        })
    }
}

/// A01 `AttachToAssembly`: authors the one `part-of` record of `part_id` (domain vocabulary
/// section 5.1), so the part becomes a member of `assembly_id`. The record lives with its source,
/// the part; the assembly is only read.
#[derive(Debug, Clone, PartialEq)]
pub struct AttachToAssembly {
    /// The part: a plate, beam, column or (for nesting) another assembly.
    pub part_id: ElementId,
    /// The target assembly.
    pub assembly_id: ElementId,
}

impl AttachToAssembly {
    /// Admits the request: a self-link is `INVALID_ARGUMENT` (`DV-REL-006`). Kinds, cardinality
    /// and cycles (`DV-REL-003` to `DV-REL-005`) need the model and are the engine's decisions.
    pub fn validate_arguments(&self) -> Result<(), CommandError> {
        if self.part_id == self.assembly_id {
            return Err(CommandError::new(
                ErrorCode::InvalidArgument,
                "assembly_id",
                "an element cannot be a part of itself",
            )
            .with_rule("DV-REL-006"));
        }
        Ok(())
    }

    /// SHA-256 of a versioned encoding of the validated request.
    pub fn content_fingerprint(&self) -> Result<ContentDigest, CommandError> {
        self.validate_arguments()?;
        let mut bytes = Vec::with_capacity(80);
        bytes.extend_from_slice(b"BeamCraft/A01/AttachToAssembly/v1\0");
        bytes.extend_from_slice(self.part_id.as_bytes());
        bytes.extend_from_slice(self.assembly_id.as_bytes());
        Sha256::digest(&bytes).map_err(|_| {
            CommandError::new(ErrorCode::LimitExceeded, "content", "content is too large")
                .with_rule("DV-LIM-001")
        })
    }
}

/// A16.1 `SetAssemblyMainPart`: names `part_id` as the assembly's one main part (A16.1). The
/// record is stored on the assembly as its `bim:mainPart` relationship; the part is only read.
#[derive(Debug, Clone, PartialEq)]
pub struct SetAssemblyMainPart {
    /// The assembly whose main part is named.
    pub assembly_id: ElementId,
    /// The part: a direct member of the assembly (plate, beam, column or nested assembly).
    pub part_id: ElementId,
}

impl SetAssemblyMainPart {
    /// SHA-256 of a versioned encoding of the request. That the part exists and is a direct
    /// member of the assembly, and that the main part is unique, are the engine's decisions.
    pub fn content_fingerprint(&self) -> Result<ContentDigest, CommandError> {
        let mut bytes = Vec::with_capacity(80);
        bytes.extend_from_slice(b"BeamCraft/A16/SetAssemblyMainPart/v1\0");
        bytes.extend_from_slice(self.assembly_id.as_bytes());
        bytes.extend_from_slice(self.part_id.as_bytes());
        Sha256::digest(&bytes).map_err(|_| {
            CommandError::new(ErrorCode::LimitExceeded, "content", "content is too large")
                .with_rule("DV-LIM-001")
        })
    }
}

/// A16 `ExplodeAssembly`: dissolves one assembly level. The assembly element is deleted and its
/// direct parts lose their `part-of` records; geometry, placement and attributes of the parts do
/// not change, and a nested sub-assembly stays an assembly with its own parts (decision of
/// 2026-10-21: explode means dissolve).
#[derive(Debug, Clone, PartialEq)]
pub struct ExplodeAssembly {
    /// The assembly to dissolve.
    pub assembly_id: ElementId,
}

impl ExplodeAssembly {
    /// SHA-256 of a versioned encoding of the request. That the target exists and is an assembly
    /// is the engine's decision.
    pub fn content_fingerprint(&self) -> Result<ContentDigest, CommandError> {
        let mut bytes = Vec::with_capacity(64);
        bytes.extend_from_slice(b"BeamCraft/A16/ExplodeAssembly/v1\0");
        bytes.extend_from_slice(self.assembly_id.as_bytes());
        Sha256::digest(&bytes).map_err(|_| {
            CommandError::new(ErrorCode::LimitExceeded, "content", "content is too large")
                .with_rule("DV-LIM-001")
        })
    }
}

/// A01 `DetachFromAssembly`: removes the `part-of` record of `part_id`. The former assembly is
/// not written and stays, even when it has no parts left.
#[derive(Debug, Clone, PartialEq)]
pub struct DetachFromAssembly {
    /// The part whose membership ends.
    pub part_id: ElementId,
}

impl DetachFromAssembly {
    /// SHA-256 of a versioned encoding of the request. Whether the part exists and is a member
    /// of an assembly are the engine's decisions.
    pub fn content_fingerprint(&self) -> Result<ContentDigest, CommandError> {
        let mut bytes = Vec::with_capacity(64);
        bytes.extend_from_slice(b"BeamCraft/A01/DetachFromAssembly/v1\0");
        bytes.extend_from_slice(self.part_id.as_bytes());
        Sha256::digest(&bytes).map_err(|_| {
            CommandError::new(ErrorCode::LimitExceeded, "content", "content is too large")
                .with_rule("DV-LIM-001")
        })
    }
}

/// D2 `CreateLevel`: a new `level` element (a storey): a name and an elevation in millimetres.
/// A level has no geometry and no placement of its own; plates and members are assigned to it
/// through `UpdateElement` (`ElementPatch::level`), and the elevation moves no element.
#[derive(Debug, Clone, PartialEq)]
pub struct CreateLevel {
    /// Identity of the new level.
    pub new_id: ElementId,
    /// Explicit user label, 1 to 256 UTF-8 bytes without control characters.
    pub name: String,
    /// Elevation in millimetres: finite and at most 1,000,000,000 in magnitude (the coordinate
    /// cap). Negative elevations are allowed (a basement).
    pub elevation_mm: f64,
}

impl CreateLevel {
    /// Admits the request: the label follows the F04 rule (`INVALID_ARGUMENT`), the elevation is
    /// finite (`NON_FINITE_NUMBER`) and within the coordinate cap (`LIMIT_EXCEEDED`). Whether the
    /// identity is free (`DUPLICATE_ID`) is the engine's decision.
    pub fn validate_arguments(&self) -> Result<(), CommandError> {
        if !label_is_valid(&self.name) {
            return Err(CommandError::new(
                ErrorCode::InvalidArgument,
                "name",
                "label must be 1 to 256 bytes without control characters",
            )
            .with_rule("DV-BASE-003"));
        }
        if !self.elevation_mm.is_finite() {
            return Err(CommandError::new(
                ErrorCode::NonFiniteNumber,
                "elevation_mm",
                "value is not finite",
            )
            .with_rule("DV-ATTR-003"));
        }
        if self.elevation_mm.abs() > COORDINATE_CAP_MM {
            return Err(CommandError::new(
                ErrorCode::LimitExceeded,
                "elevation_mm",
                "elevation exceeds the coordinate cap",
            )
            .with_rule("DV-LIM-001"));
        }
        Ok(())
    }

    /// SHA-256 of a versioned, length-prefixed encoding of the validated request.
    pub fn content_fingerprint(&self) -> Result<ContentDigest, CommandError> {
        self.validate_arguments()?;
        let mut bytes = Vec::with_capacity(72 + self.name.len());
        bytes.extend_from_slice(b"BeamCraft/D2/CreateLevel/v1\0");
        bytes.extend_from_slice(self.new_id.as_bytes());
        bytes.extend_from_slice(&(self.name.len() as u64).to_le_bytes());
        bytes.extend_from_slice(self.name.as_bytes());
        bytes.extend_from_slice(&canonical_f64_bytes(self.elevation_mm).map_err(|_| {
            CommandError::new(
                ErrorCode::NonFiniteNumber,
                "elevation_mm",
                "value cannot be encoded",
            )
            .with_rule("DV-ATTR-003")
        })?);
        Sha256::digest(&bytes).map_err(|_| {
            CommandError::new(ErrorCode::LimitExceeded, "content", "content is too large")
                .with_rule("DV-LIM-001")
        })
    }
}

/// A08 `ApproveMark`: a person approves the mark of one element, freezing its value. Whether the
/// element exists, carries a mark and is not already approved are the engine's decisions.
#[derive(Debug, Clone, PartialEq)]
pub struct ApproveMark {
    /// The element whose mark is approved.
    pub target_id: ElementId,
}

impl ApproveMark {
    /// SHA-256 of a versioned encoding of the request.
    pub fn content_fingerprint(&self) -> Result<ContentDigest, CommandError> {
        let mut bytes = Vec::with_capacity(64);
        bytes.extend_from_slice(b"BeamCraft/A08/ApproveMark/v1\0");
        bytes.extend_from_slice(self.target_id.as_bytes());
        Sha256::digest(&bytes).map_err(|_| {
            CommandError::new(ErrorCode::LimitExceeded, "content", "content is too large")
                .with_rule("DV-LIM-001")
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id::OpId;
    use crate::limits::MAX_BOLT_GROUP_SIZE;
    use crate::steel::command::{
        AddHole, BoltGroupInputs, BoltLength, BoltSpacing, BoltSpec, CreateBolt, CreateBoltGroup,
        CreatePlate, CreatePolybeam, CreateStraightMember, CreateWeld, CutByPlane, FastenerSpec,
        HOLE_POINT_CAP, HoleSpec, KeepSide, MAX_HOLES, MemberKind, SectionPlacement,
        SectionPlanePosition, UpdateBoltGroup, WeldContour, WeldGeometry, WeldLocation, WeldSide,
        WeldSpec, WeldSymbol, WeldType,
    };

    fn plate() -> CreatePlate {
        CreatePlate {
            new_id: ElementId::parse("019a7bf0-0000-7000-8000-000000000005").unwrap(),
            name: "Synthetic plate".into(),
            class: 0,
            contour_points_mm: vec![
                [0.0, 0.0, 0.0],
                [1000.0, 0.0, 0.0],
                [1000.0, 500.0, 0.0],
                [0.0, 500.0, 0.0],
            ],
            thickness_mm: 10.0,
            thickness_side: None,
            corners: Vec::new(),
            placement: Placement::IDENTITY,
            material_digest: None,
            holes_mm: Vec::new(),
        }
    }

    fn code(plate: &CreatePlate) -> ErrorCode {
        plate.validate_arguments().unwrap_err().code
    }

    #[test]
    fn the_thickness_side_has_wire_tokens_exact_offsets_and_its_own_fingerprint() {
        // The tokens round-trip; anything else does not parse.
        for (side, token) in [
            (PlateSide::Top, "top"),
            (PlateSide::Middle, "middle"),
            (PlateSide::Bottom, "bottom"),
        ] {
            assert_eq!(side.as_str(), token);
            assert_eq!(PlateSide::parse(token), Some(side));
        }
        assert_eq!(PlateSide::parse("above"), None);
        // The lower face sits at 0, -t/2 and -t; halving and negating are exact.
        assert_eq!(PlateSide::Top.offset_mm(10.0), 0.0);
        assert_eq!(PlateSide::Middle.offset_mm(10.0), -5.0);
        assert_eq!(PlateSide::Bottom.offset_mm(10.0), -10.0);
        // Absence is the top default in content: an older plate and an explicit top plate
        // evaluate the same body, but the authored arguments stay distinguishable.
        let base = plate();
        let top = CreatePlate {
            thickness_side: Some(PlateSide::Top),
            ..base.clone()
        };
        let middle = CreatePlate {
            thickness_side: Some(PlateSide::Middle),
            ..base.clone()
        };
        assert_ne!(
            base.content_fingerprint().unwrap(),
            middle.content_fingerprint().unwrap()
        );
        assert_ne!(
            top.content_fingerprint().unwrap(),
            middle.content_fingerprint().unwrap()
        );
        assert_ne!(
            base.content_fingerprint().unwrap(),
            top.content_fingerprint().unwrap()
        );
    }

    #[test]
    fn corner_treatments_are_admitted_per_vertex_and_in_the_fingerprint() {
        // A12.2: one treatment per vertex, each naming a real vertex, each with bounded values.
        let chamfer = |vertex, d1, d2| {
            let mut c = plate();
            c.corners = vec![PlateCorner {
                vertex,
                treatment: CornerTreatment::Chamfer {
                    d1_mm: d1,
                    d2_mm: d2,
                },
            }];
            c
        };
        let round = |vertex, r| {
            let mut c = plate();
            c.corners = vec![PlateCorner {
                vertex,
                treatment: CornerTreatment::Round { radius_mm: r },
            }];
            c
        };
        assert!(chamfer(0, 100.0, 100.0).validate_arguments().is_ok());
        assert!(round(3, 50.0).validate_arguments().is_ok());
        // A vertex that does not exist, and a vertex named twice.
        assert_eq!(code(&chamfer(4, 100.0, 100.0)), ErrorCode::InvalidArgument);
        let mut twice = plate();
        twice.corners = vec![
            PlateCorner {
                vertex: 1,
                treatment: CornerTreatment::Round { radius_mm: 10.0 },
            },
            PlateCorner {
                vertex: 1,
                treatment: CornerTreatment::Round { radius_mm: 20.0 },
            },
        ];
        assert_eq!(code(&twice), ErrorCode::InvalidArgument);
        // Zero, negative, non-finite and over the cap (the amendment's negative case 1).
        assert_eq!(code(&chamfer(0, 0.0, 100.0)), ErrorCode::InvalidArgument);
        assert_eq!(code(&round(0, -1.0)), ErrorCode::InvalidArgument);
        assert_eq!(code(&round(0, f64::NAN)), ErrorCode::NonFiniteNumber);
        assert_eq!(
            code(&round(0, COORDINATE_CAP_MM * 2.0)),
            ErrorCode::LimitExceeded
        );
        // Different treatments fingerprint differently, and the empty list differs from a
        // treated plate (the v8 tag covers the new block).
        let base = plate().content_fingerprint().unwrap();
        let a = chamfer(0, 100.0, 100.0).content_fingerprint().unwrap();
        let b = chamfer(0, 100.0, 50.0).content_fingerprint().unwrap();
        let c = round(0, 100.0).content_fingerprint().unwrap();
        let other_vertex = chamfer(1, 100.0, 100.0).content_fingerprint().unwrap();
        assert_ne!(base, a);
        assert_ne!(a, b);
        assert_ne!(a, c);
        assert_ne!(a, other_vertex);
    }

    fn hole() -> Vec<[f64; 2]> {
        vec![
            [100.0, 100.0],
            [100.0, 200.0],
            [200.0, 200.0],
            [200.0, 100.0],
        ]
    }

    fn add_hole() -> AddHole {
        AddHole {
            target_id: ElementId::parse("019a7bf0-0000-7000-8000-000000000006").unwrap(),
            operation_id: OpId::parse("019a7bf0-0000-7000-8000-000000000007").unwrap(),
            hole: HoleSpec::Polygon(hole()),
        }
    }

    fn hole_code(command: &AddHole) -> ErrorCode {
        command.validate_arguments().unwrap_err().code
    }

    #[test]
    fn holes_have_caps_codes_and_part_of_the_fingerprint() {
        let solid = plate().content_fingerprint().unwrap();
        let mut p = plate();
        p.holes_mm = vec![hole()];
        assert!(p.validate_arguments().is_ok());
        assert_ne!(p.content_fingerprint().unwrap(), solid);
        let mut moved = p.clone();
        moved.holes_mm[0][0][0] = 101.0;
        assert_ne!(
            moved.content_fingerprint().unwrap(),
            p.content_fingerprint().unwrap()
        );
        let mut bad = plate();
        bad.holes_mm = vec![vec![[1.0, 1.0], [2.0, 2.0]]];
        assert_eq!(code(&bad), ErrorCode::InvalidArgument);
        bad.holes_mm = vec![vec![[f64::NAN, 1.0], [2.0, 2.0], [3.0, 1.0]]];
        assert_eq!(code(&bad), ErrorCode::NonFiniteNumber);
        bad.holes_mm = vec![vec![[2e9, 1.0], [2.0, 2.0], [3.0, 1.0]]];
        assert_eq!(code(&bad), ErrorCode::LimitExceeded);
        bad.holes_mm = vec![hole(); MAX_HOLES + 1];
        assert_eq!(code(&bad), ErrorCode::LimitExceeded);
        bad.holes_mm = vec![vec![[0.0, 0.0]; HOLE_POINT_CAP + 1]];
        assert_eq!(code(&bad), ErrorCode::LimitExceeded);
        bad.holes_mm = vec![hole(); MAX_HOLES];
        assert!(bad.validate_arguments().is_ok());
    }

    #[test]
    fn an_add_hole_admits_generators_and_fingerprints_its_content() {
        let command = add_hole();
        assert!(command.validate_arguments().is_ok());
        // The polygon is authoritative; the generator parameters are part of the content.
        assert_eq!(command.polygon().unwrap(), hole());
        let fingerprint = command.content_fingerprint().unwrap();
        assert_eq!(command.content_fingerprint().unwrap(), fingerprint);
        let mut moved = command.clone();
        moved.hole = HoleSpec::Polygon(vec![
            [100.0, 100.0],
            [100.0, 201.0],
            [200.0, 200.0],
            [200.0, 100.0],
        ]);
        assert_ne!(moved.content_fingerprint().unwrap(), fingerprint);
        // The same polygon from a generator is still a different command.
        let mut from_generator = command.clone();
        from_generator.hole = HoleSpec::Rectangle {
            corner_mm: [100.0, 100.0],
            width_mm: 100.0,
            height_mm: 100.0,
        };
        assert_ne!(from_generator.content_fingerprint().unwrap(), fingerprint);
        assert_eq!(from_generator.polygon().unwrap(), hole());
    }

    #[test]
    fn a_round_hole_decides_n_from_the_fixed_threshold_table() {
        let round = |diameter: f64| HoleSpec::Round {
            centre_mm: [250.0, 250.0],
            diameter_mm: diameter,
        };
        // The examples of F06 section 2: about 5.2 mm gives N = 16, 100 mm gives 72 and
        // 500 mm gives 160.
        assert_eq!(round(5.2).polygon().unwrap().len(), 16);
        assert_eq!(round(100.0).polygon().unwrap().len(), 72);
        assert_eq!(round(500.0).polygon().unwrap().len(), 160);
        // A radius above a threshold needs the next multiple of four.
        assert_eq!(round(5.3).polygon().unwrap().len(), 20);
        // A radius above the last threshold needs more than 256 segments and is refused,
        // because no Approximated status exists.
        let error = round(1329.0).polygon().unwrap_err();
        assert_eq!(error.code, ErrorCode::UnsupportedGeometry);
        assert_eq!(error.field, "hole.radius");
        // Vertices start on +X of the centre and run clockwise (below it next).
        let polygon = round(100.0).polygon().unwrap();
        assert_eq!(polygon[0], [300.0, 250.0]);
        assert!(polygon[1][1] < 250.0);
    }

    #[test]
    fn an_add_hole_carries_the_caps_codes() {
        let mut bad = add_hole();
        bad.hole = HoleSpec::Polygon(vec![[1.0, 1.0], [2.0, 2.0]]);
        assert_eq!(hole_code(&bad), ErrorCode::InvalidArgument);
        bad.hole = HoleSpec::Polygon(vec![[f64::NAN, 1.0], [2.0, 2.0], [3.0, 1.0]]);
        assert_eq!(hole_code(&bad), ErrorCode::NonFiniteNumber);
        bad.hole = HoleSpec::Polygon(vec![[2e9, 1.0], [2.0, 2.0], [3.0, 1.0]]);
        assert_eq!(hole_code(&bad), ErrorCode::LimitExceeded);
        bad.hole = HoleSpec::Polygon(vec![[0.0, 0.0]; HOLE_POINT_CAP + 1]);
        assert_eq!(hole_code(&bad), ErrorCode::LimitExceeded);
        bad.hole = HoleSpec::Round {
            centre_mm: [0.0, 0.0],
            diameter_mm: 0.0,
        };
        assert_eq!(hole_code(&bad), ErrorCode::InvalidArgument);
        bad.hole = HoleSpec::Round {
            centre_mm: [0.0, 0.0],
            diameter_mm: f64::NAN,
        };
        assert_eq!(hole_code(&bad), ErrorCode::NonFiniteNumber);
        bad.hole = HoleSpec::Rectangle {
            corner_mm: [0.0, 0.0],
            width_mm: 0.0,
            height_mm: 1.0,
        };
        assert_eq!(hole_code(&bad), ErrorCode::InvalidArgument);
        bad.hole = HoleSpec::Rectangle {
            corner_mm: [2e9, 0.0],
            width_mm: 1.0,
            height_mm: 1.0,
        };
        assert_eq!(hole_code(&bad), ErrorCode::LimitExceeded);
    }

    fn cut() -> CutByPlane {
        CutByPlane {
            target_id: ElementId::parse("019a7bf0-0000-7000-8000-000000000005").unwrap(),
            operation_id: OpId::parse("019a7bf0-0000-7000-8000-0000000000c7").unwrap(),
            point_mm: [250.0, 0.0, 0.0],
            normal: [1.0, 0.0, 0.0],
            keep: KeepSide::Negative,
        }
    }

    #[test]
    fn a_cut_command_has_caps_codes_and_a_content_fingerprint() {
        let c = cut();
        assert!(c.validate_arguments().is_ok());
        let base = c.content_fingerprint().unwrap();
        assert_eq!(base, c.content_fingerprint().unwrap());
        let mut moved = c;
        moved.point_mm[0] = 251.0;
        assert_ne!(moved.content_fingerprint().unwrap(), base);
        let mut flipped = c;
        flipped.keep = KeepSide::Positive;
        assert_ne!(flipped.content_fingerprint().unwrap(), base);
        // A zero normal is not a plane; finiteness and the coordinate cap come first.
        let zero = CutByPlane {
            normal: [0.0; 3],
            ..cut()
        };
        assert_eq!(
            zero.validate_arguments().unwrap_err().code,
            ErrorCode::InvalidArgument
        );
        let nan = CutByPlane {
            point_mm: [f64::NAN, 0.0, 0.0],
            ..cut()
        };
        assert_eq!(
            nan.validate_arguments().unwrap_err().code,
            ErrorCode::NonFiniteNumber
        );
        let wide = CutByPlane {
            point_mm: [2e9, 0.0, 0.0],
            ..cut()
        };
        assert_eq!(
            wide.validate_arguments().unwrap_err().code,
            ErrorCode::LimitExceeded
        );
        assert_eq!(KeepSide::parse("positive"), Some(KeepSide::Positive));
        assert_eq!(KeepSide::parse("sideways"), None);
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
            class: 0,
            axis_points_mm: [[0.0; 3], [6000.0, 0.0, 0.0]],
            profile_digest: digest(b"profile"),
            placement: Placement::IDENTITY,
            roll_rad: None,
            section_placement: None,
            material_digest: None,
        }
    }

    #[test]
    fn the_section_placement_tokens_half_shifts_and_default_are_exact() {
        use crate::steel::command::SectionDepthPosition as D;
        use SectionPlanePosition as P;
        for (token, plane) in [
            ("centre", P::Centre),
            ("left", P::Left),
            ("right", P::Right),
        ] {
            assert_eq!(P::parse(token), Some(plane));
            assert_eq!(plane.as_str(), token);
        }
        for (token, depth) in [
            ("centre", D::Centre),
            ("front", D::Front),
            ("back", D::Back),
        ] {
            assert_eq!(D::parse(token), Some(depth));
            assert_eq!(depth.as_str(), token);
        }
        assert_eq!(P::parse("above"), None);
        assert_eq!(D::parse("top"), None);
        // Left and front are the negative halves; half shifts are exact.
        assert_eq!(P::Centre.half_shift_mm(200.0), 0.0);
        assert_eq!(P::Left.half_shift_mm(200.0), -100.0);
        assert_eq!(P::Right.half_shift_mm(200.0), 100.0);
        assert_eq!(D::Front.half_shift_mm(300.0), -150.0);
        assert_eq!(D::Back.half_shift_mm(300.0), 150.0);
        assert!(SectionPlacement::default().is_default());
        let neg_zero = SectionPlacement {
            plane_offset_mm: -0.0,
            ..SectionPlacement::default()
        };
        assert!(neg_zero.is_default(), "-0.0 is 0.0");
        let shifted = SectionPlacement {
            plane_offset_mm: 0.5,
            ..SectionPlacement::default()
        };
        assert!(!shifted.is_default());
    }

    #[test]
    fn a_member_section_placement_has_the_argument_rules_and_its_own_fingerprint() {
        fn placed(mutate: impl FnOnce(&mut SectionPlacement)) -> SectionPlacement {
            let mut p = SectionPlacement::default();
            mutate(&mut p);
            p
        }
        let base = member().content_fingerprint().unwrap();
        let mut m = member();
        m.section_placement = Some(placed(|p| p.plane = SectionPlanePosition::Left));
        assert!(m.validate_arguments().is_ok());
        assert_ne!(m.content_fingerprint().unwrap(), base);
        let left = m.content_fingerprint().unwrap();
        m.section_placement = Some(placed(|p| p.plane = SectionPlanePosition::Right));
        assert_ne!(m.content_fingerprint().unwrap(), left);
        for mutate in [
            |p: &mut SectionPlacement| p.plane_offset_mm = f64::NAN,
            |p: &mut SectionPlacement| p.start_offset_mm = f64::INFINITY,
            |p: &mut SectionPlacement| p.end_offset_mm = f64::NAN,
        ] {
            let mut m = member();
            m.section_placement = Some(placed(mutate));
            assert_eq!(
                m.validate_arguments().unwrap_err().code,
                ErrorCode::NonFiniteNumber
            );
        }
        let mut m = member();
        m.section_placement = Some(placed(|p| p.plane_offset_mm = 1_000_000.5));
        assert_eq!(
            m.validate_arguments().unwrap_err().code,
            ErrorCode::LimitExceeded
        );
        // An explicit default placement is valid authored input of its own.
        let mut m = member();
        m.section_placement = Some(SectionPlacement::default());
        assert!(m.validate_arguments().is_ok());
        assert_ne!(m.content_fingerprint().unwrap(), base);
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

    fn polybeam() -> CreatePolybeam {
        CreatePolybeam {
            new_id: ElementId::parse("019a7bf0-0000-7000-8000-000000000017").unwrap(),
            kind: MemberKind::Beam,
            name: "Polybeam".into(),
            class: 0,
            path_points_mm: vec![[0.0; 3], [3000.0, 0.0, 0.0], [3000.0, 4000.0, 0.0]],
            profile_digest: crate::digest::Sha256::digest(b"profile").unwrap(),
            placement: Placement::IDENTITY,
            material_digest: None,
        }
    }

    #[test]
    fn a_polybeam_has_the_f04_argument_rules() {
        use crate::limits::MAX_MEMBER_PATH_POINTS;
        assert!(polybeam().validate_arguments().is_ok());
        let code = |m: CreatePolybeam| m.validate_arguments().unwrap_err().code;
        // Two points author a straight member (P03), never a polybeam.
        let mut m = polybeam();
        m.path_points_mm.truncate(2);
        assert_eq!(code(m), ErrorCode::InvalidArgument);
        let mut m = polybeam();
        m.path_points_mm = vec![[0.0; 3]; MAX_MEMBER_PATH_POINTS + 1];
        assert_eq!(code(m), ErrorCode::LimitExceeded);
        let mut m = polybeam();
        m.path_points_mm[1][2] = f64::NAN;
        assert_eq!(code(m), ErrorCode::NonFiniteNumber);
        let mut m = polybeam();
        m.path_points_mm[2][1] = 1_000_000_000.5;
        assert_eq!(code(m), ErrorCode::LimitExceeded);
        let mut m = polybeam();
        m.name = "line\nbreak".into();
        assert_eq!(code(m), ErrorCode::InvalidArgument);
        let mut m = polybeam();
        m.placement.translation_mm[1] = f64::INFINITY;
        assert_eq!(code(m), ErrorCode::NonFiniteNumber);
    }

    #[test]
    fn the_polybeam_fingerprint_tracks_the_whole_path() {
        let base = polybeam().content_fingerprint().unwrap();
        for mutate in [
            |m: &mut CreatePolybeam| m.kind = MemberKind::Column,
            |m: &mut CreatePolybeam| m.name = "Other".into(),
            |m: &mut CreatePolybeam| {
                m.path_points_mm[1] = [3000.0, 1000.0, 0.0];
            },
            |m: &mut CreatePolybeam| m.path_points_mm.push([6000.0, 4000.0, 0.0]),
            |m: &mut CreatePolybeam| m.placement.translation_mm[1] = 1.0,
        ] {
            let mut m = polybeam();
            mutate(&mut m);
            assert_ne!(m.content_fingerprint().unwrap(), base);
        }
    }

    fn copy() -> CopyElement {
        CopyElement {
            source_id: ElementId::parse("019a7bf0-0000-7000-8000-000000000005").unwrap(),
            new_id: ElementId::parse("019a7bf0-0000-7000-8000-000000000008").unwrap(),
            name: None,
            offset_mm: [100.0, 0.0, 0.0],
        }
    }

    #[test]
    fn a_copy_has_the_f04_argument_rules() {
        assert!(copy().validate_arguments().is_ok());
        let code = |c: CopyElement| c.validate_arguments().unwrap_err().code;
        let mut c = copy();
        c.new_id = c.source_id;
        assert_eq!(code(c), ErrorCode::DuplicateId);
        let mut c = copy();
        c.name = Some(String::new());
        assert_eq!(code(c), ErrorCode::InvalidArgument);
        let mut c = copy();
        c.offset_mm[2] = f64::NAN;
        assert_eq!(code(c), ErrorCode::NonFiniteNumber);
        let mut c = copy();
        c.offset_mm[0] = 1_000_000_001.0;
        assert_eq!(code(c), ErrorCode::LimitExceeded);
    }

    #[test]
    fn the_copy_fingerprint_tracks_every_input() {
        let base = copy().content_fingerprint().unwrap();
        for mutate in [
            |c: &mut CopyElement| c.name = Some("Other".into()),
            |c: &mut CopyElement| c.offset_mm[1] = 1.0,
            |c: &mut CopyElement| {
                c.new_id = ElementId::parse("019a7bf0-0000-7000-8000-000000000009").unwrap()
            },
            |c: &mut CopyElement| {
                c.source_id = ElementId::parse("019a7bf0-0000-7000-8000-000000000004").unwrap()
            },
        ] {
            let mut c = copy();
            mutate(&mut c);
            assert_ne!(c.content_fingerprint().unwrap(), base);
        }
    }

    fn id(n: u8) -> ElementId {
        ElementId::parse(&format!("019a7bf0-0000-7000-8000-0000000000{n:02x}")).unwrap()
    }

    #[test]
    fn an_orientation_patch_has_argument_rules_and_a_fingerprint() {
        let update = |orientation: Option<[f64; 4]>| UpdateElement {
            target_id: id(5),
            patch: ElementPatch {
                orientation_xyzw: orientation,
                name: Some("Beam".into()),
                ..ElementPatch::default()
            },
        };
        let quarter = [0.0, 0.0, 0.5_f64.sqrt(), 0.5_f64.sqrt()];
        let base = update(None).content_fingerprint().unwrap();
        assert_ne!(update(Some(quarter)).content_fingerprint().unwrap(), base);
        // Setting the identity is a real (clearing) change with its own fingerprint.
        assert_ne!(
            update(Some([0.0, 0.0, 0.0, 1.0]))
                .content_fingerprint()
                .unwrap(),
            base
        );
        let code = |u: UpdateElement| u.validate_arguments().unwrap_err().code;
        assert_eq!(
            code(update(Some([0.0, 0.0, 0.5, 0.5]))),
            ErrorCode::InvalidArgument
        );
        assert_eq!(code(update(Some([0.0; 4]))), ErrorCode::InvalidArgument);
        assert_eq!(
            code(update(Some([f64::NAN, 0.0, 0.0, 1.0]))),
            ErrorCode::NonFiniteNumber
        );
        assert!(update(Some(quarter)).validate_arguments().is_ok());
        // The fingerprint covers the quaternion bytes themselves.
        let negated = [-quarter[0], -quarter[1], -quarter[2], -quarter[3]];
        assert_ne!(
            update(Some(negated)).content_fingerprint().unwrap(),
            update(Some(quarter)).content_fingerprint().unwrap()
        );
    }

    #[test]
    fn non_finite_orientation_reports_the_domain_rule_in_validation_and_encoding() {
        for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            for component in 0..4 {
                let mut orientation = [0.0, 0.0, 0.0, 1.0];
                orientation[component] = value;
                let update = UpdateElement {
                    target_id: id(5),
                    patch: ElementPatch {
                        orientation_xyzw: Some(orientation),
                        ..ElementPatch::default()
                    },
                };
                for error in [
                    Placement::validate_orientation(orientation).unwrap_err(),
                    update.validate_arguments().unwrap_err(),
                    update.content_fingerprint().unwrap_err(),
                ] {
                    assert_eq!(error.code, ErrorCode::NonFiniteNumber);
                    assert_eq!(error.field, "placement.orientation_xyzw");
                    assert_eq!(error.rule_id, Some("DV-ATTR-003"));
                }
            }
        }
    }

    #[test]
    fn a_move_patch_has_argument_rules_and_a_fingerprint() {
        let update = |translation: [f64; 3]| UpdateElement {
            target_id: id(5),
            patch: ElementPatch {
                translation_mm: Some(translation),
                ..ElementPatch::default()
            },
        };
        assert!(update([1.0, 2.0, 3.0]).validate_arguments().is_ok());
        let code = |u: UpdateElement| u.validate_arguments().unwrap_err().code;
        assert_eq!(
            code(update([f64::INFINITY, 0.0, 0.0])),
            ErrorCode::NonFiniteNumber
        );
        assert_eq!(
            code(update([0.0, 1_000_000_001.0, 0.0])),
            ErrorCode::LimitExceeded
        );
        let base = update([1.0, 2.0, 3.0]).content_fingerprint().unwrap();
        assert_ne!(update([1.0, 2.0, 4.0]).content_fingerprint().unwrap(), base);
        // A patch without a move keeps the fingerprint it had before moves existed.
        let rename = UpdateElement {
            target_id: id(5),
            patch: ElementPatch {
                name: Some("A".into()),
                ..ElementPatch::default()
            },
        };
        assert_ne!(rename.content_fingerprint().unwrap(), base);
    }

    #[test]
    fn a_thickness_side_patch_has_its_own_fingerprint() {
        // A retry that differs only in the side must not replay as the same operation (A12.1).
        let update = |side: Option<PlateSide>| UpdateElement {
            target_id: id(5),
            patch: ElementPatch {
                thickness_side: side,
                ..ElementPatch::default()
            },
        };
        assert!(update(Some(PlateSide::Bottom)).validate_arguments().is_ok());
        // A patch that names no field changes nothing and stays inadmissible, so the three
        // sides are compared against each other.
        let top = update(Some(PlateSide::Top)).content_fingerprint().unwrap();
        let middle = update(Some(PlateSide::Middle))
            .content_fingerprint()
            .unwrap();
        let bottom = update(Some(PlateSide::Bottom))
            .content_fingerprint()
            .unwrap();
        assert_ne!(top, middle);
        assert_ne!(top, bottom);
        assert_ne!(middle, bottom);
        // The side rides behind the append-only tags, so older patches keep their digests.
        let rename = UpdateElement {
            target_id: id(5),
            patch: ElementPatch {
                name: Some("A".into()),
                ..ElementPatch::default()
            },
        };
        assert_ne!(rename.content_fingerprint().unwrap(), bottom);
    }

    #[test]
    fn the_delete_fingerprint_names_the_target() {
        let a = DeleteElement { target_id: id(5) };
        let b = DeleteElement { target_id: id(6) };
        assert_eq!(
            a.content_fingerprint().unwrap(),
            a.clone().content_fingerprint().unwrap()
        );
        assert_ne!(
            a.content_fingerprint().unwrap(),
            b.content_fingerprint().unwrap()
        );
    }

    #[test]
    fn assembly_commands_admit_labels_refuse_self_links_and_fingerprint_their_content() {
        let a = ElementId::parse("019a7bf0-0000-7000-8000-000000200001").unwrap();
        let b = ElementId::parse("019a7bf0-0000-7000-8000-000000200002").unwrap();
        let create = CreateAssembly {
            new_id: a,
            name: "Frame A".into(),
        };
        assert!(create.validate_arguments().is_ok());
        let renamed = CreateAssembly {
            name: "Frame B".into(),
            ..create.clone()
        };
        assert_ne!(
            create.content_fingerprint().unwrap(),
            renamed.content_fingerprint().unwrap()
        );
        for bad in ["", "tab\there"] {
            let c = CreateAssembly {
                name: bad.into(),
                ..create.clone()
            };
            assert_eq!(
                c.validate_arguments().unwrap_err().code,
                ErrorCode::InvalidArgument
            );
        }
        let attach = AttachToAssembly {
            part_id: a,
            assembly_id: b,
        };
        assert!(attach.validate_arguments().is_ok());
        let swapped = AttachToAssembly {
            part_id: b,
            assembly_id: a,
        };
        assert_ne!(
            attach.content_fingerprint().unwrap(),
            swapped.content_fingerprint().unwrap()
        );
        let own = AttachToAssembly {
            part_id: a,
            assembly_id: a,
        };
        assert_eq!(
            own.content_fingerprint().unwrap_err().code,
            ErrorCode::InvalidArgument
        );
        let detach = DetachFromAssembly { part_id: a };
        assert_ne!(
            detach.content_fingerprint().unwrap(),
            DetachFromAssembly { part_id: b }
                .content_fingerprint()
                .unwrap()
        );
        let main = SetAssemblyMainPart {
            assembly_id: b,
            part_id: a,
        };
        assert_ne!(
            main.content_fingerprint().unwrap(),
            SetAssemblyMainPart {
                assembly_id: b,
                part_id: b,
            }
            .content_fingerprint()
            .unwrap()
        );
        let explode = ExplodeAssembly { assembly_id: a };
        assert_ne!(
            explode.content_fingerprint().unwrap(),
            ExplodeAssembly { assembly_id: b }
                .content_fingerprint()
                .unwrap()
        );
    }

    #[test]
    fn create_weld_admits_two_distinct_parts_and_a_bounded_size() {
        let id =
            |n: u32| ElementId::parse(&format!("019a7bf0-0000-7000-8000-0000003{n:05}")).unwrap();
        let weld = CreateWeld {
            new_id: id(1),
            name: "W1".into(),
            main_id: id(2),
            secondary_id: id(3),
            weld: WeldSpec {
                weld_type: WeldType::Fillet,
                size_mm: 6.0,
                location: WeldLocation::Workshop,
                side: WeldSide::Arrow,
            },
            geometry: None,
        };
        assert!(weld.validate_arguments().is_ok());
        let base = weld.content_fingerprint().unwrap();
        // Every authored property and the part order are part of the content.
        let variants = [
            CreateWeld {
                secondary_id: id(2),
                main_id: id(3),
                ..weld.clone()
            },
            CreateWeld {
                weld: WeldSpec {
                    location: WeldLocation::Site,
                    ..weld.weld
                },
                ..weld.clone()
            },
            CreateWeld {
                weld: WeldSpec {
                    side: WeldSide::Both,
                    ..weld.weld
                },
                ..weld.clone()
            },
            CreateWeld {
                weld: WeldSpec {
                    size_mm: 8.0,
                    ..weld.weld
                },
                ..weld.clone()
            },
        ];
        for v in &variants {
            assert_ne!(v.content_fingerprint().unwrap(), base);
        }
        let refused = |w: CreateWeld| w.validate_arguments().unwrap_err().code;
        assert_eq!(
            refused(CreateWeld {
                secondary_id: id(2),
                ..weld.clone()
            }),
            ErrorCode::InvalidArgument
        );
        assert_eq!(
            refused(CreateWeld {
                new_id: id(2),
                ..weld.clone()
            }),
            ErrorCode::InvalidArgument
        );
        for size in [0.0, -1.0, 1000.5] {
            assert_eq!(
                refused(CreateWeld {
                    weld: WeldSpec {
                        size_mm: size,
                        ..weld.weld
                    },
                    ..weld.clone()
                }),
                ErrorCode::InvalidArgument
            );
        }
        assert_eq!(
            refused(CreateWeld {
                weld: WeldSpec {
                    size_mm: f64::NAN,
                    ..weld.weld
                },
                ..weld.clone()
            }),
            ErrorCode::NonFiniteNumber
        );
        assert_eq!(
            refused(CreateWeld {
                name: String::new(),
                ..weld.clone()
            }),
            ErrorCode::InvalidArgument
        );
        for token in ["fillet", "workshop", "site", "arrow", "other", "both"] {
            let back = WeldType::parse(token)
                .map(WeldType::as_token)
                .or(WeldLocation::parse(token).map(WeldLocation::as_token))
                .or(WeldSide::parse(token).map(WeldSide::as_token));
            assert_eq!(back, Some(token));
        }
        assert!(WeldType::parse("butt").is_none() && WeldSide::parse("Arrow").is_none());
    }

    #[test]
    fn weld_geometry_admits_a_bounded_path_and_symbols_are_exact_tokens() {
        let id =
            |n: u32| ElementId::parse(&format!("019a7bf0-0000-7000-8000-0000003{n:05}")).unwrap();
        let weld = CreateWeld {
            new_id: id(1),
            name: "W1".into(),
            main_id: id(2),
            secondary_id: id(3),
            weld: WeldSpec {
                weld_type: WeldType::Fillet,
                size_mm: 6.0,
                location: WeldLocation::Workshop,
                side: WeldSide::Arrow,
            },
            geometry: Some(WeldGeometry {
                path_mm: vec![[0.0; 3], [100.0, 0.0, 0.0]],
                symbol: WeldSymbol {
                    all_around: false,
                    contour: WeldContour::None,
                },
            }),
        };
        assert!(weld.validate_arguments().is_ok());
        let plain = CreateWeld {
            geometry: None,
            ..weld.clone()
        };
        // A06 requests keep their exact v1 fingerprint; geometry is a second domain.
        assert_ne!(
            weld.content_fingerprint().unwrap(),
            plain.content_fingerprint().unwrap()
        );
        let same = CreateWeld {
            geometry: Some(WeldGeometry {
                path_mm: vec![[0.0; 3], [100.0, 0.0, 0.0]],
                symbol: WeldSymbol {
                    all_around: false,
                    contour: WeldContour::None,
                },
            }),
            ..weld.clone()
        };
        assert_eq!(
            weld.content_fingerprint().unwrap(),
            same.content_fingerprint().unwrap()
        );
        let variants = [
            CreateWeld {
                geometry: Some(WeldGeometry {
                    path_mm: vec![[0.0; 3], [200.0, 0.0, 0.0]],
                    ..weld.geometry.clone().unwrap()
                }),
                ..weld.clone()
            },
            CreateWeld {
                geometry: Some(WeldGeometry {
                    symbol: WeldSymbol {
                        all_around: true,
                        contour: WeldContour::None,
                    },
                    ..weld.geometry.clone().unwrap()
                }),
                ..weld.clone()
            },
            CreateWeld {
                geometry: Some(WeldGeometry {
                    symbol: WeldSymbol {
                        all_around: false,
                        contour: WeldContour::Concave,
                    },
                    ..weld.geometry.clone().unwrap()
                }),
                ..weld.clone()
            },
        ];
        for v in &variants {
            assert_ne!(
                v.content_fingerprint().unwrap(),
                weld.content_fingerprint().unwrap()
            );
        }
        let code = |w: CreateWeld| w.validate_arguments().unwrap_err().code;
        let with_path = |path: Vec<[f64; 3]>| CreateWeld {
            geometry: Some(WeldGeometry {
                path_mm: path,
                symbol: WeldSymbol {
                    all_around: false,
                    contour: WeldContour::None,
                },
            }),
            ..weld.clone()
        };
        for (path, expected) in [
            (vec![], ErrorCode::InvalidArgument),
            (vec![[0.0; 3]], ErrorCode::InvalidArgument),
            (vec![[0.0; 3], [0.0, 0.0, 0.0]], ErrorCode::InvalidArgument),
            (
                vec![[0.0; 3], [0.0, 0.0005, 0.0]],
                ErrorCode::InvalidArgument,
            ),
            (
                vec![[0.0; 3], [f64::NAN, 0.0, 0.0]],
                ErrorCode::NonFiniteNumber,
            ),
            (
                vec![[0.0; 3], [f64::INFINITY, 0.0, 0.0]],
                ErrorCode::NonFiniteNumber,
            ),
            (
                vec![[2.0e10, 0.0, 0.0], [0.0; 3]],
                ErrorCode::InvalidArgument,
            ),
            (
                vec![[0.0; 3], [1.0e6, 0.0, 0.0], [2.0e6, 0.0, 0.0]],
                ErrorCode::LimitExceeded,
            ),
        ] {
            assert_eq!(code(with_path(path)), expected);
        }
        for token in ["none", "flush", "concave", "convex"] {
            assert_eq!(
                WeldContour::parse(token).map(WeldContour::as_token),
                Some(token)
            );
        }
        assert!(WeldContour::parse("round").is_none() && WeldContour::parse("None").is_none());
    }

    #[test]
    fn create_level_admits_a_label_and_a_bounded_finite_elevation() {
        let level = CreateLevel {
            new_id: id(1),
            name: "Level 1".into(),
            elevation_mm: 3000.0,
        };
        assert!(level.validate_arguments().is_ok());
        let code = |l: CreateLevel| l.validate_arguments().unwrap_err().code;
        for elevation in [-1_000_000_000.0, 0.0, 1_000_000_000.0] {
            let l = CreateLevel {
                elevation_mm: elevation,
                ..level.clone()
            };
            assert!(l.validate_arguments().is_ok(), "{elevation}");
        }
        for elevation in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let l = CreateLevel {
                elevation_mm: elevation,
                ..level.clone()
            };
            assert_eq!(code(l), ErrorCode::NonFiniteNumber);
        }
        for elevation in [1_000_000_000.5, -1e300] {
            let l = CreateLevel {
                elevation_mm: elevation,
                ..level.clone()
            };
            assert_eq!(code(l), ErrorCode::LimitExceeded);
        }
        for name in ["", "tab\there"] {
            let l = CreateLevel {
                name: name.into(),
                ..level.clone()
            };
            assert_eq!(code(l), ErrorCode::InvalidArgument);
        }
        let base = level.content_fingerprint().unwrap();
        for changed in [
            CreateLevel {
                new_id: id(2),
                ..level.clone()
            },
            CreateLevel {
                name: "Level 2".into(),
                ..level.clone()
            },
            CreateLevel {
                elevation_mm: 3000.5,
                ..level.clone()
            },
        ] {
            assert_ne!(changed.content_fingerprint().unwrap(), base);
        }
        assert_eq!(level.clone().content_fingerprint().unwrap(), base);
    }

    #[test]
    fn a_level_patch_counts_as_a_change_and_is_part_of_the_fingerprint() {
        let patch = |level| UpdateElement {
            target_id: id(5),
            patch: ElementPatch {
                level,
                ..ElementPatch::default()
            },
        };
        assert!(patch(Some(Some(id(9)))).validate_arguments().is_ok());
        assert!(patch(Some(None)).validate_arguments().is_ok());
        assert_eq!(
            patch(None).validate_arguments().unwrap_err().code,
            ErrorCode::InvalidArgument
        );
        let set = patch(Some(Some(id(9)))).content_fingerprint().unwrap();
        let other = patch(Some(Some(id(10)))).content_fingerprint().unwrap();
        let clear = patch(Some(None)).content_fingerprint().unwrap();
        assert!(set != other && set != clear && other != clear);
    }

    fn bolt() -> CreateBolt {
        CreateBolt {
            new_id: id(1),
            name: "B1".into(),
            main_id: id(2),
            secondary_id: id(3),
            bolt: BoltSpec {
                diameter_mm: 16.0,
                count: 4,
                pitch_mm: 80.0,
                origin_mm: [0.0, 200.0, 5.0],
                direction_mm: [1.0, 0.0, 0.0],
            },
        }
    }

    #[test]
    fn create_bolt_admits_two_distinct_parts_and_bounded_properties() {
        let group = bolt();
        assert!(group.validate_arguments().is_ok());
        let base = group.content_fingerprint().unwrap();
        // Every authored property and the part order are part of the content.
        let variants = [
            CreateBolt {
                secondary_id: id(2),
                main_id: id(3),
                ..group.clone()
            },
            CreateBolt {
                bolt: BoltSpec {
                    diameter_mm: 20.0,
                    ..group.bolt
                },
                ..group.clone()
            },
            CreateBolt {
                bolt: BoltSpec {
                    count: group.bolt.count + 1,
                    ..group.bolt
                },
                ..group.clone()
            },
            CreateBolt {
                bolt: BoltSpec {
                    pitch_mm: 90.0,
                    ..group.bolt
                },
                ..group.clone()
            },
            CreateBolt {
                bolt: BoltSpec {
                    origin_mm: [101.0, 200.0, 5.0],
                    ..group.bolt
                },
                ..group.clone()
            },
            CreateBolt {
                bolt: BoltSpec {
                    direction_mm: [0.0, 1.0, 0.0],
                    ..group.bolt
                },
                ..group.clone()
            },
        ];
        for v in &variants {
            assert_ne!(v.content_fingerprint().unwrap(), base);
        }
        let refused = |b: CreateBolt| b.validate_arguments().unwrap_err();
        assert_eq!(
            refused(CreateBolt {
                secondary_id: id(2),
                ..group.clone()
            })
            .code,
            ErrorCode::InvalidArgument
        );
        assert_eq!(
            refused(CreateBolt {
                new_id: id(2),
                ..group.clone()
            })
            .code,
            ErrorCode::InvalidArgument
        );
        for diameter in [0.0, -1.0, 1000.5] {
            assert_eq!(
                refused(CreateBolt {
                    bolt: BoltSpec {
                        diameter_mm: diameter,
                        ..group.bolt
                    },
                    ..group.clone()
                })
                .code,
                ErrorCode::InvalidArgument
            );
        }
        for (count, expected) in [
            (0, ErrorCode::InvalidArgument),
            (MAX_BOLT_GROUP_SIZE + 1, ErrorCode::LimitExceeded),
        ] {
            assert_eq!(
                refused(CreateBolt {
                    bolt: BoltSpec {
                        count,
                        ..group.bolt
                    },
                    ..group.clone()
                })
                .code,
                expected
            );
        }
        assert!(
            CreateBolt {
                bolt: BoltSpec {
                    count: MAX_BOLT_GROUP_SIZE,
                    ..group.bolt
                },
                ..group.clone()
            }
            .validate_arguments()
            .is_ok()
        );
        assert_eq!(
            refused(CreateBolt {
                bolt: BoltSpec {
                    pitch_mm: 0.0,
                    ..group.bolt
                },
                ..group.clone()
            })
            .code,
            ErrorCode::InvalidArgument
        );
        assert_eq!(
            refused(CreateBolt {
                bolt: BoltSpec {
                    pitch_mm: DIMENSION_CAP_MM + 1.0,
                    ..group.bolt
                },
                ..group.clone()
            })
            .code,
            ErrorCode::InvalidArgument
        );
        assert_eq!(
            refused(CreateBolt {
                bolt: BoltSpec {
                    origin_mm: [1_000_000_001.0, 0.0, 0.0],
                    ..group.bolt
                },
                ..group.clone()
            })
            .code,
            ErrorCode::LimitExceeded
        );
        for field in [
            BoltSpec {
                diameter_mm: f64::NAN,
                ..group.bolt
            },
            BoltSpec {
                origin_mm: [0.0, f64::INFINITY, 0.0],
                ..group.bolt
            },
        ] {
            let e = refused(CreateBolt {
                bolt: field,
                ..group.clone()
            });
            assert_eq!(e.code, ErrorCode::NonFiniteNumber, "{}", e.field);
        }
        let e = refused(CreateBolt {
            bolt: BoltSpec {
                direction_mm: [2.0, 0.0, 0.0],
                ..group.bolt
            },
            ..group.clone()
        });
        assert_eq!(e.code, ErrorCode::InvalidArgument);
        assert_eq!(e.field, "direction_mm");
        assert_eq!(
            refused(CreateBolt {
                name: String::new(),
                ..group.clone()
            })
            .code,
            ErrorCode::InvalidArgument
        );
        // Signed zero and an equivalent unit direction fingerprint the same.
        let mut zero = bolt();
        zero.bolt.origin_mm[0] = -0.0;
        assert_eq!(zero.content_fingerprint().unwrap(), base);
        assert_eq!(group.clone().content_fingerprint().unwrap(), base);
    }

    /// A washered bolt definition and the digest standing in for its pinned content.
    fn bolt_group_inputs() -> BoltGroupInputs {
        let fastener = FastenerSpec {
            designation: Some("M16".into()),
            standard: None,
            grade: None,
            display_name: None,
            kind: crate::steel::fastener::FastenerKind::Bolt,
            head_kind: crate::steel::fastener::HeadKind::Hex,
            nominal_diameter_mm: 16.0,
            head_across_flats_mm: 24.0,
            head_height_mm: 10.0,
            nut_across_flats_mm: Some(24.0),
            nut_height_mm: Some(14.8),
            washer: Some(crate::steel::fastener::WasherSpec {
                outer_diameter_mm: 30.0,
                inner_diameter_mm: 17.0,
                thickness_mm: 3.0,
            }),
            hole_clearance_mm: 2.0,
            lengths_mm: vec![45.0, 50.0, 60.0, 80.0],
            thread_protrusion_mm: 5.0,
            washer_under_head: true,
            washer_under_nut: true,
            hole_mode: crate::steel::fastener::HoleMode::Generate,
            mass_each_kg: None,
        };
        fastener.validate().unwrap();
        BoltGroupInputs {
            parts: vec![id(2), id(3)],
            fastener_digest: ContentDigest::from_bytes([7u8; 32]),
            fastener,
            point1_mm: [0.0, 0.0, 0.0],
            point2_mm: [400.0, 0.0, 0.0],
            plane_normal_mm: [0.0, 0.0, 1.0],
            rotation_rad: 0.0,
            on_plane_mm: 0.0,
            depth_mm: 0.0,
            offset_start_mm: 50.0,
            spacing: BoltSpacing::Spacings(vec![80.0, 90.0]),
            washer_under_head: None,
            washer_under_nut: None,
            length: BoltLength::Auto,
            hole_mode: None,
            hole_clearance_mm: None,
        }
    }

    #[test]
    fn bolt_group_inputs_admit_two_parts_spacings_and_generators() {
        let inputs = bolt_group_inputs();
        assert!(inputs.validate_arguments().is_ok());
        let group = CreateBoltGroup {
            new_id: id(1),
            name: "G1".into(),
            inputs: inputs.clone(),
        };
        assert!(group.validate_arguments().is_ok());
        let base = group.content_fingerprint().unwrap();
        assert!(
            UpdateBoltGroup {
                target_id: id(1),
                inputs: inputs.clone(),
            }
            .validate_arguments()
            .is_ok()
        );
        // Every input and the part order are part of the content; equal inputs are equal.
        let variant = |mutate: &dyn Fn(&mut BoltGroupInputs)| {
            let mut v = inputs.clone();
            mutate(&mut v);
            CreateBoltGroup {
                new_id: id(1),
                name: "G1".into(),
                inputs: v,
            }
            .content_fingerprint()
            .unwrap()
        };
        for different in [
            variant(&|v| v.parts.reverse()),
            variant(&|v| v.point2_mm = [500.0, 0.0, 0.0]),
            variant(&|v| v.rotation_rad = 1.5),
            variant(&|v| v.on_plane_mm = 10.0),
            variant(&|v| v.depth_mm = -4.0),
            variant(&|v| v.offset_start_mm = 60.0),
            variant(&|v| {
                v.spacing = BoltSpacing::Pitch {
                    count: 3,
                    pitch_mm: 80.0,
                }
            }),
            variant(&|v| {
                v.spacing = BoltSpacing::OffsetEnd {
                    count: 3,
                    offset_end_mm: 50.0,
                }
            }),
            variant(&|v| v.washer_under_head = Some(false)),
            variant(&|v| v.length = BoltLength::Fixed(50.0)),
            variant(&|v| v.hole_mode = Some(crate::steel::fastener::HoleMode::None)),
            variant(&|v| v.hole_clearance_mm = Some(3.0)),
            variant(&|v| {
                v.fastener_digest = ContentDigest::from_bytes([8u8; 32]);
            }),
        ] {
            assert_ne!(different, base);
        }
        // The hole defaults follow the definition: a bolt generates, a screw does not, and an
        // absent clearance takes the definition's.
        assert_eq!(
            inputs.resolved_hole_mode(),
            crate::steel::fastener::HoleMode::Generate
        );
        assert_eq!(inputs.resolved_hole_clearance_mm(), 2.0);
        let mut screw = inputs.clone();
        screw.fastener.kind = crate::steel::fastener::FastenerKind::Screw;
        screw.fastener.nut_across_flats_mm = None;
        screw.fastener.nut_height_mm = None;
        screw.fastener.washer_under_nut = false;
        screw.fastener.validate().unwrap();
        assert_eq!(
            screw.resolved_hole_mode(),
            crate::steel::fastener::HoleMode::None
        );
        screw.hole_mode = Some(crate::steel::fastener::HoleMode::Generate);
        assert_eq!(
            screw.resolved_hole_mode(),
            crate::steel::fastener::HoleMode::Generate
        );
        screw.hole_clearance_mm = Some(1.0);
        assert_eq!(screw.resolved_hole_clearance_mm(), 1.0);
        assert_eq!(
            CreateBoltGroup {
                new_id: id(1),
                name: "G1".into(),
                inputs: inputs.clone(),
            }
            .content_fingerprint()
            .unwrap(),
            base
        );
    }

    #[test]
    fn bolt_group_inputs_refusals_name_their_field() {
        let field = |mutate: &dyn Fn(&mut BoltGroupInputs)| {
            let mut v = bolt_group_inputs();
            mutate(&mut v);
            v.validate_arguments().unwrap_err().field
        };
        assert_eq!(
            field(&|v| v.parts = vec![id(2)]),
            "parts",
            "one part is no group"
        );
        assert_eq!(
            field(&|v| v.parts = vec![id(2), id(2)]),
            "parts",
            "parts are distinct"
        );
        assert_eq!(field(&|v| v.point2_mm = v.point1_mm), "point2_mm");
        assert_eq!(
            field(&|v| v.plane_normal_mm = [0.0, 0.0, 2.0]),
            "plane_normal_mm"
        );
        assert_eq!(field(&|v| v.rotation_rad = f64::NAN), "rotation_rad");
        assert_eq!(
            field(&|v| v.offset_start_mm = -1.0),
            "offset_start_mm",
            "the offset start is a distance"
        );
        assert_eq!(
            field(&|v| v.spacing = BoltSpacing::Spacings(vec![])),
            "spacings_mm"
        );
        assert_eq!(
            field(&|v| v.spacing = BoltSpacing::Spacings(vec![0.0])),
            "spacings_mm"
        );
        assert_eq!(
            field(&|v| v.spacing = BoltSpacing::Pitch {
                count: 0,
                pitch_mm: 80.0
            }),
            "count"
        );
        assert_eq!(
            field(&|v| v.spacing = BoltSpacing::OffsetEnd {
                count: 1,
                offset_end_mm: 50.0
            }),
            "count",
            "an offset end needs a row"
        );
        assert_eq!(
            field(&|v| v.length = BoltLength::Fixed(55.0)),
            "length_mm",
            "a fixed length is listed"
        );
        assert_eq!(
            field(&|v| v.hole_clearance_mm = Some(-1.0)),
            "hole_clearance_mm"
        );
        assert_eq!(
            field(&|v| v.hole_clearance_mm = Some(f64::NAN)),
            "hole_clearance_mm"
        );
        // A fitted hole (zero clearance) and a positive one are admitted.
        let mut v = bolt_group_inputs();
        v.length = BoltLength::Fixed(50.0);
        v.hole_clearance_mm = Some(0.0);
        assert!(v.validate_arguments().is_ok());
    }
}
