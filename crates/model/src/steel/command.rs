// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! Steel commands: the arguments that create and edit plates, members, holes, bends, welds and
//! bolts, and their model-level admission (F04 section 7). What is true of the arguments
//! regardless of geometry lives here; contour and thickness geometry is evaluated by the engine
//! with the kernel.

use crate::command::{CommandError, Placement, label_is_valid};
use crate::digest::{ContentDigest, Sha256};
use crate::error::ErrorCode;
use crate::id::{ElementId, OpId};
use crate::limits::{
    BOLT_DIAMETER_CAP_MM, CONTOUR_POINT_CAP, COORDINATE_CAP_MM, DIMENSION_CAP_MM,
    MAX_BOLT_GROUP_SIZE, MAX_MEMBER_PATH_POINTS, MAX_RELATIONSHIP_TARGETS, MAX_WELD_LENGTH_MM,
    MAX_WELD_PATH_POINTS, MIN_CONTOUR_POINTS, MIN_MEMBER_PATH_POINTS, MIN_WELD_PATH_POINTS,
    MIN_WELD_SEGMENT_MM, ORIENTATION_NORM_TOLERANCE, ROLL_CAP_RAD, WELD_SIZE_CAP_MM,
};
pub use crate::steel::fastener::FastenerSpec;
use beamcraft_geom::canonical_f64_bytes;
use beamcraft_geom::tolerance::ABSOLUTE_FLOOR_MM;

/// Which closed half-space of the cutting plane survives (F04 section 7 `CutByPlane`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeepSide {
    /// The side the plane normal points to (`normal . (p - point) >= 0`).
    Positive,
    /// The opposite side (`normal . (p - point) <= 0`).
    Negative,
}

impl KeepSide {
    /// The wire token.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Positive => "positive",
            Self::Negative => "negative",
        }
    }

    /// Parses the wire token; anything else is `None`.
    pub fn parse(token: &str) -> Option<Self> {
        match token {
            "positive" => Some(Self::Positive),
            "negative" => Some(Self::Negative),
            _ => None,
        }
    }
}

/// One owned `plane_cut` operation record of an element (F04 section 4, amendment A-2): the
/// plane parameters and the kept side, with the record identity the engine assigned it.
///
/// The plane is the exact set `normal . (p - point_mm) = 0` of the stored binary64 numbers; a
/// scaled normal describes the same plane. Whether the plane actually crosses the element (some
/// vertex strictly on each side, no face in the plane) is the kernel's decision at replay; a
/// tangent or missing plane refuses the command that authors or replays it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CutRecord {
    /// Full UUIDv7 of the record, unused in the project (`DUPLICATE_ID`).
    pub operation_id: OpId,
    /// The per-element ordinal the engine assigned in creation order; it names the cap face
    /// `cut:f.p<ordinal>` and is never reused or renumbered.
    pub ordinal: u16,
    /// A point of the cutting plane, element-local millimetres.
    pub point_mm: [f64; 3],
    /// A non-zero normal of the plane.
    pub normal: [f64; 3],
    /// The kept half-space.
    pub keep: KeepSide,
}

impl CutRecord {
    /// Admits what does not depend on geometry: finiteness, the coordinate cap for the point and
    /// every normal component, and a non-zero normal. A non-finite number is
    /// `NON_FINITE_NUMBER`; a coordinate above 1,000,000,000 mm is `LIMIT_EXCEEDED`; a zero
    /// normal is `INVALID_ARGUMENT`.
    pub fn validate(&self) -> Result<(), CommandError> {
        if self
            .point_mm
            .iter()
            .chain(self.normal.iter())
            .any(|v| !v.is_finite())
        {
            return Err(CommandError::new(
                ErrorCode::NonFiniteNumber,
                "cut",
                "value is not finite",
            )
            .with_rule("DV-ATTR-003"));
        }
        if self
            .point_mm
            .iter()
            .chain(self.normal.iter())
            .any(|v| v.abs() > COORDINATE_CAP_MM)
        {
            return Err(CommandError::new(
                ErrorCode::LimitExceeded,
                "cut",
                "coordinate exceeds the coordinate cap",
            )
            .with_rule("DV-LIM-001"));
        }
        if self.normal.iter().all(|v| *v == 0.0) {
            return Err(CommandError::new(
                ErrorCode::InvalidArgument,
                "cut",
                "the cutting plane needs a non-zero normal",
            ));
        }
        Ok(())
    }
}

/// F04 section 7 `CutByPlane`: adds one `plane_cut` record to a valid plate, beam or column and
/// updates its derived data in one transaction. `operation_id` must be unused in the project;
/// the engine assigns the next ordinal of the target.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CutByPlane {
    /// The element to cut.
    pub target_id: ElementId,
    /// Full UUIDv7 of the new record; unused in the project.
    pub operation_id: OpId,
    /// A point of the cutting plane, element-local millimetres.
    pub point_mm: [f64; 3],
    /// A non-zero normal of the plane.
    pub normal: [f64; 3],
    /// The kept half-space.
    pub keep: KeepSide,
}

impl CutByPlane {
    /// Admits what does not depend on geometry or model state: finiteness, the coordinate cap
    /// and a non-zero normal, as [`CutRecord::validate`].
    pub fn validate_arguments(&self) -> Result<(), CommandError> {
        CutRecord {
            operation_id: self.operation_id,
            ordinal: 0,
            point_mm: self.point_mm,
            normal: self.normal,
            keep: self.keep,
        }
        .validate()
    }

    /// SHA-256 of a versioned, length-prefixed binary encoding of the validated arguments, with
    /// signed zero normalized; the same deterministic digest form as
    /// [`CreatePlate::content_fingerprint`].
    pub fn content_fingerprint(&self) -> Result<ContentDigest, CommandError> {
        self.validate_arguments()?;
        let bad =
            || CommandError::new(ErrorCode::NonFiniteNumber, "cut", "value cannot be encoded");
        let mut bytes = Vec::with_capacity(128);
        bytes.extend_from_slice(b"BeamCraft/A03/CutByPlane/v1\0");
        bytes.extend_from_slice(self.target_id.as_bytes());
        bytes.extend_from_slice(self.operation_id.as_bytes());
        bytes.push(match self.keep {
            KeepSide::Positive => 0,
            KeepSide::Negative => 1,
        });
        for v in self.point_mm.into_iter().chain(self.normal) {
            bytes.extend_from_slice(&canonical_f64_bytes(v).map_err(|_| bad())?);
        }
        Sha256::digest(&bytes).map_err(|_| {
            CommandError::new(ErrorCode::LimitExceeded, "content", "content is too large")
                .with_rule("DV-LIM-001")
        })
    }
}

/// Where the thickness of a plate is laid off from its contour plane (A12.1, the F04 plate
/// amendment section 1). The contour lies in the element-local plane `z = 0`; the side names the
/// band of `z` that carries the material. The side is the axis of the element-local frame, not a
/// world direction: a rotated placement turns it with the plate. Volume, area and mass do not
/// depend on it; the mesh does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlateSide {
    /// Material between `z = 0` and `z = +t`. The default, and what every plate authored before
    /// the field existed reads as.
    Top,
    /// Material between `z = -t/2` and `z = +t/2`, half on each side of the contour plane.
    Middle,
    /// Material between `z = -t` and `z = 0`, laid off in `-Z`.
    Bottom,
}

impl PlateSide {
    /// The wire token.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Top => "top",
            Self::Middle => "middle",
            Self::Bottom => "bottom",
        }
    }

    /// Parses the wire token.
    pub fn parse(token: &str) -> Option<Self> {
        match token {
            "top" => Some(Self::Top),
            "middle" => Some(Self::Middle),
            "bottom" => Some(Self::Bottom),
            _ => None,
        }
    }

    /// The element-local `z` of the material's lower face for a plate of the given thickness:
    /// `0`, `-t/2` or `-t`. Halving a finite `f64` and negating it are exact, so the offset is
    /// the authored thickness unchanged.
    pub const fn offset_mm(self, thickness_mm: f64) -> f64 {
        match self {
            Self::Top => 0.0,
            Self::Middle => -thickness_mm / 2.0,
            Self::Bottom => -thickness_mm,
        }
    }
}

/// The treatment of one contour vertex (A12.2, the F04 plate amendment section 2). The authored
/// contour keeps the sharp corner point; the treatment is an authored parameter, so a rounded
/// corner stays editable by its radius and is never a stored polygon.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CornerTreatment {
    /// The corner is replaced by a straight cut. `d1_mm` is measured from the vertex along the
    /// edge towards the **previous** vertex, `d2_mm` along the edge towards the **next** vertex;
    /// equal values give the symmetric cut of a single authored distance.
    Chamfer {
        /// Cut distance along the previous edge, in millimetres.
        d1_mm: f64,
        /// Cut distance along the next edge, in millimetres.
        d2_mm: f64,
    },
    /// The corner is replaced by a circular arc of the given radius, tangent to both edges.
    Round {
        /// Arc radius in millimetres.
        radius_mm: f64,
    },
}

impl CornerTreatment {
    /// The wire token of the treatment kind.
    pub const fn kind(self) -> &'static str {
        match self {
            Self::Chamfer { .. } => "chamfer",
            Self::Round { .. } => "round",
        }
    }
}

/// One contour vertex's corner treatment (A12.2): the vertex is named by its position in the
/// authored contour (amendment section 3, decision O-1: no stable vertex ids).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlateCorner {
    /// Position of the vertex in the authored contour, `0` to point count - 1.
    pub vertex: usize,
    /// The treatment authored on it.
    pub treatment: CornerTreatment,
}

impl PlateCorner {
    /// Admits one corner: the parameters are finite (`NON_FINITE_NUMBER`), greater than zero
    /// (`INVALID_ARGUMENT`) and within the F04 coordinate cap (`LIMIT_EXCEEDED`).
    pub fn validate(&self) -> Result<(), CommandError> {
        let values = match self.treatment {
            CornerTreatment::Chamfer { d1_mm, d2_mm } => [d1_mm, d2_mm],
            CornerTreatment::Round { radius_mm } => [radius_mm, radius_mm],
        };
        if values.iter().any(|v| !v.is_finite()) {
            return Err(CommandError::new(
                ErrorCode::NonFiniteNumber,
                "corner",
                "corner treatment value is not finite",
            )
            .with_rule("DV-ATTR-003"));
        }
        if values.iter().any(|v| *v <= 0.0) {
            return Err(CommandError::new(
                ErrorCode::InvalidArgument,
                "corner",
                "corner treatment value is not positive",
            ));
        }
        if values.iter().any(|v| *v > COORDINATE_CAP_MM) {
            return Err(CommandError::new(
                ErrorCode::LimitExceeded,
                "corner",
                "corner treatment value exceeds the coordinate cap",
            )
            .with_rule("DV-LIM-001"));
        }
        Ok(())
    }
}

/// Replace the parameters of an existing cut record, preserving its identity and ordinal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UpdateCut {
    /// Target, record identity and replacement plane in element-local millimetres.
    pub cut: CutByPlane,
}
impl UpdateCut {
    /// Validate the replacement independently of the target's geometry.
    pub fn validate_arguments(&self) -> Result<(), CommandError> {
        self.cut.validate_arguments()
    }
    /// Versioned request identity, distinct from creating the same plane.
    pub fn content_fingerprint(&self) -> Result<ContentDigest, CommandError> {
        let mut bytes = b"BeamCraft/A03/UpdateCut/v1\0".to_vec();
        bytes.extend_from_slice(self.cut.content_fingerprint()?.as_bytes());
        Sha256::digest(&bytes).map_err(|_| {
            CommandError::new(
                ErrorCode::LimitExceeded,
                "content",
                "cut request exceeds its bound",
            )
        })
    }
}

/// Delete one owned cut record. The logical request identity is separate from the record UUID.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RemoveCut {
    /// Part whose operation list is edited.
    pub target_id: ElementId,
    /// Existing cut record; never interpreted as this request's operation identity.
    pub operation_id: OpId,
}
impl RemoveCut {
    /// Stable typed request digest, distinct from cut creation and replacement.
    pub fn content_fingerprint(&self) -> Result<ContentDigest, CommandError> {
        let mut bytes = b"BeamCraft/A03/RemoveCut/v1\0".to_vec();
        bytes.extend_from_slice(self.target_id.as_bytes());
        bytes.extend_from_slice(self.operation_id.as_bytes());
        Sha256::digest(&bytes).map_err(|_| {
            CommandError::new(
                ErrorCode::LimitExceeded,
                "content",
                "cut request exceeds its bound",
            )
        })
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
    /// Authored integer part class (P15); zero is the default.
    pub class: i32,
    /// Element-local contour, 3 to 1024 points, closure implicit.
    pub contour_points_mm: Vec<[f64; 3]>,
    /// Extrusion thickness in millimetres.
    pub thickness_mm: f64,
    /// Where the thickness is laid off from the contour plane (A12.1); absent means `top`.
    pub thickness_side: Option<PlateSide>,
    /// Corner treatments of authored contour vertices (A12.2); empty means every corner is
    /// sharp. Each vertex is named at most once; the geometry it must really turn is the
    /// engine's decision.
    pub corners: Vec<PlateCorner>,
    /// Rigid placement.
    pub placement: Placement,
    /// Optional pinned material definition.
    pub material_digest: Option<ContentDigest>,
    /// Through-hole polygons in the plate's local XY, each at least three points, clockwise as
    /// seen from +Z (the cutter convention of the kernel). Whether a hole is strictly inside the
    /// outline and clear of the others is the kernel's decision. Empty means a solid plate.
    pub holes_mm: Vec<Vec<[f64; 2]>>,
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

/// Where the section of a beam or column sits across the member width (P14), along the rolled
/// local `X_m` of the F06 section 4 frame. The names are frame-relative, not world directions: a
/// rotated placement turns them with the member. `left` puts the section's bounding box in the
/// `-X_m` half of the frame, `right` in the `+X_m` half and `centre` straddles the axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SectionPlanePosition {
    /// The bounding box straddles the axis, from `-w/2` to `+w/2`. The default, and what every
    /// member authored before the field existed reads as.
    #[default]
    Centre,
    /// The bounding box occupies `-X_m`: from `-w` to `0`.
    Left,
    /// The bounding box occupies `+X_m`: from `0` to `+w`.
    Right,
}

impl SectionPlanePosition {
    /// The wire token.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Centre => "centre",
            Self::Left => "left",
            Self::Right => "right",
        }
    }

    /// Parses the wire token.
    pub fn parse(token: &str) -> Option<Self> {
        match token {
            "centre" => Some(Self::Centre),
            "left" => Some(Self::Left),
            "right" => Some(Self::Right),
            _ => None,
        }
    }

    /// Where the bounding-box centre moves along `X_m` for a section of the given width: `0`,
    /// `-w/2` or `+w/2`. Halving a finite `f64` and negating it are exact.
    pub const fn half_shift_mm(self, width_mm: f64) -> f64 {
        match self {
            Self::Centre => 0.0,
            Self::Left => -width_mm / 2.0,
            Self::Right => width_mm / 2.0,
        }
    }
}

/// Where the section of a beam or column sits along the member height (P14), along the rolled
/// local `Y_m` of the F06 section 4 frame. The names are frame-relative, not world directions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SectionDepthPosition {
    /// The bounding box straddles the axis, from `-h/2` to `+h/2`. The default.
    #[default]
    Centre,
    /// The bounding box occupies `-Y_m`: from `-h` to `0`.
    Front,
    /// The bounding box occupies `+Y_m`: from `0` to `+h`.
    Back,
}

