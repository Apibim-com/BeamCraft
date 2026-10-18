// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! A01 assembly membership: the engine's decision on `AttachToAssembly` and `DetachFromAssembly`
//! over the `part-of` records of a model (domain vocabulary sections 2.3 and 5.1).
//!
//! The engine never touches USD. The adapter reads the element kinds and the current `part-of`
//! targets from the pinned model into a [`Membership`]; the decision here is pure, bounded and
//! deterministic. The layer engine re-checks the same rules on the authored candidate, so a
//! defect in either place fails closed.
//!
//! Rules: the part is a plate, beam, column or assembly, and the target is an assembly
//! (`DV-REL-003`, `INVALID_ARGUMENT`); a part has at most one assembly, so attaching a member of
//! another assembly is refused rather than silently re-parented (`DV-REL-004`,
//! `DEPENDENCY_CONFLICT`); no self-link (`DV-REL-006`); no cycle of any length (`DV-REL-005`,
//! `DEPENDENCY_CONFLICT`). Nested assemblies are allowed; the vocabulary sets no depth limit, so
//! the walk is bounded by the element count.

use crate::footprint::Footprint;
use beamcraft_model::command::{AttachToAssembly, CommandError, DetachFromAssembly};
use beamcraft_model::error::ErrorCode;
use beamcraft_model::id::ElementId;
use std::collections::BTreeMap;

/// The kind of an element that can take part in an assembly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartKind {
    /// A plate.
    Plate,
    /// A beam.
    Beam,
    /// A column.
    Column,
    /// An assembly (a part only when nested).
    Assembly,
}

/// The supported elements of a model and their current `part-of` targets.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Membership {
    kinds: BTreeMap<ElementId, PartKind>,
    parent: BTreeMap<ElementId, ElementId>,
}

/// An accepted attach: the chain the decision read and the footprint it declares.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attach {
    /// The target assembly, then its assembly, and so on to the top.
    pub chain: Vec<ElementId>,
    /// The declared footprint.
    pub footprint: Footprint,
}

/// An accepted detach: the assembly the part leaves and the footprint it declares.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Detach {
    /// The former assembly.
    pub former: ElementId,
    /// The declared footprint.
    pub footprint: Footprint,
}

fn error(code: ErrorCode, field: &'static str, message: &'static str) -> CommandError {
    CommandError::new(code, field, message)
}

impl Membership {
    /// Records a supported element. A repeated identity keeps the last kind.
    pub fn insert(&mut self, id: ElementId, kind: PartKind) {
        self.kinds.insert(id, kind);
    }

    /// Records the `part-of` target of `part`.
    pub fn set_parent(&mut self, part: ElementId, assembly: ElementId) {
        self.parent.insert(part, assembly);
    }

    /// The kind of an element, if it is a supported one.
    pub fn kind(&self, id: &ElementId) -> Option<PartKind> {
        self.kinds.get(id).copied()
    }

    /// The assembly `part` belongs to.
    pub fn parent(&self, part: &ElementId) -> Option<ElementId> {
        self.parent.get(part).copied()
    }

    /// The direct parts of `assembly`, in identity order.
    pub fn parts(&self, assembly: &ElementId) -> Vec<ElementId> {
        self.parent
            .iter()
            .filter(|(_, a)| *a == assembly)
            .map(|(p, _)| *p)
            .collect()
    }

