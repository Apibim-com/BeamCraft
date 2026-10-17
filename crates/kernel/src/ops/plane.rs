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
    /// The estimate carries the exact sign (never zero for a non-zero sign) and is not used to
    /// place vertices (see [`Self::intersection`]); it is the filtered value when the filter
    /// decides, else the midpoint of an enclosure of the exact integer value.
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
            return Ok((if value > 0.0 { 1 } else { -1 }, value));
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

    /// The binary64 rounding (to nearest, ties to even) of every coordinate of the exact
    /// intersection of the plane with the segment `a`-`b`.
    ///
    /// The endpoints must have strictly opposite exact signs. The exact coordinate is the
    /// rational `(dA * b_i - dB * a_i) / (dA - dB)` with `dA`, `dB` the exact plane values; a
    /// seed from outward-rounded enclosures is moved by at most a few representable steps until
    /// two exact integer comparisons with the neighbouring midpoints prove it is the nearest
    /// double. The result does not depend on the order of the endpoints and never leaves the
    /// segment. The work is at most eight exact comparison rounds per coordinate.
    pub(crate) fn intersection(&self, a: [f64; 3], b: [f64; 3]) -> Result<[f64; 3], OpFailure> {
        let mut out = [0.0; 3];
        for (axis, slot) in out.iter_mut().enumerate() {
            let rounded = self.rounded_coordinate(a, b, axis)?;
            *slot = if rounded == 0.0 { 0.0 } else { rounded };
        }
        Ok(out)
    }

    /// Exact numerator and positive denominator of the intersection coordinate `axis`, in the
    /// common unit `2^e` (`value = num / den * 2^e`), with `e` fine enough for `extra` too.
    fn ratio(
        &self,
        a: [f64; 3],
        b: [f64; 3],
        axis: usize,
        extra: &[f64],
    ) -> Result<(Big, Big, i32), OpFailure> {
        let range = || OpFailure::numeric(OpField::Plane, OpDefect::NumericRange, Stage::Build);
        let e = min_exponent(
            a.iter()
                .chain(b.iter())
                .chain(self.point.iter())
                .chain(self.normal.iter())
                .chain(extra.iter())
                .copied(),
        );
        let s = |x: f64| scaled(x, e).map_err(|_| range());
        let value = |p: [f64; 3]| -> Result<Big, OpFailure> {
            let mut sum = Big::ZERO;
            for ((n, q), o) in self.normal.iter().zip(p).zip(self.point) {
                let diff = s(q)?.sub(&s(o)?).map_err(|_| range())?;
                sum = sum
                    .add(&s(*n)?.mul(&diff).map_err(|_| range())?)
                    .map_err(|_| range())?;
            }
            Ok(sum)
        };
        let (da, db) = (value(a)?, value(b)?);
        let (ai, bi) = (
            s(a.get(axis).copied().ok_or_else(range)?)?,
            s(b.get(axis).copied().ok_or_else(range)?)?,
        );
        let mut num = da
            .mul(&bi)
            .and_then(|x| x.sub(&db.mul(&ai)?))
            .map_err(|_| range())?;
        let mut den = da.sub(&db).map_err(|_| range())?;
        if den.sign() == 0 || da.sign() * db.sign() >= 0 {
            return Err(range());
        }
        if den.sign() < 0 {
            num = num.negated();
            den = den.negated();
        }
        Ok((num, den, e))
    }

    /// Sign of `exact coordinate - (lo + hi) / 2`.
    fn compare_midpoint(
        &self,
        a: [f64; 3],
        b: [f64; 3],
        axis: usize,
        lo: f64,
        hi: f64,
    ) -> Result<i8, OpFailure> {
        let range = || OpFailure::numeric(OpField::Plane, OpDefect::NumericRange, Stage::Build);
        let (num, den, e) = self.ratio(a, b, axis, &[lo, hi])?;
        let sum = scaled(lo, e)
            .and_then(|l| l.add(&scaled(hi, e)?))
            .map_err(|_| range())?;
        let doubled = num.add(&num).map_err(|_| range())?;
        Ok(doubled
            .sub(&sum.mul(&den).map_err(|_| range())?)
            .map_err(|_| range())?
            .sign())
    }

    fn rounded_coordinate(&self, a: [f64; 3], b: [f64; 3], axis: usize) -> Result<f64, OpFailure> {
        let range = || OpFailure::numeric(OpField::Plane, OpDefect::NumericRange, Stage::Build);
        let (ai, bi) = (
            a.get(axis).copied().ok_or_else(range)?,
            b.get(axis).copied().ok_or_else(range)?,
        );
        let (low, high) = (ai.min(bi), ai.max(bi));
        let (num, den, e) = self.ratio(a, b, axis, &[])?;
        let seed = e
            .checked_mul(3)
            .and_then(|p| num.enclose_clamped(p))
            .zip(e.checked_mul(2).and_then(|p| den.enclose(p)))
            .and_then(|(n, d)| n.div(d))
            .map_or(low, |q| q.midpoint())
            .clamp(low, high);
        let even = |x: f64| x.to_bits() & 1 == 0;
        let mut c = seed;
        for _ in 0..8 {
            let (down, up) = (c.next_down(), c.next_up());
            match self.compare_midpoint(a, b, axis, down, c)? {
                -1 => c = down,
                0 => return Ok(if even(down) { down } else { c }),
                _ => match self.compare_midpoint(a, b, axis, c, up)? {
                    1 => c = up,
                    0 => return Ok(if even(c) { c } else { up }),
                    _ => return Ok(c.clamp(low, high)),
                },
            }
        }
        Err(range())
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
