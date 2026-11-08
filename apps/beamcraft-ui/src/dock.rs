// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! The tab layout of the work area: the open view panes sit as tabs in tab groups, and the groups
//! split the work area side by side or one above the other. Dragging a tab onto the middle of a
//! group moves it there; onto a group's edge it splits that group and the tab gets a group of its
//! own on that side. A group whose last tab leaves closes, and its neighbour takes its room.
//!
//! Layout state only: the panes themselves (cameras, targets) live in `crate::views::Views`, and
//! this module knows them by their keys. Nothing here touches the model.

use egui::{Rect, pos2, vec2};

/// Most tab groups the work area is split into.
pub const MAX_GROUPS: usize = 8;
/// Height of a group's tab strip, points.
pub const TAB_STRIP: f32 = 26.0;
/// Width of the band a divider between two groups is dragged by, points.
pub const DIVIDER: f32 = 6.0;
/// The share of a group's width or height that counts as its edge for a drop.
const EDGE_SHARE: f32 = 0.25;
/// The smallest share a divider leaves either side.
const MIN_RATIO: f32 = 0.1;

/// A side of a tab group.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Side {
    /// Left of the group.
    Left,
    /// Right of the group.
    Right,
    /// Above the group.
    Top,
    /// Below the group.
    Bottom,
}

/// Where a dragged tab goes.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Drop {
    /// Into the group, as its current tab.
    Into(u64),
    /// Into a new group on `Side` of the group.
    Beside(u64, Side),
}

/// One tab group: its tabs in strip order and the one it shows.
#[derive(Clone, Debug, PartialEq)]
pub struct Group {
    /// Stable identity within the window.
    pub id: u64,
    /// The pane keys of its tabs, left to right.
    pub tabs: Vec<u64>,
    /// The tab it shows.
    pub current: Option<u64>,
}

#[derive(Clone, Debug, PartialEq)]
enum Node {
    Leaf(u64),
    Split {
        /// Side by side (a vertical divider) when true, one above the other when false.
        across: bool,
        /// The first child's share of the room.
        ratio: f32,
        first: Box<Node>,
        second: Box<Node>,
    },
}

/// A divider between two parts of the layout, for resizing.
#[derive(Clone, Debug, PartialEq)]
pub struct Divider {
    /// Which split it is: the path of first (false) and second (true) children from the root.
    pub path: Vec<bool>,
    /// The band it is dragged by.
    pub handle: Rect,
    /// Side by side (drag horizontally) when true.
    pub across: bool,
    /// The room the split divides.
    pub room: Rect,
}

/// The work area's tab layout.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Dock {
    root: Option<Node>,
    groups: Vec<Group>,
    serial: u64,
    active: Option<u64>,
    maximized: Option<u64>,
}

fn split(rect: Rect, across: bool, ratio: f32) -> (Rect, Rect) {
    let ratio = ratio.clamp(MIN_RATIO, 1.0 - MIN_RATIO);
    if across {
        let x = rect.min.x + rect.width() * ratio;
        (
            Rect::from_min_max(rect.min, pos2(x, rect.max.y)),
            Rect::from_min_max(pos2(x, rect.min.y), rect.max),
        )
    } else {
        let y = rect.min.y + rect.height() * ratio;
        (
            Rect::from_min_max(rect.min, pos2(rect.max.x, y)),
            Rect::from_min_max(pos2(rect.min.x, y), rect.max),
        )
    }
}

/// Equal shares: `items` side by side (or stacked), each split taking `1/n` of what is left.
fn chain(mut items: Vec<Node>, across: bool) -> Option<Node> {
    let last = items.pop()?;
    let mut node = last;
    let mut count = 1.0f32;
    while let Some(item) = items.pop() {
        count += 1.0;
        node = Node::Split {
            across,
            ratio: 1.0 / count,
            first: Box::new(item),
            second: Box::new(node),
        };
    }
    Some(node)
}

impl Node {
    fn leaves(&self, out: &mut Vec<u64>) {
        match self {
            Self::Leaf(id) => out.push(*id),
            Self::Split { first, second, .. } => {
                first.leaves(out);
                second.leaves(out);
            }
        }
    }

