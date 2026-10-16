// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! The pinned, immutable view snapshot and its read queries.
//!
//! A snapshot answers `ListElements`, `GetMeshSnapshot` and `QueryBounds` at exactly one pinned
//! revision and generation (F06 section 9). It holds the whole authoritative scene; what a
//! viewport draws is a read-time projection of it ([`crate::presentation`]), so view settings
//! never enter the token. Element indices are snapshot-local positions; they are never
//! identities and are never accepted as addresses by an engineering query.

use crate::element::{ElementId, ElementKind};
use crate::error::{ViewCode, ViewError};
use crate::limits::{Limits, MAX_PAGE_SIZE};
use crate::presentation::Representation;
use crate::query::bounds::Aabb;
use crate::query::mesh::MeshEntry;
use crate::scene::SceneElement;
use beamcraft_geom::{Translation3, canonical_f64_bytes};
use beamcraft_kernel::PlateResult;
use beamcraft_model::digest::{ContentDigest, Sha256};
use std::collections::BTreeMap;

/// The pinned context of a snapshot: the base revision and the workspace generation.
///
/// The project and workspace identities of the F05 context tuple belong to the host session that
/// owns the pin; they are not part of the engine's data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ViewPin {
    revision: ContentDigest,
    generation: u64,
}

impl ViewPin {
    /// Pins a revision digest and a workspace generation.
    pub const fn new(revision: ContentDigest, generation: u64) -> Self {
        Self {
            revision,
            generation,
        }
    }
    /// The base revision digest.
    pub const fn revision(&self) -> ContentDigest {
        self.revision
    }
    /// The workspace generation.
    pub const fn generation(&self) -> u64 {
        self.generation
    }
}

/// An opaque snapshot token: `vs1-` and 64 lowercase hexadecimal digits.
///
/// Compare for equality only. It covers the pin and the authoritative content of the snapshot
/// (ids, kinds, placements, mesh digests) and nothing about view settings or selection.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SnapshotToken(String);

impl SnapshotToken {
    const PREFIX: &'static str = "vs1-";

    /// Parses the exact token spelling; anything else is `VIEW_ARGUMENT_INVALID`.
    pub fn parse(text: &str) -> Result<Self, ViewError> {
        let valid = text.len() == Self::PREFIX.len() + 64
            && text.starts_with(Self::PREFIX)
            && text
                .bytes()
                .skip(Self::PREFIX.len())
                .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'));
        if valid {
            Ok(Self(text.to_owned()))
        } else {
            Err(ViewError::new(ViewCode::ArgumentInvalid))
        }
    }

    /// The token text.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    fn from_digest(digest: ContentDigest) -> Self {
        Self(format!("{}{digest}", Self::PREFIX))
    }
}

/// One representation of an element inside a snapshot.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct RepresentationRef {
    /// SHA-256 of the mesh blob; fetch it with [`ViewSnapshot::mesh`].
    pub mesh_digest: ContentDigest,
    /// Position of the blob in [`ViewSnapshot::meshes`]; valid only within this snapshot.
    pub mesh_slot: u32,
    /// Whether this is the exact geometry. The full representation is always exact; a
    /// simplified one is exact only when it is the same mesh as the full one.
    pub exact: bool,
    /// World bounds of this representation, computed by the engine from kernel vertices.
    pub world_bounds: Aabb,
}

/// One element of a snapshot.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct ElementEntry {
    /// The stable identity.
    pub id: ElementId,
    /// Snapshot-local position in ascending id order. Never an identity.
    pub index: u32,
    /// The element kind.
    pub kind: ElementKind,
    /// Translation placement in millimetres.
    pub placement_mm: [f64; 3],
    /// The full representation (operations applied): the only one engineering uses.
    pub full: RepresentationRef,
    /// The simplified representation; the same mesh as `full` when there are no operations.
    pub simplified: RepresentationRef,
}

impl ElementEntry {
    /// The reference for a chosen representation.
    pub const fn representation(&self, representation: Representation) -> &RepresentationRef {
        match representation {
            Representation::Full => &self.full,
            Representation::Simplified => &self.simplified,
        }
    }
}

/// One page of `ListElements`.
#[derive(Debug, Clone, PartialEq)]
pub struct ElementPage<'a> {
    /// Entries in ascending id order.
    pub entries: &'a [ElementEntry],
    /// Pass this as `after` to read the next page; `None` after the last page.
    pub next_cursor: Option<ElementId>,
}

/// An immutable view snapshot pinned to one revision and generation.
#[derive(Debug, Clone, PartialEq)]
pub struct ViewSnapshot {
    pin: ViewPin,
    token: SnapshotToken,
    limits: Limits,
    elements: Vec<ElementEntry>,
    meshes: Vec<MeshEntry>,
    engineering_bounds: Option<Aabb>,
}