impl SectionDepthPosition {
    /// The wire token.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Centre => "centre",
            Self::Front => "front",
            Self::Back => "back",
        }
    }

    /// Parses the wire token.
    pub fn parse(token: &str) -> Option<Self> {
        match token {
            "centre" => Some(Self::Centre),
            "front" => Some(Self::Front),
            "back" => Some(Self::Back),
            _ => None,
        }
    }

    /// Where the bounding-box centre moves along `Y_m` for a section of the given height: `0`,
    /// `-h/2` or `+h/2`. Halving a finite `f64` and negating it are exact.
    pub const fn half_shift_mm(self, height_mm: f64) -> f64 {
        match self {
            Self::Centre => 0.0,
            Self::Front => -height_mm / 2.0,
            Self::Back => height_mm / 2.0,
        }
    }
}

/// The authored position of the section about the member axis (P14): where the section's
/// bounding-box centre sits in the rolled element-local section plane and where the solid
/// physically starts and ends along the axis. The axis points stay the modelling line; the
/// offsets move the material. All values are element-local and turn with the roll and the
/// placement. The rotation about the axis is the existing `roll_rad`; it composes with this
/// position the same way it composes with the frame. The default value is the centred, unshifted
/// position: the body every member authored before the field existed reads as, bit for bit.
///
/// The shift of the bounding-box centre is `plane.half_shift_mm(w) + plane_offset_mm` along the
/// rolled `X_m` and `depth.half_shift_mm(h) + depth_offset_mm` along the rolled `Y_m`. The solid
/// spans `p1 + start_offset_mm * u .. p2 + end_offset_mm * u` along the unit axis `u`, so its
/// length is the axis length moved by `end_offset_mm - start_offset_mm` and must stay positive.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SectionPlacement {
    /// Across-width position of the section.
    pub plane: SectionPlanePosition,
    /// Extra shift along `X_m` in millimetres, applied after [`SectionPlanePosition`].
    pub plane_offset_mm: f64,
    /// Along-height position of the section.
    pub depth: SectionDepthPosition,
    /// Extra shift along `Y_m` in millimetres, applied after [`SectionDepthPosition`].
    pub depth_offset_mm: f64,
    /// Offset of the solid's start face along `+u` in millimetres.
    pub start_offset_mm: f64,
    /// Offset of the solid's end face along `+u` in millimetres.
    pub end_offset_mm: f64,
}

impl SectionPlacement {
    /// True when nothing is displaced. A member with the default position evaluates exactly as
    /// it did before the field existed: the same mesh bytes, quantities and signatures.
    pub fn is_default(&self) -> bool {
        self.plane == SectionPlanePosition::Centre
            && self.depth == SectionDepthPosition::Centre
            && self.plane_offset_mm == 0.0
            && self.depth_offset_mm == 0.0
            && self.start_offset_mm == 0.0
            && self.end_offset_mm == 0.0
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
    /// Authored integer part class (P15); zero is the default.
    pub class: i32,
    /// The two element-local axis points `p1` and `p2`.
    pub axis_points_mm: [[f64; 3]; 2],
    /// Pinned profile definition.
    pub profile_digest: ContentDigest,
    /// Rigid placement.
    pub placement: Placement,
    /// Explicit roll about the member axis; `None` is zero.
    pub roll_rad: Option<f64>,
    /// The authored position of the section about the axis (P14); absence is the centred
    /// default and stores no attributes.
    pub section_placement: Option<SectionPlacement>,
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
            )
            .with_rule("DV-BASE-003"));
        }
        let roll = self.roll_rad.unwrap_or(0.0);
        if !roll.is_finite() || self.axis_points_mm.iter().flatten().any(|v| !v.is_finite()) {
            return Err(CommandError::new(
                ErrorCode::NonFiniteNumber,
                "axis_points_mm",
                "value is not finite",
            )
            .with_rule("DV-ATTR-003"));
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
            )
            .with_rule("DV-LIM-001"));
        }
        if roll.abs() > ROLL_CAP_RAD {
            return Err(CommandError::new(
                ErrorCode::LimitExceeded,
                "roll_rad",
                "roll exceeds 2 pi",
            )
            .with_rule("DV-LIM-001"));
        }
        if let Some(placement) = &self.section_placement {
            let offsets = [
                placement.plane_offset_mm,
                placement.depth_offset_mm,
                placement.start_offset_mm,
                placement.end_offset_mm,
            ];
            if offsets.iter().any(|v| !v.is_finite()) {
                return Err(CommandError::new(
                    ErrorCode::NonFiniteNumber,
                    "section_placement",
                    "value is not finite",
                )
                .with_rule("DV-ATTR-003"));
            }
            if offsets.iter().any(|v| v.abs() > DIMENSION_CAP_MM) {
                return Err(CommandError::new(
                    ErrorCode::LimitExceeded,
                    "section_placement",
                    "offset exceeds the dimension cap",
                )
                .with_rule("DV-LIM-001"));
            }
        }
        self.placement.validate()
    }

    /// SHA-256 of a versioned, length-prefixed binary encoding of the validated arguments, with
    /// signed zero normalized; like [`CreatePlate::content_fingerprint`] it is not the F04
    /// canonical form.
    pub fn content_fingerprint(&self) -> Result<ContentDigest, CommandError> {
        self.validate_arguments()?;
        let bad = || {
            CommandError::new(
                ErrorCode::NonFiniteNumber,
                "axis_points_mm",
                "value cannot be encoded",
            )
            .with_rule("DV-ATTR-003")
        };
        let mut bytes = Vec::with_capacity(256);
        bytes.extend_from_slice(b"BeamCraft/P03/CreateStraightMember/v5\0");
        bytes.extend_from_slice(self.new_id.as_bytes());
        bytes.push(match self.kind {
            MemberKind::Beam => 0,
            MemberKind::Column => 1,
        });
        let name = self.name.as_bytes();
        bytes.extend_from_slice(&(name.len() as u64).to_le_bytes());
        bytes.extend_from_slice(name);
        bytes.extend_from_slice(&self.class.to_le_bytes());
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
        match &self.section_placement {
            None => bytes.push(0),
            Some(p) => {
                bytes.push(1);
                bytes.push(match p.plane {
                    SectionPlanePosition::Centre => 0,
                    SectionPlanePosition::Left => 1,
                    SectionPlanePosition::Right => 2,
                });
                bytes.push(match p.depth {
                    SectionDepthPosition::Centre => 0,
                    SectionDepthPosition::Front => 1,
                    SectionDepthPosition::Back => 2,
                });
                for v in [
                    p.plane_offset_mm,
                    p.depth_offset_mm,
                    p.start_offset_mm,
                    p.end_offset_mm,
                ] {
                    bytes.extend_from_slice(&canonical_f64_bytes(v).map_err(|_| bad())?);
                }
            }
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
                .with_rule("DV-LIM-001")
        })
    }
}

/// P13 curved member. `member.path_points_mm` is exactly start, on-arc, end.
/// The three points retain the arc's plane and direction; they are never stored as facets.
#[derive(Debug, Clone, PartialEq)]
pub struct CreateCurvedBeam {
    /// Shared identity, profile, material and pose, with three authored arc points.
    pub member: CreatePolybeam,
}
impl CreateCurvedBeam {
    /// Admit bounded, finite inputs. Circle existence is an engineering check.
    pub fn validate_arguments(&self) -> Result<(), CommandError> {
        self.member.validate_arguments()?;
        if self.member.path_points_mm.len() != 3 {
            return Err(CommandError::new(
                ErrorCode::InvalidArgument,
                "arc_points_mm",
                "an arc requires exactly three points",
            ));
        }
        Ok(())
    }
    /// Request identity in a separate command domain from a three-point polybeam.
    pub fn content_fingerprint(&self) -> Result<ContentDigest, CommandError> {
        self.validate_arguments()?;
        let mut bytes = b"BeamCraft/P13/CreateCurvedBeam/v1\0".to_vec();
        bytes.extend_from_slice(self.member.content_fingerprint()?.as_bytes());
        Sha256::digest(&bytes).map_err(|_| {
            CommandError::new(
                ErrorCode::LimitExceeded,
                "command",
                "request encoding exceeds capacity",
            )
        })
    }
}

/// P12 `CreatePolybeam`: a member along a polyline path with one section along the whole path.
/// The section is the pinned profile (required), the optional material is a pinned definition,
/// and the path is at least three element-local points whose interior vertices the kernel
/// mitres; a two-point path is a straight member, not a polybeam. Whether the digests resolve
/// (`NOT_FOUND`), the chords are long enough and the section is supported are the engine's
/// decisions, not argument checks.
#[derive(Debug, Clone, PartialEq)]
pub struct CreatePolybeam {
    /// The new element's full identity.
    pub new_id: ElementId,
    /// Beam or column.
    pub kind: MemberKind,
    /// Label, 1 to 256 UTF-8 bytes without control characters.
    pub name: String,
    /// Authored integer part class (P15); zero is the default.
    pub class: i32,
    /// The ordered element-local path points, `MIN_MEMBER_PATH_POINTS` to
    /// `MAX_MEMBER_PATH_POINTS` of them.
    pub path_points_mm: Vec<[f64; 3]>,
    /// Pinned profile definition, one section along the whole path.
    pub profile_digest: ContentDigest,
    /// Rigid placement.
    pub placement: Placement,
    /// Optional pinned material definition.
    pub material_digest: Option<ContentDigest>,
}

impl CreatePolybeam {
    /// Admits what does not depend on geometry: label, point count, finiteness, the coordinate
    /// cap and the placement. A non-finite number is `NON_FINITE_NUMBER`; fewer than
    /// `MIN_MEMBER_PATH_POINTS` points is `INVALID_ARGUMENT` (a two-point path is a straight
    /// member), more than `MAX_MEMBER_PATH_POINTS` points or a coordinate above the coordinate
    /// cap is `LIMIT_EXCEEDED`.
    pub fn validate_arguments(&self) -> Result<(), CommandError> {
        if !label_is_valid(&self.name) {
            return Err(CommandError::new(
                ErrorCode::InvalidArgument,
                "name",
                "label must be 1 to 256 bytes without control characters",
            )
            .with_rule("DV-BASE-003"));
        }
        if self.path_points_mm.len() < MIN_MEMBER_PATH_POINTS {
            return Err(CommandError::new(
                ErrorCode::InvalidArgument,
                "path_points_mm",
                "a polybeam path needs at least three points",
            ));
        }
        if self.path_points_mm.len() > MAX_MEMBER_PATH_POINTS {
            return Err(CommandError::new(
                ErrorCode::LimitExceeded,
                "path_points_mm",
                "path must have 2 to 1024 points",
            )
            .with_rule("DV-LIM-001"));
        }
        if self.path_points_mm.iter().flatten().any(|v| !v.is_finite()) {
            return Err(CommandError::new(
                ErrorCode::NonFiniteNumber,
                "path_points_mm",
                "value is not finite",
            )
            .with_rule("DV-ATTR-003"));
        }
        if self
            .path_points_mm
            .iter()
            .flatten()
            .any(|v| v.abs() > COORDINATE_CAP_MM)
        {
            return Err(CommandError::new(
                ErrorCode::LimitExceeded,
                "path_points_mm",
                "coordinate exceeds the coordinate cap",
            )
            .with_rule("DV-LIM-001"));
        }
        self.placement.validate()
    }

    /// SHA-256 of a versioned, length-prefixed binary encoding of the validated arguments, with
    /// signed zero normalized; like [`CreateStraightMember::content_fingerprint`] it is not the
    /// F04 canonical form.
    pub fn content_fingerprint(&self) -> Result<ContentDigest, CommandError> {
        self.validate_arguments()?;
        let bad = || {
            CommandError::new(
                ErrorCode::NonFiniteNumber,
                "path_points_mm",
                "value cannot be encoded",
            )
            .with_rule("DV-ATTR-003")
        };
        let mut bytes = Vec::with_capacity(256);
        bytes.extend_from_slice(b"BeamCraft/P12/CreatePolybeam/v1\0");
        bytes.extend_from_slice(self.new_id.as_bytes());
        bytes.push(match self.kind {
            MemberKind::Beam => 0,
            MemberKind::Column => 1,
        });
        let name = self.name.as_bytes();
        bytes.extend_from_slice(&(name.len() as u64).to_le_bytes());
        bytes.extend_from_slice(name);
        bytes.extend_from_slice(&self.class.to_le_bytes());
        bytes.extend_from_slice(self.profile_digest.as_bytes());
        match &self.material_digest {
            Some(digest) => {
                bytes.push(1);
                bytes.extend_from_slice(digest.as_bytes());
            }
            None => bytes.push(0),
        }
        bytes.extend_from_slice(&(self.path_points_mm.len() as u64).to_le_bytes());
        self.path_points_mm
            .iter()
            .flatten()
            .copied()
            .chain(self.placement.translation_mm)
            .chain(self.placement.orientation_xyzw)
            .try_for_each(|v| {
                bytes.extend_from_slice(&canonical_f64_bytes(v).map_err(|_| bad())?);
                Ok::<(), CommandError>(())
            })?;
        Sha256::digest(&bytes).map_err(|_| {
            CommandError::new(ErrorCode::LimitExceeded, "content", "content is too large")
                .with_rule("DV-LIM-001")
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
            )
            .with_rule("DV-BASE-003"));
        }
        let count = self.contour_points_mm.len();
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
            )
            .with_rule("DV-ATTR-003"));
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
            )
            .with_rule("DV-LIM-001"));
        }
        if self.thickness_mm > DIMENSION_CAP_MM {
            return Err(CommandError::new(
                ErrorCode::LimitExceeded,
                "thickness_mm",
                "thickness exceeds the dimension cap",
            )
            .with_rule("DV-LIM-001"));
        }
        self.placement.validate()?;
        self.validate_holes()?;
        self.validate_corners()?;
        Ok(())
    }

    /// Admits the corner treatments (A12.2): at most one per contour vertex, each naming a real
    /// vertex of this contour. The values are bounded by [`PlateCorner::validate`]; whether the
    /// treated corner really turns is the engine's decision.
    fn validate_corners(&self) -> Result<(), CommandError> {
        let count = self.contour_points_mm.len();
        if self.corners.len() > count {
            return Err(CommandError::new(
                ErrorCode::LimitExceeded,
                "corner",
                "more corner treatments than contour vertices",
            )
            .with_rule("DV-LIM-001"));
        }
        let mut seen = Vec::new();
        for corner in &self.corners {
            if corner.vertex >= count {
                return Err(CommandError::new(
                    ErrorCode::InvalidArgument,
                    "corner",
                    "corner names no vertex of the contour",
                ));
            }
            if seen.contains(&corner.vertex) {
                return Err(CommandError::new(
                    ErrorCode::InvalidArgument,
                    "corner",
                    "a contour vertex carries more than one corner treatment",
                ));
            }
            seen.push(corner.vertex);
            corner.validate()?;
        }
        Ok(())
    }

    fn validate_holes(&self) -> Result<(), CommandError> {
        if self.holes_mm.len() > MAX_HOLES {
            return Err(CommandError::new(
                ErrorCode::LimitExceeded,
                "holes_mm",
                "more than 64 holes",
            )
            .with_rule("DV-LIM-001"));
        }
        if self.holes_mm.iter().any(|h| h.len() > HOLE_POINT_CAP) {
            return Err(CommandError::new(
                ErrorCode::LimitExceeded,
                "holes_mm",
                "a hole has more than 1024 points",
            )
            .with_rule("DV-LIM-001"));
        }
        if self.holes_mm.iter().any(|h| h.len() < MIN_CONTOUR_POINTS) {
            return Err(CommandError::new(
                ErrorCode::InvalidArgument,
                "holes_mm",
                "a hole needs at least 3 points",
            ));
        }
        let coordinates = || self.holes_mm.iter().flatten().flatten();
        if coordinates().any(|v| !v.is_finite()) {
            return Err(CommandError::new(
                ErrorCode::NonFiniteNumber,
                "holes_mm",
                "value is not finite",
            )
            .with_rule("DV-ATTR-003"));
        }
        if coordinates().any(|v| v.abs() > COORDINATE_CAP_MM) {
            return Err(CommandError::new(
                ErrorCode::LimitExceeded,
                "holes_mm",
                "coordinate exceeds the coordinate cap",
            )
            .with_rule("DV-LIM-001"));
        }
        Ok(())
    }

    /// SHA-256 of a versioned, length-prefixed binary encoding of the validated arguments, with
    /// signed zero normalized. Two commands with the same fingerprint have identical content;
    /// it is *not* the F04 canonical form and must not be sent over a wire.
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
            .with_rule("DV-ATTR-003")
        };
        let mut bytes = Vec::with_capacity(64 + self.contour_points_mm.len() * 24);
        bytes.extend_from_slice(b"BeamCraft/P04/CreatePlate/v8\0");
        bytes.extend_from_slice(self.new_id.as_bytes());
        let name = self.name.as_bytes();
        bytes.extend_from_slice(&(name.len() as u64).to_le_bytes());
        bytes.extend_from_slice(name);
        bytes.extend_from_slice(&self.class.to_le_bytes());
        bytes.push(match self.thickness_side {
            None => 0,
            Some(PlateSide::Top) => 1,
            Some(PlateSide::Middle) => 2,
            Some(PlateSide::Bottom) => 3,
        });
        bytes.extend_from_slice(&(self.corners.len() as u64).to_le_bytes());
        for corner in &self.corners {
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
                bytes.extend_from_slice(&canonical_f64_bytes(v).map_err(|_| bad())?);
            }
        }
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
        bytes.extend_from_slice(&(self.holes_mm.len() as u64).to_le_bytes());
        for hole in &self.holes_mm {
            bytes.extend_from_slice(&(hole.len() as u64).to_le_bytes());
            for v in hole.iter().flatten() {
                bytes.extend_from_slice(&canonical_f64_bytes(*v).map_err(|_| bad())?);
            }
        }
        Sha256::digest(&bytes).map_err(|_| {
            CommandError::new(ErrorCode::LimitExceeded, "content", "content is too large")
                .with_rule("DV-LIM-001")
        })
    }
}

