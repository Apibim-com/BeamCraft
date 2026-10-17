// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! Axis-aligned boxes and a bounded static box-tree index.
//!
//! All comparisons are exact binary64 comparisons of the stored endpoints, so
//! closed-box overlap is decided without tolerance; growing a box by a margin
//! rounds outward. The tree has explicit item, node, pair and result caps, is
//! built and traversed without recursion, and never allocates from an
//! unvalidated count. Query results are returned in ascending item order.

use crate::trimesh::MAX_TRIANGLES;

/// Largest item count an index accepts.
pub const MAX_INDEX_ITEMS: usize = MAX_TRIANGLES;
/// Largest node count an index may use (a binary tree over at most 8192 items).
pub const MAX_INDEX_NODES: usize = 2 * MAX_INDEX_ITEMS;
const LEAF_SIZE: usize = 4;
const POLL_INTERVAL: u64 = 1024;

/// A closed axis-aligned box with finite corners, `min <= max` on every axis.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Aabb {
    /// Componentwise minimum corner, millimetres.
    pub min: [f64; 3],
    /// Componentwise maximum corner, millimetres.
    pub max: [f64; 3],
}

impl Aabb {
    /// Validates finite corners with `min <= max` on every axis.
    pub fn new(min: [f64; 3], max: [f64; 3]) -> Option<Self> {
        let ordered = min
            .iter()
            .zip(&max)
            .all(|(lo, hi)| lo.is_finite() && hi.is_finite() && lo <= hi);
        ordered.then_some(Self { min, max })
    }

    /// Exact bounds of finite points; `None` for an empty or non-finite set.
    pub fn from_points(points: impl IntoIterator<Item = [f64; 3]>) -> Option<Self> {
        let mut bounds: Option<Self> = None;
        for p in points {
            if p.iter().any(|v| !v.is_finite()) {
                return None;
            }
            bounds = Some(match bounds {
                None => Self { min: p, max: p },
                Some(b) => b.union(&Self { min: p, max: p }),
            });
        }
        bounds
    }

    /// Smallest box containing both.
    pub fn union(&self, other: &Self) -> Self {
        Self {
            min: [0, 1, 2].map(|i| lane(&self.min, i).min(lane(&other.min, i))),
            max: [0, 1, 2].map(|i| lane(&self.max, i).max(lane(&other.max, i))),
        }
    }

    /// Closed-box overlap: touching boxes overlap.
    pub fn overlaps(&self, other: &Self) -> bool {
        (0..3).all(|i| {
            lane(&self.min, i) <= lane(&other.max, i) && lane(&other.min, i) <= lane(&self.max, i)
        })
    }

    /// True when `other` lies inside `self`, boundary included.
    pub fn contains_box(&self, other: &Self) -> bool {
        (0..3).all(|i| {
            lane(&self.min, i) <= lane(&other.min, i) && lane(&other.max, i) <= lane(&self.max, i)
        })
    }

    /// True when the point lies inside, boundary included.
    pub fn contains_point(&self, p: [f64; 3]) -> bool {
        (0..3).all(|i| lane(&self.min, i) <= lane(&p, i) && lane(&p, i) <= lane(&self.max, i))
    }

    /// Grows every side by `margin >= 0`, rounding each new bound outward.
    pub fn inflated(&self, margin: f64) -> Option<Self> {
        if !(margin.is_finite() && margin >= 0.0) {
            return None;
        }
        Self::new(
            self.min.map(|v| (v - margin).next_down()),
            self.max.map(|v| (v + margin).next_up()),
        )
    }

    fn centre_on(&self, axis: usize) -> f64 {
        lane(&self.min, axis) * 0.5 + lane(&self.max, axis) * 0.5
    }
}

fn lane(v: &[f64; 3], i: usize) -> f64 {
    v.get(i).copied().unwrap_or(f64::NAN)
}

/// Why an index build or query stopped without a result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexError {
    /// More items than the item cap, or none.
    ItemCap,
    /// The tree would need more nodes than the node cap.
    NodeCap,
    /// More candidate pairs than the pair cap.
    PairCap,
    /// More query results than the caller's limit.
    ResultCap,
    /// The cancellation callback requested a stop.
    Cancelled,
    /// A pair callback asked to stop the scan early.
    Halted,
}

#[derive(Debug, Clone, Copy)]
struct Node {
    bounds: Aabb,
    /// Leaf: first slot in `order`. Internal: left child node.
    first: u32,
    /// Leaf: item count. Internal: right child node.
    second: u32,
    leaf: bool,
}