struct MeshRegistry {
    map: BTreeMap<ContentDigest, MeshEntry>,
    total: usize,
    limit: usize,
}

impl MeshRegistry {
    fn add(&mut self, plate: &PlateResult) -> Result<ContentDigest, ViewError> {
        let digest = plate.mesh().mesh_digest();
        let entry = MeshEntry::from_blob(plate.mesh().blob(), &digest)?;
        match self.map.get(&digest) {
            Some(existing) if existing.bytes() == entry.bytes() => {}
            Some(_) => return Err(ViewError::new(ViewCode::BlobInvalid)),
            None => {
                self.total = self
                    .total
                    .checked_add(entry.bytes().len())
                    .filter(|total| *total <= self.limit)
                    .ok_or(ViewError::new(ViewCode::BufferLimit))?;
                self.map.insert(digest, entry);
            }
        }
        Ok(digest)
    }
}

struct Pending {
    id: ElementId,
    kind: ElementKind,
    placement: [f64; 3],
    full: (ContentDigest, Aabb),
    simplified: (ContentDigest, Aabb),
}

fn world_bounds(plate: &PlateResult, placement: Translation3) -> Result<Aabb, ViewError> {
    let numeric = || ViewError::new(ViewCode::NumericUncertain);
    let mut bounds: Option<Aabb> = None;
    for vertex in plate.mesh().vertices() {
        let point = Aabb::at(placement.apply_point(*vertex).map_err(|_| numeric())?);
        bounds = Some(bounds.map_or(point, |b| b.union(point)));
    }
    bounds.ok_or_else(numeric)
}

fn slot_of(meshes: &[MeshEntry], digest: &ContentDigest) -> Result<u32, ViewError> {
    let bad = || ViewError::new(ViewCode::BlobInvalid);
    let slot = meshes
        .binary_search_by(|m| m.digest().cmp(digest))
        .map_err(|_| bad())?;
    u32::try_from(slot).map_err(|_| bad())
}

impl ViewSnapshot {
    /// Builds the snapshot of `elements` at `pin`.
    ///
    /// The element count is checked before any work. Elements are ordered by id; a duplicate id
    /// is rejected. Each blob is re-inspected and its digest recomputed; identical blobs are
    /// stored once. Nothing is partially returned on failure.
    pub fn build(
        pin: ViewPin,
        mut elements: Vec<SceneElement>,
        limits: &Limits,
    ) -> Result<Self, ViewError> {
        if elements.len() > limits.max_elements() {
            return Err(ViewError::new(ViewCode::ElementLimit));
        }
        elements.sort_by_key(SceneElement::id);
        if elements
            .windows(2)
            .any(|pair| matches!(pair, [a, b] if a.id == b.id))
        {
            return Err(ViewError::new(ViewCode::DuplicateElement));
        }
        let mut registry = MeshRegistry {
            map: BTreeMap::new(),
            total: 0,
            limit: limits.max_total_mesh_bytes(),
        };
        let mut pending = Vec::with_capacity(elements.len());
        for element in &elements {
            let full_digest = registry.add(&element.full)?;
            let full_bounds = world_bounds(&element.full, element.placement)?;
            let simplified = match &element.simplified {
                Some(plate) => (
                    registry.add(plate)?,
                    world_bounds(plate, element.placement)?,
                ),
                None => (full_digest, full_bounds),
            };
            pending.push(Pending {
                id: element.id,
                kind: element.kind,
                placement: element.placement.offset().components(),
                full: (full_digest, full_bounds),
                simplified,
            });
        }
        let meshes: Vec<MeshEntry> = registry.map.into_values().collect();
        let mut entries = Vec::with_capacity(pending.len());
        let mut engineering_bounds: Option<Aabb> = None;
        for (position, p) in pending.iter().enumerate() {
            let full = RepresentationRef {
                mesh_digest: p.full.0,
                mesh_slot: slot_of(&meshes, &p.full.0)?,
                exact: true,
                world_bounds: p.full.1,
            };
            let simplified = RepresentationRef {
                mesh_digest: p.simplified.0,
                mesh_slot: slot_of(&meshes, &p.simplified.0)?,
                exact: p.simplified.0 == p.full.0,
                world_bounds: p.simplified.1,
            };
            engineering_bounds =
                Some(engineering_bounds.map_or(full.world_bounds, |b| b.union(full.world_bounds)));
            entries.push(ElementEntry {
                id: p.id,
                index: u32::try_from(position)
                    .map_err(|_| ViewError::new(ViewCode::ElementLimit))?,
                kind: p.kind,
                placement_mm: p.placement,
                full,
                simplified,
            });
        }
        let token = SnapshotToken::from_digest(content_digest(pin, &entries)?);
        Ok(Self {
            pin,
            token,
            limits: *limits,
            elements: entries,
            meshes,
            engineering_bounds,
        })
    }

