// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! Segment, containment and distance predicates built on the exact orientation sign.
//!
//! Topology (intersection, touching, containment) is always exact. Distances are
//! written binary64 evaluations used only for the F06 section 4 floor comparisons;
//! they never decide topology and never snap or heal.

use super::Point2;
use super::exact::{Uncertain, orient2d};
use core::cmp::Ordering;

/// Exact relation of two closed segments.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SegRel {
    /// The closed segments share no point.
    Disjoint,
    /// They share exactly one point and do not cross properly (endpoint touch).
    Touch,
    /// They cross at one interior point of both.
    Cross,
    /// They are collinear and share a segment of positive length.
    Overlap,
}

fn coord(p: Point2, use_x: bool) -> f64 {
    if use_x { p.x } else { p.y }
}

fn on_segment(a: Point2, b: Point2, p: Point2) -> bool {
    a.x.min(b.x) <= p.x && p.x <= a.x.max(b.x) && a.y.min(b.y) <= p.y && p.y <= a.y.max(b.y)
}

fn opposite(a: Ordering, b: Ordering) -> bool {
    matches!(
        (a, b),
        (Ordering::Less, Ordering::Greater) | (Ordering::Greater, Ordering::Less)
    )
}

/// Exact relation of the closed segments `a-b` and `c-d` (both nondegenerate).
pub(crate) fn segment_relation(
    a: Point2,
    b: Point2,
    c: Point2,
    d: Point2,
) -> Result<SegRel, Uncertain> {
    let o1 = orient2d(a, b, c)?;
    let o2 = orient2d(a, b, d)?;
    if o1 == Ordering::Equal && o2 == Ordering::Equal {
        let use_x = a.x != b.x;
        let (a1, b1) = (coord(a, use_x), coord(b, use_x));
        let (c1, d1) = (coord(c, use_x), coord(d, use_x));
        let lo = a1.min(b1).max(c1.min(d1));
        let hi = a1.max(b1).min(c1.max(d1));
        return Ok(match lo.partial_cmp(&hi) {
            Some(Ordering::Less) => SegRel::Overlap,
            Some(Ordering::Equal) => SegRel::Touch,
            _ => SegRel::Disjoint,
        });
    }
    let o3 = orient2d(c, d, a)?;
    let o4 = orient2d(c, d, b)?;
    if opposite(o1, o2) && opposite(o3, o4) {
        return Ok(SegRel::Cross);
    }
    let touches = (o1 == Ordering::Equal && on_segment(a, b, c))
        || (o2 == Ordering::Equal && on_segment(a, b, d))
        || (o3 == Ordering::Equal && on_segment(c, d, a))
        || (o4 == Ordering::Equal && on_segment(c, d, b));
    Ok(if touches {
        SegRel::Touch
    } else {
        SegRel::Disjoint
    })
}

/// Exact point-in-polygon test by winding number; the point must not lie on the boundary.
pub(crate) fn winding_contains(p: Point2, polygon: &[Point2]) -> Result<bool, Uncertain> {
    let mut winding = 0i32;
    for (a, b) in polygon.iter().zip(polygon.iter().cycle().skip(1)) {
        if a.y <= p.y {
            if b.y > p.y && orient2d(*a, *b, p)? == Ordering::Greater {
                winding += 1;
            }
        } else if b.y <= p.y && orient2d(*a, *b, p)? == Ordering::Less {
            winding -= 1;
        }
    }
    Ok(winding != 0)
}

/// Whether the axis-aligned boxes of two segments are farther apart than `margin` (a clear
/// rejection for both intersection and floor-distance work; the margin is a safe over-estimate).
pub(crate) fn boxes_apart(a: Point2, b: Point2, c: Point2, d: Point2, margin: f64) -> bool {
    let gap_x = (a.x.min(b.x) - c.x.max(d.x)).max(c.x.min(d.x) - a.x.max(b.x));
    let gap_y = (a.y.min(b.y) - c.y.max(d.y)).max(c.y.min(d.y) - a.y.max(b.y));
    gap_x > margin || gap_y > margin
}