    /// This node without the leaf `id`; `None` when nothing is left.
    fn without(self, id: u64) -> Option<Self> {
        match self {
            Self::Leaf(leaf) => (leaf != id).then_some(self),
            Self::Split {
                across,
                ratio,
                first,
                second,
            } => match (first.without(id), second.without(id)) {
                (Some(first), Some(second)) => Some(Self::Split {
                    across,
                    ratio,
                    first: Box::new(first),
                    second: Box::new(second),
                }),
                (Some(only), None) | (None, Some(only)) => Some(only),
                (None, None) => None,
            },
        }
    }

    /// This node with the leaf `at` split: `fresh` goes on `side` of it.
    fn beside(self, at: u64, fresh: u64, side: Side) -> Self {
        match self {
            Self::Leaf(leaf) if leaf == at => {
                let (across, fresh_first) = match side {
                    Side::Left => (true, true),
                    Side::Right => (true, false),
                    Side::Top => (false, true),
                    Side::Bottom => (false, false),
                };
                let (first, second) = if fresh_first {
                    (Self::Leaf(fresh), self)
                } else {
                    (self, Self::Leaf(fresh))
                };
                Self::Split {
                    across,
                    ratio: 0.5,
                    first: Box::new(first),
                    second: Box::new(second),
                }
            }
            Self::Leaf(_) => self,
            Self::Split {
                across,
                ratio,
                first,
                second,
            } => Self::Split {
                across,
                ratio,
                first: Box::new(first.beside(at, fresh, side)),
                second: Box::new(second.beside(at, fresh, side)),
            },
        }
    }

    fn layout(
        &self,
        room: Rect,
        out: &mut Vec<(u64, Rect)>,
        dividers: &mut Vec<Divider>,
        path: &mut Vec<bool>,
    ) {
        match self {
            Self::Leaf(id) => out.push((*id, room)),
            Self::Split {
                across,
                ratio,
                first,
                second,
            } => {
                let (a, b) = split(room, *across, *ratio);
                let handle = if *across {
                    Rect::from_center_size(
                        pos2(a.max.x, room.center().y),
                        vec2(DIVIDER, room.height()),
                    )
                } else {
                    Rect::from_center_size(
                        pos2(room.center().x, a.max.y),
                        vec2(room.width(), DIVIDER),
                    )
                };
                dividers.push(Divider {
                    path: path.clone(),
                    handle,
                    across: *across,
                    room,
                });
                path.push(false);
                first.layout(a, out, dividers, path);
                path.pop();
                path.push(true);
                second.layout(b, out, dividers, path);
                path.pop();
            }
        }
    }

    fn ratio_at(&mut self, path: &[bool]) -> Option<&mut f32> {
        match (self, path.split_first()) {
            (Self::Split { ratio, .. }, None) => Some(ratio),
            (Self::Split { first, second, .. }, Some((go_second, rest))) => {
                if *go_second {
                    second.ratio_at(rest)
                } else {
                    first.ratio_at(rest)
                }
            }
            (Self::Leaf(_), _) => None,
        }
    }
}

impl Dock {
    /// The groups in no particular order.
    pub fn groups(&self) -> &[Group] {
        &self.groups
    }

    /// The group with this identity.
    pub fn group(&self, id: u64) -> Option<&Group> {
        self.groups.iter().find(|g| g.id == id)
    }

    /// The group holding the tab `key`.
    pub fn group_of(&self, key: u64) -> Option<u64> {
        self.groups
            .iter()
            .find(|g| g.tabs.contains(&key))
            .map(|g| g.id)
    }

    /// Whether `key` is the tab its group shows.
    pub fn is_shown(&self, key: u64) -> bool {
        self.groups.iter().any(|g| g.current == Some(key))
    }

    /// The group the work area is maximized to, if any.
    pub fn maximized(&self) -> Option<u64> {
        self.maximized
    }

    /// The group of the active tab, the one new tabs open in.
    pub fn active_group(&self) -> Option<u64> {
        self.active
    }

    fn fresh_group(&mut self, tabs: Vec<u64>) -> u64 {
        self.serial = self.serial.saturating_add(1);
        let id = self.serial;
        let current = tabs.first().copied();
        self.groups.push(Group { id, tabs, current });
        id
    }