    /// Every assembly, in identity order.
    pub fn assemblies(&self) -> impl Iterator<Item = ElementId> + '_ {
        self.kinds
            .iter()
            .filter(|(_, k)| **k == PartKind::Assembly)
            .map(|(id, _)| *id)
    }

    /// Decides an attach. Errors leave nothing applied.
    pub fn attach(&self, command: &AttachToAssembly) -> Result<Attach, CommandError> {
        command.validate_arguments()?;
        if self.kind(&command.part_id).is_none() {
            return Err(error(
                ErrorCode::NotFound,
                "part_id",
                "the part is not in the head revision",
            ));
        }
        match self.kind(&command.assembly_id) {
            None => {
                return Err(error(
                    ErrorCode::NotFound,
                    "assembly_id",
                    "the assembly is not in the head revision",
                ));
            }
            Some(PartKind::Assembly) => {}
            Some(_) => {
                return Err(error(
                    ErrorCode::InvalidArgument,
                    "assembly_id",
                    "the target is not an assembly",
                ));
            }
        }
        match self.parent(&command.part_id) {
            Some(a) if a == command.assembly_id => {
                return Err(error(
                    ErrorCode::InvalidArgument,
                    "assembly_id",
                    "the element is already a part of this assembly",
                ));
            }
            Some(_) => {
                return Err(error(
                    ErrorCode::DependencyConflict,
                    "part_id",
                    "the element is a part of another assembly; detach it first",
                ));
            }
            None => {}
        }
        let mut chain = Vec::new();
        let mut at = Some(command.assembly_id);
        while let Some(node) = at {
            if node == command.part_id {
                return Err(error(
                    ErrorCode::DependencyConflict,
                    "assembly_id",
                    "the attach would make a part-of cycle",
                ));
            }
            // A chain longer than the element count can only be a cycle already in the model.
            if chain.len() >= self.kinds.len() {
                return Err(error(
                    ErrorCode::DependencyConflict,
                    "assembly_id",
                    "the model already holds a part-of cycle",
                ));
            }
            chain.push(node);
            at = self.parent(&node);
        }
        let footprint = Footprint::for_part_of(command.part_id, &chain)?;
        Ok(Attach { chain, footprint })
    }

    /// Decides a detach: the part must exist and be a member of an assembly.
    pub fn detach(&self, command: &DetachFromAssembly) -> Result<Detach, CommandError> {
        if self.kind(&command.part_id).is_none() {
            return Err(error(
                ErrorCode::NotFound,
                "part_id",
                "the part is not in the head revision",
            ));
        }
        let former = self.parent(&command.part_id).ok_or_else(|| {
            error(
                ErrorCode::InvalidArgument,
                "part_id",
                "the element is not a part of an assembly",
            )
        })?;
        Ok(Detach {
            former,
            footprint: Footprint::for_part_of(command.part_id, &[])?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(n: u8) -> ElementId {
        ElementId::parse(&format!("019a7bf0-0000-7000-8000-0000002000{n:02x}")).unwrap()
    }

    fn model() -> Membership {
        let mut m = Membership::default();
        m.insert(id(1), PartKind::Assembly);
        m.insert(id(2), PartKind::Assembly);
        m.insert(id(3), PartKind::Assembly);
        m.insert(id(10), PartKind::Plate);
        m.insert(id(11), PartKind::Beam);
        m
    }

    fn attach(part: u8, assembly: u8) -> AttachToAssembly {
        AttachToAssembly {
            part_id: id(part),
            assembly_id: id(assembly),
        }
    }

    #[test]
    fn parts_and_nested_assemblies_attach_and_report_the_chain() {
        let mut m = model();
        m.set_parent(id(2), id(1));
        let a = m.attach(&attach(10, 2)).unwrap();
        assert_eq!(a.chain, vec![id(2), id(1)]);
        assert_eq!(m.attach(&attach(3, 2)).unwrap().chain, vec![id(2), id(1)]);
        assert_eq!(m.parts(&id(1)), vec![id(2)]);
        assert_eq!(m.assemblies().count(), 3);
    }

    #[test]
    fn kinds_cardinality_self_links_and_cycles_are_refused() {
        let mut m = model();
        let code = |m: &Membership, c| m.attach(&c).unwrap_err().code;
        assert_eq!(code(&m, attach(10, 11)), ErrorCode::InvalidArgument);
        assert_eq!(code(&m, attach(10, 99)), ErrorCode::NotFound);
        assert_eq!(code(&m, attach(99, 1)), ErrorCode::NotFound);
        assert_eq!(code(&m, attach(1, 1)), ErrorCode::InvalidArgument);
        m.set_parent(id(10), id(1));
        assert_eq!(code(&m, attach(10, 1)), ErrorCode::InvalidArgument);
        assert_eq!(code(&m, attach(10, 2)), ErrorCode::DependencyConflict);
        // 1 <- 2 <- 3: attaching 1 to 3 closes a cycle of length three.
        m.set_parent(id(2), id(1));
        m.set_parent(id(3), id(2));
        assert_eq!(code(&m, attach(1, 3)), ErrorCode::DependencyConflict);
        assert_eq!(code(&m, attach(1, 2)), ErrorCode::DependencyConflict);
    }

    #[test]
    fn a_cycle_already_in_the_model_stops_the_walk() {
        let mut m = model();
        m.set_parent(id(1), id(2));
        m.set_parent(id(2), id(1));
        assert_eq!(
            m.attach(&attach(10, 1)).unwrap_err().code,
            ErrorCode::DependencyConflict
        );
    }

    #[test]
    fn detach_needs_a_member() {
        let mut m = model();
        let detach = DetachFromAssembly { part_id: id(10) };
        assert_eq!(
            m.detach(&detach).unwrap_err().code,
            ErrorCode::InvalidArgument
        );
        m.set_parent(id(10), id(1));
        let d = m.detach(&detach).unwrap();
        assert_eq!(d.former, id(1));
        assert_eq!(d.footprint.writes().len(), 1);
        assert_eq!(
            m.detach(&DetachFromAssembly { part_id: id(99) })
                .unwrap_err()
                .code,
            ErrorCode::NotFound
        );
    }
}
