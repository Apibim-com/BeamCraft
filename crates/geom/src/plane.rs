// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! Finite planes for geometric distance checks; no contour/topology decisions.

use crate::tolerance::Tolerance;
use crate::{MathError, Point3, Vec3};

/// A plane through a finite origin with a normalized, nonzero normal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Plane3 {
    origin: Point3,
    normal: Vec3,
}

impl Plane3 {
    /// Validates the origin and normalizes the normal using the ordered f64 norm.
    pub fn new(origin: Point3, normal: Vec3) -> Result<Self, MathError> {
        Ok(Self {
            origin: origin.validated()?,
            normal: normal.normalized()?,
        })
    }
    /// Returns the finite plane origin.
    pub const fn origin(self) -> Point3 {
        self.origin
    }
    /// Returns the normalized plane normal.
    pub const fn normal(self) -> Vec3 {
        self.normal
    }
    /// Evaluates (point - origin) dot normal in F06 dot-product order.
    pub fn signed_distance(self, point: Point3) -> Result<f64, MathError> {
        self.origin.vector_to(point)?.dot(self.normal)
    }
    /// Checks the inclusive scaled planarity band, without moving the point.
    ///
    /// The caller supplies a plate-derived tolerance. This distance band does
    /// not certify exact planarity, winding, intersections or topology.
    pub fn within_planarity_band(
        self,
        point: Point3,
        tolerance: Tolerance,
    ) -> Result<bool, MathError> {
        Ok(self.signed_distance(point)?.abs() <= tolerance.plane_band_mm())
    }
}
