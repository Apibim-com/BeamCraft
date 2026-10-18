// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! V04 `Pick` (F06 section 9) and snap queries over a pinned view snapshot.
//!
//! Both are read-only, bounded and answered at exactly the snapshot's revision. Every hit names
//! an element and a FeatureId (or, for an axis snap, an element, a role and a parameter) and
//! carries world positions computed by the engine; no hit contains a triangle, vertex or buffer
//! index. Only bodies with FeatureIds (canonical boxes of plates, beams and columns) are scanned;
//! the others are listed in [`PickResult::skipped`] rather than approximated.
//!
//! Pick follows F06 exactly: closed face rectangles enlarged by `eps_pick` with the parallel
//! test `|u . n| < 1e-6`; edge and vertex distances to the closed finite segment
//! `{o + t*u : 0 <= t <= t_max}` with `t` clamped at its ends; the point form measures to the
//! closed features; acceptance at `radius_mm + eps_pick`; ordering by the quantized parameter or
//! distance, then ElementId, kind rank, local id and `t`; at most `max_hits` hits with a
//! `truncated` flag. Not implemented: the cancellation token and request deadline polls (the
//! engine's query API has no cancellation yet); the scan is bounded by the element and feature
//! caps instead.
//!
//! Snap (V04) finds, around a point, box vertices (`vertex`), the start and end of a member's
//! axis (`axis_end`) and the nearest point of a member's axis segment (`axis`). The axis of a
//! member box runs between the centres of its `f.z0` and `f.z1` faces, which are the authored
//! axis points plus the translation (F06 section 6).

use crate::element::ElementId;
use crate::error::{ViewCode, ViewError};
use crate::query::feature::{
    BodyKind, BoxFeatures, FeatureId, FeatureKind, V3, add, dot, finite, norm, scale, sub,
};
use crate::query::snapshot::{SnapshotToken, ViewSnapshot};
use core::cmp::Ordering;

/// The pick band of F06 section 9: one micrometre.
pub const EPS_PICK_MM: f64 = 0.001;
/// Largest absolute origin or point coordinate.
pub const MAX_COORDINATE_MM: f64 = 4_000_000_000.0;
/// Largest absolute direction component.
pub const MAX_DIRECTION_COMPONENT: f64 = 1_000_000_000.0;
/// Shortest direction before normalization.
pub const MIN_DIRECTION_NORM: f64 = 1e-6;
/// Default and largest ray length.
pub const MAX_T_MM: f64 = 4_000_000_000.0;
/// Largest query radius.
pub const MAX_RADIUS_MM: f64 = 1_000_000.0;
/// Default hit count.
pub const DEFAULT_MAX_HITS: u32 = 16;
/// Largest hit count.
pub const MAX_HITS: u32 = 64;
/// Most elements one query scans.
pub const MAX_SCANNED_ELEMENTS: usize = 65_536;
/// A face is parallel to the ray, and not hit, when `|u . n|` is below this.
const PARALLEL: f64 = 1e-6;
/// Work bound of the kept candidates before they are sorted and cut back to `max_hits`.
const KEEP: usize = 1024;

/// The ray form of a pick.
#[derive(Debug, Clone, PartialEq)]
pub struct RayPick {
    /// Ray origin in world millimetres.
    pub origin_mm: [f64; 3],
    /// Ray direction; normalized by the engine.
    pub direction: [f64; 3],
    /// Largest ray parameter.
    pub t_max_mm: f64,
    /// Feature kinds to hit: nonempty, no repeats.
    pub targets: Vec<FeatureKind>,
    /// Edge and vertex acceptance radius.
    pub radius_mm: f64,
    /// Most hits returned.
    pub max_hits: u32,
    /// Whether back-facing faces are hit.
    pub include_backfaces: bool,
}

impl RayPick {
    /// A ray pick with the F06 defaults (`t_max` 4e9 mm, radius 0, 16 hits, front faces only).
    pub fn new(origin_mm: [f64; 3], direction: [f64; 3], targets: Vec<FeatureKind>) -> Self {
        Self {
            origin_mm,
            direction,
            t_max_mm: MAX_T_MM,
            targets,
            radius_mm: 0.0,
            max_hits: DEFAULT_MAX_HITS,
            include_backfaces: false,
        }
    }
}

