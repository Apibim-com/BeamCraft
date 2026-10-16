// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! F06 section 4 default member frame, axial roll and absolute certificates.

use crate::error::{derived, finite};
use crate::tolerance::{
    MemberScale, ROLL_CAP_RAD, Tolerance, check_dimension_cap, check_point_cap,
};
use crate::{MathError, Point3, Vec3};

/// Inclusive absolute bound for all seven frame certificate residuals.
pub const FRAME_CERTIFICATE_BOUND: f64 = 1e-6;
/// Written squared multiplier of the strict near-vertical branch test.
pub const PARALLEL_SQUARED: f64 = 1e-12;

/// Residuals of the seven F06 frame identities, always absolute values.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrameCertificate {
    /// Unit-length deviations of X, Y, Z, evaluated as abs(norm*norm - 1).
    pub unit_deviations: [f64; 3],
    /// Absolute pairwise dot products in XY, XZ, YZ order.
    pub absolute_dots: [f64; 3],
    /// Absolute deviation of (X cross Y) dot Z from +1.
    pub handedness_deviation: f64,
}

impl FrameCertificate {
    /// Measures the certificate without silently repairing any axis.
    pub fn measure(x: Vec3, y: Vec3, z: Vec3) -> Result<Self, MathError> {
        let nx = x.norm()?;
        let ny = y.norm()?;
        let nz = z.norm()?;
        Ok(Self {
            unit_deviations: [
                derived((nx * nx - 1.0).abs())?,
                derived((ny * ny - 1.0).abs())?,
                derived((nz * nz - 1.0).abs())?,
            ],
            absolute_dots: [x.dot(y)?.abs(), x.dot(z)?.abs(), y.dot(z)?.abs()],
            handedness_deviation: derived((x.cross(y)?.dot(z)? - 1.0).abs())?,
        })
    }

    /// Requires every residual to be finite, nonnegative and at most 1e-6.
    ///
    /// The certificate is inspectable data; this method also rejects invalid
    /// values in a caller-constructed certificate. `Frame3` measures its own.
    pub fn is_valid(self) -> bool {
        self.unit_deviations
            .into_iter()
            .chain(self.absolute_dots)
            .chain([self.handedness_deviation])
            .all(|v| v.is_finite() && (0.0..=FRAME_CERTIFICATE_BOUND).contains(&v))
    }
}

/// A right-handed body coordinate frame certified under F06's residual bounds.
///
/// This is a body basis, not an authored element orientation or matrix input.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frame3 {
    x: Vec3,
    y: Vec3,
    z: Vec3,
    certificate: FrameCertificate,
}

impl Frame3 {
    /// Exact identity body frame.
    pub const IDENTITY: Self = Self {
        x: Vec3::X,
        y: Vec3::Y,
        z: Vec3::Z,
        certificate: FrameCertificate {
            unit_deviations: [0.0; 3],
            absolute_dots: [0.0; 3],
            handedness_deviation: 0.0,
        },
    };

    /// Certifies supplied body axes, rejecting zero, shear, scale and reflection.
    ///
    /// The absolute residual gate is F06's approximate numeric certificate, not
    /// an assertion that binary64 axes satisfy exact real orthonormal identities.
    pub fn try_new(x: Vec3, y: Vec3, z: Vec3) -> Result<Self, MathError> {
        let certificate = FrameCertificate::measure(x, y, z)?;
        if !certificate.is_valid() {
            return Err(MathError::InvalidFrame);
        }
        Ok(Self {
            x,
            y,
            z,
            certificate,
        })
    }
    /// Returns the body X axis.
    pub const fn x(self) -> Vec3 {
        self.x
    }
    /// Returns the body Y axis.
    pub const fn y(self) -> Vec3 {
        self.y
    }
    /// Returns the body Z axis.
    pub const fn z(self) -> Vec3 {
        self.z
    }
    /// Returns the certificate measured from these exact axes.
    pub const fn certificate(self) -> FrameCertificate {
        self.certificate
    }

