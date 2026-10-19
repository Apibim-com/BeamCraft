// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! A05 bolt groups: the engine's decision on `CreateBolt` (domain vocabulary sections 2.3 and
//! 5.1), following the A06 weld pattern.
//!
//! A bolt group is a `connection` element whose one `connects` record names exactly two distinct
//! parts, the main part first. The engine never touches USD: the adapter reads the kinds of the
//! model's elements into the same [`Parts`] index a weld uses, and the decision here is pure,
//! bounded and deterministic. The layer engine re-checks the structural rules on the authored
//! candidate, so a defect in either place fails closed.
//!
//! Rules: the identity is free (`DUPLICATE_ID`); both parts exist (`NOT_FOUND`) and are plates,
//! beams or columns (`DV-REL-003`, `INVALID_ARGUMENT`); they are distinct and not the group
//! itself (`DV-REL-006`, `DV-REL-007`, admitted by the command). An assembly target is refused as
//! unsupported, as for a weld. Beyond the weld rules, every bolt position
//! `origin + direction * (pitch * i)` must lie within the world bounds of both parts: the
//! adapter supplies conservative bounds and this module checks them; it never proves a real face
//! intersection (see the A05 design note).

use crate::footprint::Footprint;
use beamcraft_model::command::{CommandError, CreateBolt};
use beamcraft_model::error::ErrorCode;
use beamcraft_model::id::ElementId;
use std::collections::BTreeMap;

/// Conservative world bounds of one part: componentwise minimum, then maximum, both inclusive.
pub type Bounds = [[f64; 3]; 2];