/// The point form of a pick.
#[derive(Debug, Clone, PartialEq)]
pub struct PointPick {
    /// Query point in world millimetres.
    pub point_mm: [f64; 3],
    /// Acceptance radius.
    pub radius_mm: f64,
    /// Feature kinds to find: nonempty, no repeats.
    pub targets: Vec<FeatureKind>,
    /// Most hits returned.
    pub max_hits: u32,
}

impl PointPick {
    /// A point pick with the F06 defaults (radius 0, 16 hits).
    pub fn new(point_mm: [f64; 3], targets: Vec<FeatureKind>) -> Self {
        Self {
            point_mm,
            radius_mm: 0.0,
            targets,
            max_hits: DEFAULT_MAX_HITS,
        }
    }
}

/// One pick request.
#[derive(Debug, Clone, PartialEq)]
pub enum PickQuery {
    /// Ray form.
    Ray(RayPick),
    /// Point form.
    Point(PointPick),
}

/// One pick hit (F06 hit record).
#[derive(Debug, Clone, PartialEq)]
pub struct Hit {
    /// The owning element.
    pub element_id: ElementId,
    /// The feature.
    pub feature_id: FeatureId,
    /// Its kind.
    pub feature_kind: FeatureKind,
    /// Ray parameter (ray form only).
    pub t_mm: Option<f64>,
    /// Distance to the feature; 0 for a ray hit on a face.
    pub distance_mm: f64,
    /// The hit point (ray on a face) or the closest point of the feature.
    pub point_world_mm: [f64; 3],
    /// Outward unit normal of a face, `None` otherwise.
    pub normal_world: Option<[f64; 3]>,
    /// Whether the face faces away from the ray (or the point lies behind it).
    pub back_facing: bool,
    /// Whether a face hit lies on the exact rectangle's boundary or in its enlargement.
    pub boundary: bool,
}

/// The answer of [`pick`].
#[derive(Debug, Clone, PartialEq)]
pub struct PickResult {
    /// The snapshot the answer belongs to.
    pub token: SnapshotToken,
    /// Hits in F06 order, at most `max_hits`.
    pub hits: Vec<Hit>,
    /// True when more hits qualified than were returned.
    pub truncated: bool,
    /// Elements whose bodies have no FeatureIds and were not scanned, in id order.
    pub skipped: Vec<ElementId>,
}

fn non_finite() -> ViewError {
    ViewError::new(ViewCode::NonFinite)
}
fn input() -> ViewError {
    ViewError::new(ViewCode::PickInputInvalid)
}
fn limit() -> ViewError {
    ViewError::new(ViewCode::PickLimit)
}
fn uncertain() -> ViewError {
    ViewError::new(ViewCode::NumericUncertain)
}

fn check_point(p: [f64; 3]) -> Result<(), ViewError> {
    if p.iter().any(|v| v.abs() > MAX_COORDINATE_MM) {
        return Err(input());
    }
    Ok(())
}

fn check_common<T: PartialEq>(radius: f64, max_hits: u32, targets: &[T]) -> Result<(), ViewError> {
    if max_hits > MAX_HITS || radius > MAX_RADIUS_MM {
        return Err(limit());
    }
    if max_hits == 0 || radius < 0.0 || targets.is_empty() {
        return Err(input());
    }
    for (i, a) in targets.iter().enumerate() {
        if targets.iter().skip(i + 1).any(|b| b == a) {
            return Err(input());
        }
    }
    Ok(())
}

/// The normalized direction of a validated ray.
fn validate_ray(ray: &RayPick) -> Result<V3, ViewError> {
    let numbers = ray
        .origin_mm
        .iter()
        .chain(&ray.direction)
        .chain([&ray.t_max_mm, &ray.radius_mm]);
    if numbers.into_iter().any(|v| !v.is_finite()) {
        return Err(non_finite());
    }
    check_common(ray.radius_mm, ray.max_hits, &ray.targets)?;
    check_point(ray.origin_mm)?;
    if ray.t_max_mm < 0.0
        || ray.t_max_mm > MAX_T_MM
        || ray
            .direction
            .iter()
            .any(|v| v.abs() > MAX_DIRECTION_COMPONENT)
    {
        return Err(input());
    }
    let length = norm(ray.direction);
    if !length.is_finite() {
        return Err(uncertain());
    }
    if length < MIN_DIRECTION_NORM {
        return Err(input());
    }
    Ok(scale(ray.direction, 1.0 / length))
}

