// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! Experimental F06 adapter. Not a production kernel or a complete input validator.
//! Operation order comes from geometry-contract-v0.1 sections 2 and 4.
use glam::{DQuat, DVec3};

/// Bounded adapter failure, without exposing input text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// Nonfinite input or intermediate.
    NonFinite,
    /// Input exceeds the F04 coordinate cap.
    Limit,
    /// Quaternion range or squared norm is invalid.
    Quaternion,
    /// Axis is outside the admitted length interval.
    Axis,
    /// Frame certificate failed.
    Uncertain,
}

/// F06 ordered dot product; no explicit FMA or reassociation.
pub fn dot(a: DVec3, b: DVec3) -> f64 {
    (a.x * b.x + a.y * b.y) + a.z * b.z
}

/// F06 ordered cross product.
pub fn cross(a: DVec3, b: DVec3) -> DVec3 {
    DVec3::new(
        a.y * b.z - a.z * b.y,
        a.z * b.x - a.x * b.z,
        a.x * b.y - a.y * b.x,
    )
}

/// F06 ordered length.
pub fn norm(a: DVec3) -> f64 {
    ((a.x * a.x + a.y * a.y) + a.z * a.z).sqrt()
}

fn positive_zero(x: f64) -> f64 {
    if x == 0.0 { 0.0 } else { x }
}

fn clean(v: DVec3) -> DVec3 {
    DVec3::new(positive_zero(v.x), positive_zero(v.y), positive_zero(v.z))
}

fn bounded(v: DVec3) -> Result<(), Error> {
    if !v.is_finite() {
        Err(Error::NonFinite)
    } else if v.abs().max_element() > 1e9 {
        Err(Error::Limit)
    } else {
        Ok(())
    }
}

/// Normalize in F06 order and select its canonical sign for keys.
/// Crate constructors do not implement the F04 admission policy.
pub fn quaternion([x, y, z, w]: [f64; 4]) -> Result<DQuat, Error> {
    if ![x, y, z, w].into_iter().all(f64::is_finite) {
        return Err(Error::NonFinite);
    }
    let n = ((x * x + y * y) + z * z) + w * w;
    if [x, y, z, w].into_iter().any(|v| v.abs() > 1.0) || (n - 1.0).abs() > 1e-12 {
        return Err(Error::Quaternion);
    }
    let r = n.sqrt();
    let (mut x, mut y, mut z, mut w) = (x / r, y / r, z / r, w / r);
    let first = [x, y, z].into_iter().find(|v| *v != 0.0).unwrap_or(0.0);
    if w < 0.0 || (w == 0.0 && first < 0.0) {
        (x, y, z, w) = (-x, -y, -z, -w);
    }
    Ok(DQuat::from_xyzw(
        positive_zero(x),
        positive_zero(y),
        positive_zero(z),
        positive_zero(w),
    ))
}

/// F06 rotation matrix, stored as rows. Caller supplies an admitted normalized quaternion.
pub fn rotation(q: DQuat) -> [DVec3; 3] {
    let DQuat { x, y, z, w } = q;
    [
        DVec3::new(
            1.0 - 2.0 * (y * y + z * z),
            2.0 * (x * y - z * w),
            2.0 * (x * z + y * w),
        ),
        DVec3::new(
            2.0 * (x * y + z * w),
            1.0 - 2.0 * (x * x + z * z),
            2.0 * (y * z - x * w),
        ),
        DVec3::new(
            2.0 * (x * z - y * w),
            2.0 * (y * z + x * w),
            1.0 - 2.0 * (x * x + y * y),
        ),
    ]
}

/// Ordered matrix-vector product (rows, not columns).
pub fn multiply([a, b, c]: [DVec3; 3], v: DVec3) -> DVec3 {
    DVec3::new(dot(a, v), dot(b, v), dot(c, v))
}

/// Admitted F04 pose; fields stay private so invalid authored values cannot reach `world`.
#[derive(Debug, Clone, Copy)]
pub struct Pose {
    rows: [DVec3; 3],
    translation: DVec3,
}

impl Pose {
    /// Validate and normalize a rigid pose, preserving no authored state.
    pub fn new(q: [f64; 4], translation: DVec3) -> Result<Self, Error> {
        bounded(translation)?;
        Ok(Self {
            rows: rotation(quaternion(q)?),
            translation,
        })
    }

    /// Rotate, then translate in the written F06 order; normalize output signed zeros.
    pub fn world(self, local: DVec3) -> Result<DVec3, Error> {
        // Derived member corners may exceed the authored 1e9 coordinate cap.
        if !local.is_finite() {
            return Err(Error::NonFinite);
        }
        let v = multiply(self.rows, local) + self.translation;
        if v.is_finite() {
            Ok(clean(v))
        } else {
            Err(Error::Uncertain)
        }
    }
}

/// Member basis, with the actual branch decision retained for evidence.
#[derive(Debug, Clone, Copy)]
pub struct Frame {
    /// Local section width direction.
    pub x: DVec3,
    /// Local section height direction.
    pub y: DVec3,
    /// Member axis direction.
    pub z: DVec3,
    /// True for the strict near-vertical branch.
    pub near_vertical: bool,
}

impl Frame {
    /// Absolute norm, pairwise dot and handedness certificate, F06 section 4 rule 2d.
    pub fn certified(self) -> bool {
        let Self { x, y, z, .. } = self;
        [x, y, z].into_iter().all(|v| {
            let length = norm(v);
            v.is_finite() && (length * length - 1.0).abs() <= 1e-12
        }) && [dot(x, y), dot(x, z), dot(y, z), dot(cross(x, y), z) - 1.0]
            .into_iter()
            .all(|v| v.is_finite() && v.abs() <= 1e-12)
    }

    /// F06 default zero-roll frame, including projected +Y fallback and re-orthogonalization.
    pub fn new(p1: DVec3, p2: DVec3) -> Result<Self, Error> {
        bounded(p1)?;
        bounded(p2)?;
        let d = p2 - p1;
        let hh = d.x * d.x + d.y * d.y;
        let ll = hh + d.z * d.z;
        let length = ll.sqrt();
        if !(0.001..=1e6).contains(&length) {
            return Err(Error::Axis);
        }
        // Component division is deliberate; reciprocal multiplication changes last bits.
        let z = DVec3::new(d.x / length, d.y / length, d.z / length);
        let near_vertical = hh < 1e-12 * ll;
        let y = if near_vertical {
            let c = z.y;
            let p = DVec3::new(-z.x * c, 1.0 - z.y * c, -z.z * c);
            let n = norm(p);
            DVec3::new(p.x / n, p.y / n, p.z / n)
        } else {
            let denominator = hh.sqrt() * length;
            DVec3::new(
                (-d.x * d.z) / denominator,
                (-d.y * d.z) / denominator,
                hh / denominator,
            )
        };
        let x0 = cross(y, z);
        let n = norm(x0);
        let x = DVec3::new(x0.x / n, x0.y / n, x0.z / n);
        let frame = Self {
            x,
            y: cross(z, x),
            z,
            near_vertical,
        };
        if frame.certified() {
            Ok(frame)
        } else {
            Err(Error::Uncertain)
        }
    }
}
