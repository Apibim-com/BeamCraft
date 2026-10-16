// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! Typed failures and shell verdicts for arbitrary triangle meshes.
//!
//! These map onto the stable F06 `GEO_` vocabulary and the existing F04 codes
//! without editing [`crate::diagnostic`], which belongs to the plate builder.

use crate::diagnostic::Stage;
use beamcraft_model::error::ErrorCode;

/// Stable F06 section 3 geometry codes that a mesh check can produce.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeshCode {
    /// NaN or infinity in a submitted coordinate.
    NonFinite,
    /// A coordinate or the mesh size above its cap.
    AboveLimit,
    /// Vertex, triangle or byte cap exceeded.
    BufferLimit,
    /// Face-table or shell cap exceeded.
    TopologyLimit,
    /// Blob structure or index disagrees with its header.
    BlobInvalid,
    /// Open, non-manifold or inconsistently oriented shell: no exact status.
    NotClosed,
    /// Closed but wound inward.
    Inward,
    /// No certificate of a valid, non-degenerate, self-intersection-free shell.
    NotCertified,
    /// A numeric certificate failed.
    NumericUncertain,
    /// Cancellation observed before the result was sealed.
    Cancelled,
}

impl MeshCode {
    /// Stable F06 wire token.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NonFinite => "GEO_NON_FINITE",
            Self::AboveLimit => "GEO_ABOVE_LIMIT",
            Self::BufferLimit => "GEO_BUFFER_LIMIT",
            Self::TopologyLimit => "GEO_TOPOLOGY_LIMIT",
            Self::BlobInvalid => "GEO_BLOB_INVALID",
            Self::NotClosed => "GEO_MESH_NOT_CLOSED",
            Self::Inward => "GEO_MESH_INWARD",
            Self::NotCertified => "GEO_MESH_NOT_CERTIFIED",
            Self::NumericUncertain => "GEO_NUMERIC_UNCERTAIN",
            Self::Cancelled => "GEO_CANCELLED",
        }
    }

    /// F06 section 3 mapping onto an existing F04 code.
    pub const fn f04_code(self) -> ErrorCode {
        match self {
            Self::NonFinite => ErrorCode::NonFiniteNumber,
            Self::AboveLimit | Self::BufferLimit | Self::TopologyLimit => ErrorCode::LimitExceeded,
            Self::BlobInvalid => ErrorCode::IntegrityError,
            Self::NotClosed | Self::Inward | Self::NotCertified | Self::NumericUncertain => {
                ErrorCode::UnsupportedGeometry
            }
            Self::Cancelled => ErrorCode::Cancelled,
        }
    }

    /// F06 overall status string for a call that failed with this code.
    pub const fn status(self) -> &'static str {
        match self {
            Self::AboveLimit | Self::BufferLimit | Self::TopologyLimit => "limit_exceeded",
            Self::Cancelled => "cancelled",
            Self::NonFinite | Self::BlobInvalid => "invalid_input",
            Self::NotClosed | Self::Inward | Self::NotCertified | Self::NumericUncertain => {
                "unsupported"
            }
        }
    }
}

/// Fixed schema token of the failing part; never contains input text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fault {
    /// Blob header fields.
    Header,
    /// Vertex count or table.
    Vertices,
    /// Triangle count or table.
    Triangles,
    /// Face-table entry count.
    FaceTable,
    /// Total byte length.
    Bytes,
    /// Coordinate values.
    Coordinates,
    /// Triangle vertex indices.
    Indices,
    /// Per-triangle face indices or padding.
    Faces,
    /// Shell count.
    Shells,
    /// Derived numeric certificate.
    Numeric,
    /// An analysis presented with a mesh it was not computed from.
    Analysis,
    /// Host cancellation request.
    Cancellation,
}

impl Fault {
    /// Fixed ASCII token, shorter than F06's 64-byte bound.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Header => "mesh.header",
            Self::Vertices => "mesh.vertices",
            Self::Triangles => "mesh.triangles",
            Self::FaceTable => "mesh.face_table",
            Self::Bytes => "mesh.bytes",
            Self::Coordinates => "mesh.coordinates",
            Self::Indices => "mesh.indices",
            Self::Faces => "mesh.faces",
            Self::Shells => "mesh.shells",
            Self::Numeric => "mesh.numeric",
            Self::Analysis => "mesh.analysis",
            Self::Cancellation => "cancellation",
        }
    }
}