fn validate_point(point: &PointPick) -> Result<(), ViewError> {
    if point
        .point_mm
        .iter()
        .chain([&point.radius_mm])
        .any(|v| !v.is_finite())
    {
        return Err(non_finite());
    }
    check_common(point.radius_mm, point.max_hits, &point.targets)?;
    check_point(point.point_mm)
}

/// The closest point of the closed segment `[a, b]` to `p`, and its parameter in `[0, 1]`.
fn closest_on_segment(p: V3, a: V3, b: V3) -> (V3, f64) {
    let d = sub(b, a);
    let len2 = dot(d, d);
    let s = if len2 > 0.0 {
        (dot(sub(p, a), d) / len2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    (add(a, scale(d, s)), s)
}

/// The closest pair between the ray segment `o + s*d1`, `s` in `[0, 1]`, and the edge
/// `p + t*d2`, `t` in `[0, 1]`: the smallest `s` among minimizing pairs, the edge point and the
/// distance.
fn segment_segment(o: V3, d1: V3, p: V3, q: V3) -> (f64, V3, f64) {
    let d2 = sub(q, p);
    let r = sub(o, p);
    let (a, e) = (dot(d1, d1), dot(d2, d2));
    let at = |s: f64| add(o, scale(d1, s));
    let measure = |s: f64| {
        let (point, _) = closest_on_segment(at(s), p, q);
        (s, point, norm(sub(at(s), point)))
    };
    if a <= 0.0 {
        return measure(0.0);
    }
    let (b, c, f) = (dot(d1, d2), dot(d1, r), dot(d2, r));
    let denom = a * e - b * b;
    if e > 0.0 && denom > 1e-12 * a * e {
        // Not parallel: one minimizing pair.
        let mut s = ((b * f - c * e) / denom).clamp(0.0, 1.0);
        let t = (b * s + f) / e;
        if t < 0.0 {
            s = (-c / a).clamp(0.0, 1.0);
        } else if t > 1.0 {
            s = ((b - c) / a).clamp(0.0, 1.0);
        }
        return measure(s);
    }
    // Parallel (or a degenerate edge): the distance is minimal at a segment end or at the
    // projection of an edge end; among equal distances the smallest parameter wins.
    let project = |x: V3| (dot(sub(x, o), d1) / a).clamp(0.0, 1.0);
    let mut best = measure(0.0);
    for s in [project(p), project(q), 1.0] {
        let candidate = measure(s);
        let tie = (candidate.2 - best.2).abs() <= 1e-12 * (1.0 + best.2);
        if candidate.2 < best.2 && !tie || tie && candidate.0 < best.0 {
            best = candidate;
        }
    }
    best
}

fn wanted(targets: &[FeatureKind], kind: FeatureKind) -> bool {
    targets.contains(&kind)
}

fn hits_of(features: &BoxFeatures, query: &PickQuery, dir: V3) -> Result<Vec<Hit>, ViewError> {
    let mut out = Vec::new();
    let element_id = features.element;
    let band = |radius: f64| radius + EPS_PICK_MM;
    let id = |kind, role| features.id(kind, role).ok_or_else(uncertain);
    match query {
        PickQuery::Ray(ray) => {
            let o = ray.origin_mm;
            if wanted(&ray.targets, FeatureKind::Face) {
                for (role, face) in features.faces.iter().enumerate() {
                    let n = face.normal;
                    let un = dot(dir, n);
                    if un.abs() < PARALLEL {
                        continue;
                    }
                    let back = un > 0.0;
                    if back && !ray.include_backfaces {
                        continue;
                    }
                    let [a, b, _, d] = face.corners;
                    let t = dot(sub(a, o), n) / un;
                    if !t.is_finite() {
                        return Err(uncertain());
                    }
                    if !(0.0..=ray.t_max_mm).contains(&t) {
                        continue;
                    }
                    let hit = add(o, scale(dir, t));
                    let (eu, ev) = (sub(b, a), sub(d, a));
                    let (lu, lv) = (norm(eu), norm(ev));
                    let rel = sub(hit, a);
                    let (s, r) = (dot(rel, eu) / lu, dot(rel, ev) / lv);
                    if !(s.is_finite() && r.is_finite() && finite(hit)) {
                        return Err(uncertain());
                    }
                    let enlarged = |x: f64, l: f64| (-EPS_PICK_MM..=l + EPS_PICK_MM).contains(&x);
                    if !(enlarged(s, lu) && enlarged(r, lv)) {
                        continue;
                    }
                    out.push(Hit {
                        element_id,
                        feature_id: id(FeatureKind::Face, role)?,
                        feature_kind: FeatureKind::Face,
                        t_mm: Some(t),
                        distance_mm: 0.0,
                        point_world_mm: hit,
                        normal_world: Some(n),
                        back_facing: back,
                        boundary: !(s > 0.0 && s < lu && r > 0.0 && r < lv),
                    });
                }
            }
            let d1 = scale(dir, ray.t_max_mm);
            if wanted(&ray.targets, FeatureKind::Edge) {
                for role in 0..12 {
                    let [p, q] = features.edge(role).ok_or_else(uncertain)?;
                    let (s, point, distance) = segment_segment(o, d1, p, q);
                    let t = s * ray.t_max_mm;
                    if !(distance.is_finite() && t.is_finite() && finite(point)) {
                        return Err(uncertain());
                    }
                    if distance <= band(ray.radius_mm) {
                        out.push(Hit {
                            element_id,
                            feature_id: id(FeatureKind::Edge, role)?,
                            feature_kind: FeatureKind::Edge,
                            t_mm: Some(t),
                            distance_mm: distance,
                            point_world_mm: point,
                            normal_world: None,
                            back_facing: false,
                            boundary: false,
                        });
                    }
                }
            }
            if wanted(&ray.targets, FeatureKind::Vertex) {
                for (role, v) in features.vertices.iter().enumerate() {
                    let t = dot(sub(*v, o), dir).clamp(0.0, ray.t_max_mm);
                    let distance = norm(sub(add(o, scale(dir, t)), *v));
                    if !(distance.is_finite() && t.is_finite()) {
                        return Err(uncertain());
                    }
                    if distance <= band(ray.radius_mm) {
                        out.push(Hit {
                            element_id,
                            feature_id: id(FeatureKind::Vertex, role)?,
                            feature_kind: FeatureKind::Vertex,
                            t_mm: Some(t),
                            distance_mm: distance,
                            point_world_mm: *v,
                            normal_world: None,
                            back_facing: false,
                            boundary: false,
                        });
                    }
                }
            }
        }
        PickQuery::Point(point) => {
            let p = point.point_mm;
            if wanted(&point.targets, FeatureKind::Face) {
                for (role, face) in features.faces.iter().enumerate() {
                    let [a, b, _, d] = face.corners;
                    let (eu, ev) = (sub(b, a), sub(d, a));
                    let (lu, lv) = (norm(eu), norm(ev));
                    let rel = sub(p, a);
                    let (s, r) = (dot(rel, eu) / lu, dot(rel, ev) / lv);
                    let (cs, cr) = (s.clamp(0.0, lu), r.clamp(0.0, lv));
                    let q = add(a, add(scale(eu, cs / lu), scale(ev, cr / lv)));
                    let distance = norm(sub(p, q));
                    if !(distance.is_finite() && finite(q)) {
                        return Err(uncertain());
                    }
                    if distance <= band(point.radius_mm) {
                        out.push(Hit {
                            element_id,
                            feature_id: id(FeatureKind::Face, role)?,
                            feature_kind: FeatureKind::Face,
                            t_mm: None,
                            distance_mm: distance,
                            point_world_mm: q,
                            normal_world: Some(face.normal),
                            back_facing: dot(sub(p, q), face.normal) < 0.0,
                            boundary: !(s > 0.0 && s < lu && r > 0.0 && r < lv),
                        });
                    }
                }
            }
            if wanted(&point.targets, FeatureKind::Edge) {
                for role in 0..12 {
                    let [a, b] = features.edge(role).ok_or_else(uncertain)?;
                    let (q, _) = closest_on_segment(p, a, b);
                    let distance = norm(sub(p, q));
                    if !distance.is_finite() {
                        return Err(uncertain());
                    }
                    if distance <= band(point.radius_mm) {
                        out.push(Hit {
                            element_id,
                            feature_id: id(FeatureKind::Edge, role)?,
                            feature_kind: FeatureKind::Edge,
                            t_mm: None,
                            distance_mm: distance,
                            point_world_mm: q,
                            normal_world: None,
                            back_facing: false,
                            boundary: false,
                        });
                    }
                }
            }
            if wanted(&point.targets, FeatureKind::Vertex) {
                for (role, v) in features.vertices.iter().enumerate() {
                    let distance = norm(sub(p, *v));
                    if !distance.is_finite() {
                        return Err(uncertain());
                    }
                    if distance <= band(point.radius_mm) {
                        out.push(Hit {
                            element_id,
                            feature_id: id(FeatureKind::Vertex, role)?,
                            feature_kind: FeatureKind::Vertex,
                            t_mm: None,
                            distance_mm: distance,
                            point_world_mm: *v,
                            normal_world: None,
                            back_facing: false,
                            boundary: false,
                        });
                    }
                }
            }
        }
    }
    Ok(out)
}

/// The F06 quantum of a parameter or distance.
fn quantum(x: f64) -> f64 {
    (x / EPS_PICK_MM).floor()
}

fn order(a: &Hit, b: &Hit) -> Ordering {
    let key = |h: &Hit| quantum(h.t_mm.unwrap_or(h.distance_mm));
    key(a)
        .total_cmp(&key(b))
        .then_with(|| a.element_id.to_string().cmp(&b.element_id.to_string()))
        .then_with(|| a.feature_kind.rank().cmp(&b.feature_kind.rank()))
        .then_with(|| a.feature_id.local_id().cmp(&b.feature_id.local_id()))
        .then_with(|| {
            a.t_mm
                .unwrap_or(a.distance_mm)
                .total_cmp(&b.t_mm.unwrap_or(b.distance_mm))
        })
}

/// Keeps the best `max` of `items` under `cmp`, and whether more than `max` qualified.
fn keep_best<T>(items: &mut Vec<T>, max: usize, cmp: impl Fn(&T, &T) -> Ordering) -> bool {
    items.sort_by(&cmp);
    let truncated = items.len() > max;
    items.truncate(max);
    truncated
}

/// `Pick`: the ordered hits of a ray or point query against the snapshot.
///
/// Errors carry no partial hits: a non-finite number is `GEO_NON_FINITE`; `max_hits` above 64,
/// a radius above 1,000,000 mm or more than 65,536 elements is `GEO_PICK_LIMIT`; a zero hit
/// count, a negative radius or `t_max`, a `t_max` above 4e9 mm, an out-of-range origin, point or
/// direction component, a too-short direction or empty or repeated targets is
/// `GEO_PICK_INPUT_INVALID`; a non-finite intermediate is `GEO_NUMERIC_UNCERTAIN`.
pub fn pick(snapshot: &ViewSnapshot, query: &PickQuery) -> Result<PickResult, ViewError> {
    let (dir, max_hits) = match query {
        PickQuery::Ray(ray) => (validate_ray(ray)?, ray.max_hits),
        PickQuery::Point(point) => {
            validate_point(point)?;
            ([0.0; 3], point.max_hits)
        }
    };
    if snapshot.len() > MAX_SCANNED_ELEMENTS {
        return Err(limit());
    }
    let max = usize::try_from(max_hits).map_err(|_| limit())?;
    let mut hits = Vec::new();
    let mut truncated = false;
    let mut skipped = Vec::new();
    for entry in snapshot.elements() {
        let Some(features) = BoxFeatures::world(snapshot, entry) else {
            skipped.push(entry.id);
            continue;
        };
        hits.extend(hits_of(&features, query, dir)?);
        if hits.len() > KEEP {
            truncated |= keep_best(&mut hits, max, order);
        }
    }
    truncated |= keep_best(&mut hits, max, order);
    Ok(PickResult {
        token: snapshot.token().clone(),
        hits,
        truncated,
        skipped,
    })
}

/// What a snap candidate snaps to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapKind {
    /// A box vertex.
    Vertex,
    /// The start or end point of a member's axis.
    AxisEnd,
    /// The nearest point of a member's axis segment.
    Axis,
}

impl SnapKind {
    /// The wire token.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Vertex => "vertex",
            Self::AxisEnd => "axis_end",
            Self::Axis => "axis",
        }
    }

    /// Parses an exact wire token.
    pub fn parse(token: &str) -> Option<Self> {
        match token {
            "vertex" => Some(Self::Vertex),
            "axis_end" => Some(Self::AxisEnd),
            "axis" => Some(Self::Axis),
            _ => None,
        }
    }

    const fn rank(self) -> u8 {
        match self {
            Self::Vertex => 0,
            Self::AxisEnd => 1,
            Self::Axis => 2,
        }
    }
}

