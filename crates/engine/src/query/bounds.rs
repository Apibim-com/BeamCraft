// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! Axis-aligned bounds, computed by the engine from kernel vertices (never by a renderer).

use beamcraft_geom::Point3;

/// An axis-aligned box in world millimetres. All components are finite.
///
/// This is a display and framing aid. It is never a volume or any exact quantity (F06 section 8).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Aabb {
    /// Component-wise minimum.
    pub min: [f64; 3],
    /// Component-wise maximum.
    pub max: [f64; 3],
}

impl Aabb {
    /// A degenerate box at one point.
    pub const fn at(point: Point3) -> Self {
        Self {
            min: [point.x, point.y, point.z],
            max: [point.x, point.y, point.z],
        }
    }

    /// The smallest box containing both.
    #[must_use]
    pub fn union(self, other: Self) -> Self {
        let [a0, a1, a2] = self.min;
        let [b0, b1, b2] = other.min;
        let [c0, c1, c2] = self.max;
        let [d0, d1, d2] = other.max;
        Self {
            min: [a0.min(b0), a1.min(b1), a2.min(b2)],
            max: [c0.max(d0), c1.max(d1), c2.max(d2)],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn union_takes_component_extremes() {
        let a = Aabb::at(Point3::new(1.0, -2.0, 3.0));
        let b = Aabb::at(Point3::new(-1.0, 5.0, 3.0));
        let u = a.union(b);
        assert_eq!(u.min, [-1.0, -2.0, 3.0]);
        assert_eq!(u.max, [1.0, 5.0, 3.0]);
        assert_eq!(u, b.union(a));
    }
}
