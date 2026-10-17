// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! Deterministic cap triangulation: hole bridging followed by ear clipping.
//!
//! Every decision is an exact orientation sign. Each cavity is joined to the outer polygon
//! by one bridge segment that touches no contour edge or earlier bridge except at the
//! shared endpoints (and never runs along one), producing one weakly simple cycle in which
//! the bridge endpoints appear twice. Ear clipping then removes strictly convex vertices
//! whose triangle holds no other vertex, closed boundary included, other than copies of its
//! own corners. The result is not trusted: the extrusion self-check proves it is a tiling
//! (every triangle positive, every interior edge cancelled), which by the winding-number
//! argument implies no overlap, gap or T-junction.
//!
//! Work is bounded by a fixed budget of exact orientation evaluations; exhausting it is a
//! typed failure, never a hang. No recursion is used.

use super::diagnostic::{Defect, ProfileCode, ProfileFailure, ProfileField, Stage};
use super::exact::{Uncertain, orient2d};
use super::predicates::{SegRel, boxes_apart, segment_relation};
use super::{Point2, Profile};
use core::cmp::Ordering;

/// Largest number of exact orientation evaluations of one triangulation.
pub(crate) const WORK_BUDGET: u64 = 150_000_000;

enum TriError {
    Uncertain,
    Stuck,
    Budget,
}

impl From<Uncertain> for TriError {
    fn from(_: Uncertain) -> Self {
        Self::Uncertain
    }
}

impl From<TriError> for ProfileFailure {
    fn from(error: TriError) -> Self {
        let (code, defect) = match error {
            TriError::Uncertain => (ProfileCode::NumericUncertain, Defect::None),
            TriError::Stuck => (ProfileCode::SelfCheckFailed, Defect::None),
            TriError::Budget => (ProfileCode::AboveLimit, Defect::WorkBudget),
        };
        Self::one(code, ProfileField::Mesh, Stage::Build, defect, None, None)
    }
}

struct Tri {
    pts: Vec<Point2>,
    edges: Vec<(u32, u32)>,
    bridges: Vec<(u32, u32)>,
    budget: u64,
}

pub(crate) fn triangulate(profile: &Profile) -> Result<Vec<[u32; 3]>, ProfileFailure> {
    let mut pts: Vec<Point2> = profile.outer().to_vec();
    let mut spans: Vec<(usize, usize)> = vec![(0, profile.outer().len())];
    for cavity in profile.cavities() {
        spans.push((pts.len(), cavity.points().len()));
        pts.extend_from_slice(cavity.points());
    }
    let to_id = |i: usize| u32::try_from(i).map_err(|_| TriError::Stuck);
    let mut edges: Vec<(u32, u32)> = Vec::with_capacity(pts.len());
    let mut rings: Vec<(u32, u32)> = Vec::with_capacity(spans.len());
    for (s, &(start, len)) in spans.iter().enumerate() {
        for k in 0..len {
            edges.push((to_id(start + k)?, to_id(start + (k + 1) % len)?));
        }
        if s > 0 {
            rings.push((to_id(start)?, to_id(len)?));
        }
    }
    let outer_len = to_id(profile.outer().len())?;
    let mut tri = Tri {
        pts,
        edges,
        bridges: Vec::new(),
        budget: WORK_BUDGET,
    };
    let cycle = tri.bridge_holes(outer_len, &rings)?;
    Ok(tri.clip_ears(&cycle)?)
}

impl Tri {
    fn spend(&mut self, units: u64) -> Result<(), TriError> {
        self.budget = self.budget.checked_sub(units).ok_or(TriError::Budget)?;
        Ok(())
    }

    fn pt(&self, id: u32) -> Result<Point2, TriError> {
        usize::try_from(id)
            .ok()
            .and_then(|i| self.pts.get(i))
            .copied()
            .ok_or(TriError::Stuck)
    }

    fn orient(&mut self, a: Point2, b: Point2, c: Point2) -> Result<Ordering, TriError> {
        self.spend(1)?;
        Ok(orient2d(a, b, c)?)
    }

