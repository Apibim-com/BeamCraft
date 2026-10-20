// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! A06 welds: the engine's decision on `CreateWeld` (domain vocabulary sections 2.3 and 5.1).
//!
//! A weld is a `connection` element whose one `connects` record names exactly two distinct parts,
//! the main part first. The engine never touches USD: the adapter reads the kinds of the model's
//! elements into a [`Parts`] value, and the decision here is pure, bounded and deterministic. The
//! layer engine re-checks the same rules on the authored candidate, so a defect in either place
//! fails closed.
//!
//! Rules: the identity is free (`DUPLICATE_ID`); both parts exist (`NOT_FOUND`) and are plates,
//! beams or columns (`DV-REL-003`, `INVALID_ARGUMENT`); they are distinct and not the weld itself
//! (`DV-REL-006`, `DV-REL-007`, admitted by the command). The vocabulary also allows an assembly
//! as a `connects` target; a weld joins parts, so this slice refuses it as an unsupported target.

use crate::footprint::Footprint;
use beamcraft_model::command::{CommandError, CreateWeld};
use beamcraft_model::error::ErrorCode;
use beamcraft_model::id::ElementId;
use std::collections::BTreeMap;

/// The kind of an element as far as a weld is concerned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartKind {
    /// A plate, beam or column: a part a weld can join.
    Part,
    /// An assembly: a valid `connects` target in the vocabulary, not supported for a weld.
    Assembly,
    /// Any other element (a weld, a reference wrapper, unknown content).
    Other,
}

/// The elements of a model and their kinds.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Parts {
    pub(crate) kinds: BTreeMap<ElementId, PartKind>,
}

fn error(code: ErrorCode, field: &'static str, message: &'static str) -> CommandError {
    CommandError::new(code, field, message)
}

impl Parts {
    /// Records an element. A repeated identity keeps the last kind.
    pub fn insert(&mut self, id: ElementId, kind: PartKind) {
        self.kinds.insert(id, kind);
    }

    /// The kind of an element, if it is in the model.
    pub fn kind(&self, id: &ElementId) -> Option<PartKind> {
        self.kinds.get(id).copied()
    }

    /// Decides a weld creation and returns its footprint. Errors leave nothing applied.
    pub fn create(&self, command: &CreateWeld) -> Result<Footprint, CommandError> {
        command.validate_arguments()?;
        if self.kinds.contains_key(&command.new_id) {
            return Err(error(
                ErrorCode::DuplicateId,
                "new_id",
                "the identity is already in use",
            ));
        }
        for (id, field) in [
            (&command.main_id, "main_id"),
            (&command.secondary_id, "secondary_id"),
        ] {
            match self.kind(id) {
                None => {
                    return Err(error(
                        ErrorCode::NotFound,
                        field,
                        "the part is not in the head revision",
                    ));
                }
                Some(PartKind::Part) => {}
                Some(PartKind::Assembly) => {
                    return Err(error(
                        ErrorCode::UnsupportedGeometry,
                        field,
                        "a weld joins plates, beams or columns, not assemblies",
                    ));
                }
                Some(PartKind::Other) => {
                    return Err(error(
                        ErrorCode::InvalidArgument,
                        field,
                        "a weld joins plates, beams or columns",
                    ));
                }
            }
        }
        Footprint::for_weld(command.new_id, [command.main_id, command.secondary_id])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::footprint::{Entry, Group, Target};
    use beamcraft_model::command::{WeldLocation, WeldSide, WeldSpec, WeldType};

    fn id(n: u32) -> ElementId {
        ElementId::parse(&format!("019a7bf0-0000-7000-8000-0000004{n:05}")).unwrap()
    }

    fn command(main: u32, secondary: u32) -> CreateWeld {
        CreateWeld {
            new_id: id(9),
            name: "W".into(),
            main_id: id(main),
            secondary_id: id(secondary),
            weld: WeldSpec {
                weld_type: WeldType::Fillet,
                size_mm: 5.0,
                location: WeldLocation::Site,
                side: WeldSide::Other,
            },
        }
    }

    fn parts() -> Parts {
        let mut parts = Parts::default();
        parts.insert(id(1), PartKind::Part);
        parts.insert(id(2), PartKind::Part);
        parts.insert(id(3), PartKind::Assembly);
        parts.insert(id(4), PartKind::Other);
        parts
    }

    #[test]
    fn a_weld_reads_both_parts_and_writes_only_itself() {
        let footprint = parts().create(&command(1, 2)).unwrap();
        for part in [id(1), id(2)] {
            let lifecycle = Entry {
                target: Target::Element(part),
                group: Group::Lifecycle,
            };
            assert!(footprint.reads().contains(&lifecycle));
            assert!(
                !footprint
                    .writes()
                    .iter()
                    .any(|e| e.target == lifecycle.target)
            );
        }
        assert_eq!(footprint.writes().len(), 7);
        assert!(
            footprint
                .writes()
                .iter()
                .all(|e| e.target == Target::Element(id(9)))
        );
    }

    #[test]
    fn missing_wrong_kind_and_taken_identities_are_refused() {
        let parts = parts();
        let code = |c: CreateWeld| parts.create(&c).unwrap_err().code;
        assert_eq!(code(command(1, 7)), ErrorCode::NotFound);
        assert_eq!(code(command(3, 1)), ErrorCode::UnsupportedGeometry);
        assert_eq!(code(command(1, 4)), ErrorCode::InvalidArgument);
        assert_eq!(code(command(1, 1)), ErrorCode::InvalidArgument);
        let taken = CreateWeld {
            new_id: id(4),
            ..command(1, 2)
        };
        assert_eq!(code(taken), ErrorCode::DuplicateId);
    }
}