    /// Maps body vector components into the enclosing coordinate system.
    pub fn map_vector(self, v: Vec3) -> Result<Vec3, MathError> {
        let [vx, vy, vz] = v.components();
        Vec3::derived(
            (self.x.x() * vx + self.y.x() * vy) + self.z.x() * vz,
            (self.x.y() * vx + self.y.y() * vy) + self.z.y() * vz,
            (self.x.z() * vx + self.y.z() * vy) + self.z.z() * vz,
        )
    }
}

/// Which written default-frame branch was used, before any explicit roll.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemberFrameRule {
    /// hh >= 1e-12*LL: project element-local +Z.
    Horizontal,
    /// hh < 1e-12*LL: project element-local +Y (including anti-parallel).
    NearVertical,
}

/// A member's certified body frame and validated local scale.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MemberFrame {
    frame: Frame3,
    rule: MemberFrameRule,
    scale: MemberScale,
}

impl MemberFrame {
    /// Validates member math inputs, constructs/certifies the default frame,
    /// then applies/certifies explicit right-handed roll about local +Z.
    ///
    /// Omitted and signed-zero roll bypass trigonometry. Nonzero roll uses the
    /// isolated platform sin/cos function; cross-target reproducibility is OPEN.
    /// This primitive returns one typed error; full kernel diagnostic aggregation
    /// and profile support checks belong to later tasks.
    pub fn new(
        start: Point3,
        end: Point3,
        width_mm: f64,
        height_mm: f64,
        roll_rad: Option<f64>,
    ) -> Result<Self, MathError> {
        let roll = roll_rad.unwrap_or(0.0);
        // Finiteness precedes all cap and range checks, including scale derivation.
        start.validated()?;
        end.validated()?;
        finite(width_mm)?;
        finite(height_mm)?;
        finite(roll)?;
        check_point_cap(start)?;
        check_point_cap(end)?;
        check_dimension_cap(width_mm)?;
        check_dimension_cap(height_mm)?;
        if roll.abs() > ROLL_CAP_RAD {
            return Err(MathError::AboveLimit);
        }
        let scale = Tolerance::for_member(start, end, width_mm, height_mm)?;
        let d = scale.delta;
        let (dx, dy, dz) = (d.x(), d.y(), d.z());
        let hh = derived(dx * dx + dy * dy)?;
        let ll = derived(hh + dz * dz)?;
        let ell = derived(ll.sqrt())?;
        let z = d.divided_by(ell)?;
        let (initial_y, rule) = if hh >= PARALLEL_SQUARED * ll {
            let h = hh.sqrt();
            let denominator = h * ell;
            (
                Vec3::derived(
                    (-dx * dz) / denominator,
                    (-dy * dz) / denominator,
                    hh / denominator,
                )?,
                MemberFrameRule::Horizontal,
            )
        } else {
            let c = z.y();
            let p = Vec3::derived(-z.x() * c, 1.0 - z.y() * c, -z.z() * c)?;
            (p.normalized()?, MemberFrameRule::NearVertical)
        };
        let x = initial_y.cross(z)?.normalized()?;
        let y = z.cross(x)?;
        let default = Frame3::try_new(x, y, z)?;
        let frame = if roll == 0.0 {
            default
        } else {
            let (sin, cos) = roll_sin_cos(roll)?;
            // Preserve cos*X + sin*Y and -sin*X + cos*Y component order.
            let xr = x.scaled(cos)?.added(y.scaled(sin)?)?;
            let yr = x.scaled(-sin)?.added(y.scaled(cos)?)?;
            Frame3::try_new(xr, yr, z)?
        };
        Ok(Self { frame, rule, scale })
    }
    /// Returns the final rolled frame.
    pub const fn frame(self) -> Frame3 {
        self.frame
    }
    /// Returns the default-frame branch, independent of roll.
    pub const fn rule(self) -> MemberFrameRule {
        self.rule
    }
    /// Returns validated axis length, section half-diagonal and tolerance.
    pub const fn scale(self) -> MemberScale {
        self.scale
    }
}

// The sole trig boundary. OPEN owner decision: reproducible evaluation profile.
// Do not replace with sin_cos/libm, special angles or another approximation silently.
fn roll_sin_cos(roll: f64) -> Result<(f64, f64), MathError> {
    Ok((derived(roll.sin())?, derived(roll.cos())?))
}