    // Interior-cone test at `v` for a counter-clockwise boundary (material on the left).
    fn in_cone(
        &mut self,
        prev: Point2,
        v: Point2,
        next: Point2,
        target: Point2,
    ) -> Result<bool, TriError> {
        if self.orient(v, next, prev)? != Ordering::Less {
            Ok(self.orient(v, target, prev)? == Ordering::Greater
                && self.orient(target, v, next)? == Ordering::Greater)
        } else {
            let a = self.orient(v, target, next)? != Ordering::Less;
            let b = self.orient(target, v, prev)? != Ordering::Less;
            Ok(!(a && b))
        }
    }

    // Whether the bridge `m-v` conflicts with an edge `e0-e1` (by vertex id).
    fn conflicts(&mut self, m: u32, v: u32, e0: u32, e1: u32) -> Result<bool, TriError> {
        let (pm, pv) = (self.pt(m)?, self.pt(v)?);
        let (p0, p1) = (self.pt(e0)?, self.pt(e1)?);
        if boxes_apart(pm, pv, p0, p1, 0.0) {
            return Ok(false);
        }
        let edge_has = |s: u32| e0 == s || e1 == s;
        let (share_m, share_v) = (edge_has(m), edge_has(v));
        if share_m && share_v {
            return Ok(true);
        }
        if !share_m && !share_v {
            self.spend(4)?;
            return Ok(segment_relation(pm, pv, p0, p1)? != SegRel::Disjoint);
        }
        // One shared endpoint: only a collinear run in the same direction is a conflict.
        let (shared_id, s, y) = if share_m { (m, pm, pv) } else { (v, pv, pm) };
        let x = if e0 == shared_id { p1 } else { p0 };
        if self.orient(s, x, y)? != Ordering::Equal {
            return Ok(false);
        }
        let use_x = s.x != x.x;
        let pick = |p: Point2| if use_x { p.x } else { p.y };
        Ok((pick(x) > pick(s)) == (pick(y) > pick(s)))
    }