/// The A11 `CreateLoftedPlate`: a plate whose body is the ruled loft between two face
/// contours that need not be coplanar. The sheet thickness is the distance between the
/// contours themselves (the G13 ruled primitive takes no separate thickness input), so the
/// command has no thickness, holes, cuts or corner treatments; those operate on extruded
/// plates and are refused on a lofted one. The two contours must have the same vertex count:
/// the rulings join `start[i]` to `end[i]` by index, and a renumbered end contour twists the
/// walls through the body, which the kernel's certification refuses instead of repairing.
#[derive(Debug, Clone, PartialEq)]
pub struct CreateLoftedPlate {
    /// The new element's full identity; creation also checks that it is unused.
    pub new_id: ElementId,
    /// Label, 1 to 256 UTF-8 bytes without control characters. Not an identity.
    pub name: String,
    /// Authored integer part class (P15); zero is the default.
    pub class: i32,
    /// Element-local start face contour, 3 to 1024 points, closure implicit.
    pub start_points_mm: Vec<[f64; 3]>,
    /// Element-local end face contour, the same vertex count as the start contour.
    pub end_points_mm: Vec<[f64; 3]>,
    /// Rigid placement.
    pub placement: Placement,
    /// Optional pinned material definition.
    pub material_digest: Option<ContentDigest>,
}

impl CreateLoftedPlate {
    /// Admits what does not depend on geometry: label, contour counts, finiteness and the
    /// coordinate cap, then the placement. Floor, planarity, winding and the certified loft
    /// are the engine's.
    pub fn validate_arguments(&self) -> Result<(), CommandError> {
        if !label_is_valid(&self.name) {
            return Err(CommandError::new(
                ErrorCode::InvalidArgument,
                "name",
                "label must be 1 to 256 bytes without control characters",
            )
            .with_rule("DV-BASE-003"));
        }
        for (field, points) in [
            ("start_points_mm", &self.start_points_mm),
            ("end_points_mm", &self.end_points_mm),
        ] {
            if points.len() > CONTOUR_POINT_CAP {
                return Err(CommandError::new(
                    ErrorCode::LimitExceeded,
                    field,
                    "contour has more than 1024 points",
                )
                .with_rule("DV-LIM-001"));
            }
            if points.len() < MIN_CONTOUR_POINTS {
                return Err(CommandError::new(
                    ErrorCode::InvalidArgument,
                    field,
                    "contour needs at least 3 points",
                ));
            }
            if points.iter().flatten().any(|v| !v.is_finite()) {
                return Err(CommandError::new(
                    ErrorCode::NonFiniteNumber,
                    field,
                    "value is not finite",
                )
                .with_rule("DV-ATTR-003"));
            }
            if points.iter().flatten().any(|v| v.abs() > COORDINATE_CAP_MM) {
                return Err(CommandError::new(
                    ErrorCode::LimitExceeded,
                    field,
                    "coordinate exceeds the coordinate cap",
                )
                .with_rule("DV-LIM-001"));
            }
        }
        if self.start_points_mm.len() != self.end_points_mm.len() {
            return Err(CommandError::new(
                ErrorCode::InvalidArgument,
                "end_points_mm",
                "the two contours must have the same vertex count",
            ));
        }
        self.placement.validate()
    }

    /// SHA-256 of a versioned, length-prefixed binary encoding of the validated arguments,
    /// with signed zero normalized, in the manner of [`CreatePlate::content_fingerprint`].
    pub fn content_fingerprint(&self) -> Result<ContentDigest, CommandError> {
        self.validate_arguments()?;
        let bad = || {
            CommandError::new(
                ErrorCode::NonFiniteNumber,
                "start_points_mm",
                "value cannot be encoded",
            )
            .with_rule("DV-ATTR-003")
        };
        let mut bytes =
            Vec::with_capacity(64 + (self.start_points_mm.len() + self.end_points_mm.len()) * 24);
        bytes.extend_from_slice(b"BeamCraft/A11/CreateLoftedPlate/v1\0");
        bytes.extend_from_slice(self.new_id.as_bytes());
        let name = self.name.as_bytes();
        bytes.extend_from_slice(&(name.len() as u64).to_le_bytes());
        bytes.extend_from_slice(name);
        bytes.extend_from_slice(&self.class.to_le_bytes());
        for points in [&self.start_points_mm, &self.end_points_mm] {
            bytes.extend_from_slice(&(points.len() as u64).to_le_bytes());
            for v in points.iter().flatten().copied() {
                bytes.extend_from_slice(&canonical_f64_bytes(v).map_err(|_| bad())?);
            }
        }
        for v in self
            .placement
            .translation_mm
            .iter()
            .copied()
            .chain(self.placement.orientation_xyzw)
        {
            bytes.extend_from_slice(&canonical_f64_bytes(v).map_err(|_| bad())?);
        }
        match &self.material_digest {
            Some(digest) => {
                bytes.push(1);
                bytes.extend_from_slice(digest.as_bytes());
            }
            None => bytes.push(0),
        }
        Sha256::digest(&bytes).map_err(|_| {
            CommandError::new(ErrorCode::LimitExceeded, "content", "content is too large")
                .with_rule("DV-LIM-001")
        })
    }
}

/// Most through holes of one plate.
pub const MAX_HOLES: usize = 64;
/// Most points of one hole polygon.
pub const HOLE_POINT_CAP: usize = 1024;

/// The hole of F04 section 7 `AddHole`: a clockwise polygon as submitted, or a generator whose
/// parameters name the polygon the command authors. The stored polygon is always authoritative
/// (F04 section 4); a generator is resolved at the input edge by [`HoleSpec::polygon`].
#[derive(Debug, Clone, PartialEq)]
pub enum HoleSpec {
    /// The clockwise polygon itself, 3 to 1024 points in plate-local XY, closure implicit.
    /// Winding, self-intersection, placement inside the outline and clearance from the other
    /// holes are the kernel's decision, with exact predicates.
    Polygon(Vec<[f64; 2]>),
    /// A round hole: the regular inscribed `N`-gon of F06 section 2 (A-2), with `N` decided
    /// from the radius against the fixed threshold table of [`HoleSpec::polygon`].
    Round {
        /// Centre in plate-local XY, each coordinate within the coordinate cap.
        centre_mm: [f64; 2],
        /// Diameter in millimetres, finite and greater than zero.
        diameter_mm: f64,
    },
    /// An axis-aligned rectangular hole from one corner, four clockwise points.
    Rectangle {
        /// The first corner in plate-local XY, each coordinate within the coordinate cap.
        corner_mm: [f64; 2],
        /// Extent along +X, finite and greater than zero.
        width_mm: f64,
        /// Extent along +Y, finite and greater than zero.
        height_mm: f64,
    },
    /// A round hole in a member (A04.2, the F06 member-hole amendment): a straight prism
    /// cutter perpendicular to the member axis, stored in the member frame and replayed with
    /// the member's records. A polygon generator is not admitted for a member in this slice.
    Member(MemberHole),
}

/// How far the member-hole cutter reaches along its axis (F06 member-hole amendment section 1):
/// past the whole body, or an explicit range that must end in free space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HoleExtent {
    /// The cutter reaches past the body on both sides.
    Through,
    /// A range measured along `axis_direction` from `axis_point_mm`; `from_mm < to_mm`. A range
    /// that ends inside material would be a blind hole, which is `UNSUPPORTED_GEOMETRY`.
    Range {
        /// Range start in millimetres along the axis direction.
        from_mm: f64,
        /// Range end in millimetres along the axis direction, greater than `from_mm`.
        to_mm: f64,
    },
}

impl HoleExtent {
    /// Admits finiteness, the coordinate cap and `from < to`.
    pub fn validate(&self) -> Result<(), CommandError> {
        match self {
            Self::Through => Ok(()),
            Self::Range { from_mm, to_mm } => {
                hole_values_finite([*from_mm, *to_mm].into_iter())?;
                hole_values_in_cap([*from_mm, *to_mm].into_iter())?;
                if from_mm < to_mm {
                    Ok(())
                } else {
                    Err(CommandError::new(
                        ErrorCode::InvalidArgument,
                        "hole",
                        "a hole range needs from below to",
                    ))
                }
            }
        }
    }

    /// The wire token.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Through => "through",
            Self::Range { .. } => "range",
        }
    }

    /// Parses the wire token with its range values; anything else is `None`.
    pub fn parse(token: &str, from_mm: f64, to_mm: f64) -> Option<Self> {
        match token {
            "through" => Some(Self::Through),
            "range" => Some(Self::Range { from_mm, to_mm }),
            _ => None,
        }
    }
}

/// The arguments of one member hole (F06 member-hole amendment section 1). The record is stored
/// in the member frame, so it follows the member when it is moved or rotated: the origin is the
/// first axis point, `u` along `X_r`, `v` along `Y_r`, `w` along the member axis `Z_m`,
/// including the roll.
#[derive(Debug, Clone, PartialEq)]
pub struct MemberHole {
    /// A point `(u, v, w)` on the hole axis in the member frame, finite, within the coordinate
    /// cap.
    pub axis_point_mm: [f64; 3],
    /// The unit direction of the hole axis in the member frame. The first slice admits exactly
    /// one of `(+-1, 0, 0)` or `(0, +-1, 0)`, the section directions; any other direction,
    /// including one with a component along the member axis, is `UNSUPPORTED_GEOMETRY` until
    /// G14.2.
    pub axis_direction: [f64; 3],
    /// How far the cutter reaches.
    pub extent: HoleExtent,
    /// The round generator's diameter in millimetres, finite and greater than zero. The stored
    /// polygon is authoritative (F06 section 1): the regular inscribed `N`-gon of
    /// [`HoleSpec::polygon`], with vertex 0 on `+Z_m` from the axis point.
    pub diameter_mm: f64,
}

impl MemberHole {
    /// Admits finiteness, the caps, a positive diameter, a section direction and the extent.
    pub fn validate(&self) -> Result<(), CommandError> {
        hole_values_finite(self.axis_point_mm.iter().copied())?;
        hole_values_in_cap(self.axis_point_mm.iter().copied())?;
        hole_values_finite([self.diameter_mm].into_iter())?;
        if self.diameter_mm <= 0.0 {
            return Err(CommandError::new(
                ErrorCode::InvalidArgument,
                "hole",
                "a hole diameter must be positive",
            ));
        }
        let is_section_direction = matches!(
            self.axis_direction,
            [1.0, 0.0, 0.0] | [-1.0, 0.0, 0.0] | [0.0, 1.0, 0.0] | [0.0, -1.0, 0.0]
        );
        if !is_section_direction {
            return Err(CommandError::new(
                ErrorCode::UnsupportedGeometry,
                "hole.axis_direction",
                "only a section direction (+-X_r, +-Y_r) is supported until G14.2",
            ));
        }
        self.extent.validate()
    }
}

/// The largest radius each permitted segment count serves (F06 section 2, A-2): the row at
/// index `k` is the largest radius whose chord deviation `r * (1 - cos(pi / N))` stays within
/// 0.05 mm for `N = 16 + 4k`, so the decision compares the radius with these fixed numbers and
/// no platform `cos` result decides it. The values are `0.05 / (1 - cos(pi / N))`.
const ROUND_HOLE_RADIUS_LIMITS: [f64; 61] = [
    2.602_171_722_995_434,     // N=16
    4.061_190_969_939_727_5,   // N=20
    5.844_440_655_460_139,     // N=24
    7.951_919_378_839_592,     // N=28
    1.038_362_655_579_763_1e1, // N=32
    1.313_956_190_743_764_2e1, // N=36
    1.621_972_528_694_607e1,   // N=40
    1.962_411_661_105_958e1,   // N=44
    2.335_273_582_969_88e1,    // N=48
    2.740_558_291_127_616_6e1, // N=52
    3.178_265_783_506_647_3e1, // N=56
    3.648_396_058_701_346e1,   // N=60
    4.150_949_115_731_939_5e1, // N=64
    4.685_924_953_897_042e1,   // N=68
    5.253_323_572_685_768e1,   // N=72
    5.853_144_971_716_463_4e1, // N=76
    6.485_389_150_701_836e1,   // N=80
    7.150_056_109_420_748e1,   // N=84
    7.847_145_847_701_5e1,     // N=88
    8.576_658_365_409_166e1,   // N=92
    9.338_593_662_435_471e1,   // N=96
    1.013_295_173_869_295_7e2, // N=100
    1.095_973_259_411_158_3e2, // N=104
    1.181_893_622_863_406_5e2, // N=108
    1.271_056_264_221_183_9e2, // N=112
    1.363_461_183_480_881e2,   // N=116
    1.459_108_380_638_777_5e2, // N=120
    1.557_997_855_692_493_7e2, // N=124
    1.660_129_608_639_590_2e2, // N=128
    1.765_503_639_477_979e2,   // N=132
    1.874_119_948_205_258e2,   // N=136
    1.985_978_534_820_98e2,    // N=140
    2.101_079_399_324_427_5e2, // N=144
    2.219_422_541_712_727_7e2, // N=148
    2.341_007_961_985_759_2e2, // N=152
    2.465_835_660_143_244_5e2, // N=156
    2.593_905_636_183_075_7e2, // N=160
    2.725_217_890_107_020_4e2, // N=164
    2.859_772_421_911_766_3e2, // N=168
    2.997_569_231_598_656e2,   // N=172
    3.138_608_319_165_718e2,   // N=176
    3.282_889_684_614_987_7e2, // N=180
    3.430_413_327_942_437e2,   // N=184
    3.581_179_249_152_984e2,   // N=188
    3.735_187_448_240_587e2,   // N=192
    3.892_437_925_207_74e2,    // N=196
    4.052_930_680_054_579_4e2, // N=200
    4.216_665_712_780_745e2,   // N=204
    4.383_643_023_387_047e2,   // N=208
    4.553_862_611_870_239e2,   // N=212
    4.727_324_478_229_255_6e2, // N=216
    4.904_028_622_470_072_5e2, // N=220
    5.083_975_044_590_468_4e2, // N=224
    5.267_163_744_590_734e2,   // N=228
    5.453_594_722_465_092e2,   // N=232
    5.643_267_978_213_062e2,   // N=236
    5.836_183_511_843_763e2,   // N=240
    6.032_341_323_355_112e2,   // N=244
    6.231_741_412_739_907e2,   // N=248
    6.434_383_780_007_807e2,   // N=252
    6.640_268_425_141_733e2,   // N=256
];

