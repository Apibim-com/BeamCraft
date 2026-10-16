// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! Fixed-size binary64 point and vector operations in F06 operand order.

use crate::MathError;
use crate::error::{canonical_zero, derived, finite};

/// A point in millimetres, retaining the original scaffold's raw input representation.
///
/// `new` and public fields allow unvalidated input for compatibility. Use `try_new`
/// at input boundaries. Every checked math operation validates raw points again.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point3 {
    /// X coordinate, mm (unvalidated if constructed through raw input).
    pub x: f64,
    /// Y coordinate, mm (unvalidated if constructed through raw input).
    pub y: f64,
    /// Z coordinate, mm (unvalidated if constructed through raw input).
    pub z: f64,
}

impl Point3 {
    /// The model origin.
    pub const ORIGIN: Self = Self::new(0.0, 0.0, 0.0);

    /// Creates a raw point; use `try_new` for finite-f64 validation.
    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    /// Creates a finite point, canonicalizing negative zero.
    pub fn try_new(x: f64, y: f64, z: f64) -> Result<Self, MathError> {
        Ok(Self {
            x: finite(x)?,
            y: finite(y)?,
            z: finite(z)?,
        })
    }

    /// Validates all coordinates and canonicalizes negative zero.
    pub fn validated(self) -> Result<Self, MathError> {
        Self::try_new(self.x, self.y, self.z)
    }

    /// Legacy unchecked distance for scaffold callers; may return NaN or infinity.
    ///
    /// Kernel code must use `checked_distance_to`, which rejects invalid inputs/results.
    pub fn distance_to(self, other: Self) -> f64 {
        let (dx, dy, dz) = (other.x - self.x, other.y - self.y, other.z - self.z);
        ((dx * dx + dy * dy) + dz * dz).sqrt()
    }

    /// Computes the finite distance using F06's ordered subtraction and norm.
    pub fn checked_distance_to(self, other: Self) -> Result<f64, MathError> {
        self.vector_to(other)?.norm()
    }

    /// Subtracts this point from `other`, rejecting non-finite arithmetic.
    pub fn vector_to(self, other: Self) -> Result<Vec3, MathError> {
        let a = self.validated()?;
        let b = other.validated()?;
        Vec3::derived(b.x - a.x, b.y - a.y, b.z - a.z)
    }

    /// Translates a point by a finite vector, checking each output coordinate.
    pub fn translated(self, offset: Vec3) -> Result<Self, MathError> {
        let p = self.validated()?;
        Self::from_derived(p.x + offset.x, p.y + offset.y, p.z + offset.z)
    }

    pub(crate) fn from_derived(x: f64, y: f64, z: f64) -> Result<Self, MathError> {
        Ok(Self {
            x: derived(x)?,
            y: derived(y)?,
            z: derived(z)?,
        })
    }
}

/// A finite three-component vector; negative zero is always canonicalized.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vec3 {
    x: f64,
    y: f64,
    z: f64,
}

impl Vec3 {
    /// The zero vector (not a valid direction).
    pub const ZERO: Self = Self {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };
    /// The positive X direction.
    pub const X: Self = Self {
        x: 1.0,
        y: 0.0,
        z: 0.0,
    };
    /// The positive Y direction.
    pub const Y: Self = Self {
        x: 0.0,
        y: 1.0,
        z: 0.0,
    };
    /// The positive Z direction.
    pub const Z: Self = Self {
        x: 0.0,
        y: 0.0,
        z: 1.0,
    };

    /// Validates finite components; rejects NaN/infinity without clamping.
    pub fn new(x: f64, y: f64, z: f64) -> Result<Self, MathError> {
        Ok(Self {
            x: finite(x)?,
            y: finite(y)?,
            z: finite(z)?,
        })
    }

    /// Returns X, Y, Z in that order.
    pub const fn components(self) -> [f64; 3] {
        [self.x, self.y, self.z]
    }
    /// Returns the X component.
    pub const fn x(self) -> f64 {
        self.x
    }
    /// Returns the Y component.
    pub const fn y(self) -> f64 {
        self.y
    }
    /// Returns the Z component.
    pub const fn z(self) -> f64 {
        self.z
    }

    /// Evaluates `(x*x + y*y) + z*z`, then sqrt, without reassociation.
    pub fn norm(self) -> Result<f64, MathError> {
        derived(((self.x * self.x + self.y * self.y) + self.z * self.z).sqrt())
    }

    /// Evaluates `(ax*bx + ay*by) + az*bz` in F06 order.
    pub fn dot(self, other: Self) -> Result<f64, MathError> {
        derived((self.x * other.x + self.y * other.y) + self.z * other.z)
    }

    /// Computes the right-handed cross product in F06 component order.
    pub fn cross(self, other: Self) -> Result<Self, MathError> {
        Self::derived(
            self.y * other.z - self.z * other.y,
            self.z * other.x - self.x * other.z,
            self.x * other.y - self.y * other.x,
        )
    }

    /// Multiplies each component by a finite scalar, checking overflow.
    pub fn scaled(self, factor: f64) -> Result<Self, MathError> {
        finite(factor)?;
        Self::derived(self.x * factor, self.y * factor, self.z * factor)
    }

    /// Adds two vectors, rejecting non-finite results.
    pub fn added(self, other: Self) -> Result<Self, MathError> {
        Self::derived(self.x + other.x, self.y + other.y, self.z + other.z)
    }

    /// Divides components directly by the norm; rejects zero and underflowed norms.
    pub fn normalized(self) -> Result<Self, MathError> {
        if self == Self::ZERO {
            return Err(MathError::ZeroAxis);
        }
        let length = self.norm()?;
        if length == 0.0 {
            return Err(MathError::NumericUncertain);
        }
        self.divided_by(length)
    }

    pub(crate) fn divided_by(self, divisor: f64) -> Result<Self, MathError> {
        Self::derived(self.x / divisor, self.y / divisor, self.z / divisor)
    }

    pub(crate) fn derived(x: f64, y: f64, z: f64) -> Result<Self, MathError> {
        Ok(Self {
            x: derived(x)?,
            y: derived(y)?,
            z: derived(z)?,
        })
    }
}

/// Encodes one finite f64 as canonical big-endian binary64 (F06 section 7).
///
/// NaN/infinity are rejected; both signed zeros encode as eight zero bytes.
pub fn canonical_f64_bytes(value: f64) -> Result<[u8; 8], MathError> {
    finite(value)?;
    Ok(canonical_zero(value).to_be_bytes())
}
