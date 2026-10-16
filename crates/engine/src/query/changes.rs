// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! `GetChanges`: incremental deltas between retained snapshots.
//!
//! A delta lists added, changed and removed elements in ascending id order and names the mesh
//! blobs the consumer lacks; a placement-only edit therefore resends no mesh. Deltas are over the
//! authoritative scene, not a projection: view settings are not revision data, so a consumer
//! re-projects locally after applying a delta. A token the history cannot answer is
//! `GEO_STALE_TOKEN` (F06 `resnapshot_required`), with no partial list.

use crate::element::ElementId;
use crate::error::{ViewCode, ViewError};
use crate::limits::MAX_RETAINED_SNAPSHOTS;
use crate::query::snapshot::{ElementEntry, SnapshotToken, ViewPin, ViewSnapshot};
use beamcraft_model::digest::ContentDigest;
use std::collections::{BTreeSet, VecDeque};

/// What happened to an element between two snapshots.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeKind {
    /// Present only in the newer snapshot.
    Added,
    /// Present in both with different content.
    Changed,
    /// Present only in the older snapshot.
    Removed,
}

impl ChangeKind {
    /// The stable token.
    pub const fn as_token(self) -> &'static str {
        match self {
            Self::Added => "added",
            Self::Changed => "changed",
            Self::Removed => "removed",
        }
    }
}

/// Which parts of a changed element differ.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ChangedFields {
    /// The element kind.
    pub kind: bool,
    /// The placement.
    pub placement: bool,
    /// A mesh digest of either representation (a shape change).
    pub shape: bool,
}

/// One entry of a delta.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Change {
    /// The element identity.
    pub id: ElementId,
    /// What happened.
    pub kind: ChangeKind,
    /// For `Changed`, which parts differ; all false otherwise.
    pub fields: ChangedFields,
    /// The element as of the newer snapshot; `None` for `Removed`.
    pub entry: Option<ElementEntry>,
}

/// The delta from one snapshot to a newer one.
#[derive(Debug, Clone, PartialEq)]
pub struct ChangeSet {
    /// Token the consumer holds.
    pub from: SnapshotToken,
    /// Token after applying the delta.
    pub to: SnapshotToken,
    /// Pin of the newer snapshot.
    pub to_pin: ViewPin,
    /// Entries in ascending id order.
    pub changes: Vec<Change>,
    /// Mesh blobs the newer snapshot needs that the older one lacked, ascending. A consumer
    /// fetches only these with [`ViewSnapshot::mesh`].
    pub new_meshes: Vec<ContentDigest>,
}

/// Computes the delta between two snapshots. Both are bounded, so the result is too.
pub fn diff(old: &ViewSnapshot, new: &ViewSnapshot) -> ChangeSet {
    let mut changes = Vec::new();
    let mut needed: BTreeSet<ContentDigest> = BTreeSet::new();
    let (mut a, mut b) = (
        old.elements().iter().peekable(),
        new.elements().iter().peekable(),
    );
    loop {
        let step = match (a.peek(), b.peek()) {
            (None, None) => break,
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (Some(x), Some(y)) => x.id.cmp(&y.id),
        };
        match step {
            std::cmp::Ordering::Less => {
                if let Some(x) = a.next() {
                    changes.push(Change {
                        id: x.id,
                        kind: ChangeKind::Removed,
                        fields: ChangedFields::default(),
                        entry: None,
                    });
                }
            }
            std::cmp::Ordering::Greater => {
                if let Some(y) = b.next() {
                    needed.insert(y.full.mesh_digest);
                    needed.insert(y.simplified.mesh_digest);
                    changes.push(Change {
                        id: y.id,
                        kind: ChangeKind::Added,
                        fields: ChangedFields::default(),
                        entry: Some(y.clone()),
                    });
                }
            }
            std::cmp::Ordering::Equal => {
                if let (Some(x), Some(y)) = (a.next(), b.next()) {
                    let fields = ChangedFields {
                        kind: x.kind != y.kind,
                        placement: x.placement_mm != y.placement_mm,
                        shape: x.full.mesh_digest != y.full.mesh_digest
                            || x.simplified.mesh_digest != y.simplified.mesh_digest,
                    };
                    if fields.kind || fields.placement || fields.shape {
                        needed.insert(y.full.mesh_digest);
                        needed.insert(y.simplified.mesh_digest);
                        changes.push(Change {
                            id: y.id,
                            kind: ChangeKind::Changed,
                            fields,
                            entry: Some(y.clone()),
                        });
                    }
                }
            }
        }
    }
    ChangeSet {
        from: old.token().clone(),
        to: new.token().clone(),
        to_pin: new.pin(),
        changes,
        new_meshes: needed
            .into_iter()
            .filter(|d| old.mesh(d).is_none())
            .collect(),
    }
}

/// A bounded history of recent snapshots that answers `GetChanges`.
#[derive(Debug, Clone)]
pub struct SnapshotHistory {
    capacity: usize,
    retained: VecDeque<ViewSnapshot>,
}

impl SnapshotHistory {
    /// A history retaining `capacity` snapshots, clamped to 1 through 8.
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity: capacity.clamp(1, MAX_RETAINED_SNAPSHOTS),
            retained: VecDeque::new(),
        }
    }

    /// Records a newer snapshot as the latest, evicting the oldest beyond the capacity. A snapshot
    /// with the latest token is not stored twice.
    pub fn push(&mut self, snapshot: ViewSnapshot) {
        if self.retained.back().map(ViewSnapshot::token) == Some(snapshot.token()) {
            return;
        }
        self.retained.push_back(snapshot);
        while self.retained.len() > self.capacity {
            self.retained.pop_front();
        }
    }

    /// The latest snapshot.
    pub fn latest(&self) -> Option<&ViewSnapshot> {
        self.retained.back()
    }

    /// Number of retained snapshots.
    pub fn len(&self) -> usize {
        self.retained.len()
    }

    /// Whether nothing is retained.
    pub fn is_empty(&self) -> bool {
        self.retained.is_empty()
    }

    /// `GetChanges`: the delta from `since` to the latest snapshot. An unknown, expired or
    /// evicted token, or an empty history, is `GEO_STALE_TOKEN` and returns no partial list.
    pub fn changes_since(&self, since: &SnapshotToken) -> Result<ChangeSet, ViewError> {
        let stale = || ViewError::new(ViewCode::StaleToken);
        let latest = self.retained.back().ok_or_else(stale)?;
        let old = self
            .retained
            .iter()
            .find(|s| s.token() == since)
            .ok_or_else(stale)?;
        Ok(diff(old, latest))
    }
}