/// The first non-finite hole coordinate refuses the command (`NON_FINITE_NUMBER`).
fn hole_values_finite(mut values: impl Iterator<Item = f64>) -> Result<(), CommandError> {
    values.all(|v| v.is_finite()).then_some(()).ok_or_else(|| {
        CommandError::new(ErrorCode::NonFiniteNumber, "hole", "value is not finite")
            .with_rule("DV-ATTR-003")
    })
}

/// The first hole coordinate above the coordinate cap refuses the command (`LIMIT_EXCEEDED`).
fn hole_values_in_cap(mut values: impl Iterator<Item = f64>) -> Result<(), CommandError> {
    values
        .all(|v| v.abs() <= COORDINATE_CAP_MM)
        .then_some(())
        .ok_or_else(|| {
            CommandError::new(
                ErrorCode::LimitExceeded,
                "hole",
                "coordinate exceeds the coordinate cap",
            )
            .with_rule("DV-LIM-001")
        })
}

impl HoleSpec {
    /// Admits what does not depend on geometry: finiteness, positivity of generator sizes and
    /// the polygon caps. A non-finite number is `NON_FINITE_NUMBER`; a cap or a non-positive
    /// size is `LIMIT_EXCEEDED` or `INVALID_ARGUMENT` as for `CreatePlate.holes_mm`.
    pub fn validate(&self) -> Result<(), CommandError> {
        match self {
            Self::Polygon(points) => {
                if points.len() > HOLE_POINT_CAP {
                    return Err(CommandError::new(
                        ErrorCode::LimitExceeded,
                        "hole",
                        "a hole has more than 1024 points",
                    )
                    .with_rule("DV-LIM-001"));
                }
                if points.len() < MIN_CONTOUR_POINTS {
                    return Err(CommandError::new(
                        ErrorCode::InvalidArgument,
                        "hole",
                        "a hole needs at least 3 points",
                    ));
                }
                hole_values_finite(points.iter().flatten().copied())?;
                hole_values_in_cap(points.iter().flatten().copied())
            }
            Self::Round {
                centre_mm,
                diameter_mm,
            } => {
                hole_values_finite(centre_mm.iter().copied().chain([*diameter_mm]))?;
                hole_values_in_cap(centre_mm.iter().copied())?;
                if *diameter_mm <= 0.0 {
                    return Err(CommandError::new(
                        ErrorCode::InvalidArgument,
                        "hole",
                        "a hole diameter must be positive",
                    ));
                }
                Ok(())
            }
            Self::Rectangle {
                corner_mm,
                width_mm,
                height_mm,
            } => {
                hole_values_finite(corner_mm.iter().copied().chain([*width_mm, *height_mm]))?;
                hole_values_in_cap(corner_mm.iter().copied())?;
                if *width_mm <= 0.0 || *height_mm <= 0.0 {
                    return Err(CommandError::new(
                        ErrorCode::InvalidArgument,
                        "hole",
                        "a hole width and height must be positive",
                    ));
                }
                Ok(())
            }
            Self::Member(member) => member.validate(),
        }
    }

    /// The clockwise polygon this spec authors, at the input edge (the vertices use the
    /// platform `sin` and `cos` and are the doubles they round to). A round hole of radius
    /// `r` is the regular inscribed `N`-gon of F06 section 2: the smallest multiple of four,
    /// at least 16 and at most 256, whose chord deviation stays within 0.05 mm. A radius
    /// above the last threshold needs `N > 256` and is `UNSUPPORTED_GEOMETRY` (field
    /// `hole.radius`), because no `Approximated` status exists in 0.1.x.
    pub fn polygon(&self) -> Result<Vec<[f64; 2]>, CommandError> {
        self.validate()?;
        match self {
            Self::Polygon(points) => Ok(points.clone()),
            Self::Round {
                centre_mm,
                diameter_mm,
            } => {
                let radius = diameter_mm / 2.0;
                let index = ROUND_HOLE_RADIUS_LIMITS
                    .iter()
                    .position(|limit| radius <= *limit)
                    .ok_or_else(|| {
                        CommandError::new(
                            ErrorCode::UnsupportedGeometry,
                            "hole.radius",
                            "the radius needs more than 256 segments",
                        )
                    })?;
                let segments = 16 + 4 * index;
                Ok((0..segments)
                    .map(|k| {
                        let theta = -core::f64::consts::TAU * (k as f64) / (segments as f64);
                        [
                            centre_mm[0] + radius * theta.cos(),
                            centre_mm[1] + radius * theta.sin(),
                        ]
                    })
                    .collect())
            }
            Self::Rectangle {
                corner_mm: [x, y],
                width_mm,
                height_mm,
            } => Ok(vec![
                [*x, *y],
                [*x, y + height_mm],
                [x + width_mm, y + height_mm],
                [x + width_mm, *y],
            ]),
            // The authoritative member-hole polygon, relative to the axis point in the plane
            // perpendicular to the axis: the plate round generator at the origin, whose vertex 0
            // is the hole frame's +x, i.e. `+Z_m` from the axis point (F06 section 1).
            Self::Member(member) => Self::Round {
                centre_mm: [0.0, 0.0],
                diameter_mm: member.diameter_mm,
            }
            .polygon(),
        }
    }
}

/// What one through-cut record of a plate authors (A13): a fastener hole (A04) or a polygon
/// cut. Both replay through the same certified kernel subtraction and share the per-element
/// ordinal counter and the `cut:f.h<ordinal>` wall ids; the kind keeps the operations distinct
/// for numbering and reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HoleKind {
    /// A through hole authored by `AddHole` (A04).
    Hole,
    /// A polygon cut authored by `CutByPolygon` or derived from `CutByPart` (A13).
    PolygonCut,
}

impl HoleKind {
    /// The wire token.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Hole => "hole",
            Self::PolygonCut => "polygon_cut",
        }
    }

    /// Parses the wire token; anything else is `None`.
    pub fn parse(token: &str) -> Option<Self> {
        match token {
            "hole" => Some(Self::Hole),
            "polygon_cut" => Some(Self::PolygonCut),
            _ => None,
        }
    }
}

/// One through-hole operation record of a plate (F04 section 4, A04): an owned record with an
/// engine-assigned ordinal and the authoritative clockwise polygon. The records of one element
/// are always replayed from the base shape in ordinal order, so no stored intermediate mesh can
/// go stale. Ordinals are never reused or renumbered; `UpdateHole` and `RemoveHole`, later
/// slices of A04, address a record by its `operation_id`. Polygon cuts (A13) share the record
/// form and the ordinal counter, told apart by `kind`.
#[derive(Debug, Clone, PartialEq)]
pub struct HoleRecord {
    /// Full UUIDv7 of the record, unused in the project (`DUPLICATE_ID`).
    pub operation_id: OpId,
    /// The per-element ordinal in creation order, starting at zero. It names the record's
    /// FeatureIds (`cut:f.h<ordinal>...`).
    pub ordinal: usize,
    /// The authoritative clockwise polygon in plate-local XY, as generated at the input edge.
    pub polygon_mm: Vec<[f64; 2]>,
    /// Whether the record is a hole (A04) or a polygon cut (A13).
    pub kind: HoleKind,
    /// The bolt group that generated the record (A14.4); `None` for a hand-placed hole. A
    /// generated record is owned by its group: it is created, regenerated and removed with the
    /// group as one unit, and `UpdateHole`/`RemoveHole` refuse it with `DEPENDENCY_CONFLICT`.
    pub owner: Option<ElementId>,
}

/// One member-hole operation record of a member (A04.2, the F06 member-hole amendment): an
/// owned record with an engine-assigned ordinal, stored entirely in the member frame, so it
/// follows the member when it is moved or rotated. The records of one member are replayed from
/// the base shape in ordinal order. Ordinals are never reused or renumbered; `UpdateHole` and
/// `RemoveHole` address a record by its `operation_id`. Member records live on the member, so
/// they never conflate with the plate records (A04) or polygon cuts (A13) under
/// `update-hole`/`remove-hole`.
#[derive(Debug, Clone, PartialEq)]
pub struct MemberHoleRecord {
    /// Full UUIDv7 of the record, unused in the project (`DUPLICATE_ID`).
    pub operation_id: OpId,
    /// The per-member ordinal in creation order, starting at zero. It names the record's
    /// FeatureIds (`cut:f.h<ordinal>...`).
    pub ordinal: usize,
    /// A point `(u, v, w)` on the hole axis in the member frame.
    pub axis_point_mm: [f64; 3],
    /// The unit direction of the hole axis in the member frame: a section direction of the
    /// first slice (`(+-1, 0, 0)` or `(0, +-1, 0)`).
    pub axis_direction: [f64; 3],
    /// How far the cutter reaches.
    pub extent: HoleExtent,
    /// The authoritative clockwise polygon relative to the axis point in the plane
    /// perpendicular to the axis, hole-frame XY (x along `+Z_m`, y along `axis_direction x
    /// +Z_m`), vertex 0 on `+Z_m`.
    pub polygon_mm: Vec<[f64; 2]>,
}

/// Replace one existing hole's authoritative polygon, preserving its identity and ordinal.
#[derive(Debug, Clone, PartialEq)]
pub struct UpdateHole {
    /// Target, existing record UUID and replacement hole specification.
    pub hole: AddHole,
}
impl UpdateHole {
    /// Admit the shape arguments independently of model state.
    pub fn validate_arguments(&self) -> Result<(), CommandError> {
        self.hole.validate_arguments()
    }
    /// Stable typed request identity, separate from creation.
    pub fn content_fingerprint(&self) -> Result<ContentDigest, CommandError> {
        let mut b = b"BeamCraft/A04/UpdateHole/v1\0".to_vec();
        b.extend_from_slice(self.hole.content_fingerprint()?.as_bytes());
        Sha256::digest(&b).map_err(|_| {
            CommandError::new(
                ErrorCode::LimitExceeded,
                "content",
                "hole request exceeds its bound",
            )
        })
    }
}

/// Remove an existing hole by record identity, never by an unstable polygon index.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RemoveHole {
    /// Target owned plate.
    pub target_id: ElementId,
    /// Existing hole record.
    pub operation_id: OpId,
}
impl RemoveHole {
    /// Canonical typed removal digest.
    pub fn content_fingerprint(&self) -> Result<ContentDigest, CommandError> {
        let mut b = b"BeamCraft/A04/RemoveHole/v1\0".to_vec();
        b.extend_from_slice(self.target_id.as_bytes());
        b.extend_from_slice(self.operation_id.as_bytes());
        Sha256::digest(&b).map_err(|_| {
            CommandError::new(
                ErrorCode::LimitExceeded,
                "content",
                "hole request exceeds its bound",
            )
        })
    }
}

/// F04 section 7 `AddHole`: one through-hole operation record on a valid plate (A04) or member
/// (A04.2, the F06 member-hole amendment). The record's `operation_id` is also the logical
/// command id: a retry with identical content replays the recorded result, different content
/// under an accepted id is `OP_ID_REUSE`, and an id that already names a record of the project
/// is `DUPLICATE_ID`. The engine assigns the ordinal from the target's per-element counter. A
/// member target carries a [`HoleSpec::Member`] generator; a polygon generator on a member is
/// `UNSUPPORTED_GEOMETRY` in this slice.
#[derive(Debug, Clone, PartialEq)]
pub struct AddHole {
    /// The plate or member that will own the record.
    pub target_id: ElementId,
    /// Full UUIDv7 of the record, unused in the project.
    pub operation_id: OpId,
    /// The hole to author.
    pub hole: HoleSpec,
}

impl AddHole {
    /// Admits the arguments that do not depend on geometry or state.
    pub fn validate_arguments(&self) -> Result<(), CommandError> {
        self.hole.validate()
    }

    /// The clockwise polygon this command authors; the stored polygon is authoritative.
    pub fn polygon(&self) -> Result<Vec<[f64; 2]>, CommandError> {
        self.hole.polygon()
    }

    /// SHA-256 of a versioned, length-prefixed binary encoding of the validated arguments, with
    /// signed zero normalized, as for [`CreatePlate::content_fingerprint`]. It is *not* the F04
    /// canonical form and must not be sent over a wire. Fails on arguments that do not
    /// validate, so a fingerprint exists only for admissible content.
    pub fn content_fingerprint(&self) -> Result<ContentDigest, CommandError> {
        self.validate_arguments()?;
        let bad = || {
            CommandError::new(
                ErrorCode::NonFiniteNumber,
                "hole",
                "value cannot be encoded",
            )
            .with_rule("DV-ATTR-003")
        };
        let polygon = self.polygon()?;
        let mut bytes = Vec::with_capacity(64 + polygon.len() * 16);
        bytes.extend_from_slice(b"BeamCraft/A04/AddHole/v1\0");
        bytes.extend_from_slice(self.target_id.as_bytes());
        bytes.extend_from_slice(self.operation_id.as_bytes());
        match &self.hole {
            HoleSpec::Polygon(_) => bytes.push(0),
            HoleSpec::Round {
                centre_mm,
                diameter_mm,
            } => {
                bytes.push(1);
                for v in centre_mm.iter().copied().chain([*diameter_mm]) {
                    bytes.extend_from_slice(&canonical_f64_bytes(v).map_err(|_| bad())?);
                }
            }
            HoleSpec::Rectangle {
                corner_mm,
                width_mm,
                height_mm,
            } => {
                bytes.push(2);
                for v in corner_mm.iter().copied().chain([*width_mm, *height_mm]) {
                    bytes.extend_from_slice(&canonical_f64_bytes(v).map_err(|_| bad())?);
                }
            }
            HoleSpec::Member(member) => {
                bytes.push(3);
                for v in member
                    .axis_point_mm
                    .iter()
                    .copied()
                    .chain(member.axis_direction.iter().copied())
                    .chain([member.diameter_mm])
                {
                    bytes.extend_from_slice(&canonical_f64_bytes(v).map_err(|_| bad())?);
                }
                bytes.push(match member.extent {
                    HoleExtent::Through => 0,
                    HoleExtent::Range { .. } => 1,
                });
                if let HoleExtent::Range { from_mm, to_mm } = member.extent {
                    for v in [from_mm, to_mm] {
                        bytes.extend_from_slice(&canonical_f64_bytes(v).map_err(|_| bad())?);
                    }
                }
            }
        }
        bytes.extend_from_slice(&(polygon.len() as u64).to_le_bytes());
        for v in polygon.iter().flatten() {
            bytes.extend_from_slice(&canonical_f64_bytes(*v).map_err(|_| bad())?);
        }
        Sha256::digest(&bytes).map_err(|_| {
            CommandError::new(ErrorCode::LimitExceeded, "content", "content is too large")
                .with_rule("DV-LIM-001")
        })
    }
}

/// Most bend records on one bent plate (A10). The records together must describe one
/// constant-cross-section body; the cap keeps the derived cross-section inside the loft
/// vertex budget with room for the bend arcs.
pub const MAX_BENDS: usize = 16;