/// A rejected mesh input or a call that produced no result.
///
/// Carries no geometry, value or partial result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MeshError {
    /// Stable geometry code; the F04 mapping is [`MeshCode::f04_code`].
    pub code: MeshCode,
    /// Which part of the input failed.
    pub fault: Fault,
    /// Evaluation stage that observed the failure.
    pub stage: Stage,
}

impl MeshError {
    pub(crate) const fn new(code: MeshCode, fault: Fault, stage: Stage) -> Self {
        Self { code, fault, stage }
    }

    pub(crate) const fn cancelled(stage: Stage) -> Self {
        Self::new(MeshCode::Cancelled, Fault::Cancellation, stage)
    }

    pub(crate) const fn numeric(stage: Stage) -> Self {
        Self::new(MeshCode::NumericUncertain, Fault::Numeric, stage)
    }

    /// F06 overall status string.
    pub const fn status(&self) -> &'static str {
        self.code.status()
    }
}

impl core::fmt::Display for MeshError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("mesh check failed")
    }
}

impl core::error::Error for MeshError {}

/// Why a verdict could not be certified either way.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Uncertainty {
    /// More overlapping triangle-box pairs than the explicit pair cap.
    PairBudget,
    /// The spatial index would exceed its explicit node or item cap.
    IndexBudget,
    /// More exact predicate fallbacks than the explicit budget allows.
    PredicateBudget,
    /// A value left the range the exact or enclosure arithmetic supports.
    NumericRange,
    /// Every probe direction for a shell-containment test was degenerate.
    Containment,
}

/// Verdict on a triangle shell, in the documented precedence order.
///
/// Only [`ShellStatus::Certified`] allows mesh measurements. Every other
/// value is an explicit refusal to claim more than was proved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellStatus {
    /// Closed, 2-manifold, consistently oriented, outward, non-degenerate and
    /// free of certified self-intersection, nesting and shell contact.
    Certified,
    /// An undirected edge belongs to exactly one triangle.
    Open,
    /// An edge with three or more triangles, a duplicate triangle or a
    /// vertex whose triangles do not form one fan.
    NonManifold,
    /// An interior edge is traversed twice in the same direction.
    InconsistentOrientation,
    /// A closed shell whose signed volume is not positive.
    Inward,
    /// A collapsed, zero-area or sub-`eps_geom` triangle, or an unused vertex.
    Degenerate,
    /// Two triangles meet or cross beyond their shared edge or vertex, or one
    /// shell lies inside another.
    Overlapping,
    /// The check could not decide; never treated as exact.
    Uncertain(Uncertainty),
}

impl ShellStatus {
    /// Returns true only for a certified shell.
    pub const fn is_certified(self) -> bool {
        matches!(self, Self::Certified)
    }

    /// Stable `GEO_` code for a refusal, `None` for a certified shell.
    pub const fn code(self) -> Option<MeshCode> {
        match self {
            Self::Certified => None,
            Self::Open | Self::NonManifold | Self::InconsistentOrientation => {
                Some(MeshCode::NotClosed)
            }
            Self::Inward => Some(MeshCode::Inward),
            Self::Degenerate | Self::Overlapping => Some(MeshCode::NotCertified),
            Self::Uncertain(Uncertainty::NumericRange) => Some(MeshCode::NumericUncertain),
            Self::Uncertain(_) => Some(MeshCode::NotCertified),
        }
    }

    /// Fixed lowercase token for logs and tests.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Certified => "certified",
            Self::Open => "open",
            Self::NonManifold => "non_manifold",
            Self::InconsistentOrientation => "inconsistent_orientation",
            Self::Inward => "inward",
            Self::Degenerate => "degenerate",
            Self::Overlapping => "overlapping",
            Self::Uncertain(_) => "uncertain",
        }
    }
}
