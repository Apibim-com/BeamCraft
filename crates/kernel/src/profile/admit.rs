// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! Profile admission in F06 stage order, with exact topology and floor-based minima.

use super::diagnostic::{
    Collector, Defect, ProfileCode, ProfileDiagnostic, ProfileFailure, ProfileField, Stage,
};
use super::exact::{Uncertain, orient2d, signed_area_sign};
use super::predicates::{
    SegRel, boxes_apart, segment_distance, segment_relation, winding_contains,
};
use super::{
    Cavity, MAX_CAVITIES, MAX_CONTOUR_POINTS, MAX_TOTAL_POINTS, Point2, Profile, at, shoelace,
};
use beamcraft_geom::Point3;
use beamcraft_geom::tolerance::{COORDINATE_CAP_MM, DIMENSION_CAP_MM, Tolerance};
use core::cmp::Ordering;

/// One contour with its fixed diagnostic location.
#[derive(Clone, Copy)]
struct Contour<'a> {
    points: &'a [Point2],
    field: ProfileField,
    cavity: Option<u32>,
}

fn contours<'a>(outer: &'a [Point2], cavities: &'a [Cavity]) -> Vec<Contour<'a>> {
    let mut list = Vec::with_capacity(cavities.len().saturating_add(1));
    list.push(Contour {
        points: outer,
        field: ProfileField::OuterContour,
        cavity: None,
    });
    for (i, cavity) in cavities.iter().enumerate() {
        list.push(Contour {
            points: cavity.points(),
            field: ProfileField::CavityContour,
            cavity: u32::try_from(i).ok(),
        });
    }
    list
}

fn index(i: usize) -> Option<u32> {
    u32::try_from(i).ok()
}

fn record(
    collector: &mut Collector,
    code: ProfileCode,
    stage: Stage,
    contour: &Contour<'_>,
    defect: Defect,
    at_index: Option<usize>,
) {
    collector.push(ProfileDiagnostic {
        code,
        field: contour.field,
        stage,
        defect,
        cavity: contour.cavity,
        index: at_index.and_then(index),
    });
}

fn uncertain(stage: Stage, contour: &Contour<'_>) -> ProfileFailure {
    ProfileFailure::one(
        ProfileCode::NumericUncertain,
        contour.field,
        stage,
        Defect::None,
        contour.cavity,
        None,
    )
}

pub(crate) fn admit(
    outer: &[Point2],
    cavities: &[Cavity],
    thickness: Option<f64>,
) -> Result<Profile, ProfileFailure> {
    check_counts(outer, cavities)?;
    let list = contours(outer, cavities);
    check_finite(&list, thickness)?;
    let tolerance = check_caps(outer, &list, thickness)?;
    check_contours(&list, thickness, tolerance)?;
    Ok(Profile::from_parts(
        outer.to_vec(),
        cavities.to_vec(),
        tolerance,
    ))
}

// Counts are bounded before any traversal or allocation (F04 section 3, F06 section 2).
fn check_counts(outer: &[Point2], cavities: &[Cavity]) -> Result<(), ProfileFailure> {
    let mut c = Collector::default();
    let global = |field| ProfileDiagnostic {
        code: ProfileCode::AboveLimit,
        field,
        stage: Stage::Caps,
        defect: Defect::None,
        cavity: None,
        index: None,
    };
    if cavities.len() > MAX_CAVITIES {
        c.push(global(ProfileField::CavityCount));
    }
    let mut total = outer.len();
    for cavity in cavities.iter().take(MAX_CAVITIES.saturating_add(1)) {
        total = total.saturating_add(cavity.points().len());
    }
    if total > MAX_TOTAL_POINTS {
        c.push(global(ProfileField::TotalPoints));
    }
    for contour in contours(outer, cavities.get(..MAX_CAVITIES).unwrap_or(cavities)) {
        let n = contour.points.len();
        if n > MAX_CONTOUR_POINTS {
            record(
                &mut c,
                ProfileCode::AboveLimit,
                Stage::Caps,
                &contour,
                Defect::None,
                None,
            );
        } else if n < 3 {
            record(
                &mut c,
                ProfileCode::ContourInvalid,
                Stage::Caps,
                &contour,
                Defect::TooFewPoints,
                None,
            );
        }
    }
    c.into_result()
}