/// How a plate is bent or rolled (A10). The authored contour of a bent plate is its
/// developed mid-surface outline (the unfold) in the element-local plane `z = 0`; bending
/// about the mid-surface preserves it, so the unfolded outline is the authored contour and
/// the analytic volume stays `outline area x thickness`. All coordinates are element-local
/// millimetres in the contour plane, angles are radians.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BendSpec {
    /// The sheet is folded about the line `a_mm`–`b_mm` by `angle_rad` (positive folds the
    /// far side towards `+Z` by the right-hand rule about `a->b`) on the inner bend radius
    /// `radius_mm`; the outer radius is `radius_mm + thickness`. Several folds of one plate
    /// must be parallel.
    Fold {
        /// First point of the fold line, in the contour plane.
        a_mm: [f64; 3],
        /// Second point of the fold line, in the contour plane; equal to `a_mm` is refused.
        b_mm: [f64; 3],
        /// Inner bend radius in millimetres.
        radius_mm: f64,
        /// Signed bend angle in radians, inside `(-pi, pi)`, never zero.
        angle_rad: f64,
    },
    /// The sheet is rolled about the axis `a_mm`–`b_mm` into a cylinder section of the inner
    /// radius `radius_mm` (the arc follows from the developed length of the blank), or, with
    /// `radius_b_mm`, into a cone section of the two inner mid-surface radii at the axis
    /// ends. A plate carries at most one roll and no fold beside it.
    Roll {
        /// First point of the roll axis, in the contour plane; for a cone it is the centre
        /// of the blank's inner arc.
        a_mm: [f64; 3],
        /// Second point of the roll axis, in the contour plane; equal to `a_mm` is refused.
        b_mm: [f64; 3],
        /// Inner radius at `a_mm` in millimetres.
        radius_mm: f64,
        /// Inner radius at `b_mm` in millimetres; `None` rolls a cylinder. Equal to
        /// `radius_mm` is refused: a true cone cannot have equal radii.
        radius_b_mm: Option<f64>,
    },
}

impl BendSpec {
    /// The wire token of the bend kind.
    pub const fn kind(self) -> &'static str {
        match self {
            Self::Fold { .. } => "fold",
            Self::Roll { .. } => "roll",
        }
    }

    /// Admits what does not depend on the plate: finite values, the coordinate cap for the
    /// line points, positive radii and a bend angle inside `(-pi, pi)` and away from zero.
    pub fn validate(&self) -> Result<(), CommandError> {
        let (points, radii, angle): ([[f64; 3]; 2], &[f64], Option<f64>) = match *self {
            Self::Fold {
                a_mm,
                b_mm,
                radius_mm,
                angle_rad,
            } => ([a_mm, b_mm], &[radius_mm][..], Some(angle_rad)),
            Self::Roll {
                a_mm,
                b_mm,
                radius_mm,
                radius_b_mm,
            } => (
                [a_mm, b_mm],
                match radius_b_mm {
                    Some(rb) => &[radius_mm, rb][..],
                    None => &[radius_mm][..],
                },
                None,
            ),
        };
        if points
            .iter()
            .flatten()
            .copied()
            .chain(radii.iter().copied())
            .any(|v| !v.is_finite())
        {
            return Err(CommandError::new(
                ErrorCode::NonFiniteNumber,
                "bend",
                "bend value is not finite",
            )
            .with_rule("DV-ATTR-003"));
        }
        if points.iter().flatten().any(|v| v.abs() > COORDINATE_CAP_MM) {
            return Err(CommandError::new(
                ErrorCode::LimitExceeded,
                "bend",
                "bend coordinate exceeds the coordinate cap",
            )
            .with_rule("DV-LIM-001"));
        }
        if radii.iter().any(|r| *r <= 0.0) {
            return Err(CommandError::new(
                ErrorCode::InvalidArgument,
                "bend",
                "bend radius is not positive",
            ));
        }
        if radii.iter().any(|r| *r > DIMENSION_CAP_MM) {
            return Err(CommandError::new(
                ErrorCode::LimitExceeded,
                "bend",
                "bend radius exceeds the dimension cap",
            )
            .with_rule("DV-LIM-001"));
        }
        if let Some(angle) = angle
            && (angle == 0.0 || angle.abs() >= core::f64::consts::PI)
        {
            return Err(CommandError::new(
                ErrorCode::InvalidArgument,
                "bend",
                "fold angle is outside the open range (-pi, pi)",
            ));
        }
        if points[0] == points[1] {
            return Err(CommandError::new(
                ErrorCode::InvalidArgument,
                "bend",
                "the bend line needs two distinct points",
            ));
        }
        // A cone whose two radii are equal is a cylinder: the stored encoding keeps one radius
        // for a cylinder and two for a cone, so an equal-radius cone would read back as a
        // cylinder on replay and refuse its own sector blank. A true cone cannot have them.
        if let Self::Roll {
            radius_mm,
            radius_b_mm: Some(radius_b),
            ..
        } = *self
            && radius_b == radius_mm
        {
            return Err(CommandError::new(
                ErrorCode::InvalidArgument,
                "bend",
                "a cone roll needs two distinct radii; drop radius_b to roll a cylinder",
            ));
        }
        Ok(())
    }
}

/// One owned bend operation record (A10). As for a cut, the record is authoritative and
/// replayed in creation order; a fold appends to the plate's fold set, a roll requires the
/// plate to carry no other bend.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BendRecord {
    /// Full UUIDv7 of the record, unused in the project (`DUPLICATE_ID`).
    pub operation_id: OpId,
    /// The bend the record authors.
    pub spec: BendSpec,
}

impl BendRecord {
    /// Admits the record's arguments independently of the plate.
    pub fn validate(&self) -> Result<(), CommandError> {
        self.spec.validate()
    }
}

/// F04 section 7 `BendPlate` (A10): appends one bend operation record to a valid plate and
/// updates its derived data in one transaction. The record's `operation_id` is also the
/// logical command id, as for `AddHole`. Whether the records together describe a supported
/// bent body (a rectangular blank for folds, a cylinder or cone section for a roll, no
/// overlap) is the engine's decision at evaluation, and a refusal there refuses the command.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BendPlate {
    /// The plate that will own the record; a member target is `UNSUPPORTED_GEOMETRY`.
    pub target_id: ElementId,
    /// Full UUIDv7 of the record, unused in the project.
    pub operation_id: OpId,
    /// The bend to author.
    pub spec: BendSpec,
}

impl BendPlate {
    /// Admits the arguments that do not depend on geometry or model state.
    pub fn validate_arguments(&self) -> Result<(), CommandError> {
        self.spec.validate()
    }

    /// SHA-256 of a versioned binary encoding of the validated arguments, with signed zero
    /// normalized, as for [`AddHole::content_fingerprint`]. It is *not* the F04 canonical
    /// form and must not be sent over a wire.
    pub fn content_fingerprint(&self) -> Result<ContentDigest, CommandError> {
        self.validate_arguments()?;
        let bad = || {
            CommandError::new(
                ErrorCode::NonFiniteNumber,
                "bend",
                "value cannot be encoded",
            )
            .with_rule("DV-ATTR-003")
        };
        let (a, b, tail): ([f64; 3], [f64; 3], &[f64]) = match self.spec {
            BendSpec::Fold {
                a_mm,
                b_mm,
                radius_mm,
                angle_rad,
            } => (a_mm, b_mm, &[radius_mm, angle_rad]),
            BendSpec::Roll {
                a_mm,
                b_mm,
                radius_mm,
                radius_b_mm,
            } => (
                a_mm,
                b_mm,
                // A cylinder repeats its one radius; the kind byte already separates the two
                // encodings, so the fill value cannot collide with a cone's.
                &[radius_mm, radius_b_mm.unwrap_or(radius_mm)],
            ),
        };
        let mut bytes = Vec::with_capacity(128);
        bytes.extend_from_slice(b"BeamCraft/A10/BendPlate/v1\0");
        bytes.extend_from_slice(self.target_id.as_bytes());
        bytes.extend_from_slice(self.operation_id.as_bytes());
        bytes.push(match self.spec {
            BendSpec::Fold { .. } => 0,
            BendSpec::Roll { .. } => 1,
        });
        for v in a.iter().copied().chain(b).chain(tail.iter().copied()) {
            bytes.extend_from_slice(&canonical_f64_bytes(v).map_err(|_| bad())?);
        }
        Sha256::digest(&bytes).map_err(|_| {
            CommandError::new(ErrorCode::LimitExceeded, "content", "content is too large")
                .with_rule("DV-LIM-001")
        })
    }
}

/// F04 section 7 `CutByLine` (A13): cuts a valid plate, beam or column with the plane through
/// the line `point_a_mm`–`point_b_mm` and the given axis direction, keeping one side. The plane
/// is derived once at admission and stored as an ordinary `plane_cut` record; the line itself is
/// not kept in the record.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CutByLine {
    /// The element to cut.
    pub target_id: ElementId,
    /// Full UUIDv7 of the new record; unused in the project.
    pub operation_id: OpId,
    /// First point of the cutting line, element-local millimetres.
    pub point_a_mm: [f64; 3],
    /// Second point of the cutting line, element-local millimetres; equal to `point_a_mm` is
    /// `INVALID_ARGUMENT`.
    pub point_b_mm: [f64; 3],
    /// A non-zero direction the cutting plane contains besides the line; parallel to the line
    /// is `INVALID_ARGUMENT`.
    pub axis: [f64; 3],
    /// The kept half-space of the derived plane.
    pub keep: KeepSide,
}

impl CutByLine {
    /// Admits what does not depend on geometry or model state: finiteness, the coordinate cap,
    /// distinct line points and a non-zero axis. Whether the derived plane actually crosses the
    /// element is the kernel's decision at replay.
    pub fn validate_arguments(&self) -> Result<(), CommandError> {
        let values = [self.point_a_mm, self.point_b_mm, self.axis];
        if values.iter().flatten().any(|v| !v.is_finite()) {
            return Err(CommandError::new(
                ErrorCode::NonFiniteNumber,
                "cut_by_line",
                "value is not finite",
            )
            .with_rule("DV-ATTR-003"));
        }
        if values.iter().flatten().any(|v| v.abs() > COORDINATE_CAP_MM) {
            return Err(CommandError::new(
                ErrorCode::LimitExceeded,
                "cut_by_line",
                "coordinate exceeds the coordinate cap",
            )
            .with_rule("DV-LIM-001"));
        }
        if self.point_a_mm == self.point_b_mm {
            return Err(CommandError::new(
                ErrorCode::InvalidArgument,
                "cut_by_line",
                "the two line points must differ",
            ));
        }
        if self.axis == [0.0; 3] {
            return Err(CommandError::new(
                ErrorCode::InvalidArgument,
                "cut_by_line",
                "the cutting plane needs a non-zero axis",
            ));
        }
        Ok(())
    }

    /// The derived plane as a `plane_cut` record geometry: the plane contains the line and the
    /// axis, so its normal is `normalize(cross(b - a, axis))`. An axis parallel to the line is
    /// `INVALID_ARGUMENT`.
    pub fn plane(&self) -> Result<([f64; 3], [f64; 3]), CommandError> {
        let dir = [
            self.point_b_mm[0] - self.point_a_mm[0],
            self.point_b_mm[1] - self.point_a_mm[1],
            self.point_b_mm[2] - self.point_a_mm[2],
        ];
        let n = [
            dir[1] * self.axis[2] - dir[2] * self.axis[1],
            dir[2] * self.axis[0] - dir[0] * self.axis[2],
            dir[0] * self.axis[1] - dir[1] * self.axis[0],
        ];
        let length = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
        if !length.is_finite() || length == 0.0 {
            return Err(CommandError::new(
                ErrorCode::InvalidArgument,
                "cut_by_line",
                "the axis must not be parallel to the line",
            ));
        }
        Ok((
            self.point_a_mm,
            [n[0] / length, n[1] / length, n[2] / length],
        ))
    }

    /// SHA-256 of a versioned, length-prefixed binary encoding of the validated arguments, as
    /// for [`CutByPlane::content_fingerprint`].
    pub fn content_fingerprint(&self) -> Result<ContentDigest, CommandError> {
        fingerprint_cut(
            b"BeamCraft/A13/CutByLine/v1",
            self.target_id,
            self.operation_id,
            self.validate_arguments(),
            |bytes| {
                for value in self
                    .point_a_mm
                    .into_iter()
                    .chain(self.point_b_mm)
                    .chain(self.axis)
                {
                    push_f64(bytes, value)?;
                }
                Ok(())
            },
        )
    }
}

/// F04 section 7 `CutByPolygon` (A13): cuts a valid plate with the prism of a closed polygon in
/// plate-local XY swept through the whole thickness, removing the material inside it. The
/// polygon admission is the kernel's (clockwise, simple, strictly inside the outline, clear of
/// every other record), exactly as for a hole; the record is a cut, not a hole.
#[derive(Debug, Clone, PartialEq)]
pub struct CutByPolygon {
    /// The plate to cut; a member target is `UNSUPPORTED_GEOMETRY` (the kernel subtracts
    /// Z-prisms only).
    pub target_id: ElementId,
    /// Full UUIDv7 of the new record; unused in the project.
    pub operation_id: OpId,
    /// The clockwise polygon in plate-local XY, 3 to 1024 points, closure implicit.
    pub polygon_mm: Vec<[f64; 2]>,
}

impl CutByPolygon {
    /// Admits the polygon shape (count, finiteness, caps) as [`HoleSpec::validate`] does.
    pub fn validate_arguments(&self) -> Result<(), CommandError> {
        HoleSpec::Polygon(self.polygon_mm.clone()).validate()
    }

    /// The clockwise polygon this command authors; the stored polygon is authoritative.
    pub fn polygon(&self) -> Result<Vec<[f64; 2]>, CommandError> {
        HoleSpec::Polygon(self.polygon_mm.clone()).polygon()
    }

    /// SHA-256 of a versioned, length-prefixed binary encoding of the validated arguments, as
    /// for [`CutByPlane::content_fingerprint`].
    pub fn content_fingerprint(&self) -> Result<ContentDigest, CommandError> {
        let polygon = self.polygon()?;
        fingerprint_cut(
            b"BeamCraft/A13/CutByPolygon/v1",
            self.target_id,
            self.operation_id,
            self.validate_arguments(),
            |bytes| {
                for value in polygon.iter().flatten().copied() {
                    push_f64(bytes, value)?;
                }
                Ok(())
            },
        )
    }
}

/// F04 section 7 `CutByPart` (A13): cuts a plate with another plate, removing the material
/// inside the tool's prism. The tool's contour and reach are derived once at admission in the
/// target's local frame and stored as a polygon-cut record; moving the tool afterwards never
/// updates the cut. A tool whose relative rotation is not about the target normal, or whose
/// prism does not cross the target, is refused.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CutByPart {
    /// The plate to cut.
    pub target_id: ElementId,
    /// Full UUIDv7 of the new record; unused in the project.
    pub operation_id: OpId,
    /// The plate used as the cutter.
    pub tool_id: ElementId,
}

impl CutByPart {
    /// Nothing to admit beyond identity; everything else is the engine's decision.
    pub const fn validate_arguments(&self) -> Result<(), CommandError> {
        Ok(())
    }

    /// SHA-256 of a versioned, length-prefixed binary encoding of the validated arguments, as
    /// for [`CutByPlane::content_fingerprint`].
    pub fn content_fingerprint(&self) -> Result<ContentDigest, CommandError> {
        fingerprint_cut(
            b"BeamCraft/A13/CutByPart/v1",
            self.target_id,
            self.operation_id,
            self.validate_arguments(),
            |bytes| {
                bytes.extend_from_slice(self.tool_id.as_bytes());
                Ok(())
            },
        )
    }
}

/// F04 section 7 `FitEndToPart` (A13): trims a plate, beam or column flush with the face of
/// another plate. The engine fits to the tool face nearest the target's extent midpoint and
/// keeps the side the target body lies on; the derived plane is stored as an ordinary
/// `plane_cut` record. Moving the tool afterwards never updates the fit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FitEndToPart {
    /// The element to fit.
    pub target_id: ElementId,
    /// Full UUIDv7 of the new record; unused in the project.
    pub operation_id: OpId,
    /// The plate whose face the end is fitted to.
    pub tool_id: ElementId,
}

impl FitEndToPart {
    /// Nothing to admit beyond identity; everything else is the engine's decision.
    pub const fn validate_arguments(&self) -> Result<(), CommandError> {
        Ok(())
    }

