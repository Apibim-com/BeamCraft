// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! Declared read and write sets of a command (F04 section 8).
//!
//! The engine derives them from command semantics; a caller's claim can never narrow them. Each
//! entry names an owned element or an immutable digest and one semantic group. Different property
//! names alone do not prove independence: the groups below are the unit of conflict.
//!
//! P01 knows one operation, so only `Create` is derived. Update, copy and delete footprints
//! arrive with their commands (P04, P05, P08).

use beamcraft_model::command::CommandError;
use beamcraft_model::digest::ContentDigest;
use beamcraft_model::error::ErrorCode;
use beamcraft_model::id::ElementId;
use beamcraft_model::limits::MAX_FOOTPRINT_ENTRIES;
use std::collections::BTreeSet;

/// What an entry refers to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Target {
    /// An owned element, by full identity.
    Element(ElementId),
    /// An immutable definition, by full content digest.
    Definition(ContentDigest),
}

/// The semantic groups of F04 section 8.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Group {
    /// The label.
    Label,
    /// The complete shape-input group (contour, thickness, axis, roll, profile).
    ShapeInputs,
    /// The rigid placement.
    Placement,
    /// The material reference.
    MaterialRef,
    /// Relationships to other elements or definitions.
    Relationship,
    /// Existence and identity.
    Lifecycle,
    /// Derived geometry, quantities and their signatures.
    Derived,
}

impl Group {
    /// The F04 token of the group.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Label => "label",
            Self::ShapeInputs => "shape_inputs",
            Self::Placement => "placement",
            Self::MaterialRef => "material_ref",
            Self::Relationship => "relationship",
            Self::Lifecycle => "lifecycle",
            Self::Derived => "derived",
        }
    }
}

/// One footprint entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Entry {
    /// The element or definition.
    pub target: Target,
    /// The semantic group.
    pub group: Group,
}

/// The declared read and write sets of one command, in deterministic order.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Footprint {
    reads: BTreeSet<Entry>,
    writes: BTreeSet<Entry>,
}

impl Footprint {
    /// The footprint of `CreatePlate` (F04 section 8: "read ID absence and pinned definitions;
    /// write new lifecycle, complete authored fields and relationships"): it reads the absence of
    /// the new ID (lifecycle), reads the pinned material definition's relationship when there is
    /// one, and writes the new element's lifecycle, every authored group (including the material
    /// reference and relationships, written as "none" when there is no material) and the derived
    /// group. A definition is immutable and is never written by this command.
    ///
    /// Fails with `LIMIT_EXCEEDED` if the footprint cannot be bounded (F04: at most 4096 entries).
    pub fn for_create(
        id: ElementId,
        material: Option<ContentDigest>,
    ) -> Result<Self, CommandError> {
        let element = Target::Element(id);
        let entry = |group| Entry {
            target: element,
            group,
        };
        let mut reads = BTreeSet::from([entry(Group::Lifecycle)]);
        if let Some(digest) = material {
            reads.insert(Entry {
                target: Target::Definition(digest),
                group: Group::Relationship,
            });
        }
        let writes = BTreeSet::from([
            entry(Group::Lifecycle),
            entry(Group::Label),
            entry(Group::ShapeInputs),
            entry(Group::Placement),
            entry(Group::MaterialRef),
            entry(Group::Relationship),
            entry(Group::Derived),
        ]);
        let footprint = Self { reads, writes };
        footprint.check_bound()?;
        Ok(footprint)
    }

    /// The footprint of publishing one immutable definition (P02): it writes the definition's own
    /// entry and no element. A definition is content-addressed, so two commands that publish the
    /// same digest write identical bytes; the entry is still a write, because another command
    /// that reads the definition must not run against a half-published one.
    pub fn for_define(digest: ContentDigest) -> Self {
        let entry = Entry {
            target: Target::Definition(digest),
            group: Group::Relationship,
        };
        Self {
            reads: BTreeSet::from([entry]),
            writes: BTreeSet::from([entry]),
        }
    }

    fn check_bound(&self) -> Result<(), CommandError> {
        if self.reads.len().saturating_add(self.writes.len()) > MAX_FOOTPRINT_ENTRIES {
            return Err(CommandError::new(
                ErrorCode::LimitExceeded,
                "footprint",
                "footprint exceeds 4096 entries",
            ));
        }
        Ok(())
    }

