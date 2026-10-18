// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! Quantity records with input signatures, and their freshness (F04 section 5, P04).
//!
//! A record has a status, a value (only when `current`), the signature of the inputs it was
//! computed from and the algorithm label. Shape and mass have separate signatures, so a change
//! of material invalidates the mass and keeps the shape, and a rename invalidates nothing.
//!
//! The engine never trusts a stored record: it recomputes the expected signatures from the
//! authored inputs and compares. A record whose signature differs is `stale`; one that is absent
//! is `missing`. A `stale` value is never reported as `current`.
//!
//! These signatures are P04's own, taken over the typed authored inputs, the algorithm label and
//! the tolerance policy version. They are not the F06 section 7 shape key (`kernel::keys` is
//! planned): no claim of that key is made.

use crate::command::{AuthoredPlate, Quantities, QuantityStatus};
use crate::member::{AuthoredMember, ProfileInfo};
use beamcraft_geom::canonical_f64_bytes;
use beamcraft_geom::tolerance::TOLERANCE_POLICY_VERSION;
use beamcraft_model::digest::{ContentDigest, Sha256};
use beamcraft_model::section::SectionShape;

/// A quantity record as the engine computes it and as the derived layer stores it.
#[derive(Debug, Clone, PartialEq)]
pub struct QuantityRecord {
    /// Volume in mm3.
    pub volume_mm3: f64,
    /// Status token of the volume.
    pub volume_status: &'static str,
    /// Mass in kg, present exactly when `mass_status` is `current`.
    pub mass_kg: Option<f64>,
    /// Status token of the mass.
    pub mass_status: &'static str,
    /// Signature of the shape inputs.
    pub shape_signature: ContentDigest,
    /// Signature of the mass inputs: the shape signature and the density or its absence.
    pub mass_signature: ContentDigest,
    /// Algorithm label of the volume.
    pub algorithm: String,
}

/// What a reader found stored next to a mesh.
#[derive(Debug, Clone, PartialEq)]
pub struct StoredRecord {
    /// Stored volume in mm3.
    pub volume_mm3: f64,
    /// Stored volume status token.
    pub volume_status: String,
    /// Stored mass in kg.
    pub mass_kg: Option<f64>,
    /// Stored mass status token.
    pub mass_status: String,
    /// Stored shape signature.
    pub shape_signature: ContentDigest,
    /// Stored mass signature.
    pub mass_signature: ContentDigest,
    /// Stored algorithm label.
    pub algorithm: String,
}

/// Freshness of one stored quantity against the inputs as they are now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Freshness {
    /// The stored signature is the expected one and the stored status is `current`.
    Current,
    /// A record exists but its signature differs: a previous value, not the present one.
    Stale,
    /// No record is stored.
    Missing,
}

impl Freshness {
    /// The wire token.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Current => "current",
            Self::Stale => "stale",
            Self::Missing => "missing",
        }
    }
}

/// Freshness of the stored volume and mass.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Assessment {
    /// Volume freshness.
    pub volume: Freshness,
    /// Mass freshness.
    pub mass: Freshness,
}

/// Compares a stored record with the one the engine computes from the present inputs.
pub fn assess(stored: Option<&StoredRecord>, expected: &QuantityRecord) -> Assessment {
    let Some(stored) = stored else {
        return Assessment {
            volume: Freshness::Missing,
            mass: Freshness::Missing,
        };
    };
    let volume = if stored.shape_signature == expected.shape_signature
        && stored.volume_status == expected.volume_status
        && stored.volume_mm3 == expected.volume_mm3
        && stored.algorithm == expected.algorithm
    {
        Freshness::Current
    } else {
        Freshness::Stale
    };
    let mass = if stored.mass_signature == expected.mass_signature
        && stored.mass_status == expected.mass_status
        && stored.mass_kg == expected.mass_kg
    {
        Freshness::Current
    } else {
        Freshness::Stale
    };
    Assessment { volume, mass }
}

fn digest(bytes: &[u8]) -> ContentDigest {
    // The encodings are bounded by the command caps, far below the digest's input limit; an
    // overflow would only make two different inputs share the digest of the empty message, which
    // the status of a stale record then exposes, so the fallback needs no panic.
    Sha256::digest(bytes).unwrap_or_else(|_| ContentDigest::from_bytes([0; 32]))
}