    /// Takes `key` out of its group; a group left empty closes and its neighbour takes its room.
    pub fn detach(&mut self, key: u64) {
        let Some(id) = self.group_of(key) else {
            return;
        };
        let mut empty = false;
        if let Some(group) = self.groups.iter_mut().find(|g| g.id == id) {
            let at = group.tabs.iter().position(|k| *k == key);
            group.tabs.retain(|k| *k != key);
            if group.current == Some(key) {
                // The neighbour on the left shows, as a closed browser tab does.
                group.current = at
                    .and_then(|i| group.tabs.get(i.saturating_sub(1)))
                    .or_else(|| group.tabs.first())
                    .copied();
            }
            empty = group.tabs.is_empty();
        }
        if empty {
            self.groups.retain(|g| g.id != id);
            self.root = self.root.take().and_then(|r| r.without(id));
            if self.maximized == Some(id) {
                self.maximized = None;
            }
            if self.active == Some(id) {
                self.active = self.groups.first().map(|g| g.id);
            }
        }
    }

    /// Puts `key` where `drop` says, taking it out of its group first. A split past
    /// [`MAX_GROUPS`] or onto an unknown group opens the tab in the target group instead (or the
    /// active one).
    pub fn insert(&mut self, key: u64, drop: Drop) {
        let target = match drop {
            Drop::Into(id) | Drop::Beside(id, _) => id,
        };
        // A lone tab dropped beside its own group goes nowhere new.
        if let Drop::Beside(id, _) = drop
            && self.group(id).is_some_and(|g| g.tabs == [key])
        {
            return;
        }
        self.detach(key);
        let target = if self.group(target).is_some() {
            Some(target)
        } else {
            self.active.or_else(|| self.groups.first().map(|g| g.id))
        };
        let Some(target) = target else {
            let id = self.fresh_group(vec![key]);
            self.root = Some(Node::Leaf(id));
            self.active = Some(id);
            return;
        };
        match drop {
            Drop::Beside(_, side) if self.groups.len() < MAX_GROUPS => {
                let id = self.fresh_group(vec![key]);
                self.root = self.root.take().map(|r| r.beside(target, id, side));
                self.maximized = None;
                self.active = Some(id);
            }
            _ => {
                if let Some(group) = self.groups.iter_mut().find(|g| g.id == target) {
                    group.tabs.push(key);
                    group.current = Some(key);
                }
                self.active = Some(target);
            }
        }
    }

    /// Shows `key` in its group and makes that group active.
    pub fn select(&mut self, key: u64) {
        if let Some(group) = self.groups.iter_mut().find(|g| g.tabs.contains(&key)) {
            group.current = Some(key);
            self.active = Some(group.id);
        }
    }

    /// Follows the panes: a key not in the layout yet opens as a tab of the active group, a
    /// layout key without a pane leaves, and the active pane is shown in its group.
    pub fn sync(&mut self, keys: &[u64], active: Option<u64>) {
        let known: Vec<u64> = self.groups.iter().flat_map(|g| g.tabs.clone()).collect();
        for key in known.iter().filter(|k| !keys.contains(k)) {
            self.detach(*key);
        }
        for key in keys.iter().filter(|k| !known.contains(k)) {
            let target = self.active.unwrap_or(0);
            self.insert(*key, Drop::Into(target));
        }
        if let Some(key) = active {
            self.select(key);
        }
    }

    /// Every group with its rect in `area`: the maximized group alone fills it.
    pub fn layout(&self, area: Rect) -> Vec<(u64, Rect)> {
        if let Some(id) = self.maximized.filter(|id| self.group(*id).is_some()) {
            return vec![(id, area)];
        }
        let mut out = Vec::new();
        if let Some(root) = &self.root {
            root.layout(area, &mut out, &mut Vec::new(), &mut Vec::new());
        }
        out
    }

    /// The dividers between groups, for resizing; none while a group is maximized.
    pub fn dividers(&self, area: Rect) -> Vec<Divider> {
        let mut dividers = Vec::new();
        if self.maximized.is_none()
            && let Some(root) = &self.root
        {
            root.layout(area, &mut Vec::new(), &mut dividers, &mut Vec::new());
        }
        dividers
    }

    /// Moves the divider at `path` to the pointer at `pos` within `room`.
    pub fn drag_divider(&mut self, path: &[bool], room: Rect, across: bool, pos: egui::Pos2) {
        let Some(ratio) = self.root.as_mut().and_then(|r| r.ratio_at(path)) else {
            return;
        };
        let share = if across {
            (pos.x - room.min.x) / room.width().max(1.0)
        } else {
            (pos.y - room.min.y) / room.height().max(1.0)
        };
        if share.is_finite() {
            *ratio = share.clamp(MIN_RATIO, 1.0 - MIN_RATIO);
        }
    }