/// Static median-split box tree over a fixed list of boxes.
#[derive(Debug, Clone)]
pub struct BoxIndex {
    boxes: Vec<Aabb>,
    order: Vec<u32>,
    nodes: Vec<Node>,
}

impl BoxIndex {
    /// Builds the tree. `max_nodes` is clamped to [`MAX_INDEX_NODES`]; `cancelled`
    /// is polled every 1024 node constructions and at entry.
    pub fn build(
        boxes: &[Aabb],
        max_nodes: usize,
        mut cancelled: impl FnMut() -> bool,
    ) -> Result<Self, IndexError> {
        if boxes.is_empty() || boxes.len() > MAX_INDEX_ITEMS {
            return Err(IndexError::ItemCap);
        }
        let max_nodes = max_nodes.min(MAX_INDEX_NODES);
        // The root is itself a node: a zero budget cannot hold even a one-leaf tree.
        if max_nodes == 0 {
            return Err(IndexError::NodeCap);
        }
        if cancelled() {
            return Err(IndexError::Cancelled);
        }
        let count = u32::try_from(boxes.len()).map_err(|_| IndexError::ItemCap)?;
        let mut order: Vec<u32> = (0..count).collect();
        let mut nodes: Vec<Node> = Vec::with_capacity(2 * boxes.len() / LEAF_SIZE + 2);
        let mut stack: Vec<(usize, usize, usize)> = Vec::with_capacity(32);
        let placeholder = Node {
            bounds: boxes.first().copied().ok_or(IndexError::ItemCap)?,
            first: 0,
            second: 0,
            leaf: true,
        };
        nodes.push(placeholder);
        stack.push((0, 0, boxes.len()));
        let mut built: u64 = 0;
        while let Some((node, start, end)) = stack.pop() {
            built += 1;
            if built.is_multiple_of(POLL_INTERVAL) && cancelled() {
                return Err(IndexError::Cancelled);
            }
            let slice = order.get(start..end).ok_or(IndexError::ItemCap)?;
            let mut bounds = boxes
                .get(*slice.first().ok_or(IndexError::ItemCap)? as usize)
                .copied()
                .ok_or(IndexError::ItemCap)?;
            for &id in slice {
                bounds = bounds.union(boxes.get(id as usize).ok_or(IndexError::ItemCap)?);
            }
            let make_leaf = end - start <= LEAF_SIZE;
            let slot = nodes.get_mut(node).ok_or(IndexError::NodeCap)?;
            if make_leaf {
                *slot = Node {
                    bounds,
                    first: u32::try_from(start).map_err(|_| IndexError::ItemCap)?,
                    second: u32::try_from(end - start).map_err(|_| IndexError::ItemCap)?,
                    leaf: true,
                };
                continue;
            }
            if nodes.len() + 2 > max_nodes {
                return Err(IndexError::NodeCap);
            }
            let axis = (0..3)
                .map(|a| {
                    let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
                    for &id in order.get(start..end).unwrap_or(&[]) {
                        let c = boxes.get(id as usize).map_or(0.0, |b| b.centre_on(a));
                        lo = lo.min(c);
                        hi = hi.max(c);
                    }
                    (hi - lo, a)
                })
                .fold(
                    (-1.0, 0),
                    |best, cur| if cur.0 > best.0 { cur } else { best },
                )
                .1;
            if let Some(part) = order.get_mut(start..end) {
                part.sort_unstable_by(|x, y| {
                    let key = |id: &u32| boxes.get(*id as usize).map_or(0.0, |b| b.centre_on(axis));
                    key(x).total_cmp(&key(y)).then(x.cmp(y))
                });
            }
            let left = nodes.len();
            nodes.push(placeholder);
            nodes.push(placeholder);
            let mid = start + (end - start) / 2;
            if let Some(slot) = nodes.get_mut(node) {
                *slot = Node {
                    bounds,
                    first: u32::try_from(left).map_err(|_| IndexError::NodeCap)?,
                    second: u32::try_from(left + 1).map_err(|_| IndexError::NodeCap)?,
                    leaf: false,
                };
            }
            stack.push((left + 1, mid, end));
            stack.push((left, start, mid));
        }
        Ok(Self {
            boxes: boxes.to_vec(),
            order,
            nodes,
        })
    }

    /// Number of indexed boxes.
    pub fn len(&self) -> usize {
        self.boxes.len()
    }

