// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! Geometry primitives for BeamCraft.
//!
//! Units are millimetres, right-handed, Z up.

/// A point in 3D model space, in millimetres.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point3 {
    /// X coordinate, mm.
    pub x: f64,
    /// Y coordinate, mm.
    pub y: f64,
    /// Z coordinate, mm.
    pub z: f64,
}

impl Point3 {
    /// The model origin.
    pub const ORIGIN: Self = Self::new(0.0, 0.0, 0.0);

    /// Creates a point from its coordinates.
    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    /// Euclidean distance to `other`, in millimetres.
    pub fn distance_to(self, other: Self) -> f64 {
        let (dx, dy, dz) = (other.x - self.x, other.y - self.y, other.z - self.z);
        (dx * dx + dy * dy + dz * dz).sqrt()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distance_is_euclidean() {
        let d = Point3::ORIGIN.distance_to(Point3::new(3.0, 4.0, 12.0));
        assert!((d - 13.0).abs() < 1e-12);
    }
}
