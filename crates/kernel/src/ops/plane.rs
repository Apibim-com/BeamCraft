// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! The cutting plane and its exact side predicate.

use super::ExactBudget;
use super::diagnostic::{OpCode, OpDefect, OpFailure, OpField};
use crate::bigint::Big;
use crate::diagnostic::Stage;
use crate::enclosure::Enclosure;
use crate::exact::{min_exponent, scaled};
use crate::trimesh::MAX_COORDINATE_MM;

/// Unit roundoff `2^-53`.
const U: f64 = 1.110_223_024_625_156_5e-16;
/// Smallest permanent for which the filter is trusted.
const FILTER_FLOOR: f64 = 1e-200;
/// Margin over the rounding-error bound beyond which the binary64 value itself is also precise.
const PRECISE: f64 = 1_048_576.0;
/// Smallest largest-component magnitude of an accepted normal.
const MIN_NORMAL_COMPONENT: f64 = 1e-100;

/// A plane through `point` with (not necessarily unit) `normal`: the exact set of points `p` with
/// `normal . (p - point) = 0` for the submitted binary64 numbers.
///
/// Which side of it a vertex lies on is decided exactly; the normal is never normalised for
/// that decision, so a scaled normal describes the same plane.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CutPlane {
    point: [f64; 3],
    normal: [f64; 3],
}

impl CutPlane {
    /// Creates a plane without validating it; [`super::plane_cut`] validates finiteness, the
    /// coordinate cap (4e9 mm for every component), a non-zero normal and a normal whose
    /// largest component is at least 1e-100 (a smaller one is refused as `NumericRange`
    /// because its length could not be bounded; scale it, the plane is the same).
    pub const fn new(point: [f64; 3], normal: [f64; 3]) -> Self {
        Self { point, normal }
    }

    /// A point on the plane.
    pub const fn point(&self) -> [f64; 3] {
        self.point
    }

    /// The submitted normal; the kept side of [`super::Keep::Positive`] is where it points.
    pub const fn normal(&self) -> [f64; 3] {
        self.normal
    }

    pub(crate) fn validate(&self) -> Result<(), OpFailure> {
        let fail = |code, stage| OpFailure::new(code, OpField::Plane, OpDefect::None, stage);
        let all = || self.point.iter().chain(self.normal.iter());
        if all().any(|v| !v.is_finite()) {
            return Err(fail(OpCode::NonFinite, Stage::Finiteness));
        }
        if all().any(|v| v.abs() > MAX_COORDINATE_MM) {
            return Err(fail(OpCode::AboveLimit, Stage::Caps));
        }
        if self.normal.iter().all(|v| *v == 0.0) {
            return Err(fail(OpCode::NotPositive, Stage::Ranges));
        }
        // The distance bound squares the normal; below this magnitude that would underflow,
        // so such a normal is refused instead of being approximated (scale it: the plane is
        // the same).
        if self.normal.iter().all(|v| v.abs() < MIN_NORMAL_COMPONENT) {
            return Err(OpFailure::numeric(
                OpField::Plane,
                OpDefect::NumericRange,
                Stage::Ranges,
            ));
        }
        Ok(())
    }

    /// Dominant axis of the normal (largest magnitude, lowest index on ties).
    pub(crate) fn dominant_axis(&self) -> usize {
        let [x, y, z] = self.normal.map(f64::abs);
        if x >= y && x >= z {
            0
        } else if y >= z {
            1
        } else {
            2
        }
    }

    /// The exact sign of `normal . (p - point)` and a binary64 estimate of the value.
    ///
    /// The estimate carries the exact sign (never zero for a non-zero sign) and is only used to
    /// place a new vertex; it is the filtered value when that is precise, else the midpoint of
    /// an enclosure of the exact integer value.
    pub(crate) fn evaluate(
        &self,
        p: [f64; 3],
        budget: &mut ExactBudget,
    ) -> Result<(i8, f64), OpFailure> {
        let [n0, n1, n2] = self.normal;
        let [d0, d1, d2] = [
            p[0] - self.point[0],
            p[1] - self.point[1],
            p[2] - self.point[2],
        ];
        let terms = [n0 * d0, n1 * d1, n2 * d2];
        let value = terms[0] + terms[1] + terms[2];
        let permanent = terms.iter().map(|t| t.abs()).sum::<f64>();
        let bound = 8.0 * U * permanent;
        if permanent > FILTER_FLOOR && value.abs() > bound {
            let sign = if value > 0.0 { 1 } else { -1 };
            if value.abs() > PRECISE * bound {
                return Ok((sign, value));
            }
            budget.charge()?;
            return self.exact(p).map(|(s, v, _)| (s, v));
        }
        budget.charge()?;
        self.exact(p).map(|(s, v, _)| (s, v))
    }

    /// Exact evaluation: sign, estimate and the enclosure of the exact value.
    fn exact(&self, p: [f64; 3]) -> Result<(i8, f64, Enclosure), OpFailure> {
        let range = || OpFailure::numeric(OpField::Plane, OpDefect::NumericRange, Stage::Build);
        let e = min_exponent(
            p.iter()
                .chain(self.point.iter())
                .chain(self.normal.iter())
                .copied(),
        );
        let s = |x: f64| scaled(x, e).map_err(|_| range());
        let mut sum = Big::ZERO;
        for axis in 0..3 {
            let (n, q, o) = (
                self.normal.get(axis).copied().ok_or_else(range)?,
                p.get(axis).copied().ok_or_else(range)?,
                self.point.get(axis).copied().ok_or_else(range)?,
            );
            let diff = s(q)?.sub(&s(o)?).map_err(|_| range())?;
            sum = sum
                .add(&s(n)?.mul(&diff).map_err(|_| range())?)
                .map_err(|_| range())?;
        }
        let sign = sum.sign();
        let enclosure = sum
            .enclose_clamped(e.checked_mul(2).ok_or_else(range)?)
            .ok_or_else(range)?;
        let mid = enclosure.midpoint();
        let estimate = if sign > 0 && mid > 0.0 || sign < 0 && mid < 0.0 {
            mid
        } else {
            f64::from(sign) * f64::MIN_POSITIVE
        };
        Ok((sign, estimate, enclosure))
    }

