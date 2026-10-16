// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! F06 section 4 geometric scale; independent of test comparison and picking.

use crate::error::{derived, finite};
use crate::{MathError, Point3, Vec3};

/// F06's absolute minimum length in mm; not a test comparison rule.
pub const ABSOLUTE_FLOOR_MM: f64 = 0.001;
/// Exactly representable local-size multiplier, 2^-22.
pub const LOCAL_SIZE_FACTOR: f64 = 1.0 / 4_194_304.0;
/// Maximum submitted coordinate or placement component in mm.
pub const COORDINATE_CAP_MM: f64 = 1_000_000_000.0;
/// Maximum dimension or member local size in mm.
pub const DIMENSION_CAP_MM: f64 = 1_000_000.0;
/// Maximum plate local size in mm.
pub const PLATE_SIZE_CAP_MM: f64 = 2_000_000.0;
/// Maximum contour count, checked before any traversal.
pub const CONTOUR_POINT_CAP: usize = 1024;
/// Inclusive roll cap in radians (F06's binary64 literal).
pub const ROLL_CAP_RAD: f64 = core::f64::consts::TAU;
/// Version of the implemented F06 geometric tolerance policy.
pub const TOLERANCE_POLICY_VERSION: u16 = 3;

/// Validated local scale and geometric floor; placement never enters this type.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tolerance {
    local_size_mm: f64,
    eps_geom_mm: f64,
}

impl Tolerance {
    /// Derives plate D in ascending point order, after count/finiteness/coordinate caps.
    ///
    /// This is only scale derivation, not contour admission, winding or topology validation.
    pub fn for_plate(points: &[Point3]) -> Result<Self, MathError> {
        if points.len() > CONTOUR_POINT_CAP {
            return Err(MathError::AboveLimit);
        }
        let Some(origin) = points.first().copied() else {
            return Err(MathError::InvalidPointCount);
        };
        if points.len() < 3 {
            return Err(MathError::InvalidPointCount);
        }
        for p in points {
            p.validated()?;
        }
        for p in points {
            check_point_cap(*p)?;
        }
        let mut size = 0.0;
        for p in points {
            let distance = origin.checked_distance_to(*p)?;
            if distance > size {
                size = distance;
            }
        }
        Self::from_derived_size(size, PLATE_SIZE_CAP_MM)
    }

    /// Derives member ell, H and L, then validates axis/section minima.
    ///
    /// Nonpositive section dimensions contribute zero to H and are rejected only
    /// after scale derivation, as F06 requires. No floor substitutes for a failed norm.
    pub fn for_member(
        start: Point3,
        end: Point3,
        width: f64,
        height: f64,
    ) -> Result<MemberScale, MathError> {
        start.validated()?;
        end.validated()?;
        finite(width)?;
        finite(height)?;
        check_point_cap(start)?;
        check_point_cap(end)?;
        check_dimension_cap(width)?;
        check_dimension_cap(height)?;
        let delta = start.vector_to(end)?;
        let ell = delta.norm()?;
        let hw = if width > 0.0 { width / 2.0 } else { 0.0 };
        let hh = if height > 0.0 { height / 2.0 } else { 0.0 };
        let half_diagonal = derived((hw * hw + hh * hh).sqrt())?;
        let size = if ell > half_diagonal {
            ell
        } else {
            half_diagonal
        };
        let tolerance = Self::from_derived_size(size, DIMENSION_CAP_MM)?;
        if ell < tolerance.eps_geom_mm {
            return Err(MathError::AxisTooShort);
        }
        tolerance.check_dimension(width)?;
        tolerance.check_dimension(height)?;
        Ok(MemberScale {
            delta,
            axis_length_mm: ell,
            section_half_diagonal_mm: half_diagonal,
            tolerance,
        })
    }

    /// Returns element-local body size L in mm.
    pub const fn local_size_mm(self) -> f64 {
        self.local_size_mm
    }
    /// Returns max(binary64(0.001), 2^-22 * L) in mm.
    pub const fn eps_geom_mm(self) -> f64 {
        self.eps_geom_mm
    }
    /// Computes eps_geom + binary64(1e-6)*D in the written operation order.
    pub fn plane_band_mm(self) -> f64 {
        let scaled_relative = 1e-6 * self.local_size_mm;
        self.eps_geom_mm + scaled_relative
    }
    /// Checks finiteness, upper cap, positivity and inclusive geometric minimum.
    pub fn check_dimension(self, value: f64) -> Result<(), MathError> {
        finite(value)?;
        check_dimension_cap(value)?;
        if value <= 0.0 {
            return Err(MathError::NotPositive);
        }
        if value < self.eps_geom_mm {
            return Err(MathError::BelowMinimum);
        }
        Ok(())
    }
    /// Tests strict point coincidence under this body's geometric floor.
    pub fn coincident(self, a: Point3, b: Point3) -> Result<bool, MathError> {
        Ok(a.checked_distance_to(b)? < self.eps_geom_mm)
    }

    pub(crate) fn from_derived_size(size: f64, cap: f64) -> Result<Self, MathError> {
        let size = derived(size)?;
        if size < 0.0 {
            return Err(MathError::NumericUncertain);
        }
        if size > cap {
            return Err(MathError::AboveLimit);
        }
        let scaled = derived(LOCAL_SIZE_FACTOR * size)?;
        let eps = if scaled > ABSOLUTE_FLOOR_MM {
            scaled
        } else {
            ABSOLUTE_FLOOR_MM
        };
        Ok(Self {
            local_size_mm: size,
            eps_geom_mm: eps,
        })
    }
}

/// Validated member scale and axis, shared with default-frame construction.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MemberScale {
    pub(crate) delta: Vec3,
    axis_length_mm: f64,
    section_half_diagonal_mm: f64,
    tolerance: Tolerance,
}

impl MemberScale {
    /// Returns the axis length ell in mm.
    pub const fn axis_length_mm(self) -> f64 {
        self.axis_length_mm
    }
    /// Returns the section half-diagonal H in mm.
    pub const fn section_half_diagonal_mm(self) -> f64 {
        self.section_half_diagonal_mm
    }
    /// Returns the body-scaled tolerance policy.
    pub const fn tolerance(self) -> Tolerance {
        self.tolerance
    }
}

pub(crate) fn check_point_cap(point: Point3) -> Result<(), MathError> {
    if point.x.abs() > COORDINATE_CAP_MM
        || point.y.abs() > COORDINATE_CAP_MM
        || point.z.abs() > COORDINATE_CAP_MM
    {
        Err(MathError::AboveLimit)
    } else {
        Ok(())
    }
}

pub(crate) fn check_dimension_cap(value: f64) -> Result<(), MathError> {
    if value > DIMENSION_CAP_MM {
        Err(MathError::AboveLimit)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn f06_neg_50_injected_nonfinite_derived_scale_has_no_floor() {
        for size in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert_eq!(
                Tolerance::from_derived_size(size, PLATE_SIZE_CAP_MM),
                Err(MathError::NumericUncertain)
            );
        }
    }
}