/// One snap request.
#[derive(Debug, Clone, PartialEq)]
pub struct SnapQuery {
    /// Query point in world millimetres.
    pub point_mm: [f64; 3],
    /// Acceptance radius.
    pub radius_mm: f64,
    /// What to snap to: nonempty, no repeats.
    pub kinds: Vec<SnapKind>,
    /// Most candidates returned.
    pub max_hits: u32,
}

/// One snap candidate.
#[derive(Debug, Clone, PartialEq)]
pub struct SnapCandidate {
    /// The owning element.
    pub element_id: ElementId,
    /// What it snaps to.
    pub kind: SnapKind,
    /// The vertex FeatureId of a `vertex` candidate.
    pub feature_id: Option<FeatureId>,
    /// For axis candidates, the position along the axis from its start (0) to its end (1).
    pub axis_parameter: Option<f64>,
    /// The snap point in world millimetres.
    pub point_world_mm: [f64; 3],
    /// Distance from the query point.
    pub distance_mm: f64,
}

/// The answer of [`snap`].
#[derive(Debug, Clone, PartialEq)]
pub struct SnapResult {
    /// The snapshot the answer belongs to.
    pub token: SnapshotToken,
    /// Candidates ordered by quantized distance, kind, ElementId, FeatureId and parameter.
    pub candidates: Vec<SnapCandidate>,
    /// True when more candidates qualified than were returned.
    pub truncated: bool,
    /// Elements whose bodies have no FeatureIds and were not scanned, in id order.
    pub skipped: Vec<ElementId>,
}