    /// Entries the command reads.
    pub const fn reads(&self) -> &BTreeSet<Entry> {
        &self.reads
    }

    /// Entries the command writes.
    pub const fn writes(&self) -> &BTreeSet<Entry> {
        &self.writes
    }

    /// Whether two commands conflict: one writes an entry the other reads or writes.
    pub fn conflicts_with(&self, other: &Self) -> bool {
        self.writes
            .iter()
            .any(|e| other.writes.contains(e) || other.reads.contains(e))
            || other.reads.iter().any(|e| self.writes.contains(e))
    }

    /// Whether every entry of `observed` is covered by this declared set: the layer engine's
    /// own conservative footprint may only be a subset of what the engine declared.
    pub fn covers_writes(&self, observed: &BTreeSet<Entry>) -> bool {
        observed.is_subset(&self.writes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(n: u8) -> ElementId {
        ElementId::parse(&format!("019a7bf0-0000-7000-8000-0000000000{n:02x}")).unwrap()
    }

    #[test]
    fn create_reads_the_absence_and_writes_the_complete_authored_state() {
        let f = Footprint::for_create(id(5), None).unwrap();
        assert_eq!(f.reads().len(), 1);
        assert!(f.reads().iter().all(|e| e.group == Group::Lifecycle));
        let groups: Vec<_> = f.writes().iter().map(|e| e.group).collect();
        for group in [
            Group::Lifecycle,
            Group::Label,
            Group::ShapeInputs,
            Group::Placement,
            Group::MaterialRef,
            Group::Relationship,
            Group::Derived,
        ] {
            assert!(groups.contains(&group), "{group:?}");
        }
        assert!(
            f.writes()
                .iter()
                .all(|e| e.target == Target::Element(id(5)))
        );
    }

    #[test]
    fn publishing_a_definition_writes_only_that_definition() {
        let digest = beamcraft_model::digest::Sha256::digest(b"definition").unwrap();
        let other = beamcraft_model::digest::Sha256::digest(b"other").unwrap();
        let publish = Footprint::for_define(digest);
        assert!(
            publish
                .writes()
                .iter()
                .all(|e| e.target == Target::Definition(digest))
        );
        // A plate that pins it conflicts with the publication, one that pins another does not.
        assert!(publish.conflicts_with(&Footprint::for_create(id(5), Some(digest)).unwrap()));
        assert!(!publish.conflicts_with(&Footprint::for_create(id(5), Some(other)).unwrap()));
    }

    #[test]
    fn a_pinned_material_is_read_never_written() {
        let digest = beamcraft_model::digest::Sha256::digest(b"material").unwrap();
        let f = Footprint::for_create(id(5), Some(digest)).unwrap();
        let read = Entry {
            target: Target::Definition(digest),
            group: Group::Relationship,
        };
        assert!(f.reads().contains(&read));
        assert!(
            f.writes()
                .iter()
                .all(|e| e.target == Target::Element(id(5)))
        );
        // Two plates that pin the same definition only read it, so they stay independent.
        let other = Footprint::for_create(id(6), Some(digest)).unwrap();
        assert!(!f.conflicts_with(&other));
    }

    #[test]
    fn different_new_elements_are_independent_the_same_one_conflicts() {
        let a = Footprint::for_create(id(5), None).unwrap();
        let b = Footprint::for_create(id(6), None).unwrap();
        assert!(!a.conflicts_with(&b));
        assert!(!b.conflicts_with(&a));
        assert!(a.conflicts_with(&a));
    }

    #[test]
    fn the_declared_set_covers_a_subset_and_rejects_anything_wider() {
        let f = Footprint::for_create(id(5), None).unwrap();
        let narrower: BTreeSet<Entry> = f.writes().iter().copied().take(2).collect();
        assert!(f.covers_writes(&narrower));
        let mut wider = f.writes().clone();
        wider.insert(Entry {
            target: Target::Element(id(9)),
            group: Group::Label,
        });
        assert!(!f.covers_writes(&wider));
    }

    #[test]
    fn group_tokens_are_the_f04_names() {
        assert_eq!(Group::ShapeInputs.as_str(), "shape_inputs");
        assert_eq!(Group::MaterialRef.as_str(), "material_ref");
    }
}