    /// SHA-256 of a versioned, length-prefixed binary encoding of the validated arguments, as
    /// for [`CutByPlane::content_fingerprint`].
    pub fn content_fingerprint(&self) -> Result<ContentDigest, CommandError> {
        fingerprint_cut(
            b"BeamCraft/A13/FitEndToPart/v1",
            self.target_id,
            self.operation_id,
            self.validate_arguments(),
            |bytes| {
                bytes.extend_from_slice(self.tool_id.as_bytes());
                Ok(())
            },
        )
    }
}

/// F04 section 7 `ChamferPart` (A13): chamfers one corner of a valid plate by cutting it with
/// the plane through the two points `first_distance_mm` and `second_distance_mm` back along the
/// two contour edges at the vertex, keeping the far side. The derived plane is stored as an
/// ordinary `plane_cut` record; the vertex distances are not kept in the record.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChamferPart {
    /// The plate to chamfer.
    pub target_id: ElementId,
    /// Full UUIDv7 of the new record; unused in the project.
    pub operation_id: OpId,
    /// Index of the contour vertex to chamfer, into the stored contour order; out of range is
    /// `NOT_FOUND` at the engine.
    pub vertex_index: usize,
    /// Distance back along the edge from the previous vertex, in millimetres.
    pub first_distance_mm: f64,
    /// Distance back along the edge to the next vertex, in millimetres.
    pub second_distance_mm: f64,
}

impl ChamferPart {
    /// Admits the distances: finite, positive, within the coordinate cap. Whether they fit the
    /// edges and whether the derived plane crosses the element are the engine's and the
    /// kernel's decisions.
    pub fn validate_arguments(&self) -> Result<(), CommandError> {
        for (name, value) in [
            ("first_distance_mm", self.first_distance_mm),
            ("second_distance_mm", self.second_distance_mm),
        ] {
            if !value.is_finite() {
                return Err(CommandError::new(
                    ErrorCode::NonFiniteNumber,
                    name,
                    "value is not finite",
                )
                .with_rule("DV-ATTR-003"));
            }
            if value <= 0.0 {
                return Err(CommandError::new(
                    ErrorCode::InvalidArgument,
                    name,
                    "a chamfer distance must be positive",
                ));
            }
            if value > COORDINATE_CAP_MM {
                return Err(CommandError::new(
                    ErrorCode::LimitExceeded,
                    name,
                    "distance exceeds the coordinate cap",
                )
                .with_rule("DV-LIM-001"));
            }
        }
        Ok(())
    }

    /// SHA-256 of a versioned, length-prefixed binary encoding of the validated arguments, as
    /// for [`CutByPlane::content_fingerprint`].
    pub fn content_fingerprint(&self) -> Result<ContentDigest, CommandError> {
        fingerprint_cut(
            b"BeamCraft/A13/ChamferPart/v1",
            self.target_id,
            self.operation_id,
            self.validate_arguments(),
            |bytes| {
                push_f64(bytes, self.first_distance_mm)?;
                push_f64(bytes, self.second_distance_mm)?;
                push_f64(bytes, self.vertex_index as f64)
            },
        )
    }
}

/// F04 section 7 `SplitPart` (A13): splits a valid plate, beam or column in two with a plane.
/// The target keeps the `keep` half-space and gains the matching `plane_cut` record; a new
/// element with the given identity receives a copy of the target's authored state and the
/// complementary cut. One command, one revision, one undo unit.
#[derive(Debug, Clone, PartialEq)]
pub struct SplitPart {
    /// The element to split.
    pub target_id: ElementId,
    /// Full identity of the new element holding the other half; unused in the project.
    pub new_id: ElementId,
    /// Full UUIDv7 of the new record; unused in the project.
    pub operation_id: OpId,
    /// Label of the new element.
    pub new_name: String,
    /// A point of the splitting plane, element-local millimetres.
    pub point_mm: [f64; 3],
    /// A non-zero normal of the splitting plane.
    pub normal: [f64; 3],
    /// The half-space the target keeps; the new element keeps the other.
    pub keep: KeepSide,
}

impl SplitPart {
    /// Admits the label and the plane arguments as `CutByPlane` does.
    pub fn validate_arguments(&self) -> Result<(), CommandError> {
        if !label_is_valid(&self.new_name) {
            return Err(CommandError::new(
                ErrorCode::InvalidArgument,
                "new_name",
                "label must be 1 to 256 bytes without control characters",
            )
            .with_rule("DV-BASE-003"));
        }
        CutRecord {
            operation_id: self.operation_id,
            ordinal: 0,
            point_mm: self.point_mm,
            normal: self.normal,
            keep: self.keep,
        }
        .validate()
    }

    /// SHA-256 of a versioned, length-prefixed binary encoding of the validated arguments, as
    /// for [`CutByPlane::content_fingerprint`].
    pub fn content_fingerprint(&self) -> Result<ContentDigest, CommandError> {
        fingerprint_cut(
            b"BeamCraft/A13/SplitPart/v1",
            self.target_id,
            self.operation_id,
            self.validate_arguments(),
            |bytes| {
                bytes.extend_from_slice(self.new_id.as_bytes());
                for value in self.point_mm.into_iter().chain(self.normal) {
                    push_f64(bytes, value)?;
                }
                // The label bytes, length-prefixed as `CreatePlate` does, so same-length
                // renames are never one recorded content.
                let name = self.new_name.as_bytes();
                bytes.extend_from_slice(&(name.len() as u64).to_le_bytes());
                bytes.extend_from_slice(name);
                push_f64(
                    bytes,
                    match self.keep {
                        KeepSide::Positive => 0.0,
                        KeepSide::Negative => 1.0,
                    },
                )
            },
        )
    }
}

/// F04 section 7 `CombineParts` (A13): combines two straight members with the same pinned
/// profile, orientation and material into one longer member along the same axis. The survivor
/// keeps its identity and extends over the union of the two axis ranges, which must overlap or
/// touch; the consumed element is deleted. The effect is the resulting authored state and the
/// revision ledger entry; the journal, not a per-element record, carries the operation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CombineParts {
    /// The member that survives and extends.
    pub survivor_id: ElementId,
    /// The member consumed by the combination; it is deleted.
    pub consumed_id: ElementId,
    /// Full UUIDv7 of the operation; unused in the project.
    pub operation_id: OpId,
}

impl CombineParts {
    /// The two members must differ; everything else is the engine's decision.
    pub fn validate_arguments(&self) -> Result<(), CommandError> {
        if self.survivor_id == self.consumed_id {
            return Err(CommandError::new(
                ErrorCode::InvalidArgument,
                "consumed_id",
                "a member cannot be combined with itself",
            ));
        }
        Ok(())
    }

    /// SHA-256 of a versioned, length-prefixed binary encoding of the validated arguments, as
    /// for [`CutByPlane::content_fingerprint`].
    pub fn content_fingerprint(&self) -> Result<ContentDigest, CommandError> {
        fingerprint_cut(
            b"BeamCraft/A13/CombineParts/v1",
            self.survivor_id,
            self.operation_id,
            self.validate_arguments(),
            |bytes| {
                bytes.extend_from_slice(self.consumed_id.as_bytes());
                Ok(())
            },
        )
    }
}

/// The shared fingerprint form of the A13 commands: a version tag, the primary element, the
/// record identity and a caller-provided tail, digested as [`CutByPlane`] does. A failed
/// admission or an inexpressible value has no fingerprint.
fn fingerprint_cut(
    tag: &[u8],
    element: ElementId,
    operation: OpId,
    admitted: Result<(), CommandError>,
    tail: impl FnOnce(&mut Vec<u8>) -> Result<(), CommandError>,
) -> Result<ContentDigest, CommandError> {
    admitted?;
    let bad = || {
        CommandError::new(ErrorCode::NonFiniteNumber, "cut", "value cannot be encoded")
            .with_rule("DV-ATTR-003")
    };
    let mut bytes = Vec::with_capacity(96);
    bytes.extend_from_slice(tag);
    bytes.push(0);
    bytes.extend_from_slice(element.as_bytes());
    bytes.extend_from_slice(operation.as_bytes());
    tail(&mut bytes).map_err(|_| bad())?;
    Sha256::digest(&bytes).map_err(|_| {
        CommandError::new(ErrorCode::LimitExceeded, "content", "content is too large")
            .with_rule("DV-LIM-001")
    })
}

/// Appends the canonical bytes of one f64 to a fingerprint buffer.
fn push_f64(bytes: &mut Vec<u8>, value: f64) -> Result<(), CommandError> {
    let bad = CommandError::new(ErrorCode::NonFiniteNumber, "cut", "value cannot be encoded")
        .with_rule("DV-ATTR-003");
    bytes.extend_from_slice(&canonical_f64_bytes(value).map_err(|_| bad)?);
    Ok(())
}

/// The weld types A06 supports. Every other type (butt, plug, spot and so on) is not modelled and
/// has no token here, so it cannot be requested.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeldType {
    /// A fillet weld.
    Fillet,
}

/// Where a weld is made. It is always authored explicitly; there is no default.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeldLocation {
    /// Made in the workshop.
    Workshop,
    /// Made on site.
    Site,
}

/// The side of the joint a weld is on, relative to the main part's reference side. It is always
/// authored explicitly; there is no default.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeldSide {
    /// The reference (arrow) side.
    Arrow,
    /// The opposite side.
    Other,
    /// Both sides.
    Both,
}

impl WeldType {
    /// The stored token.
    pub const fn as_token(self) -> &'static str {
        match self {
            Self::Fillet => "fillet",
        }
    }
    /// Parses an exact token.
    pub fn parse(token: &str) -> Option<Self> {
        (token == "fillet").then_some(Self::Fillet)
    }
}

impl WeldLocation {
    /// The stored token.
    pub const fn as_token(self) -> &'static str {
        match self {
            Self::Workshop => "workshop",
            Self::Site => "site",
        }
    }
    /// Parses an exact token.
    pub fn parse(token: &str) -> Option<Self> {
        match token {
            "workshop" => Some(Self::Workshop),
            "site" => Some(Self::Site),
            _ => None,
        }
    }
}

impl WeldSide {
    /// The stored token.
    pub const fn as_token(self) -> &'static str {
        match self {
            Self::Arrow => "arrow",
            Self::Other => "other",
            Self::Both => "both",
        }
    }
    /// Parses an exact token.
    pub fn parse(token: &str) -> Option<Self> {
        match token {
            "arrow" => Some(Self::Arrow),
            "other" => Some(Self::Other),
            "both" => Some(Self::Both),
            _ => None,
        }
    }
}

/// The authored properties of one weld (A06). No solid geometry is derived from them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WeldSpec {
    /// The weld type.
    pub weld_type: WeldType,
    /// Nominal size in millimetres: finite, greater than 0 and at most [`WELD_SIZE_CAP_MM`].
    pub size_mm: f64,
    /// Workshop or site.
    pub location: WeldLocation,
    /// Arrow side, other side or both.
    pub side: WeldSide,
}

/// The contour symbol of a weld (A15): the finish of its face as the drawing symbol shows it.
/// `none` is an authored absence; there is no default.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeldContour {
    /// No contour symbol.
    None,
    /// Flush contour.
    Flush,
    /// Concave contour.
    Concave,
    /// Convex contour.
    Convex,
}

impl WeldContour {
    /// The stored token.
    pub const fn as_token(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Flush => "flush",
            Self::Concave => "concave",
            Self::Convex => "convex",
        }
    }
    /// Parses an exact token.
    pub fn parse(token: &str) -> Option<Self> {
        match token {
            "none" => Some(Self::None),
            "flush" => Some(Self::Flush),
            "concave" => Some(Self::Concave),
            "convex" => Some(Self::Convex),
            _ => None,
        }
    }
}

/// The symbol data of one weld (A15): what the drawing symbol states besides the type and size.
/// Everything is authored explicitly; the stored bytes keep exactly what was authored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WeldSymbol {
    /// All-around symbol: the weld runs around the whole joint perimeter. Symbol data only; the
    /// solid shape stays the authored path.
    pub all_around: bool,
    /// Contour symbol of the weld face.
    pub contour: WeldContour,
}

/// The optional geometry of one weld (A15): the path it runs along and its symbol data. The path
/// is a world-space polyline in millimetres; the type and size of the parent [`WeldSpec`] shape
/// its solid. At least [`MIN_WELD_PATH_POINTS`] and at most [`MAX_WELD_PATH_POINTS`] points, each
/// within the coordinate cap, consecutive points at least [`MIN_WELD_SEGMENT_MM`] apart and a
/// total length of at most [`MAX_WELD_LENGTH_MM`].
#[derive(Debug, Clone, PartialEq)]
pub struct WeldGeometry {
    /// Ordered world points of the path in millimetres.
    pub path_mm: Vec<[f64; 3]>,
    /// The symbol data.
    pub symbol: WeldSymbol,
}

/// A06 `CreateWeld`: a new `connection` element (domain vocabulary section 2.3) that is a weld. Its
/// one `connects` record (section 5.1) names exactly two distinct parts in order, the main part
/// first; the record lives with the weld, so the parts are only read. It carries the common base
/// and the weld properties; it has no body, material or derived geometry, and its placement is
/// the identity in this slice.
#[derive(Debug, Clone, PartialEq)]
pub struct CreateWeld {
    /// Identity of the new weld.
    pub new_id: ElementId,
    /// Explicit user label.
    pub name: String,
    /// The main part: a plate, beam or column.
    pub main_id: ElementId,
    /// The part welded to it: a plate, beam or column.
    pub secondary_id: ElementId,
    /// The weld properties.
    pub weld: WeldSpec,
    /// Optional geometry (A15): the path the weld runs along and its symbol data. `None` is an
    /// A06 weld without a body; `Some` derives the solid and stores the symbol data.
    pub geometry: Option<WeldGeometry>,
}

impl CreateWeld {
    /// Admits the request: the label (`DV-BASE-003`), two distinct parts (`DV-REL-006`,
    /// `DV-REL-007`), a weld that is not one of its own parts, and a finite size in
    /// `(0, WELD_SIZE_CAP_MM]`. Whether the parts exist and their kinds are the engine's decisions.
    pub fn validate_arguments(&self) -> Result<(), CommandError> {
        if !label_is_valid(&self.name) {
            return Err(CommandError::new(
                ErrorCode::InvalidArgument,
                "name",
                "label must be 1 to 256 bytes without control characters",
            )
            .with_rule("DV-BASE-003"));
        }
        if self.main_id == self.secondary_id {
            return Err(CommandError::new(
                ErrorCode::InvalidArgument,
                "secondary_id",
                "a weld connects two distinct parts",
            )
            .with_rule("DV-REL-007"));
        }
        if self.new_id == self.main_id || self.new_id == self.secondary_id {
            return Err(CommandError::new(
                ErrorCode::InvalidArgument,
                "new_id",
                "a weld cannot connect itself",
            )
            .with_rule("DV-REL-006"));
        }
        let size = self.weld.size_mm;
        if !size.is_finite() {
            return Err(CommandError::new(
                ErrorCode::NonFiniteNumber,
                "size_mm",
                "weld size must be finite",
            )
            .with_rule("DV-ATTR-003"));
        }
        if size <= 0.0 || size > WELD_SIZE_CAP_MM {
            return Err(CommandError::new(
                ErrorCode::InvalidArgument,
                "size_mm",
                "weld size must be greater than 0 and at most 1000 mm",
            ));
        }
        self.validate_geometry()
    }