fn check_finite(list: &[Contour<'_>], thickness: Option<f64>) -> Result<(), ProfileFailure> {
    let mut c = Collector::default();
    if thickness.is_some_and(|t| !t.is_finite()) {
        c.push(ProfileDiagnostic {
            code: ProfileCode::NonFinite,
            field: ProfileField::Thickness,
            stage: Stage::Finiteness,
            defect: Defect::None,
            cavity: None,
            index: None,
        });
    }
    for contour in list {
        for (i, p) in contour.points.iter().enumerate() {
            if !(p.x.is_finite() && p.y.is_finite()) {
                record(
                    &mut c,
                    ProfileCode::NonFinite,
                    Stage::Finiteness,
                    contour,
                    Defect::None,
                    Some(i),
                );
            }
        }
    }
    c.into_result()
}

fn check_caps(
    outer: &[Point2],
    list: &[Contour<'_>],
    thickness: Option<f64>,
) -> Result<Tolerance, ProfileFailure> {
    let mut c = Collector::default();
    if thickness.is_some_and(|t| t > DIMENSION_CAP_MM) {
        c.push(ProfileDiagnostic {
            code: ProfileCode::AboveLimit,
            field: ProfileField::Thickness,
            stage: Stage::Caps,
            defect: Defect::None,
            cavity: None,
            index: None,
        });
    }
    for contour in list {
        for (i, p) in contour.points.iter().enumerate() {
            if p.x.abs() > COORDINATE_CAP_MM || p.y.abs() > COORDINATE_CAP_MM {
                record(
                    &mut c,
                    ProfileCode::AboveLimit,
                    Stage::Caps,
                    contour,
                    Defect::None,
                    Some(i),
                );
            }
        }
    }
    // F04 section 3: every edge, the closing edge and cavity edges included, is at most 1e6 mm.
    for contour in list {
        for i in 0..contour.points.len() {
            if edge_length(at(contour.points, i), at(contour.points, i + 1)) > DIMENSION_CAP_MM {
                record(
                    &mut c,
                    ProfileCode::AboveLimit,
                    Stage::Caps,
                    contour,
                    Defect::None,
                    Some(i),
                );
            }
        }
    }
    c.into_result()?;
    // Plate D: maximum distance from the first outer point, scanned in index order.
    let scale: Vec<Point3> = outer.iter().map(|p| Point3::new(p.x, p.y, 0.0)).collect();
    Tolerance::for_plate(&scale).map_err(|e| {
        let code = match e {
            beamcraft_geom::MathError::AboveLimit => ProfileCode::AboveLimit,
            _ => ProfileCode::NumericUncertain,
        };
        ProfileFailure::global(code, ProfileField::LocalSize, Stage::Caps)
    })
}

fn edge_length(a: Point2, b: Point2) -> f64 {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    (dx * dx + dy * dy).sqrt()
}

fn along(p: Point2, use_x: bool) -> f64 {
    if use_x { p.x } else { p.y }
}

fn contour_box(points: &[Point2]) -> (Point2, Point2) {
    let mut lo = Point2::new(f64::INFINITY, f64::INFINITY);
    let mut hi = Point2::new(f64::NEG_INFINITY, f64::NEG_INFINITY);
    for p in points {
        lo = Point2::new(lo.x.min(p.x), lo.y.min(p.y));
        hi = Point2::new(hi.x.max(p.x), hi.y.max(p.y));
    }
    (lo, hi)
}

/// Ranges and contour validity: one F06 stage, so every applicable failure is collected (at
/// most 32) in input order: the thickness, then each contour (edges, area and winding, then
/// simplicity), then contour pairs, then containment. A check runs only when its prerequisites
/// held: simplicity needs non-degenerate edges and a non-zero area, a pair check needs both
/// contours simple, containment needs a simple outer contour, a simple cavity and no
/// crossing between the two. The support decision (nested islands) is made only when the
/// whole stage found nothing invalid.
fn check_contours(
    list: &[Contour<'_>],
    thickness: Option<f64>,
    tolerance: Tolerance,
) -> Result<(), ProfileFailure> {
    let eps = tolerance.eps_geom_mm();
    let mut c = Collector::default();
    if let Some(t) = thickness {
        let code = if t <= 0.0 {
            Some(ProfileCode::NotPositive)
        } else if t < eps {
            Some(ProfileCode::BelowMinimum)
        } else {
            None
        };
        if let Some(code) = code {
            c.push(ProfileDiagnostic {
                code,
                field: ProfileField::Thickness,
                stage: Stage::Ranges,
                defect: Defect::None,
                cavity: None,
                index: None,
            });
        }
    }
    let mut simple = vec![false; list.len()];
    for (slot, contour) in simple.iter_mut().zip(list) {
        let invalid = |c: &mut Collector, defect, at_index| {
            record(
                c,
                ProfileCode::ContourInvalid,
                Stage::Ranges,
                contour,
                defect,
                at_index,
            );
        };
        let pts = contour.points;
        let mut usable = true;
        for i in 0..pts.len() {
            if edge_length(at(pts, i), at(pts, i + 1)) < eps {
                usable = false;
                invalid(&mut c, Defect::EdgeTooShort, Some(i));
            }
        }
        let sign = signed_area_sign(pts).map_err(|Uncertain| uncertain(Stage::Ranges, contour))?;
        let wanted = if contour.cavity.is_none() {
            Ordering::Greater
        } else {
            Ordering::Less
        };
        if sign == Ordering::Equal {
            usable = false;
            invalid(&mut c, Defect::ZeroArea, None);
        } else if sign != wanted {
            invalid(&mut c, Defect::WrongWinding, None);
        } else if shoelace(pts).0.abs() < eps * eps {
            invalid(&mut c, Defect::AreaTooSmall, None);
        }
        if usable {
            *slot = simplicity(contour, eps, &mut c)?;
        }
    }
    relations(list, &simple, eps, &mut c)?;
    if c.has_any() {
        return c.into_result();
    }
    nested_islands(list)
}

// Returns whether the contour is simple: no folding back, no touching or crossing between
// non-adjacent edges, and no two such edges closer than the floor.
fn simplicity(contour: &Contour<'_>, eps: f64, c: &mut Collector) -> Result<bool, ProfileFailure> {
    let pts = contour.points;
    let n = pts.len();
    let fail = |_: Uncertain| uncertain(Stage::Ranges, contour);
    let mut simple = true;
    let mut report = |c: &mut Collector, defect, at_index| {
        simple = false;
        record(
            c,
            ProfileCode::ContourInvalid,
            Stage::Ranges,
            contour,
            defect,
            Some(at_index),
        );
    };
    for v in 0..n {
        let (prev, cur, next) = (at(pts, v + n - 1), at(pts, v), at(pts, v + 1));
        if orient2d(prev, cur, next).map_err(fail)? == Ordering::Equal {
            let use_x = prev.x != cur.x;
            if (along(cur, use_x) > along(prev, use_x)) != (along(next, use_x) > along(cur, use_x))
            {
                report(c, Defect::Backtracking, v);
            }
        }
    }
    for i in 0..n {
        for j in i.saturating_add(2)..n {
            if i == 0 && j + 1 == n {
                continue;
            }
            let (a, b) = (at(pts, i), at(pts, i + 1));
            let (p, q) = (at(pts, j), at(pts, j + 1));
            if boxes_apart(a, b, p, q, 2.0 * eps) {
                continue;
            }
            if segment_relation(a, b, p, q).map_err(fail)? != SegRel::Disjoint {
                report(c, Defect::SelfIntersection, i);
            } else if segment_distance(a, b, p, q) < eps {
                report(c, Defect::NarrowFeature, i);
            }
        }
    }
    Ok(simple)
}

// Pair checks between simple contours, then containment of each cavity in the outer contour.
fn relations(
    list: &[Contour<'_>],
    simple: &[bool],
    eps: f64,
    c: &mut Collector,
) -> Result<(), ProfileFailure> {
    let is_simple = |i: usize| simple.get(i).copied().unwrap_or(false);
    let boxes: Vec<(Point2, Point2)> = list.iter().map(|k| contour_box(k.points)).collect();
    let mut crossing = vec![false; list.len().saturating_mul(list.len())];
    for (j, later) in list.iter().enumerate().skip(1) {
        for (i, earlier) in list.iter().enumerate().take(j) {
            if !(is_simple(i) && is_simple(j)) {
                continue;
            }
            let (Some((l1, h1)), Some((l2, h2))) = (boxes.get(i), boxes.get(j)) else {
                continue;
            };
            if boxes_apart(*l1, *h1, *l2, *h2, 2.0 * eps) {
                continue;
            }
            let fail = |_: Uncertain| uncertain(Stage::Ranges, later);
            let mut found = None;
            'pairs: for u in 0..earlier.points.len() {
                let (a, b) = (at(earlier.points, u), at(earlier.points, u + 1));
                for v in 0..later.points.len() {
                    let (p, q) = (at(later.points, v), at(later.points, v + 1));
                    if boxes_apart(a, b, p, q, 2.0 * eps) {
                        continue;
                    }
                    if segment_relation(a, b, p, q).map_err(fail)? != SegRel::Disjoint {
                        found = Some((Defect::ContoursIntersect, v));
                        break 'pairs;
                    }
                    if segment_distance(a, b, p, q) < eps {
                        found = Some((Defect::ContoursTooClose, v));
                        break 'pairs;
                    }
                }
            }
            if let Some((defect, v)) = found {
                if let Some(slot) = crossing.get_mut(j * list.len() + i) {
                    *slot = true;
                }
                record(
                    c,
                    ProfileCode::ContourInvalid,
                    Stage::Ranges,
                    later,
                    defect,
                    Some(v),
                );
            }
        }
    }
    // No contour pair touches or crosses, so one vertex decides containment of a whole contour.
    let Some(outer) = list.first() else {
        return Ok(());
    };
    for (j, cavity) in list.iter().enumerate().skip(1) {
        let blocked = crossing.get(j * list.len()).copied().unwrap_or(true);
        if !(is_simple(0) && is_simple(j)) || blocked {
            continue;
        }
        let fail = |_: Uncertain| uncertain(Stage::Ranges, cavity);
        if !winding_contains(at(cavity.points, 0), outer.points).map_err(fail)? {
            record(
                c,
                ProfileCode::ContourInvalid,
                Stage::Ranges,
                cavity,
                Defect::CavityOutside,
                None,
            );
        }
    }
    Ok(())
}

// The support decision, reached only when the whole stage found nothing invalid.
fn nested_islands(list: &[Contour<'_>]) -> Result<(), ProfileFailure> {
    let mut nested = Collector::default();
    for (j, cavity) in list.iter().enumerate().skip(1) {
        let fail = |_: Uncertain| uncertain(Stage::Ranges, cavity);
        for (i, other) in list.iter().enumerate().skip(1) {
            if i != j && winding_contains(at(cavity.points, 0), other.points).map_err(fail)? {
                record(
                    &mut nested,
                    ProfileCode::UnsupportedContour,
                    Stage::Ranges,
                    cavity,
                    Defect::CavityNested,
                    None,
                );
                break;
            }
        }
    }
    nested.into_result()
}