/// Distance from `p` to the closed segment `a-b`, in the written operation order.
pub(crate) fn point_segment_distance(p: Point2, a: Point2, b: Point2) -> f64 {
    let (abx, aby) = (b.x - a.x, b.y - a.y);
    let (apx, apy) = (p.x - a.x, p.y - a.y);
    let length_sq = abx * abx + aby * aby;
    let t = if length_sq > 0.0 {
        ((apx * abx + apy * aby) / length_sq).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let (dx, dy) = (p.x - (a.x + t * abx), p.y - (a.y + t * aby));
    (dx * dx + dy * dy).sqrt()
}

/// Distance between two non-intersecting closed segments.
pub(crate) fn segment_distance(a: Point2, b: Point2, c: Point2, d: Point2) -> f64 {
    point_segment_distance(a, c, d)
        .min(point_segment_distance(b, c, d))
        .min(point_segment_distance(c, a, b))
        .min(point_segment_distance(d, a, b))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f64, y: f64) -> Point2 {
        Point2 { x, y }
    }

    #[test]
    fn every_relation_is_classified() {
        let (a, b) = (p(0.0, 0.0), p(4.0, 0.0));
        assert_eq!(
            segment_relation(a, b, p(2.0, -1.0), p(2.0, 1.0)),
            Ok(SegRel::Cross)
        );
        assert_eq!(
            segment_relation(a, b, p(2.0, 0.0), p(2.0, 1.0)),
            Ok(SegRel::Touch)
        );
        assert_eq!(
            segment_relation(a, b, p(4.0, 0.0), p(5.0, 3.0)),
            Ok(SegRel::Touch)
        );
        assert_eq!(
            segment_relation(a, b, p(4.0, 0.0), p(6.0, 0.0)),
            Ok(SegRel::Touch)
        );
        assert_eq!(
            segment_relation(a, b, p(3.0, 0.0), p(6.0, 0.0)),
            Ok(SegRel::Overlap)
        );
        assert_eq!(
            segment_relation(a, b, p(5.0, 0.0), p(6.0, 0.0)),
            Ok(SegRel::Disjoint)
        );
        assert_eq!(
            segment_relation(a, b, p(0.0, 1.0), p(4.0, 1.0)),
            Ok(SegRel::Disjoint)
        );
        assert_eq!(
            segment_relation(a, b, p(5.0, -1.0), p(5.0, 1.0)),
            Ok(SegRel::Disjoint)
        );
        // Vertical collinear overlap uses the y axis.
        assert_eq!(
            segment_relation(p(0.0, 0.0), p(0.0, 3.0), p(0.0, 2.0), p(0.0, 9.0)),
            Ok(SegRel::Overlap)
        );
    }

    #[test]
    fn containment_is_exact_for_interior_and_exterior_points() {
        let square = [p(0.0, 0.0), p(2.0, 0.0), p(2.0, 2.0), p(0.0, 2.0)];
        assert_eq!(winding_contains(p(1.0, 1.0), &square), Ok(true));
        assert_eq!(winding_contains(p(3.0, 1.0), &square), Ok(false));
        assert_eq!(winding_contains(p(1.0, -1e-300), &square), Ok(false));
        let clockwise = [square[3], square[2], square[1], square[0]];
        assert_eq!(winding_contains(p(1.0, 1.0), &clockwise), Ok(true));
    }

    #[test]
    fn distances_are_exact_for_axis_aligned_cases() {
        let (a, b) = (p(0.0, 0.0), p(100.0, 0.0));
        assert_eq!(point_segment_distance(p(50.0, 0.001), a, b), 0.001);
        assert_eq!(point_segment_distance(p(-3.0, 4.0), a, b), 5.0);
        assert_eq!(segment_distance(a, b, p(10.0, 2.0), p(20.0, 7.0)), 2.0);
        assert!(boxes_apart(a, b, p(0.0, 10.0), p(1.0, 11.0), 2.0));
        assert!(!boxes_apart(a, b, p(0.0, 1.0), p(1.0, 2.0), 2.0));
    }
}