    /// True for an index without boxes (never produced by `build`).
    pub fn is_empty(&self) -> bool {
        self.boxes.is_empty()
    }

    /// Number of tree nodes, at most [`MAX_INDEX_NODES`].
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// The box stored for `item`.
    pub fn item_box(&self, item: u32) -> Option<&Aabb> {
        self.boxes.get(usize::try_from(item).ok()?)
    }

    /// Visits every item whose box overlaps `query`; `visit` returns false to stop.
    /// Returns the number of nodes touched. Order is traversal order.
    fn walk(&self, query: &Aabb, mut visit: impl FnMut(u32) -> bool) -> u64 {
        let mut stack: Vec<u32> = Vec::with_capacity(32);
        stack.push(0);
        let mut touched = 0;
        while let Some(n) = stack.pop() {
            let Some(node) = usize::try_from(n).ok().and_then(|i| self.nodes.get(i)) else {
                continue;
            };
            touched += 1;
            if !node.bounds.overlaps(query) {
                continue;
            }
            if node.leaf {
                let from = node.first as usize;
                let to = from + node.second as usize;
                for &id in self.order.get(from..to).unwrap_or(&[]) {
                    let hit = self
                        .boxes
                        .get(id as usize)
                        .is_some_and(|b| b.overlaps(query));
                    if hit && !visit(id) {
                        return touched;
                    }
                }
            } else {
                stack.push(node.second);
                stack.push(node.first);
            }
        }
        touched
    }

    /// All items whose box overlaps `query`, ascending; fails above `limit` results.
    pub fn overlapping(&self, query: &Aabb, limit: usize) -> Result<Vec<u32>, IndexError> {
        let mut out = Vec::new();
        let mut over = false;
        self.walk(query, |id| {
            if out.len() >= limit {
                over = true;
                return false;
            }
            out.push(id);
            true
        });
        if over {
            return Err(IndexError::ResultCap);
        }
        out.sort_unstable();
        Ok(out)
    }