    /// The pinned context.
    pub const fn pin(&self) -> ViewPin {
        self.pin
    }

    /// The snapshot token.
    pub const fn token(&self) -> &SnapshotToken {
        &self.token
    }

    /// The effective limits this snapshot was built under.
    pub const fn limits(&self) -> Limits {
        self.limits
    }

    /// Number of elements.
    pub const fn len(&self) -> usize {
        self.elements.len()
    }

    /// Whether the snapshot has no element.
    pub const fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }

    /// All elements in ascending id order.
    pub fn elements(&self) -> &[ElementEntry] {
        &self.elements
    }

    /// All distinct mesh blobs, in ascending digest order.
    pub fn meshes(&self) -> &[MeshEntry] {
        &self.meshes
    }

    /// The blob with this digest, if the snapshot references it.
    pub fn mesh(&self, digest: &ContentDigest) -> Option<&MeshEntry> {
        self.meshes
            .binary_search_by(|m| m.digest().cmp(digest))
            .ok()
            .and_then(|i| self.meshes.get(i))
    }

    /// The blob at a snapshot-local slot.
    pub fn mesh_at(&self, slot: u32) -> Option<&MeshEntry> {
        self.meshes.get(usize::try_from(slot).ok()?)
    }

    /// The element with this id.
    pub fn element(&self, id: &ElementId) -> Option<&ElementEntry> {
        self.elements
            .binary_search_by(|e| e.id.cmp(id))
            .ok()
            .and_then(|i| self.elements.get(i))
    }

    /// The element at a snapshot-local index.
    pub fn element_at(&self, index: u32) -> Option<&ElementEntry> {
        self.elements.get(usize::try_from(index).ok()?)
    }

    /// `ListElements`: at most `page_size` (1 to 1,024) entries after the cursor id.
    ///
    /// The cursor is the last id of the previous page; it need not exist in the snapshot.
    pub fn list_elements(
        &self,
        after: Option<&ElementId>,
        page_size: usize,
    ) -> Result<ElementPage<'_>, ViewError> {
        if page_size == 0 || page_size > MAX_PAGE_SIZE {
            return Err(ViewError::new(ViewCode::ArgumentInvalid));
        }
        let start = after.map_or(0, |c| self.elements.partition_point(|e| e.id <= *c));
        let tail = self.elements.get(start..).unwrap_or_default();
        let entries = tail.get(..page_size).unwrap_or(tail);
        let next_cursor = if tail.len() > entries.len() {
            entries.last().map(|e| e.id)
        } else {
            None
        };
        Ok(ElementPage {
            entries,
            next_cursor,
        })
    }

    /// The full representation of an element: the only one volume, mass, bills of materials and
    /// snapping may use, whatever the view settings say.
    pub fn engineering_representation(
        &self,
        id: &ElementId,
    ) -> Result<&RepresentationRef, ViewError> {
        self.element(id)
            .map(|e| &e.full)
            .ok_or(ViewError::new(ViewCode::UnknownElement))
    }

    /// `QueryBounds` for one element's full representation, in world space.
    pub fn element_bounds(&self, id: &ElementId) -> Result<Aabb, ViewError> {
        self.engineering_representation(id).map(|r| r.world_bounds)
    }

    /// World bounds of every element's full representation, independent of any view setting.
    pub const fn engineering_bounds(&self) -> Option<Aabb> {
        self.engineering_bounds
    }
}

fn content_digest(pin: ViewPin, entries: &[ElementEntry]) -> Result<ContentDigest, ViewError> {
    let limit = || ViewError::new(ViewCode::BufferLimit);
    let numeric = || ViewError::new(ViewCode::NumericUncertain);
    let mut h = Sha256::new();
    let mut put = |bytes: &[u8]| h.update(bytes).map_err(|_| limit());
    put(b"BCVIEW-SNAPSHOT\0")?;
    put(pin.revision.as_bytes())?;
    put(&pin.generation.to_be_bytes())?;
    put(&u64::try_from(entries.len())
        .map_err(|_| limit())?
        .to_be_bytes())?;
    for e in entries {
        put(e.id.as_bytes())?;
        put(&[e.kind as u8])?;
        for c in e.placement_mm {
            put(&canonical_f64_bytes(c).map_err(|_| numeric())?)?;
        }
        put(e.full.mesh_digest.as_bytes())?;
        put(e.simplified.mesh_digest.as_bytes())?;
    }
    Ok(h.finalize())
}