impl crate::weld::Parts {
    /// Decides a bolt group creation and returns its footprint. Errors leave nothing applied.
    ///
    /// `bounds` gives the world box of every part the adapter knows. A supported part without
    /// bounds, or any bolt position outside one of the two boxes, refuses the group as
    /// `INVALID_ARGUMENT`: a bolt must lie on both joined parts.
    pub fn create_bolt(
        &self,
        command: &CreateBolt,
        bounds: &BTreeMap<ElementId, Bounds>,
    ) -> Result<Footprint, CommandError> {
        command.validate_arguments()?;
        if self.kinds.contains_key(&command.new_id) {
            return Err(CommandError::new(
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
                    return Err(CommandError::new(
                        ErrorCode::NotFound,
                        field,
                        "the part is not in the head revision",
                    ));
                }
                Some(crate::weld::PartKind::Part) => {}
                Some(crate::weld::PartKind::Assembly) => {
                    return Err(CommandError::new(
                        ErrorCode::UnsupportedGeometry,
                        field,
                        "a bolt group joins plates, beams or columns, not assemblies",
                    ));
                }
                Some(crate::weld::PartKind::Other) => {
                    return Err(CommandError::new(
                        ErrorCode::InvalidArgument,
                        field,
                        "a bolt group joins plates, beams or columns",
                    ));
                }
            }
        }
        let bolt = &command.bolt;
        let [dx, dy, dz] = bolt.direction_mm;
        let [ox, oy, oz] = bolt.origin_mm;
        for i in 0..bolt.count {
            let t = bolt.pitch_mm * i as f64;
            let [x, y, z] = [ox + dx * t, oy + dy * t, oz + dz * t];
            let on = |part: &ElementId| match bounds.get(part) {
                Some([[xmin, ymin, zmin], [xmax, ymax, zmax]]) => {
                    x >= *xmin && x <= *xmax && y >= *ymin && y <= *ymax && z >= *zmin && z <= *zmax
                }
                None => false,
            };
            if !on(&command.main_id) || !on(&command.secondary_id) {
                return Err(CommandError::new(
                    ErrorCode::InvalidArgument,
                    "origin_mm",
                    "the bolt group does not lie on both parts",
                ));
            }
        }
        Footprint::for_bolt(command.new_id, [command.main_id, command.secondary_id])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::weld::PartKind;
    use beamcraft_model::command::BoltSpec;

    fn id(n: u32) -> ElementId {
        ElementId::parse(&format!("019a7bf0-0000-7000-8000-0000005{n:05}")).unwrap()
    }

    fn spec() -> BoltSpec {
        BoltSpec {
            diameter_mm: 16.0,
            count: 3,
            pitch_mm: 80.0,
            origin_mm: [100.0, 250.0, 5.0],
            direction_mm: [1.0, 0.0, 0.0],
        }
    }

    fn command() -> CreateBolt {
        CreateBolt {
            new_id: id(9),
            name: "B".into(),
            main_id: id(1),
            secondary_id: id(2),
            bolt: spec(),
        }
    }

    fn parts() -> crate::weld::Parts {
        let mut parts = crate::weld::Parts::default();
        parts.insert(id(1), PartKind::Part);
        parts.insert(id(2), PartKind::Part);
        parts.insert(id(3), PartKind::Assembly);
        parts.insert(id(4), PartKind::Other);
        parts
    }

    fn bounds(x: f64) -> BTreeMap<ElementId, Bounds> {
        // A plate 0..1000 in x, 0..500 in y, 0..10 in z; a member whose conservative box is the
        // axis line 0..3000 in x expanded to +-60 in y and z.
        let mut map = BTreeMap::new();
        map.insert(id(1), [[0.0, 0.0, 0.0], [x, 500.0, 10.0]]);
        map.insert(id(2), [[-60.0, -60.0, -60.0], [3060.0, 60.0, 60.0]]);
        map
    }

    #[test]
    fn a_group_reads_both_parts_and_writes_only_itself() {
        let footprint = parts().create_bolt(&command(), &bounds(1000.0)).unwrap();
        for part in [id(1), id(2)] {
            let lifecycle = crate::footprint::Entry {
                target: crate::footprint::Target::Element(part),
                group: crate::footprint::Group::Lifecycle,
            };
            assert!(footprint.reads().contains(&lifecycle));
        }
        assert_eq!(footprint.writes().len(), 7);
        assert!(
            footprint
                .writes()
                .iter()
                .all(|e| e.target == crate::footprint::Target::Element(id(9)))
        );
    }

    #[test]
    fn missing_wrong_kind_and_taken_identities_are_refused() {
        let parts = parts();
        let code = |c: CreateBolt, b: &BTreeMap<ElementId, Bounds>| {
            parts.create_bolt(&c, b).unwrap_err().code
        };
        let unknown = CreateBolt {
            secondary_id: id(7),
            ..command()
        };
        assert_eq!(code(unknown, &bounds(1000.0)), ErrorCode::NotFound);
        let assembly = CreateBolt {
            secondary_id: id(3),
            ..command()
        };
        assert_eq!(
            code(assembly, &bounds(1000.0)),
            ErrorCode::UnsupportedGeometry
        );
        let other = CreateBolt {
            secondary_id: id(4),
            ..command()
        };
        assert_eq!(code(other, &bounds(1000.0)), ErrorCode::InvalidArgument);
        let taken = CreateBolt {
            new_id: id(4),
            ..command()
        };
        assert_eq!(code(taken, &bounds(1000.0)), ErrorCode::DuplicateId);
    }

    #[test]
    fn every_bolt_position_must_lie_on_both_parts() {
        let parts = parts();
        let code = |c: CreateBolt, b: &BTreeMap<ElementId, Bounds>| {
            parts.create_bolt(&c, b).unwrap_err().code
        };
        // Three bolts at x = 100, 180, 260 fit the 1000 mm plate; a 940 mm plate loses the last
        // one, so the whole group is refused.
        assert!(parts.create_bolt(&command(), &bounds(1000.0)).is_ok());
        let short = CreateBolt {
            bolt: BoltSpec {
                count: 12,
                pitch_mm: 80.0,
                ..spec()
            },
            ..command()
        };
        assert_eq!(code(short, &bounds(1000.0)), ErrorCode::InvalidArgument);
        let out_of_plane = CreateBolt {
            bolt: BoltSpec {
                origin_mm: [100.0, 250.0, 70.0],
                ..spec()
            },
            ..command()
        };
        assert_eq!(
            code(out_of_plane, &bounds(1000.0)),
            ErrorCode::InvalidArgument
        );
        // The first bolt is checked like any other: an origin outside is refused with count 1.
        let single = CreateBolt {
            bolt: BoltSpec {
                count: 1,
                origin_mm: [2000.0, 250.0, 5.0],
                ..spec()
            },
            ..command()
        };
        assert_eq!(code(single, &bounds(1000.0)), ErrorCode::InvalidArgument);
        // A supported part without bounds is unreachable, never guessed.
        let empty = BTreeMap::new();
        assert_eq!(code(command(), &empty), ErrorCode::InvalidArgument);
    }
}