    /// Where a drop at `pos` goes: the group under it, by edge or middle of its `body`.
    pub fn drop_at(&self, area: Rect, pos: egui::Pos2) -> Option<(Drop, Rect)> {
        let (id, rect) = self
            .layout(area)
            .into_iter()
            .find(|(_, rect)| rect.contains(pos))?;
        let body = Rect::from_min_max(pos2(rect.min.x, rect.min.y + TAB_STRIP), rect.max);
        if !body.contains(pos) {
            return Some((Drop::Into(id), rect));
        }
        let fx = (pos.x - body.min.x) / body.width().max(1.0);
        let fy = (pos.y - body.min.y) / body.height().max(1.0);
        let side = [
            (fx, Side::Left),
            (1.0 - fx, Side::Right),
            (fy, Side::Top),
            (1.0 - fy, Side::Bottom),
        ]
        .into_iter()
        .filter(|(d, _)| *d < EDGE_SHARE)
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, side)| side);
        Some(match side {
            Some(side) => {
                let (across, first) = match side {
                    Side::Left => (true, true),
                    Side::Right => (true, false),
                    Side::Top => (false, true),
                    Side::Bottom => (false, false),
                };
                let (a, b) = split(rect, across, 0.5);
                (Drop::Beside(id, side), if first { a } else { b })
            }
            None => (Drop::Into(id), rect),
        })
    }

    /// Maximizes the active group, or restores the layout.
    pub fn toggle_maximize(&mut self) {
        self.maximized = match self.maximized {
            Some(_) => None,
            None => self.active.filter(|_| self.groups.len() > 1),
        };
    }

    /// Every tab in one group, `order` left to right, showing `active`.
    pub fn arrange_tabs(&mut self, order: &[u64], active: Option<u64>) {
        self.groups.clear();
        self.root = None;
        self.maximized = None;
        if order.is_empty() {
            self.active = None;
            return;
        }
        let id = self.fresh_group(order.to_vec());
        self.root = Some(Node::Leaf(id));
        self.active = Some(id);
        if let Some(key) = active {
            self.select(key);
        }
    }

    /// One group per tab (the first [`MAX_GROUPS`]; the rest join the last group): side by side
    /// (`Some(true)`), stacked (`Some(false)`) or a square-ish grid (`None`, four make 2 x 2).
    pub fn arrange_tiles(&mut self, order: &[u64], across: Option<bool>, active: Option<u64>) {
        self.groups.clear();
        self.root = None;
        self.maximized = None;
        if order.is_empty() {
            self.active = None;
            return;
        }
        let split_at = order.len().min(MAX_GROUPS);
        let (own, rest) = order.split_at(split_at);
        let mut ids: Vec<u64> = own.iter().map(|k| self.fresh_group(vec![*k])).collect();
        if let (Some(last), false) = (ids.last().copied(), rest.is_empty())
            && let Some(group) = self.groups.iter_mut().find(|g| g.id == last)
        {
            group.tabs.extend_from_slice(rest);
        }
        let leaves = |ids: &[u64]| ids.iter().map(|id| Node::Leaf(*id)).collect::<Vec<_>>();
        self.root = match across {
            Some(across) => chain(leaves(&ids), across),
            None => {
                let n = ids.len();
                let columns = (1..=n).find(|c| c * c >= n).unwrap_or(1);
                let mut rows = Vec::new();
                while !ids.is_empty() {
                    let take = columns.min(ids.len());
                    let row: Vec<u64> = ids.drain(..take).collect();
                    rows.extend(chain(leaves(&row), true));
                }
                chain(rows, false)
            }
        };
        self.active = self.groups.first().map(|g| g.id);
        if let Some(key) = active {
            self.select(key);
        }
    }

    /// The leaf order of the groups, left to right and top to bottom.
    pub fn order(&self) -> Vec<u64> {
        let mut out = Vec::new();
        if let Some(root) = &self.root {
            root.leaves(&mut out);
        }
        out
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    fn area() -> Rect {
        Rect::from_min_size(pos2(0.0, 0.0), vec2(1200.0, 800.0))
    }

    #[test]
    fn new_tabs_open_in_the_active_group_and_closing_the_last_one_empties_it() {
        let mut dock = Dock::default();
        dock.sync(&[1, 2, 3], Some(3));
        assert_eq!(dock.groups().len(), 1);
        assert_eq!(dock.groups()[0].tabs, [1, 2, 3]);
        assert!(dock.is_shown(3) && !dock.is_shown(1));
        assert_eq!(dock.layout(area()), [(dock.groups()[0].id, area())]);
        // Closing the shown tab shows its left neighbour.
        dock.sync(&[1, 2], Some(2));
        assert_eq!(dock.groups()[0].current, Some(2));
        dock.sync(&[], None);
        assert!(dock.groups().is_empty() && dock.layout(area()).is_empty());
    }

    #[test]
    fn a_tab_dropped_on_an_edge_splits_its_group_and_an_empty_group_closes() {
        let mut dock = Dock::default();
        dock.sync(&[1, 2], Some(2));
        let first = dock.group_of(1).unwrap();
        // The right edge of the body splits; the tab gets the right half.
        let pos = pos2(1150.0, 400.0);
        let (drop, preview) = dock.drop_at(area(), pos).unwrap();
        assert_eq!(drop, Drop::Beside(first, Side::Right));
        assert_eq!(preview.min.x, 600.0);
        dock.insert(2, drop);
        assert_eq!(dock.groups().len(), 2);
        let rects = dock.layout(area());
        assert_eq!(rects.len(), 2);
        assert_eq!(rects[0].1.max.x, 600.0);
        assert_eq!(dock.dividers(area()).len(), 1);
        // The middle of a group, or its tab strip, moves the tab into it.
        let (drop, _) = dock.drop_at(area(), pos2(300.0, 10.0)).unwrap();
        assert_eq!(drop, Drop::Into(first));
        dock.insert(2, drop);
        assert_eq!(dock.groups().len(), 1, "the emptied group closed");
        assert_eq!(dock.layout(area())[0].1, area());
        // A lone tab dropped beside its own group stays.
        dock.sync(&[1], Some(1));
        dock.insert(1, Drop::Beside(first, Side::Left));
        assert_eq!(dock.groups().len(), 1);
    }

    #[test]
    fn dividers_resize_and_maximize_hides_the_others() {
        let mut dock = Dock::default();
        dock.sync(&[1, 2], Some(1));
        let g = dock.group_of(1).unwrap();
        dock.insert(2, Drop::Beside(g, Side::Bottom));
        let divider = dock.dividers(area()).remove(0);
        assert!(!divider.across);
        dock.drag_divider(
            &divider.path,
            divider.room,
            divider.across,
            pos2(0.0, 200.0),
        );
        assert_eq!(dock.layout(area())[0].1.max.y, 200.0);
        dock.drag_divider(&divider.path, divider.room, divider.across, pos2(0.0, 5.0));
        assert_eq!(
            dock.layout(area())[0].1.max.y,
            80.0,
            "a share stays at least 10 %"
        );
        dock.toggle_maximize();
        assert_eq!(dock.layout(area()).len(), 1);
        assert!(dock.dividers(area()).is_empty());
        dock.toggle_maximize();
        assert_eq!(dock.layout(area()).len(), 2);
    }

    #[test]
    fn tiles_lay_one_group_per_tab() {
        let mut dock = Dock::default();
        dock.arrange_tiles(&[1, 2, 3, 4], None, Some(4));
        let rects = dock.layout(area());
        let columns: std::collections::BTreeSet<u32> =
            rects.iter().map(|(_, r)| r.min.x.to_bits()).collect();
        let rows: std::collections::BTreeSet<u32> =
            rects.iter().map(|(_, r)| r.min.y.to_bits()).collect();
        assert_eq!((columns.len(), rows.len()), (2, 2));
        assert_eq!(
            dock.groups()
                .iter()
                .find(|g| g.current == Some(4))
                .map(|g| g.id),
            dock.active_group()
        );
        dock.arrange_tiles(&[1, 2, 3], Some(true), None);
        let rects = dock.layout(area());
        assert!(rects.iter().all(|(_, r)| r.height() == 800.0));
        assert!((rects[0].1.width() - 400.0).abs() < 0.01, "{rects:?}");
        dock.arrange_tabs(&[1, 2, 3], Some(2));
        assert_eq!(dock.groups().len(), 1);
        assert!(dock.is_shown(2));
        let many: Vec<u64> = (1..=10).collect();
        dock.arrange_tiles(&many, Some(false), None);
        assert_eq!(dock.groups().len(), MAX_GROUPS);
        assert_eq!(
            dock.groups().iter().map(|g| g.tabs.len()).sum::<usize>(),
            10
        );
    }
}