    /// Admits the optional geometry (A15): a bounded path of finite world points within the
    /// coordinate cap, consecutive points at least [`MIN_WELD_SEGMENT_MM`] apart and a total
    /// length of at most [`MAX_WELD_LENGTH_MM`]. Absent geometry passes.
    fn validate_geometry(&self) -> Result<(), CommandError> {
        let Some(geometry) = &self.geometry else {
            return Ok(());
        };
        if geometry.path_mm.len() < MIN_WELD_PATH_POINTS
            || geometry.path_mm.len() > MAX_WELD_PATH_POINTS
        {
            return Err(CommandError::new(
                ErrorCode::InvalidArgument,
                "path_mm",
                "a weld path has 2 to 1024 points",
            ));
        }
        let mut total = 0.0;
        for (point, next) in geometry.path_mm.iter().zip(geometry.path_mm.iter().skip(1)) {
            for coordinate in point.iter().chain(next.iter()) {
                if !coordinate.is_finite() {
                    return Err(CommandError::new(
                        ErrorCode::NonFiniteNumber,
                        "path_mm",
                        "a weld path point must be finite",
                    )
                    .with_rule("DV-ATTR-003"));
                }
                if coordinate.abs() > COORDINATE_CAP_MM {
                    return Err(CommandError::new(
                        ErrorCode::InvalidArgument,
                        "path_mm",
                        "a weld path point exceeds the coordinate cap",
                    ));
                }
            }
            let segment = (next[0] - point[0]).powi(2)
                + (next[1] - point[1]).powi(2)
                + (next[2] - point[2]).powi(2);
            if !segment.is_finite() {
                return Err(CommandError::new(
                    ErrorCode::NonFiniteNumber,
                    "path_mm",
                    "a weld path segment must be finite",
                )
                .with_rule("DV-ATTR-003"));
            }
            if segment < MIN_WELD_SEGMENT_MM * MIN_WELD_SEGMENT_MM {
                return Err(CommandError::new(
                    ErrorCode::InvalidArgument,
                    "path_mm",
                    "consecutive weld path points must be at least 0.001 mm apart",
                ));
            }
            total += segment.sqrt();
        }
        if total > MAX_WELD_LENGTH_MM {
            return Err(CommandError::new(
                ErrorCode::LimitExceeded,
                "path_mm",
                "the weld path is longer than 1000000 mm",
            ));
        }
        Ok(())
    }

    /// SHA-256 of a versioned, length-prefixed encoding of the validated request.
    pub fn content_fingerprint(&self) -> Result<ContentDigest, CommandError> {
        self.validate_arguments()?;
        let mut bytes = Vec::with_capacity(128 + self.name.len());
        bytes.extend_from_slice(b"BeamCraft/A06/CreateWeld/v1\0");
        bytes.extend_from_slice(self.new_id.as_bytes());
        bytes.extend_from_slice(&(self.name.len() as u64).to_le_bytes());
        bytes.extend_from_slice(self.name.as_bytes());
        bytes.extend_from_slice(self.main_id.as_bytes());
        bytes.extend_from_slice(self.secondary_id.as_bytes());
        for token in [
            self.weld.weld_type.as_token(),
            self.weld.location.as_token(),
            self.weld.side.as_token(),
        ] {
            // Tokens are short constants; the length prefix keeps the encoding unambiguous.
            bytes.extend_from_slice(&(token.len() as u64).to_le_bytes());
            bytes.extend_from_slice(token.as_bytes());
        }
        let size = canonical_f64_bytes(self.weld.size_mm).map_err(|_| {
            CommandError::new(
                ErrorCode::NonFiniteNumber,
                "size_mm",
                "weld size must be finite",
            )
            .with_rule("DV-ATTR-003")
        })?;
        bytes.extend_from_slice(&size);
        if let Some(geometry) = &self.geometry {
            // A second domain keeps every A06 request (geometry absent) at its exact v1 bytes.
            bytes.extend_from_slice(b"BeamCraft/A15/CreateWeldGeometry/v1\0");
            let count = u64::try_from(geometry.path_mm.len()).map_err(|_| {
                CommandError::new(
                    ErrorCode::LimitExceeded,
                    "path_mm",
                    "the weld path is too long",
                )
            })?;
            bytes.extend_from_slice(&count.to_le_bytes());
            for point in &geometry.path_mm {
                for coordinate in point {
                    bytes.extend_from_slice(&canonical_f64_bytes(*coordinate).map_err(|_| {
                        CommandError::new(
                            ErrorCode::NonFiniteNumber,
                            "path_mm",
                            "a weld path point must be finite",
                        )
                        .with_rule("DV-ATTR-003")
                    })?);
                }
            }
            bytes.extend_from_slice(&[u8::from(geometry.symbol.all_around)]);
            let contour = geometry.symbol.contour.as_token();
            bytes.extend_from_slice(&(contour.len() as u64).to_le_bytes());
            bytes.extend_from_slice(contour.as_bytes());
        }
        Sha256::digest(&bytes).map_err(|_| {
            CommandError::new(ErrorCode::LimitExceeded, "content", "content is too large")
                .with_rule("DV-LIM-001")
        })
    }
}

/// The authored properties of one bolt group (A05). The group is `count` bolts in a straight
/// world line `origin + direction * (pitch * i)`, `i` in `0..count`. No solid geometry is derived
/// from them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoltSpec {
    /// Nominal bolt diameter in millimetres: finite, greater than 0 and at most
    /// [`BOLT_DIAMETER_CAP_MM`].
    pub diameter_mm: f64,
    /// Bolts in the group, 1 to [`MAX_BOLT_GROUP_SIZE`].
    pub count: usize,
    /// Centre-to-centre distance along the line in millimetres: finite, greater than 0 and at
    /// most the dimension cap.
    pub pitch_mm: f64,
    /// World position of the first bolt in millimetres, each component within the coordinate cap.
    pub origin_mm: [f64; 3],
    /// Unit world direction of the line: finite, squared norm within
    /// [`ORIENTATION_NORM_TOLERANCE`] of one. Stored normalized; builders pass a unit vector.
    pub direction_mm: [f64; 3],
}

/// A05 `CreateBolt`: a new `connection` element (domain vocabulary section 2.3) that is a bolt
/// group. Its one `connects` record (section 5.1) names exactly two distinct parts in order, the
/// main part first; the record lives with the group, so the parts are only read. It carries the
/// common base and the bolt properties; it has no body, material or derived geometry, and its
/// placement is the identity in this slice.
#[derive(Debug, Clone, PartialEq)]
pub struct CreateBolt {
    /// Identity of the new bolt group.
    pub new_id: ElementId,
    /// Explicit user label.
    pub name: String,
    /// The main part: a plate, beam or column.
    pub main_id: ElementId,
    /// The part bolted to it: a plate, beam or column.
    pub secondary_id: ElementId,
    /// The bolt properties.
    pub bolt: BoltSpec,
}

impl CreateBolt {
    /// Admits the request: the label (`DV-BASE-003`), two distinct parts (`DV-REL-006`,
    /// `DV-REL-007`), a group that is not one of its own parts, a finite diameter in
    /// `(0, BOLT_DIAMETER_CAP_MM]`, a count of 1 to [`MAX_BOLT_GROUP_SIZE`], a finite pitch in
    /// `(0, dimension cap]`, a finite origin within the coordinate cap and a finite unit
    /// direction. Whether the parts exist, their kinds and whether every bolt position lies on
    /// both parts are the engine's decisions.
    pub fn validate_arguments(&self) -> Result<(), CommandError> {
        if !label_is_valid(&self.name) {
            return Err(CommandError::new(
                ErrorCode::InvalidArgument,
                "name",
                "label must be 1 to 256 bytes without control characters",
            )
            .with_rule("DV-BASE-003"));
        }
        if self.main_id == self.secondary_id {
            return Err(CommandError::new(
                ErrorCode::InvalidArgument,
                "secondary_id",
                "a bolt group connects two distinct parts",
            )
            .with_rule("DV-REL-007"));
        }
        if self.new_id == self.main_id || self.new_id == self.secondary_id {
            return Err(CommandError::new(
                ErrorCode::InvalidArgument,
                "new_id",
                "a bolt group cannot connect itself",
            )
            .with_rule("DV-REL-006"));
        }
        let bolt = &self.bolt;
        let scalars = [bolt.diameter_mm, bolt.pitch_mm];
        if scalars.iter().any(|v| !v.is_finite()) || bolt.origin_mm.iter().any(|v| !v.is_finite()) {
            return Err(CommandError::new(
                ErrorCode::NonFiniteNumber,
                "bolt",
                "bolt diameter, pitch or origin is not finite",
            )
            .with_rule("DV-ATTR-003"));
        }
        if bolt.direction_mm.iter().any(|v| !v.is_finite()) {
            return Err(CommandError::new(
                ErrorCode::NonFiniteNumber,
                "direction_mm",
                "direction is not finite",
            )
            .with_rule("DV-ATTR-003"));
        }
        if bolt.diameter_mm <= 0.0 || bolt.diameter_mm > BOLT_DIAMETER_CAP_MM {
            return Err(CommandError::new(
                ErrorCode::InvalidArgument,
                "diameter_mm",
                "bolt diameter must be greater than 0 and at most 1000 mm",
            ));
        }
        if bolt.count == 0 {
            return Err(CommandError::new(
                ErrorCode::InvalidArgument,
                "count",
                "a bolt group has at least one bolt",
            ));
        }
        if bolt.count > MAX_BOLT_GROUP_SIZE {
            return Err(CommandError::new(
                ErrorCode::LimitExceeded,
                "count",
                "a bolt group has more than 512 bolts",
            )
            .with_rule("DV-LIM-001"));
        }
        if bolt.pitch_mm <= 0.0 || bolt.pitch_mm > DIMENSION_CAP_MM {
            return Err(CommandError::new(
                ErrorCode::InvalidArgument,
                "pitch_mm",
                "bolt pitch must be greater than 0 and at most the dimension cap",
            ));
        }
        if bolt.origin_mm.iter().any(|v| v.abs() > COORDINATE_CAP_MM) {
            return Err(CommandError::new(
                ErrorCode::LimitExceeded,
                "origin_mm",
                "origin component exceeds the coordinate cap",
            )
            .with_rule("DV-LIM-001"));
        }
        let squared = bolt.direction_mm.iter().map(|v| v * v).sum::<f64>();
        if (squared - 1.0).abs() > ORIENTATION_NORM_TOLERANCE {
            return Err(CommandError::new(
                ErrorCode::InvalidArgument,
                "direction_mm",
                "direction must be a unit vector",
            ));
        }
        Ok(())
    }

    /// SHA-256 of a versioned, length-prefixed encoding of the validated request.
    pub fn content_fingerprint(&self) -> Result<ContentDigest, CommandError> {
        self.validate_arguments()?;
        let bad = || {
            CommandError::new(
                ErrorCode::NonFiniteNumber,
                "bolt",
                "a bolt property cannot be encoded",
            )
            .with_rule("DV-ATTR-003")
        };
        let mut bytes = Vec::with_capacity(160 + self.name.len());
        bytes.extend_from_slice(b"BeamCraft/A05/CreateBolt/v1\0");
        bytes.extend_from_slice(self.new_id.as_bytes());
        bytes.extend_from_slice(&(self.name.len() as u64).to_le_bytes());
        bytes.extend_from_slice(self.name.as_bytes());
        bytes.extend_from_slice(self.main_id.as_bytes());
        bytes.extend_from_slice(self.secondary_id.as_bytes());
        for v in [self.bolt.diameter_mm, self.bolt.pitch_mm] {
            bytes.extend_from_slice(&canonical_f64_bytes(v).map_err(|_| bad())?);
        }
        bytes.extend_from_slice(&(self.bolt.count as u64).to_le_bytes());
        for v in self
            .bolt
            .origin_mm
            .iter()
            .copied()
            .chain(self.bolt.direction_mm)
        {
            bytes.extend_from_slice(&canonical_f64_bytes(v).map_err(|_| bad())?);
        }
        Sha256::digest(&bytes).map_err(|_| {
            CommandError::new(ErrorCode::LimitExceeded, "content", "content is too large")
                .with_rule("DV-LIM-001")
        })
    }
}

/// The spacing pattern of an A14.3 bolt group (bolt amendment section 4): an explicit list of
/// distances, or a generator kept for editing and expanded to the authoritative list. The
/// pattern travels with the group, so a moved `point2` re-expands an `offset_end` pattern.
#[derive(Debug, Clone, PartialEq)]
pub enum BoltSpacing {
    /// The distances between neighbouring fasteners; authoritative as authored.
    Spacings(Vec<f64>),
    /// `count` fasteners at one equal pitch.
    Pitch {
        /// Fasteners in the group, 1 to [`MAX_BOLT_GROUP_SIZE`].
        count: usize,
        /// The one spacing in millimetres.
        pitch_mm: f64,
    },
    /// `count` fasteners equally spaced so the last lies `offset_end_mm` before `point2`.
    OffsetEnd {
        /// Fasteners in the group, 2 to [`MAX_BOLT_GROUP_SIZE`].
        count: usize,
        /// Distance from the last fastener to `point2` in millimetres.
        offset_end_mm: f64,
    },
}

/// The length choice of an A14.3 bolt group (bolt amendment section 4). `auto` picks the
/// smallest standard length that covers the grip, the far washers, the nut and the thread
/// protrusion, so every fastener of the group is identical.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BoltLength {
    /// Pick the smallest standard length that covers the joint.
    Auto,
    /// One exact value from the definition's list of standard lengths.
    Fixed(f64),
}

/// The authored inputs of an A14.3 bolt group (bolt amendment section 4): two points, a
/// reference plane with rotation, the position values, offsets and spacings over a pinned
/// fastener definition. The group is one straight row of identical fasteners; the bearing face
/// of the first lies at `point1 + on_plane * y + depth * z` and the shank runs along `+z`.
#[derive(Debug, Clone, PartialEq)]
pub struct BoltGroupInputs {
    /// Connected parts: 2 to [`MAX_RELATIONSHIP_TARGETS`] distinct plates, beams or columns,
    /// the main part first. Every part must be crossed by the axis of at least one fastener.
    pub parts: Vec<ElementId>,
    /// Content digest of the pinned fastener definition the group places.
    pub fastener_digest: ContentDigest,
    /// The complete definition the digest pins, re-admitted by the engine.
    pub fastener: FastenerSpec,
    /// World start of the row in millimetres.
    pub point1_mm: [f64; 3],
    /// World end of the row; distinct from `point1_mm`.
    pub point2_mm: [f64; 3],
    /// Unit normal of the reference plane: its component perpendicular to the row is the shank
    /// axis at zero rotation; parallel to the row is refused.
    pub plane_normal_mm: [f64; 3],
    /// Rotation about the row in radians, turning the plane axes together.
    pub rotation_rad: f64,
    /// Offset of the whole group along the rotated plane axis `y`, in millimetres.
    pub on_plane_mm: f64,
    /// Offset of the whole group along the shank axis `z`, in millimetres.
    pub depth_mm: f64,
    /// Distance along the row from `point1` to the first fastener, in millimetres.
    pub offset_start_mm: f64,
    /// The spacings or the generator that expands to them.
    pub spacing: BoltSpacing,
    /// Washer under the head; `None` takes the definition's preference.
    pub washer_under_head: Option<bool>,
    /// Washer under the nut; `None` takes the definition's preference.
    pub washer_under_nut: Option<bool>,
    /// The length choice, resolved once for the whole group.
    pub length: BoltLength,
    /// Whether the group cuts holes in the parts it passes through (bolt amendment section 5);
    /// `None` takes the definition's own `hole_mode`.
    pub hole_mode: Option<crate::steel::fastener::HoleMode>,
    /// The clearance the generated holes use beyond the nominal diameter, in millimetres;
    /// `None` takes the definition's `hole_clearance_mm`.
    pub hole_clearance_mm: Option<f64>,
}

impl BoltGroupInputs {
    /// The hole mode the group follows: the command's own setting, or the definition's
    /// preference (bolt amendment section 5: `generate` is a bolt's default, `none` a screw's).
    pub fn resolved_hole_mode(&self) -> crate::steel::fastener::HoleMode {
        use crate::steel::fastener::{FastenerKind, HoleMode};
        self.hole_mode.unwrap_or(match self.fastener.kind {
            FastenerKind::Bolt => HoleMode::Generate,
            FastenerKind::Screw => HoleMode::None,
        })
    }

