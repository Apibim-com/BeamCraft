// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! The selection API: identity sets, and the mapping from display hints to identities.
//!
//! A selection is viewport state. It is never part of a snapshot token, a key or any engineering
//! input (F06 section 7). It holds element ids only; instance and triangle indices are accepted
//! as display hints and resolved to ids through a projection of one snapshot, then discarded.
//!
//! Geometric ray and point picking (F06 `Pick`) and `LookupFeature` live in
//! [`crate::query::pick`] and [`crate::query::feature`]. A display-only GPU pick can feed
//! [`Selection::from_triangles`] or [`Selection::from_instances`] as a hint; anything that
//! selects for engineering use must come from the engine's `Pick`.

use crate::element::ElementId;
use crate::error::{ViewCode, ViewError};
use crate::presentation::ViewProjection;
use crate::query::snapshot::{SnapshotToken, ViewSnapshot};
use std::collections::BTreeSet;

/// A set of element ids selected against one snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selection {
    token: SnapshotToken,
    ids: BTreeSet<ElementId>,
}

fn collect(
    token: &SnapshotToken,
    cap: usize,
    ids: impl ExactSizeIterator<Item = Result<ElementId, ViewError>>,
) -> Result<Selection, ViewError> {
    if ids.len() > cap {
        return Err(ViewError::new(ViewCode::SelectionLimit));
    }
    Ok(Selection {
        token: token.clone(),
        ids: ids.collect::<Result<_, _>>()?,
    })
}

impl Selection {
    /// Selects by identity. Every id must exist in the snapshot (`VIEW_UNKNOWN_ELEMENT`);
    /// repeated ids collapse. The input length is checked against the selection cap first.
    pub fn from_ids(snapshot: &ViewSnapshot, ids: &[ElementId]) -> Result<Self, ViewError> {
        collect(
            snapshot.token(),
            snapshot.limits().max_selection(),
            ids.iter().map(|id| {
                snapshot
                    .element(id)
                    .map(|e| e.id)
                    .ok_or(ViewError::new(ViewCode::UnknownElement))
            }),
        )
    }

    /// Resolves display instance slots of a projection to ids.
    pub fn from_instances(
        projection: &ViewProjection<'_>,
        instances: &[u32],
    ) -> Result<Self, ViewError> {
        collect(
            projection.token(),
            projection.snapshot().limits().max_selection(),
            instances
                .iter()
                .map(|i| projection.element_for_instance(*i)),
        )
    }

    /// Resolves triangle indices of a projection's merged buffer to ids.
    pub fn from_triangles(
        projection: &ViewProjection<'_>,
        triangles: &[u32],
    ) -> Result<Self, ViewError> {
        collect(
            projection.token(),
            projection.snapshot().limits().max_selection(),
            triangles
                .iter()
                .map(|t| projection.element_for_triangle(*t)),
        )
    }

    /// The snapshot token this selection was made against.
    pub const fn token(&self) -> &SnapshotToken {
        &self.token
    }

    /// The selected ids in ascending order.
    pub fn ids(&self) -> impl ExactSizeIterator<Item = &ElementId> {
        self.ids.iter()
    }

    /// Number of selected elements.
    pub fn len(&self) -> usize {
        self.ids.len()
    }

    /// Whether nothing is selected.
    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }

    /// Whether `id` is selected.
    pub fn contains(&self, id: &ElementId) -> bool {
        self.ids.contains(id)
    }

    /// Instance slots to highlight in a projection of the same snapshot; selected elements the
    /// view hides have no slot and are skipped. Another snapshot's projection is
    /// `GEO_STALE_TOKEN`: use [`Self::rebased`] first.
    pub fn instances(&self, projection: &ViewProjection<'_>) -> Result<Vec<u32>, ViewError> {
        if projection.token() != &self.token {
            return Err(ViewError::new(ViewCode::StaleToken));
        }
        Ok(self
            .ids
            .iter()
            .filter_map(|id| projection.instance_of(id))
            .collect())
    }

    /// Carries the selection to a newer snapshot, dropping ids that no longer exist. Returns the
    /// new selection and how many ids were dropped. The survivors must fit the destination
    /// snapshot's selection cap, exactly as [`Self::from_ids`] enforces it; otherwise
    /// `VIEW_SELECTION_LIMIT` and no selection is built.
    pub fn rebased(&self, snapshot: &ViewSnapshot) -> Result<(Self, usize), ViewError> {
        let survivors = || self.ids.iter().filter(|id| snapshot.element(id).is_some());
        if survivors().count() > snapshot.limits().max_selection() {
            return Err(ViewError::new(ViewCode::SelectionLimit));
        }
        let kept: BTreeSet<ElementId> = survivors().copied().collect();
        let dropped = self.ids.len().saturating_sub(kept.len());
        Ok((
            Self {
                token: snapshot.token().clone(),
                ids: kept,
            },
            dropped,
        ))
    }
}