    /// Upper bound of the distance from `p` to the plane, from the exact value and an
    /// enclosure of the normal's length.
    pub(crate) fn deviation_bound_mm(&self, p: [f64; 3]) -> Result<f64, OpFailure> {
        let range = || OpFailure::numeric(OpField::Plane, OpDefect::NumericRange, Stage::Checks);
        let (_, _, value) = self.exact(p).map_err(|_| range())?;
        let magnitude = value.lo.abs().max(value.hi.abs());
        let mut squares = Enclosure::point(0.0);
        for n in self.normal {
            let e = Enclosure::point(n);
            squares = squares.add(e.mul(e).ok_or_else(range)?).ok_or_else(range)?;
        }
        let length = squares.sqrt().ok_or_else(range)?;
        let distance = Enclosure::point(magnitude).div(length).ok_or_else(range)?;
        Ok(distance.hi)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn side(plane: &CutPlane, p: [f64; 3]) -> (i8, f64) {
        plane.evaluate(p, &mut ExactBudget::new(10)).unwrap()
    }

    #[test]
    fn cancellation_of_rounded_sums_is_decided_exactly() {
        let plane = CutPlane::new([0.0; 3], [1.0, 1.0, 1.0]);
        // 0.1 + 0.2 - 0.30000000000000004 rounds to 0 in binary64 but is -2.8e-17 exactly.
        let mut budget = ExactBudget::new(10);
        let (sign, value) = plane
            .evaluate([0.1, 0.2, -0.300_000_000_000_000_04], &mut budget)
            .unwrap();
        assert_eq!(sign, -1);
        assert!(value < 0.0 && (value + 2.775_557_561_562_891_4e-17).abs() < 1e-30);
        assert_eq!(budget.used(), 1, "one exact fallback was charged");
        // A true zero and a clear sign never need more than the filter or one fallback.
        assert_eq!(side(&plane, [1.0, 2.0, -3.0]).0, 0);
        assert_eq!(side(&plane, [1.0, 2.0, 3.0]).0, 1);
        assert_eq!(side(&plane, [-1.0, -2.0, -3.0]).0, -1);
        // Huge cancelling terms around a unit residue.
        assert_eq!(side(&plane, [1e16, -1e16, 1.0]).0, 1);
        assert_eq!(side(&plane, [1e16, -1e16, -1.0]).0, -1);
    }

    #[test]
    fn exact_signs_match_an_integer_reference_on_a_dyadic_grid() {
        let mut state = 0x2545_f491_4f6c_dd1d_u64;
        let mut next = move || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        for _ in 0..2000 {
            let mut n = [0i64; 3];
            let mut o = [0i64; 3];
            let mut q = [0i64; 3];
            for k in 0..3 {
                n[k] = (next() % 21) as i64 - 10;
                o[k] = (next() % 9) as i64 - 4;
                q[k] = (next() % 9) as i64 - 4;
            }
            if n == [0, 0, 0] {
                continue;
            }
            // Eighths: every value is a dyadic rational, so the integer dot product is exact.
            let f = |v: [i64; 3]| v.map(|x| x as f64 / 8.0);
            let plane = CutPlane::new(f(o), f(n));
            let reference: i64 = (0..3).map(|k| n[k] * (q[k] - o[k])).sum();
            let (sign, value) = side(&plane, f(q));
            assert_eq!(i64::from(sign), reference.signum());
            assert_eq!(sign == 0, value == 0.0 || sign == 0);
        }
    }

    #[test]
    fn the_distance_bound_is_zero_on_the_plane_and_small_next_to_it() {
        let plane = CutPlane::new([0.0; 3], [0.0, 0.0, 2.0]);
        assert_eq!(plane.deviation_bound_mm([5.0, -3.0, 0.0]).unwrap(), 0.0);
        let d = plane.deviation_bound_mm([5.0, -3.0, 1e-9]).unwrap();
        assert!((1e-9..1.000_001e-9).contains(&d), "{d}");
    }

    #[test]
    fn validation_orders_finiteness_cap_then_zero_and_tiny_normals() {
        let ok = [1.0, 0.0, 0.0];
        let code = |p: [f64; 3], n: [f64; 3]| CutPlane::new(p, n).validate().map_err(|e| e.code);
        assert_eq!(code([0.0; 3], ok), Ok(()));
        assert_eq!(code([f64::NAN, 0.0, 0.0], [0.0; 3]), Err(OpCode::NonFinite));
        assert_eq!(code([5e9, 0.0, 0.0], [0.0; 3]), Err(OpCode::AboveLimit));
        assert_eq!(code([0.0; 3], [0.0; 3]), Err(OpCode::NotPositive));
        assert_eq!(
            code([0.0; 3], [1e-101, 0.0, 0.0]),
            Err(OpCode::NumericUncertain)
        );
        assert_eq!(code([0.0; 3], [1e-100, 0.0, 0.0]), Ok(()));
        assert_eq!(CutPlane::new([0.0; 3], [1.0, -3.0, 3.0]).dominant_axis(), 1);
        assert_eq!(CutPlane::new([0.0; 3], [3.0, -3.0, 3.0]).dominant_axis(), 0);
    }
}
