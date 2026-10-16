// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! Typed failures for the checked math API; no model-layer dependency.

/// A rejected input or failed numeric certificate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MathError {
    /// A submitted scalar is NaN or infinite (F06 GEO_NON_FINITE).
    NonFinite,
    /// A submitted or derived size exceeds an F06 cap (GEO_ABOVE_LIMIT).
    AboveLimit,
    /// A dimension is zero or negative (GEO_NOT_POSITIVE).
    NotPositive,
    /// A positive dimension is below the geometric floor (GEO_BELOW_MINIMUM).
    BelowMinimum,
    /// A member axis is zero or shorter than its floor (GEO_AXIS_TOO_SHORT).
    AxisTooShort,
    /// A direction to normalize is exactly zero.
    ZeroAxis,
    /// Axes fail the absolute F06 frame certificate (GEO_NUMERIC_UNCERTAIN).
    InvalidFrame,
    /// Finite arithmetic could not produce a finite, usable result (GEO_NUMERIC_UNCERTAIN).
    NumericUncertain,
    /// A plate scale request has fewer than three contour points.
    InvalidPointCount,
}

impl core::fmt::Display for MathError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            Self::NonFinite => "non-finite input",
            Self::AboveLimit => "size exceeds its limit",
            Self::NotPositive => "dimension is not positive",
            Self::BelowMinimum => "dimension is below the geometric floor",
            Self::AxisTooShort => "member axis is too short",
            Self::ZeroAxis => "direction is zero",
            Self::InvalidFrame => "frame certificate failed",
            Self::NumericUncertain => "numeric result is uncertain",
            Self::InvalidPointCount => "at least three contour points are required",
        })
    }
}

impl core::error::Error for MathError {}

pub(crate) fn finite(value: f64) -> Result<f64, MathError> {
    if value.is_finite() {
        Ok(canonical_zero(value))
    } else {
        Err(MathError::NonFinite)
    }
}

pub(crate) fn derived(value: f64) -> Result<f64, MathError> {
    finite(value).map_err(|_| MathError::NumericUncertain)
}

pub(crate) const fn canonical_zero(value: f64) -> f64 {
    if value == 0.0 { 0.0 } else { value }
}