    fn bridge_ok(&mut self, m: u32, v: u32) -> Result<bool, TriError> {
        let scanned = self.edges.len().saturating_add(self.bridges.len());
        self.spend(u64::try_from(scanned).map_err(|_| TriError::Budget)?)?;
        for k in 0..self.edges.len() {
            let Some(&(e0, e1)) = self.edges.get(k) else {
                continue;
            };
            if self.conflicts(m, v, e0, e1)? {
                return Ok(false);
            }
        }
        for k in 0..self.bridges.len() {
            let Some(&(e0, e1)) = self.bridges.get(k) else {
                continue;
            };
            if self.conflicts(m, v, e0, e1)? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn bridge_holes(&mut self, outer_len: u32, rings: &[(u32, u32)]) -> Result<Vec<u32>, TriError> {
        let mut cycle: Vec<u32> = (0..outer_len).collect();
        // Rightmost vertex of each ring (first one on ties) and ring order by it, descending.
        let mut order: Vec<(f64, usize, u32)> = Vec::with_capacity(rings.len());
        for (h, &(start, len)) in rings.iter().enumerate() {
            let mut best: Option<(f64, u32)> = None;
            for id in start..start.saturating_add(len) {
                let x = self.pt(id)?.x;
                if best.is_none_or(|(bx, _)| x > bx) {
                    best = Some((x, id));
                }
            }
            let (x, id) = best.ok_or(TriError::Stuck)?;
            order.push((x, h, id));
        }
        order.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
        for (_, h, m) in order {
            let &(start, len) = rings.get(h).ok_or(TriError::Stuck)?;
            let pm = self.pt(m)?;
            let local = m - start;
            let before = start + (local + len - 1) % len;
            let after = start + (local + 1) % len;
            let (pb, pa) = (self.pt(before)?, self.pt(after)?);
            let mut candidates: Vec<(f64, usize)> = Vec::with_capacity(cycle.len());
            for (q, &id) in cycle.iter().enumerate() {
                let p = self.pt(id)?;
                let (dx, dy) = (p.x - pm.x, p.y - pm.y);
                candidates.push((dx * dx + dy * dy, q));
            }
            candidates.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
            let n = cycle.len();
            let mut chosen = None;
            for &(_, q) in &candidates {
                let v = *cycle.get(q).ok_or(TriError::Stuck)?;
                let pv = self.pt(v)?;
                if !self.in_cone(pb, pm, pa, pv)? {
                    continue;
                }
                let prev = *cycle.get((q + n - 1) % n).ok_or(TriError::Stuck)?;
                let next = *cycle.get((q + 1) % n).ok_or(TriError::Stuck)?;
                let (pp, pn) = (self.pt(prev)?, self.pt(next)?);
                if !self.in_cone(pp, pv, pn, pm)? || !self.bridge_ok(m, v)? {
                    continue;
                }
                chosen = Some((q, v));
                break;
            }
            let (q, v) = chosen.ok_or(TriError::Stuck)?;
            let mut insert: Vec<u32> = (0..len).map(|t| start + (local + t) % len).collect();
            insert.push(m);
            insert.push(v);
            let at = q + 1;
            cycle.splice(at..at, insert);
            self.bridges.push((m, v));
        }
        Ok(cycle)
    }

    fn clip_ears(&mut self, cycle: &[u32]) -> Result<Vec<[u32; 3]>, TriError> {
        let n = cycle.len();
        let mut prev: Vec<usize> = (0..n).map(|i| (i + n - 1) % n).collect();
        let mut next: Vec<usize> = (0..n).map(|i| (i + 1) % n).collect();
        let mut alive = vec![true; n];
        let mut remaining = n;
        let mut current = 0usize;
        let mut out: Vec<[u32; 3]> = Vec::with_capacity(n.saturating_sub(2));
        let id = |i: usize| cycle.get(i).copied().ok_or(TriError::Stuck);
        let link = |v: &Vec<usize>, i: usize| v.get(i).copied().ok_or(TriError::Stuck);
        while remaining > 3 {
            let mut found = None;
            let mut c = current;
            for _ in 0..remaining {
                if self.is_ear(cycle, &prev, &next, &alive, c)? {
                    found = Some(c);
                    break;
                }
                c = link(&next, c)?;
            }
            let e = found.ok_or(TriError::Stuck)?;
            let (a, b) = (link(&prev, e)?, link(&next, e)?);
            out.push([id(a)?, id(e)?, id(b)?]);
            *next.get_mut(a).ok_or(TriError::Stuck)? = b;
            *prev.get_mut(b).ok_or(TriError::Stuck)? = a;
            *alive.get_mut(e).ok_or(TriError::Stuck)? = false;
            remaining -= 1;
            current = a;
        }
        let (a, b) = (current, link(&next, current)?);
        let c = link(&next, b)?;
        let (pa, pb, pc) = (self.pt(id(a)?)?, self.pt(id(b)?)?, self.pt(id(c)?)?);
        if remaining != 3 || self.orient(pa, pb, pc)? != Ordering::Greater {
            return Err(TriError::Stuck);
        }
        out.push([id(a)?, id(b)?, id(c)?]);
        Ok(out)
    }

    fn is_ear(
        &mut self,
        cycle: &[u32],
        prev: &[usize],
        next: &[usize],
        alive: &[bool],
        c: usize,
    ) -> Result<bool, TriError> {
        let get = |v: &[usize], i: usize| v.get(i).copied().ok_or(TriError::Stuck);
        let (a, b) = (get(prev, c)?, get(next, c)?);
        let ids = [a, c, b].map(|i| cycle.get(i).copied());
        let [Some(ia), Some(ic), Some(ib)] = ids else {
            return Err(TriError::Stuck);
        };
        let (pa, pc, pb) = (self.pt(ia)?, self.pt(ic)?, self.pt(ib)?);
        if self.orient(pa, pc, pb)? != Ordering::Greater {
            return Ok(false);
        }
        let (lo_x, hi_x) = (pa.x.min(pc.x).min(pb.x), pa.x.max(pc.x).max(pb.x));
        let (lo_y, hi_y) = (pa.y.min(pc.y).min(pb.y), pa.y.max(pc.y).max(pb.y));
        for (w, &alive_w) in alive.iter().enumerate() {
            if !alive_w || w == a || w == c || w == b {
                continue;
            }
            let iw = cycle.get(w).copied().ok_or(TriError::Stuck)?;
            if iw == ia || iw == ic || iw == ib {
                continue;
            }
            self.spend(1)?;
            let pw = self.pt(iw)?;
            if pw.x < lo_x || pw.x > hi_x || pw.y < lo_y || pw.y > hi_y {
                continue;
            }
            if self.orient(pa, pc, pw)? != Ordering::Less
                && self.orient(pc, pb, pw)? != Ordering::Less
                && self.orient(pb, pa, pw)? != Ordering::Less
            {
                return Ok(false);
            }
        }
        Ok(true)
    }
}