fn snap_order(a: &SnapCandidate, b: &SnapCandidate) -> Ordering {
    let local = |c: &SnapCandidate| c.feature_id.as_ref().map(FeatureId::local_id);
    quantum(a.distance_mm)
        .total_cmp(&quantum(b.distance_mm))
        .then_with(|| a.kind.rank().cmp(&b.kind.rank()))
        .then_with(|| a.element_id.to_string().cmp(&b.element_id.to_string()))
        .then_with(|| local(a).cmp(&local(b)))
        .then_with(|| {
            a.axis_parameter
                .unwrap_or(0.0)
                .total_cmp(&b.axis_parameter.unwrap_or(0.0))
        })
}

/// Snap: the candidates within `radius_mm + eps_pick` of a point.
///
/// Inputs follow the point form of [`pick`] (same codes and caps); `kinds` replaces `targets`.
pub fn snap(snapshot: &ViewSnapshot, query: &SnapQuery) -> Result<SnapResult, ViewError> {
    if query
        .point_mm
        .iter()
        .chain([&query.radius_mm])
        .any(|v| !v.is_finite())
    {
        return Err(non_finite());
    }
    check_common(query.radius_mm, query.max_hits, &query.kinds)?;
    check_point(query.point_mm)?;
    if snapshot.len() > MAX_SCANNED_ELEMENTS {
        return Err(limit());
    }
    let max = usize::try_from(query.max_hits).map_err(|_| limit())?;
    let p = query.point_mm;
    let band = query.radius_mm + EPS_PICK_MM;
    let mut candidates = Vec::new();
    let mut truncated = false;
    let mut skipped = Vec::new();
    for entry in snapshot.elements() {
        let Some(features) = BoxFeatures::world(snapshot, entry) else {
            skipped.push(entry.id);
            continue;
        };
        let mut push = |kind, feature_id, axis_parameter, point: V3| -> Result<(), ViewError> {
            let distance = norm(sub(p, point));
            if !(distance.is_finite() && finite(point)) {
                return Err(uncertain());
            }
            if distance <= band {
                candidates.push(SnapCandidate {
                    element_id: entry.id,
                    kind,
                    feature_id,
                    axis_parameter,
                    point_world_mm: point,
                    distance_mm: distance,
                });
            }
            Ok(())
        };
        if query.kinds.contains(&SnapKind::Vertex) {
            for (role, v) in features.vertices.iter().enumerate() {
                let id = features
                    .id(FeatureKind::Vertex, role)
                    .ok_or_else(uncertain)?;
                push(SnapKind::Vertex, Some(id), None, *v)?;
            }
        }
        if features.body == BodyKind::Member {
            // Face roles 4 and 5 are `f.z0` (at the axis start) and `f.z1` (at its end).
            let start = features.face_centre(4).ok_or_else(uncertain)?;
            let end = features.face_centre(5).ok_or_else(uncertain)?;
            if query.kinds.contains(&SnapKind::AxisEnd) {
                push(SnapKind::AxisEnd, None, Some(0.0), start)?;
                push(SnapKind::AxisEnd, None, Some(1.0), end)?;
            }
            if query.kinds.contains(&SnapKind::Axis) {
                let (point, s) = closest_on_segment(p, start, end);
                push(SnapKind::Axis, None, Some(s), point)?;
            }
        }
        if candidates.len() > KEEP {
            truncated |= keep_best(&mut candidates, max, snap_order);
        }
    }
    truncated |= keep_best(&mut candidates, max, snap_order);
    Ok(SnapResult {
        token: snapshot.token().clone(),
        candidates,
        truncated,
        skipped,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::limits::Limits;
    use crate::query::snapshot::ViewPin;
    use crate::synthetic::plate_scene;
    use beamcraft_model::digest::Sha256;

    fn snapshot(count: u32) -> ViewSnapshot {
        let pin = ViewPin::new(Sha256::digest(b"v04").unwrap(), 1);
        ViewSnapshot::build(pin, plate_scene(count, false).unwrap(), &Limits::default()).unwrap()
    }

    fn bounds(s: &ViewSnapshot, i: usize) -> ([f64; 3], [f64; 3]) {
        let b = s.elements()[i].full.world_bounds;
        (b.min, b.max)
    }

    #[test]
    fn a_vertical_ray_hits_the_top_face_first_and_the_bottom_only_from_behind() {
        let s = snapshot(1);
        let (min, max) = bounds(&s, 0);
        let centre = [
            (min[0] + max[0]) / 2.0,
            (min[1] + max[1]) / 2.0,
            max[2] + 100.0,
        ];
        let ray = RayPick::new(centre, [0.0, 0.0, -1.0], vec![FeatureKind::Face]);
        let result = pick(&s, &PickQuery::Ray(ray.clone())).unwrap();
        assert_eq!(result.hits.len(), 1);
        let top = &result.hits[0];
        assert!(top.feature_id.to_string().ends_with(":plate:f.z1"));
        assert_eq!(top.t_mm, Some(100.0));
        assert_eq!(top.normal_world, Some([0.0, 0.0, 1.0]));
        assert!(!top.back_facing && !top.boundary);
        let both = pick(
            &s,
            &PickQuery::Ray(RayPick {
                include_backfaces: true,
                ..ray
            }),
        )
        .unwrap();
        assert_eq!(both.hits.len(), 2);
        assert!(both.hits[1].back_facing);
        assert!(both.hits[1].feature_id.to_string().ends_with(":plate:f.z0"));
    }

    #[test]
    fn edge_and_vertex_bands_are_inclusive_at_eps_and_clamp_t_at_the_segment_end() {
        let s = snapshot(1);
        let (_, max) = bounds(&s, 0);
        // A ray that stops 0.0009 mm before the top corner: the vertex qualifies by distance to
        // the segment end, t is the end, never extended.
        let origin = [max[0], max[1], max[2] + 10.0];
        let mut ray = RayPick::new(origin, [0.0, 0.0, -1.0], vec![FeatureKind::Vertex]);
        ray.t_max_mm = 10.0 - 0.0009;
        let hits = pick(&s, &PickQuery::Ray(ray.clone())).unwrap().hits;
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].t_mm, Some(ray.t_max_mm));
        assert!(
            hits[0]
                .feature_id
                .to_string()
                .ends_with(":plate:v.x1.y1.z1")
        );
        ray.t_max_mm = 10.0 - 0.0011;
        assert!(pick(&s, &PickQuery::Ray(ray)).unwrap().hits.is_empty());
        // Point form: an edge just inside the band qualifies, just outside it does not (a point
        // at exactly 0.001 mm is not representable at this magnitude).
        let (min, max) = bounds(&s, 0);
        let mid_x = (min[0] + max[0]) / 2.0;
        let near = PointPick::new([mid_x, max[1] + 0.0009, max[2]], vec![FeatureKind::Edge]);
        let hits = pick(&s, &PickQuery::Point(near)).unwrap().hits;
        assert_eq!(hits.len(), 1);
        assert!(hits[0].feature_id.to_string().ends_with(":plate:e.y1.z1"));
        let far = PointPick::new([mid_x, max[1] + 0.0011, max[2]], vec![FeatureKind::Edge]);
        assert!(pick(&s, &PickQuery::Point(far)).unwrap().hits.is_empty());
    }

    #[test]
    fn hits_are_ordered_truncated_and_carry_no_index() {
        let s = snapshot(3);
        let (min, _) = bounds(&s, 0);
        let (_, max) = bounds(&s, 2);
        // A ray along all three plates' bottom-front edge line collects many features.
        let ray = RayPick {
            radius_mm: 1.0,
            max_hits: 5,
            ..RayPick::new(
                [min[0] - 10.0, min[1], min[2]],
                [1.0, 0.0, 0.0],
                vec![FeatureKind::Edge, FeatureKind::Vertex],
            )
        };
        let result = pick(&s, &PickQuery::Ray(ray)).unwrap();
        assert_eq!(result.hits.len(), 5);
        assert!(result.truncated);
        for pair in result.hits.windows(2) {
            assert_ne!(order(&pair[0], &pair[1]), Ordering::Greater);
        }
        assert!(max[0] > min[0]);
    }

    #[test]
    fn inputs_are_validated_before_any_scan() {
        let s = snapshot(1);
        let code = |q: PickQuery| pick(&s, &q).unwrap_err().code();
        let ray = RayPick::new([0.0; 3], [0.0, 0.0, 1.0], vec![FeatureKind::Face]);
        let with = |f: &dyn Fn(&mut RayPick)| {
            let mut r = ray.clone();
            f(&mut r);
            PickQuery::Ray(r)
        };
        assert_eq!(
            code(with(&|r| r.origin_mm[0] = f64::NAN)),
            ViewCode::NonFinite
        );
        assert_eq!(code(with(&|r| r.max_hits = 65)), ViewCode::PickLimit);
        assert_eq!(
            code(with(&|r| r.radius_mm = 1_000_001.0)),
            ViewCode::PickLimit
        );
        for bad in [
            with(&|r| r.max_hits = 0),
            with(&|r| r.radius_mm = -1.0),
            with(&|r| r.t_max_mm = -1.0),
            with(&|r| r.direction = [0.0, 0.0, 1e-7]),
            with(&|r| r.targets.clear()),
            with(&|r| r.targets.push(FeatureKind::Face)),
            with(&|r| r.origin_mm[2] = 4.1e9),
        ] {
            assert_eq!(code(bad), ViewCode::PickInputInvalid);
        }
    }

    #[test]
    fn snap_finds_vertices_and_skips_plates_for_axes() {
        let s = snapshot(1);
        let (_, max) = bounds(&s, 0);
        let query = SnapQuery {
            point_mm: [max[0] + 0.5, max[1], max[2]],
            radius_mm: 1.0,
            kinds: vec![SnapKind::Vertex, SnapKind::Axis, SnapKind::AxisEnd],
            max_hits: 8,
        };
        let result = snap(&s, &query).unwrap();
        assert_eq!(result.candidates.len(), 1);
        let c = &result.candidates[0];
        assert_eq!(c.kind, SnapKind::Vertex);
        assert_eq!(c.point_world_mm, max);
        assert_eq!(c.distance_mm, 0.5);
        let repeated = SnapQuery {
            kinds: vec![SnapKind::Axis, SnapKind::Axis],
            ..query
        };
        assert_eq!(
            snap(&s, &repeated).unwrap_err().code(),
            ViewCode::PickInputInvalid
        );
    }
}