    /// Calls `f(i, j)` for every pair `i < j` of overlapping boxes, in ascending
    /// `i` then ascending `j` order. Fails once more than `max_pairs` pairs exist;
    /// `cancelled` is polled about every 1024 units of work.
    pub fn for_each_overlapping_pair(
        &self,
        max_pairs: u64,
        mut cancelled: impl FnMut() -> bool,
        mut f: impl FnMut(u32, u32) -> Result<(), IndexError>,
    ) -> Result<(), IndexError> {
        let mut pairs: u64 = 0;
        let mut work: u64 = 0;
        let mut partners: Vec<u32> = Vec::new();
        for (i, query) in self.boxes.iter().enumerate() {
            let me = u32::try_from(i).map_err(|_| IndexError::ItemCap)?;
            partners.clear();
            work += self.walk(query, |j| {
                if j > me {
                    partners.push(j);
                }
                true
            });
            partners.sort_unstable();
            for &j in &partners {
                pairs += 1;
                work += 1;
                if pairs > max_pairs {
                    return Err(IndexError::PairCap);
                }
                if work >= POLL_INTERVAL {
                    work = 0;
                    if cancelled() {
                        return Err(IndexError::Cancelled);
                    }
                }
                f(me, j)?;
            }
            if work >= POLL_INTERVAL {
                work = 0;
                if cancelled() {
                    return Err(IndexError::Cancelled);
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cube(x: f64, y: f64, z: f64, s: f64) -> Aabb {
        Aabb::new([x, y, z], [x + s, y + s, z + s]).unwrap()
    }

    fn lattice(n: usize) -> Vec<Aabb> {
        let mut boxes = Vec::new();
        for i in 0..n {
            for j in 0..n {
                for k in 0..n {
                    // Neighbouring cubes overlap by exactly their shared faces.
                    boxes.push(cube(i as f64 * 2.0, j as f64 * 2.0, k as f64 * 2.0, 2.0));
                }
            }
        }
        boxes
    }

    #[test]
    fn closed_overlap_includes_touching_and_inflation_is_outward() {
        let a = cube(0.0, 0.0, 0.0, 1.0);
        assert!(a.overlaps(&cube(1.0, 1.0, 1.0, 1.0)));
        assert!(!a.overlaps(&cube(1.0000000000000002, 0.0, 0.0, 1.0)));
        let grown = a.inflated(0.0).unwrap();
        assert!(grown.min[0] < 0.0 && grown.max[0] > 1.0 && grown.contains_box(&a));
        assert!(a.inflated(-1.0).is_none() && a.inflated(f64::NAN).is_none());
        assert!(Aabb::new([1.0, 0.0, 0.0], [0.0, 0.0, 0.0]).is_none());
        assert!(Aabb::from_points([[0.0, 0.0, f64::NAN]]).is_none());
    }

    #[test]
    fn queries_match_brute_force() {
        let boxes = lattice(5);
        let index = BoxIndex::build(&boxes, MAX_INDEX_NODES, || false).unwrap();
        for q in [
            cube(1.0, 1.0, 1.0, 0.5),
            cube(-3.0, -3.0, -3.0, 20.0),
            cube(3.9, 3.9, 3.9, 0.1),
        ] {
            let expected: Vec<u32> = boxes
                .iter()
                .enumerate()
                .filter(|(_, b)| b.overlaps(&q))
                .map(|(i, _)| i as u32)
                .collect();
            assert_eq!(index.overlapping(&q, 10_000).unwrap(), expected);
        }
        assert!(!index.is_empty());
        assert!(index.node_count() <= MAX_INDEX_NODES);
    }

    #[test]
    fn pairs_match_brute_force_in_stable_order() {
        let boxes = lattice(4);
        let index = BoxIndex::build(&boxes, MAX_INDEX_NODES, || false).unwrap();
        let mut got = Vec::new();
        index
            .for_each_overlapping_pair(
                u64::MAX,
                || false,
                |i, j| {
                    got.push((i, j));
                    Ok(())
                },
            )
            .unwrap();
        let mut expected = Vec::new();
        for i in 0..boxes.len() {
            for j in i + 1..boxes.len() {
                if boxes[i].overlaps(&boxes[j]) {
                    expected.push((i as u32, j as u32));
                }
            }
        }
        assert_eq!(got, expected);
    }

    #[test]
    fn caps_are_enforced_at_cap_plus_one() {
        let ok = vec![cube(0.0, 0.0, 0.0, 1.0); MAX_INDEX_ITEMS];
        let index = BoxIndex::build(&ok, MAX_INDEX_NODES, || false).unwrap();
        assert!(index.node_count() <= MAX_INDEX_NODES);
        let over = vec![cube(0.0, 0.0, 0.0, 1.0); MAX_INDEX_ITEMS + 1];
        assert_eq!(
            BoxIndex::build(&over, MAX_INDEX_NODES, || false).unwrap_err(),
            IndexError::ItemCap
        );
        assert_eq!(
            BoxIndex::build(&[], 4, || false).unwrap_err(),
            IndexError::ItemCap
        );
        // A zero node budget rejects even a one-leaf index (the root is a node).
        for n in 1..=4 {
            let few: Vec<Aabb> = lattice(2).into_iter().take(n).collect();
            assert_eq!(
                BoxIndex::build(&few, 0, || false).unwrap_err(),
                IndexError::NodeCap
            );
            assert_eq!(BoxIndex::build(&few, 1, || false).unwrap().node_count(), 1);
        }
        let boxes = lattice(3);
        assert_eq!(
            BoxIndex::build(&boxes, 1, || false).unwrap_err(),
            IndexError::NodeCap
        );
        let index = BoxIndex::build(&boxes, MAX_INDEX_NODES, || false).unwrap();
        assert_eq!(
            index
                .for_each_overlapping_pair(3, || false, |_, _| Ok(()))
                .unwrap_err(),
            IndexError::PairCap
        );
        assert_eq!(
            index
                .overlapping(&cube(-1.0, -1.0, -1.0, 100.0), 26)
                .unwrap_err(),
            IndexError::ResultCap
        );
        assert_eq!(
            index
                .overlapping(&cube(-1.0, -1.0, -1.0, 100.0), 27)
                .unwrap()
                .len(),
            27
        );
    }

    #[test]
    fn cancellation_is_observed_in_build_and_pair_scan() {
        let boxes = lattice(6);
        assert_eq!(
            BoxIndex::build(&boxes, MAX_INDEX_NODES, || true).unwrap_err(),
            IndexError::Cancelled
        );
        let index = BoxIndex::build(&boxes, MAX_INDEX_NODES, || false).unwrap();
        assert_eq!(
            index
                .for_each_overlapping_pair(u64::MAX, || true, |_, _| Ok(()))
                .unwrap_err(),
            IndexError::Cancelled
        );
    }
}