fn float(bytes: &mut Vec<u8>, value: f64) {
    bytes.extend_from_slice(&canonical_f64_bytes(value).unwrap_or([0xff; 8]));
}

/// Signature of the shape inputs of a plate: outline, thickness and holes, with the algorithm
/// and tolerance policy. Name, placement and material are not shape inputs.
pub fn plate_shape_signature(plate: &AuthoredPlate, algorithm: &str) -> ContentDigest {
    let mut bytes = Vec::with_capacity(256);
    bytes.extend_from_slice(b"BeamCraft/P04/PlateShape/v1\0");
    bytes.extend_from_slice(&TOLERANCE_POLICY_VERSION.to_le_bytes());
    bytes.extend_from_slice(&(algorithm.len() as u64).to_le_bytes());
    bytes.extend_from_slice(algorithm.as_bytes());
    bytes.extend_from_slice(&(plate.contour_points_mm.len() as u64).to_le_bytes());
    for v in plate.contour_points_mm.iter().flatten() {
        float(&mut bytes, *v);
    }
    float(&mut bytes, plate.thickness_mm);
    bytes.extend_from_slice(&(plate.holes_mm.len() as u64).to_le_bytes());
    for hole in &plate.holes_mm {
        bytes.extend_from_slice(&(hole.len() as u64).to_le_bytes());
        for v in hole.iter().flatten() {
            float(&mut bytes, *v);
        }
    }
    digest(&bytes)
}

/// Signature of the shape inputs of a member: axis, roll and the resolved section, with the
/// algorithm and tolerance policy. The profile digest is not an input; its values are.
pub fn member_shape_signature(
    member: &AuthoredMember,
    profile: &ProfileInfo,
    algorithm: &str,
) -> ContentDigest {
    let mut bytes = Vec::with_capacity(160);
    bytes.extend_from_slice(b"BeamCraft/P04/MemberShape/v1\0");
    bytes.extend_from_slice(&TOLERANCE_POLICY_VERSION.to_le_bytes());
    bytes.extend_from_slice(&(algorithm.len() as u64).to_le_bytes());
    bytes.extend_from_slice(algorithm.as_bytes());
    for v in member.axis_points_mm.iter().flatten() {
        float(&mut bytes, *v);
    }
    match member.roll_rad {
        Some(roll) => {
            bytes.push(1);
            float(&mut bytes, roll);
        }
        None => bytes.push(0),
    }
    match profile {
        // The P04 bytes of a rectangle are kept, so stored signatures stay current.
        ProfileInfo::Rectangle {
            width_mm,
            height_mm,
        } => {
            float(&mut bytes, *width_mm);
            float(&mut bytes, *height_mm);
        }
        ProfileInfo::Section(shape) => {
            bytes.extend_from_slice(b"section\0");
            bytes.extend_from_slice(shape.token().as_bytes());
            bytes.push(0);
            match shape {
                SectionShape::Parametric(p) => {
                    p.values().iter().for_each(|v| float(&mut bytes, *v))
                }
                SectionShape::Custom(c) => {
                    bytes.extend_from_slice(&(c.cavities.len() as u64).to_le_bytes());
                    for contour in core::iter::once(&c.outer).chain(c.cavities.iter()) {
                        bytes.extend_from_slice(&(contour.len() as u64).to_le_bytes());
                        contour.iter().flatten().for_each(|v| float(&mut bytes, *v));
                    }
                }
            }
        }
    }
    digest(&bytes)
}

/// Signature of the mass inputs: the shape signature and the density, or its absence.
pub fn mass_signature(shape: &ContentDigest, density_kg_m3: Option<f64>) -> ContentDigest {
    let mut bytes = Vec::with_capacity(64);
    bytes.extend_from_slice(b"BeamCraft/P04/Mass/v1\0");
    bytes.extend_from_slice(shape.as_bytes());
    match density_kg_m3 {
        Some(density) => {
            bytes.push(1);
            float(&mut bytes, density);
        }
        None => bytes.push(0),
    }
    digest(&bytes)
}