    /// The clearance the generated holes use beyond the nominal diameter: the group's own
    /// value, or the definition's.
    pub fn resolved_hole_clearance_mm(&self) -> f64 {
        self.hole_clearance_mm
            .unwrap_or(self.fastener.hole_clearance_mm)
    }

    /// Admits the request: the definition re-admitted (`F04` codes), 2 to
    /// [`MAX_RELATIONSHIP_TARGETS`] distinct parts, two distinct points within the coordinate
    /// cap, a finite unit plane normal, finite position values and spacings of up to
    /// [`MAX_BOLT_GROUP_SIZE`] fasteners, and a fixed length the definition lists. Whether the
    /// parts exist, are supported and are crossed, and which standard length `auto` picks, are
    /// the engine's decisions.
    pub fn validate_arguments(&self) -> Result<(), CommandError> {
        self.fastener.validate()?;
        if self.parts.len() < 2 || self.parts.len() > MAX_RELATIONSHIP_TARGETS {
            return Err(CommandError::new(
                ErrorCode::InvalidArgument,
                "parts",
                "a bolt group connects 2 to 1024 parts",
            )
            .with_rule("DV-REL-004"));
        }
        for (index, part) in self.parts.iter().enumerate() {
            if self.parts.iter().skip(index + 1).any(|other| other == part) {
                return Err(CommandError::new(
                    ErrorCode::InvalidArgument,
                    "parts",
                    "a bolt group connects distinct parts",
                )
                .with_rule("DV-REL-007"));
            }
        }
        for (point, field) in [
            (&self.point1_mm, "point1_mm"),
            (&self.point2_mm, "point2_mm"),
        ] {
            if point.iter().any(|v| !v.is_finite()) {
                return Err(CommandError::new(
                    ErrorCode::NonFiniteNumber,
                    field,
                    "the point is not finite",
                )
                .with_rule("DV-ATTR-003"));
            }
            if point.iter().any(|v| v.abs() > COORDINATE_CAP_MM) {
                return Err(CommandError::new(
                    ErrorCode::LimitExceeded,
                    field,
                    "the point exceeds the coordinate cap",
                )
                .with_rule("DV-LIM-001"));
            }
        }
        let apart = self
            .point2_mm
            .iter()
            .zip(&self.point1_mm)
            .map(|(b, a)| {
                let d = b - a;
                d * d
            })
            .sum::<f64>();
        if apart < ABSOLUTE_FLOOR_MM * ABSOLUTE_FLOOR_MM {
            return Err(CommandError::new(
                ErrorCode::InvalidArgument,
                "point2_mm",
                "the two points of the row are distinct",
            ));
        }
        if self.plane_normal_mm.iter().any(|v| !v.is_finite()) {
            return Err(CommandError::new(
                ErrorCode::NonFiniteNumber,
                "plane_normal_mm",
                "the plane normal is not finite",
            )
            .with_rule("DV-ATTR-003"));
        }
        let squared = self.plane_normal_mm.iter().map(|v| v * v).sum::<f64>();
        if (squared - 1.0).abs() > ORIENTATION_NORM_TOLERANCE {
            return Err(CommandError::new(
                ErrorCode::InvalidArgument,
                "plane_normal_mm",
                "the plane normal must be a unit vector",
            ));
        }
        if !self.rotation_rad.is_finite() {
            return Err(CommandError::new(
                ErrorCode::NonFiniteNumber,
                "rotation_rad",
                "the rotation is not finite",
            )
            .with_rule("DV-ATTR-003"));
        }
        for (value, field) in [
            (&self.on_plane_mm, "on_plane_mm"),
            (&self.depth_mm, "depth_mm"),
        ] {
            if !value.is_finite() {
                return Err(CommandError::new(
                    ErrorCode::NonFiniteNumber,
                    field,
                    "the offset is not finite",
                )
                .with_rule("DV-ATTR-003"));
            }
            if value.abs() > DIMENSION_CAP_MM {
                return Err(CommandError::new(
                    ErrorCode::LimitExceeded,
                    field,
                    "the offset exceeds the dimension cap",
                )
                .with_rule("DV-LIM-001"));
            }
        }
        if !self.offset_start_mm.is_finite() {
            return Err(CommandError::new(
                ErrorCode::NonFiniteNumber,
                "offset_start_mm",
                "the offset is not finite",
            )
            .with_rule("DV-ATTR-003"));
        }
        if !(0.0..=DIMENSION_CAP_MM).contains(&self.offset_start_mm) {
            return Err(CommandError::new(
                ErrorCode::InvalidArgument,
                "offset_start_mm",
                "the offset start is a distance from point1",
            ));
        }
        match &self.spacing {
            BoltSpacing::Spacings(spacings) => {
                if spacings.is_empty() || spacings.len() >= MAX_BOLT_GROUP_SIZE {
                    return Err(CommandError::new(
                        ErrorCode::InvalidArgument,
                        "spacings_mm",
                        "the spacings give 2 to 512 fasteners",
                    ));
                }
                for spacing in spacings {
                    if !spacing.is_finite() {
                        return Err(CommandError::new(
                            ErrorCode::NonFiniteNumber,
                            "spacings_mm",
                            "a spacing is not finite",
                        )
                        .with_rule("DV-ATTR-003"));
                    }
                    if *spacing <= 0.0 || *spacing > DIMENSION_CAP_MM {
                        return Err(CommandError::new(
                            ErrorCode::InvalidArgument,
                            "spacings_mm",
                            "a spacing is positive and within the dimension cap",
                        ));
                    }
                }
            }
            BoltSpacing::Pitch { count, pitch_mm } => {
                if *count == 0 || *count > MAX_BOLT_GROUP_SIZE {
                    return Err(CommandError::new(
                        ErrorCode::InvalidArgument,
                        "count",
                        "the group has 1 to 512 fasteners",
                    ));
                }
                if !pitch_mm.is_finite() {
                    return Err(CommandError::new(
                        ErrorCode::NonFiniteNumber,
                        "pitch_mm",
                        "the pitch is not finite",
                    )
                    .with_rule("DV-ATTR-003"));
                }
                if *pitch_mm <= 0.0 || *pitch_mm > DIMENSION_CAP_MM {
                    return Err(CommandError::new(
                        ErrorCode::InvalidArgument,
                        "pitch_mm",
                        "the pitch is positive and within the dimension cap",
                    ));
                }
            }
            BoltSpacing::OffsetEnd {
                count,
                offset_end_mm,
            } => {
                if *count < 2 || *count > MAX_BOLT_GROUP_SIZE {
                    return Err(CommandError::new(
                        ErrorCode::InvalidArgument,
                        "count",
                        "an offset-end pattern has 2 to 512 fasteners",
                    ));
                }
                if !offset_end_mm.is_finite() {
                    return Err(CommandError::new(
                        ErrorCode::NonFiniteNumber,
                        "offset_end_mm",
                        "the offset end is not finite",
                    )
                    .with_rule("DV-ATTR-003"));
                }
                if *offset_end_mm < 0.0 || *offset_end_mm > DIMENSION_CAP_MM {
                    return Err(CommandError::new(
                        ErrorCode::InvalidArgument,
                        "offset_end_mm",
                        "the offset end is a distance from point2",
                    ));
                }
            }
        }
        if let BoltLength::Fixed(length) = self.length {
            if !length.is_finite() {
                return Err(CommandError::new(
                    ErrorCode::NonFiniteNumber,
                    "length_mm",
                    "the fixed length is not finite",
                )
                .with_rule("DV-ATTR-003"));
            }
            if !self.fastener.lengths_mm.contains(&length) {
                return Err(CommandError::new(
                    ErrorCode::InvalidArgument,
                    "length_mm",
                    "the fixed length is one of the definition's standard lengths",
                ));
            }
        }
        if let Some(clearance) = self.hole_clearance_mm {
            if !clearance.is_finite() {
                return Err(CommandError::new(
                    ErrorCode::NonFiniteNumber,
                    "hole_clearance_mm",
                    "value is not finite",
                ));
            }
            if !(0.0..=DIMENSION_CAP_MM).contains(&clearance) {
                return Err(CommandError::new(
                    ErrorCode::InvalidArgument,
                    "hole_clearance_mm",
                    "hole clearance is negative or above the dimension cap",
                ));
            }
        }
        Ok(())
    }

    /// SHA-256 of a versioned, length-prefixed encoding of the validated request. The definition
    /// travels by its digest: content-addressed pinning makes equal digests equal definitions.
    pub fn content_fingerprint(&self) -> Result<ContentDigest, CommandError> {
        self.validate_arguments()?;
        let bad = || {
            CommandError::new(
                ErrorCode::NonFiniteNumber,
                "placement",
                "a placement value cannot be encoded",
            )
            .with_rule("DV-ATTR-003")
        };
        let mut bytes = Vec::with_capacity(320 + 32 * self.parts.len());
        bytes.extend_from_slice(b"BeamCraft/A14/BoltGroupInputs/v2\0");
        bytes.extend_from_slice(&(self.parts.len() as u64).to_le_bytes());
        for part in &self.parts {
            bytes.extend_from_slice(part.as_bytes());
        }
        bytes.extend_from_slice(self.fastener_digest.as_bytes());
        for point in [self.point1_mm, self.point2_mm, self.plane_normal_mm] {
            for v in point {
                bytes.extend_from_slice(&canonical_f64_bytes(v).map_err(|_| bad())?);
            }
        }
        for v in [
            self.rotation_rad,
            self.on_plane_mm,
            self.depth_mm,
            self.offset_start_mm,
        ] {
            bytes.extend_from_slice(&canonical_f64_bytes(v).map_err(|_| bad())?);
        }
        match &self.spacing {
            BoltSpacing::Spacings(spacings) => {
                bytes.extend_from_slice(&[0]);
                bytes.extend_from_slice(&(spacings.len() as u64).to_le_bytes());
                for spacing in spacings {
                    bytes.extend_from_slice(&canonical_f64_bytes(*spacing).map_err(|_| bad())?);
                }
            }
            BoltSpacing::Pitch { count, pitch_mm } => {
                bytes.extend_from_slice(&[1]);
                bytes.extend_from_slice(&(*count as u64).to_le_bytes());
                bytes.extend_from_slice(&canonical_f64_bytes(*pitch_mm).map_err(|_| bad())?);
            }
            BoltSpacing::OffsetEnd {
                count,
                offset_end_mm,
            } => {
                bytes.extend_from_slice(&[2]);
                bytes.extend_from_slice(&(*count as u64).to_le_bytes());
                bytes.extend_from_slice(&canonical_f64_bytes(*offset_end_mm).map_err(|_| bad())?);
            }
        }
        for washer in [self.washer_under_head, self.washer_under_nut] {
            bytes.extend_from_slice(&[match washer {
                None => 0,
                Some(false) => 1,
                Some(true) => 2,
            }]);
        }
        match self.length {
            BoltLength::Auto => bytes.extend_from_slice(&[0]),
            BoltLength::Fixed(length) => {
                bytes.extend_from_slice(&[1]);
                bytes.extend_from_slice(&canonical_f64_bytes(length).map_err(|_| bad())?);
            }
        }
        bytes.extend_from_slice(&[match self.hole_mode {
            None => 0,
            Some(mode) => 1 + mode as u8,
        }]);
        match self.hole_clearance_mm {
            None => bytes.extend_from_slice(&[0]),
            Some(clearance) => {
                bytes.extend_from_slice(&[1]);
                bytes.extend_from_slice(&canonical_f64_bytes(clearance).map_err(|_| bad())?);
            }
        }
        Sha256::digest(&bytes).map_err(|_| {
            CommandError::new(ErrorCode::LimitExceeded, "content", "content is too large")
                .with_rule("DV-LIM-001")
        })
    }
}

/// A14.3 `CreateBoltGroup`: a new `connection` element that places one row of identical
/// fasteners of a pinned definition between 2 to 1024 parts (bolt amendment section 6). With
/// `hole_mode = generate` the group also writes one through-hole operation record per crossing
/// fastener on every connected plate (A14.4); members refuse until the G14 contract lands.
#[derive(Debug, Clone, PartialEq)]
pub struct CreateBoltGroup {
    /// Identity of the new bolt group.
    pub new_id: ElementId,
    /// Explicit user label.
    pub name: String,
    /// The complete placement inputs.
    pub inputs: BoltGroupInputs,
}

impl CreateBoltGroup {
    /// Admits the request: the label (`DV-BASE-003`), the inputs and a group that is not one of
    /// its own parts (`DV-REL-006`).
    pub fn validate_arguments(&self) -> Result<(), CommandError> {
        if !label_is_valid(&self.name) {
            return Err(CommandError::new(
                ErrorCode::InvalidArgument,
                "name",
                "label must be 1 to 256 bytes without control characters",
            )
            .with_rule("DV-BASE-003"));
        }
        self.inputs.validate_arguments()?;
        if self.inputs.parts.contains(&self.new_id) {
            return Err(CommandError::new(
                ErrorCode::InvalidArgument,
                "new_id",
                "a bolt group cannot connect itself",
            )
            .with_rule("DV-REL-006"));
        }
        Ok(())
    }

    /// SHA-256 of a versioned, length-prefixed encoding of the validated request.
    pub fn content_fingerprint(&self) -> Result<ContentDigest, CommandError> {
        self.validate_arguments()?;
        let mut bytes = Vec::with_capacity(96 + self.name.len());
        bytes.extend_from_slice(b"BeamCraft/A14/CreateBoltGroup/v1\0");
        bytes.extend_from_slice(self.new_id.as_bytes());
        bytes.extend_from_slice(&(self.name.len() as u64).to_le_bytes());
        bytes.extend_from_slice(self.name.as_bytes());
        bytes.extend_from_slice(self.inputs.content_fingerprint()?.as_bytes());
        Sha256::digest(&bytes).map_err(|_| {
            CommandError::new(ErrorCode::LimitExceeded, "content", "content is too large")
                .with_rule("DV-LIM-001")
        })
    }
}

/// A14.3 `UpdateBoltGroup`: one atomic edit of an existing group (bolt amendment section 6).
/// The caller sends the complete new inputs — an edit of one value sends the others unchanged,
/// and a `repin` sends the digest and definition of the current library entry. A stored A05
/// group is upgraded by supplying the placement inputs and a fastener.
#[derive(Debug, Clone, PartialEq)]
pub struct UpdateBoltGroup {
    /// Identity of the edited group.
    pub target_id: ElementId,
    /// The complete new placement inputs.
    pub inputs: BoltGroupInputs,
}

impl UpdateBoltGroup {
    /// Admits the request: the inputs and a group that is not one of its own parts
    /// (`DV-REL-006`).
    pub fn validate_arguments(&self) -> Result<(), CommandError> {
        self.inputs.validate_arguments()?;
        if self.inputs.parts.contains(&self.target_id) {
            return Err(CommandError::new(
                ErrorCode::InvalidArgument,
                "target_id",
                "a bolt group cannot connect itself",
            )
            .with_rule("DV-REL-006"));
        }
        Ok(())
    }

    /// SHA-256 of a versioned, length-prefixed encoding of the validated request.
    pub fn content_fingerprint(&self) -> Result<ContentDigest, CommandError> {
        self.validate_arguments()?;
        let mut bytes = Vec::with_capacity(64);
        bytes.extend_from_slice(b"BeamCraft/A14/UpdateBoltGroup/v1\0");
        bytes.extend_from_slice(self.target_id.as_bytes());
        bytes.extend_from_slice(self.inputs.content_fingerprint()?.as_bytes());
        Sha256::digest(&bytes).map_err(|_| {
            CommandError::new(ErrorCode::LimitExceeded, "content", "content is too large")
                .with_rule("DV-LIM-001")
        })
    }
}