/// The record of the engine's quantities under the given shape signature.
pub fn record_for(shape: ContentDigest, quantities: &Quantities) -> QuantityRecord {
    let token = |s: QuantityStatus| s.as_str();
    QuantityRecord {
        volume_mm3: quantities.volume_mm3,
        volume_status: token(quantities.volume_status),
        mass_kg: quantities.mass_kg,
        mass_status: token(quantities.mass_status),
        shape_signature: shape,
        mass_signature: mass_signature(&shape, quantities.density_kg_m3),
        algorithm: quantities.algorithm.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use beamcraft_model::command::Placement;
    use beamcraft_model::id::ElementId;

    fn plate() -> AuthoredPlate {
        AuthoredPlate {
            id: ElementId::parse("019a7bf0-0000-7000-8000-000000000005").unwrap(),
            name: "Plate".into(),
            contour_points_mm: vec![
                [0.0, 0.0, 0.0],
                [1000.0, 0.0, 0.0],
                [1000.0, 500.0, 0.0],
                [0.0, 500.0, 0.0],
            ],
            thickness_mm: 10.0,
            placement: Placement::IDENTITY,
            material: None,
            holes_mm: Vec::new(),
        }
    }

    #[test]
    fn only_shape_inputs_enter_the_shape_signature() {
        let base = plate_shape_signature(&plate(), "a");
        let mut renamed = plate();
        renamed.name = "Other".into();
        renamed.material = Some(Sha256::digest(b"m").unwrap());
        renamed.placement.translation_mm = [5.0, 0.0, 0.0];
        assert_eq!(plate_shape_signature(&renamed, "a"), base);
        let mut thicker = plate();
        thicker.thickness_mm = 12.0;
        assert_ne!(plate_shape_signature(&thicker, "a"), base);
        let mut holed = plate();
        holed.holes_mm = vec![vec![[1.0, 1.0], [1.0, 2.0], [2.0, 2.0]]];
        assert_ne!(plate_shape_signature(&holed, "a"), base);
        assert_ne!(plate_shape_signature(&plate(), "b"), base);
    }

    #[test]
    fn density_changes_the_mass_signature_and_never_the_shape_signature() {
        let shape = plate_shape_signature(&plate(), "a");
        let none = mass_signature(&shape, None);
        assert_ne!(mass_signature(&shape, Some(7850.0)), none);
        assert_ne!(
            mass_signature(&shape, Some(7850.0)),
            mass_signature(&shape, Some(2700.0))
        );
        assert_eq!(mass_signature(&shape, None), none);
    }

    fn record(volume: f64, density: Option<f64>, shape: ContentDigest) -> QuantityRecord {
        QuantityRecord {
            volume_mm3: volume,
            volume_status: "current",
            mass_kg: density.map(|d| volume * d / 1e9),
            mass_status: if density.is_some() {
                "current"
            } else {
                "unknown"
            },
            shape_signature: shape,
            mass_signature: mass_signature(&shape, density),
            algorithm: "a".into(),
        }
    }

    fn stored(r: &QuantityRecord) -> StoredRecord {
        StoredRecord {
            volume_mm3: r.volume_mm3,
            volume_status: r.volume_status.into(),
            mass_kg: r.mass_kg,
            mass_status: r.mass_status.into(),
            shape_signature: r.shape_signature,
            mass_signature: r.mass_signature,
            algorithm: r.algorithm.clone(),
        }
    }

    #[test]
    fn a_stored_record_is_current_stale_or_missing_never_assumed() {
        let shape = plate_shape_signature(&plate(), "a");
        let now = record(5e6, Some(7850.0), shape);
        let old = stored(&now);
        assert_eq!(
            assess(Some(&old), &now),
            Assessment {
                volume: Freshness::Current,
                mass: Freshness::Current
            }
        );
        // Material changed: the volume is still current, the stored mass is stale.
        let relabelled = record(5e6, Some(2700.0), shape);
        assert_eq!(
            assess(Some(&old), &relabelled),
            Assessment {
                volume: Freshness::Current,
                mass: Freshness::Stale
            }
        );
        // Shape changed: both are stale.
        let mut thicker = plate();
        thicker.thickness_mm = 12.0;
        let new_shape = plate_shape_signature(&thicker, "a");
        assert_eq!(
            assess(Some(&old), &record(6e6, Some(7850.0), new_shape)),
            Assessment {
                volume: Freshness::Stale,
                mass: Freshness::Stale
            }
        );
        // A tampered value under a matching signature is not current either.
        let mut forged = old.clone();
        forged.volume_mm3 = 1.0;
        assert_eq!(assess(Some(&forged), &now).volume, Freshness::Stale);
        assert_eq!(
            assess(None, &now),
            Assessment {
                volume: Freshness::Missing,
                mass: Freshness::Missing
            }
        );
    }
}
