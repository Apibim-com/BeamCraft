// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! F04 command application for `CreatePlate` (P01): engineering validation, kernel evaluation
//! and the derived results of one accepted command.
//!
//! The engine never touches USD, files or a clock. It turns a typed command into an
//! [`AuthoredPlate`] (what the model layer must carry), a sealed kernel body (what the derived
//! layer must carry), quantities and a declared footprint. The adapter that writes USD and the
//! host that publishes revisions live in other crates and are told only these results.
//!
//! # What P01 supports, and what it does not
//!
//! F04 section 3 (P-1) admits any planar contour and evaluates a four-point rectangle in its own
//! body frame. The kernel today builds only the origin rectangle (G02). The first slice therefore
//! admits exactly the contour of F04-POS-01: `(0,0,0), (w,0,0), (w,l,0), (0,l,0)`, plus its
//! clockwise mirror, which is rejected as `INVALID_ARGUMENT`. Every other contour that passes the
//! structural stages (count, finiteness, caps, `D`, floor, repeated points) is rejected as
//! `UNSUPPORTED_GEOMETRY`. F04 would *accept* such a contour with an `unsupported` geometry status
//! (translated or rotated rectangles are supported by F04); P01 cannot record that status, so it
//! refuses the command and authors nothing. Exact planarity, winding and simplicity for general
//! polygons (so that F04-NEG-29 and F04-NEG-30 return `INVALID_ARGUMENT`) belong to A02/G04.
//! A placement with a rotation other than the identity is valid input but unsupported for
//! evaluation here (`UNSUPPORTED_GEOMETRY`): the view contract carries translations only.

use crate::footprint::Footprint;
use beamcraft_geom::tolerance::TOLERANCE_POLICY_VERSION;
use beamcraft_geom::{MathError, Point3};
use beamcraft_kernel::diagnostic::Failure;
use beamcraft_kernel::measure::analytic_mass_kg;
use beamcraft_kernel::{PlateResult, RectangularPlate, rectangular_plate};
use beamcraft_model::command::{CommandError, CreatePlate, Placement};
use beamcraft_model::definition::MaterialSpec;
use beamcraft_model::digest::ContentDigest;
use beamcraft_model::error::ErrorCode;
use beamcraft_model::id::ElementId;

/// Version label of the volume algorithm in a [`Quantities`] record.
pub const VOLUME_ALGORITHM: &str = "kernel-g02-rectangular-plate-volume/1;tolerance-policy-3";

const _: () = assert!(TOLERANCE_POLICY_VERSION == 3);

/// The physical content of a resolved immutable material definition.
///
/// The adapter that reads the committed model hands this over; the engine never reads USD. A
/// density of `None` is unknown and stays unknown.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MaterialInfo {
    /// Density in kg/m3, when the definition authors one.
    pub density_kg_m3: Option<f64>,
}

/// Resolves a pinned material digest. A digest with no definition is `NOT_FOUND`; a digest that
/// names a definition of another kind is `INVALID_ARGUMENT`.
pub type MaterialLookup<'a> = &'a dyn Fn(&ContentDigest) -> Result<MaterialInfo, CommandError>;

/// A lookup that knows no definition: any pinned material is `NOT_FOUND`.
pub fn no_materials(_: &ContentDigest) -> Result<MaterialInfo, CommandError> {
    Err(CommandError::new(
        ErrorCode::NotFound,
        "material_digest",
        "material definition is not in the model",
    ))
}

/// The authored state of one plate: exactly the inputs F04 section 4 stores in the model layer.
/// Nothing derived is in it.
#[derive(Debug, Clone, PartialEq)]
pub struct AuthoredPlate {
    /// Full element identity.
    pub id: ElementId,
    /// Label.
    pub name: String,
    /// Element-local contour as submitted, in millimetres.
    pub contour_points_mm: Vec<[f64; 3]>,
    /// Thickness in millimetres.
    pub thickness_mm: f64,
    /// Rigid placement as submitted.
    pub placement: Placement,
    /// Pinned physical material definition; absence means the material and density are unknown.
    pub material: Option<ContentDigest>,
}

impl AuthoredPlate {
    /// The authored fields of an accepted command (the command's arguments, unchanged).
    pub fn from_command(command: &CreatePlate) -> Self {
        Self {
            id: command.new_id,
            name: command.name.clone(),
            contour_points_mm: command.contour_points_mm.clone(),
            thickness_mm: command.thickness_mm,
            placement: command.placement,
            material: command.material_digest,
        }
    }

    /// The `CreatePlate` that would author exactly this state; used to compare content.
    pub fn to_command(&self) -> CreatePlate {
        CreatePlate {
            new_id: self.id,
            name: self.name.clone(),
            contour_points_mm: self.contour_points_mm.clone(),
            thickness_mm: self.thickness_mm,
            placement: self.placement,
            material_digest: self.material,
        }
    }
}

/// Status of one quantity (F04 section 5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuantityStatus {
    /// A supported computation over matching authored inputs.
    Current,
    /// No value: an input such as density is absent.
    Unknown,
}

impl QuantityStatus {
    /// The wire token.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Current => "current",
            Self::Unknown => "unknown",
        }
    }
}

/// The quantity records of one plate. Mass is `current` only with a resolved material that
/// authors a density; otherwise it is `unknown` and carries no value: density is never guessed.
/// The input signature of F06 section 7 is not implemented (`keys` are planned), so no signature
/// is claimed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Quantities {
    /// Volume in mm3: `(width * length) * thickness`.
    pub volume_mm3: f64,
    /// Volume in m3: the mm3 value divided by 1e9.
    pub volume_m3: f64,
    /// Status of the volume record.
    pub volume_status: QuantityStatus,
    /// Density in kg/m3 used for the mass, when the material authors one.
    pub density_kg_m3: Option<f64>,
    /// Mass in kg, present exactly when `mass_status` is `current`.
    pub mass_kg: Option<f64>,
    /// Status of the mass record.
    pub mass_status: QuantityStatus,
    /// Algorithm version of the volume record.
    pub algorithm: &'static str,
}

/// A plate evaluated by the kernel: the sealed body and its supported dimensions.
#[derive(Debug, Clone, PartialEq)]
pub struct PlateBody {
    /// Sealed kernel result (mesh, volume, tolerance).
    pub result: PlateResult,
    /// Width along local X, in millimetres.
    pub width_mm: f64,
    /// Length along local Y, in millimetres.
    pub length_mm: f64,
    /// Thickness along local Z, in millimetres.
    pub thickness_mm: f64,
}

impl PlateBody {
    /// The quantity records of this body with no material: volume only, mass `unknown`.
    pub fn quantities(&self) -> Quantities {
        Quantities {
            volume_mm3: self.result.volume_mm3(),
            volume_m3: self.result.volume_m3(),
            volume_status: QuantityStatus::Current,
            density_kg_m3: None,
            mass_kg: None,
            mass_status: QuantityStatus::Unknown,
            algorithm: VOLUME_ALGORITHM,
        }
    }

    /// The quantity records of this body with the physical material of the element.
    ///
    /// The density is re-admitted here because a committed definition is untrusted input. Mass is
    /// `fl(fl(V_mm3 * density) / 1e9)` (F06 section 2); an absent density keeps it `unknown`.
    pub fn quantities_for(
        &self,
        material: Option<MaterialInfo>,
    ) -> Result<Quantities, CommandError> {
        let mut quantities = self.quantities();
        let Some(density) = material.and_then(|m| m.density_kg_m3) else {
            return Ok(quantities);
        };
        MaterialSpec {
            density_kg_m3: Some(density),
        }
        .validate()?;
        let record = analytic_mass_kg(quantities.volume_mm3, Some(density)).map_err(|code| {
            CommandError::new(
                code.f04_code(),
                "density_kg_m3",
                "mass could not be computed",
            )
        })?;
        quantities.density_kg_m3 = Some(density);
        quantities.mass_kg = record.kg;
        quantities.mass_status = if record.kg.is_some() {
            QuantityStatus::Current
        } else {
            QuantityStatus::Unknown
        };
        Ok(quantities)
    }
}

/// Everything one accepted `CreatePlate` produces for the layers and the host.
#[derive(Debug, Clone, PartialEq)]
pub struct CreatePlateOutcome {
    /// The authored state for the model layer.
    pub authored: AuthoredPlate,
    /// The evaluated body for the derived layer.
    pub body: PlateBody,
    /// Quantity records.
    pub quantities: Quantities,
    /// Declared read and write sets; callers cannot narrow them.
    pub footprint: Footprint,
    /// Digest of the typed arguments, for replay and reuse decisions.
    pub fingerprint: ContentDigest,
}

fn map_math(error: MathError, field: &'static str) -> CommandError {
    let (code, message) = match error {
        MathError::NonFinite => (ErrorCode::NonFiniteNumber, "value is not finite"),
        MathError::AboveLimit => (ErrorCode::LimitExceeded, "value exceeds its limit"),
        MathError::InvalidPointCount => (
            ErrorCode::InvalidArgument,
            "contour needs at least 3 points",
        ),
        MathError::NotPositive | MathError::BelowMinimum => (
            ErrorCode::InvalidArgument,
            "dimension is not above the geometric floor",
        ),
        MathError::AxisTooShort | MathError::ZeroAxis | MathError::InvalidFrame => {
            (ErrorCode::InvalidArgument, "degenerate geometry")
        }
        MathError::NumericUncertain => (
            ErrorCode::UnsupportedGeometry,
            "numeric result is uncertain",
        ),
    };
    CommandError::new(code, field, message)
}

fn map_kernel(failure: &Failure) -> CommandError {
    failure.diagnostics().next().map_or(
        CommandError::new(
            ErrorCode::UnsupportedGeometry,
            "contour_points_mm",
            "geometry evaluation failed",
        ),
        |d| {
            let field = match d.field {
                beamcraft_kernel::diagnostic::Field::Width
                | beamcraft_kernel::diagnostic::Field::Length => "contour_points_mm",
                beamcraft_kernel::diagnostic::Field::Thickness => "thickness_mm",
                _ => "geometry",
            };
            CommandError::new(d.code.f04_code(), field, "geometry evaluation failed")
        },
    )
}

/// How a contour relates to the first-slice shape.
enum Contour {
    /// `(0,0,0), (w,0,0), (w,l,0), (0,l,0)` with `w, l > 0`.
    OriginRectangle { width: f64, length: f64 },
    /// The same rectangle listed clockwise.
    ClockwiseOriginRectangle,
    /// Anything else.
    Other,
}

fn classify(points: &[[f64; 3]]) -> Contour {
    let [p0, p1, p2, p3] = match points {
        [a, b, c, d] => [*a, *b, *c, *d],
        _ => return Contour::Other,
    };
    let zero = |v: f64| v == 0.0;
    if !(zero(p0[0]) && zero(p0[1]) && zero(p0[2])) {
        return Contour::Other;
    }
    if zero(p1[1]) && zero(p1[2]) && p1[0] > 0.0 {
        let (w, l) = (p1[0], p2[1]);
        if p2[0] == w && zero(p2[2]) && l > 0.0 && zero(p3[0]) && p3[1] == l && zero(p3[2]) {
            return Contour::OriginRectangle {
                width: w,
                length: l,
            };
        }
    }
    if zero(p1[0]) && zero(p1[2]) && p1[1] > 0.0 {
        let (l, w) = (p1[1], p2[0]);
        if p2[1] == l && zero(p2[2]) && w > 0.0 && p3[0] == w && zero(p3[1]) && zero(p3[2]) {
            return Contour::ClockwiseOriginRectangle;
        }
    }
    Contour::Other
}

/// Evaluates an authored plate with the kernel: engineering validation in the F04 order that
/// P01 implements, then the sealed rectangular body.
///
/// Order: count and finiteness and coordinate caps (`contour_points_mm`), local size `D` and its
/// cap, thickness range and floor, repeated consecutive points, then the supported shape (see
/// the module documentation). The caller is expected to have run
/// [`CreatePlate::validate_arguments`] for label, placement and the argument-level caps; this
/// function repeats the checks it depends on and never panics on any input.
pub fn evaluate_plate(authored: &AuthoredPlate) -> Result<PlateBody, CommandError> {
    let points = &authored.contour_points_mm;
    let raw: Vec<Point3> = points
        .iter()
        .map(|[x, y, z]| Point3::new(*x, *y, *z))
        .collect();
    let tolerance = beamcraft_geom::tolerance::Tolerance::for_plate(&raw)
        .map_err(|e| map_math(e, "contour_points_mm"))?;
    tolerance
        .check_dimension(authored.thickness_mm)
        .map_err(|e| map_math(e, "thickness_mm"))?;
    for (index, a) in raw.iter().enumerate() {
        let b = raw
            .get(index.saturating_add(1))
            .or_else(|| raw.first())
            .copied()
            .unwrap_or(*a);
        if tolerance
            .coincident(*a, b)
            .map_err(|e| map_math(e, "contour_points_mm"))?
        {
            return Err(CommandError::new(
                ErrorCode::InvalidArgument,
                "contour_points_mm",
                "repeated consecutive contour points",
            ));
        }
    }
    match classify(points) {
        Contour::OriginRectangle { width, length } => {
            let result = rectangular_plate(RectangularPlate {
                width_mm: width,
                length_mm: length,
                thickness_mm: authored.thickness_mm,
            })
            .map_err(|f| map_kernel(&f))?;
            Ok(PlateBody {
                result,
                width_mm: width,
                length_mm: length,
                thickness_mm: authored.thickness_mm,
            })
        }
        Contour::ClockwiseOriginRectangle => Err(CommandError::new(
            ErrorCode::InvalidArgument,
            "contour_points_mm",
            "contour winding is clockwise",
        )),
        Contour::Other => Err(CommandError::new(
            ErrorCode::UnsupportedGeometry,
            "contour_points_mm",
            "only the origin rectangle contour is evaluated in this slice",
        )),
    }
}

/// The contour of the origin rectangle of F04-POS-01: `(0,0,0), (w,0,0), (w,l,0), (0,l,0)`,
/// counter-clockwise seen from +Z. This is the one contour the kernel evaluates today, so a
/// dimension-based front end (the CLI's `--width --length`) builds it here instead of inventing
/// its own.
///
/// A non-finite dimension is `NON_FINITE_NUMBER`; zero or negative is `INVALID_ARGUMENT`. Caps
/// and the geometric floor are checked by [`create_plate`].
pub fn origin_rectangle_contour(
    width_mm: f64,
    length_mm: f64,
) -> Result<Vec<[f64; 3]>, CommandError> {
    if !width_mm.is_finite() || !length_mm.is_finite() {
        return Err(CommandError::new(
            ErrorCode::NonFiniteNumber,
            "contour_points_mm",
            "value is not finite",
        ));
    }
    if width_mm <= 0.0 || length_mm <= 0.0 {
        return Err(CommandError::new(
            ErrorCode::InvalidArgument,
            "contour_points_mm",
            "dimension is not positive",
        ));
    }
    Ok(vec![
        [0.0, 0.0, 0.0],
        [width_mm, 0.0, 0.0],
        [width_mm, length_mm, 0.0],
        [0.0, length_mm, 0.0],
    ])
}

/// Applies `CreatePlate` to an isolated draft in memory: argument admission, engineering
/// validation, kernel evaluation, quantities and the declared footprint. Nothing is published.
///
/// `existing` is the set of element IDs already owned by the draft; a creation ID in it is
/// `DUPLICATE_ID`, checked after argument admission and before geometry as F04 section 8 orders.
/// A pinned material is resolved next (`materials`), so a missing definition is `NOT_FOUND`
/// before any geometry runs and its read is part of the footprint.
/// A placement rotation other than the identity is `UNSUPPORTED_GEOMETRY` (module documentation).
pub fn create_plate(
    command: &CreatePlate,
    existing: &dyn Fn(&ElementId) -> bool,
    materials: MaterialLookup<'_>,
) -> Result<CreatePlateOutcome, CommandError> {
    command.validate_arguments()?;
    if existing(&command.new_id) {
        return Err(CommandError::new(
            ErrorCode::DuplicateId,
            "new_id",
            "element id is already owned",
        ));
    }
    let material = command
        .material_digest
        .as_ref()
        .map(materials)
        .transpose()?;
    let footprint = Footprint::for_create(command.new_id, command.material_digest)?;
    let authored = AuthoredPlate::from_command(command);
    let body = evaluate_plate(&authored)?;
    if !command.placement.is_identity_rotation() {
        return Err(CommandError::new(
            ErrorCode::UnsupportedGeometry,
            "placement.orientation_xyzw",
            "rotated placement is not evaluated in this slice",
        ));
    }
    let quantities = body.quantities_for(material)?;
    Ok(CreatePlateOutcome {
        fingerprint: command.content_fingerprint()?,
        authored,
        body,
        quantities,
        footprint,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn command(contour: Vec<[f64; 3]>, thickness: f64) -> CreatePlate {
        CreatePlate {
            new_id: ElementId::parse("019a7bf0-0000-7000-8000-000000000005").unwrap(),
            name: "Synthetic plate".into(),
            contour_points_mm: contour,
            thickness_mm: thickness,
            placement: Placement::IDENTITY,
            material_digest: None,
        }
    }

    fn rect(w: f64, l: f64) -> Vec<[f64; 3]> {
        vec![[0.0, 0.0, 0.0], [w, 0.0, 0.0], [w, l, 0.0], [0.0, l, 0.0]]
    }

    fn none(_: &ElementId) -> bool {
        false
    }

    // Shadows the glob import: these tests have no material definitions unless they say so.
    fn create_plate(
        command: &CreatePlate,
        existing: &dyn Fn(&ElementId) -> bool,
    ) -> Result<CreatePlateOutcome, CommandError> {
        super::create_plate(command, existing, &no_materials)
    }

    fn digest(text: &[u8]) -> ContentDigest {
        beamcraft_model::digest::Sha256::digest(text).unwrap()
    }

    #[test]
    fn a_pinned_material_with_density_gives_a_current_mass_and_is_in_the_footprint() {
        let known = digest(b"steel");
        let lookup = |d: &ContentDigest| {
            if *d == known {
                Ok(MaterialInfo {
                    density_kg_m3: Some(1000.0),
                })
            } else {
                no_materials(d)
            }
        };
        let mut c = command(rect(1000.0, 500.0), 10.0);
        c.material_digest = Some(known);
        let out = super::create_plate(&c, &none, &lookup).unwrap();
        // F04 section 5 analytic example: 0.005 m3 at 1000 kg/m3 is 5 kg.
        assert_eq!(out.quantities.mass_status, QuantityStatus::Current);
        assert_eq!(out.quantities.density_kg_m3, Some(1000.0));
        assert!((out.quantities.mass_kg.unwrap() - 5.0).abs() <= 1e-9);
        assert_eq!(out.authored.material, Some(known));
        assert_eq!(out.authored.to_command(), c);
        assert!(out.footprint.reads().iter().any(|e| {
            e.target == crate::footprint::Target::Definition(known)
                && e.group == crate::footprint::Group::Relationship
        }));
    }

    #[test]
    fn unknown_density_stays_unknown_and_a_missing_definition_is_not_found() {
        let known = digest(b"no-density");
        let lookup = |d: &ContentDigest| {
            if *d == known {
                Ok(MaterialInfo {
                    density_kg_m3: None,
                })
            } else {
                no_materials(d)
            }
        };
        let mut c = command(rect(1000.0, 500.0), 10.0);
        c.material_digest = Some(known);
        let out = super::create_plate(&c, &none, &lookup).unwrap();
        assert_eq!(out.quantities.mass_status, QuantityStatus::Unknown);
        assert_eq!(out.quantities.mass_kg, None);
        assert_eq!(out.quantities.density_kg_m3, None);
        c.material_digest = Some(digest(b"absent"));
        assert_eq!(code(&c), ErrorCode::NotFound);
        // The definition is resolved before geometry runs.
        c.thickness_mm = 0.0;
        assert_eq!(code(&c), ErrorCode::NotFound);
    }

    #[test]
    fn a_committed_density_is_admitted_again_before_it_makes_a_mass() {
        let body = evaluate_plate(&AuthoredPlate::from_command(&command(
            rect(1000.0, 500.0),
            10.0,
        )))
        .unwrap();
        for (density, expected) in [
            (f64::NAN, ErrorCode::NonFiniteNumber),
            (0.0, ErrorCode::InvalidArgument),
            (-7850.0, ErrorCode::InvalidArgument),
            (2_000_000.0, ErrorCode::LimitExceeded),
        ] {
            let error = body
                .quantities_for(Some(MaterialInfo {
                    density_kg_m3: Some(density),
                }))
                .unwrap_err();
            assert_eq!(error.code, expected, "{density}");
        }
        assert_eq!(
            body.quantities_for(None).unwrap().mass_status,
            QuantityStatus::Unknown
        );
    }

    fn code(c: &CreatePlate) -> ErrorCode {
        create_plate(c, &none).unwrap_err().code
    }

    #[test]
    fn the_first_scope_plate_is_created_with_independent_volume() {
        let out = create_plate(&command(rect(1000.0, 500.0), 10.0), &none).unwrap();
        // Independent closed form: 1000 * 500 * 10 mm3 = 5,000,000 mm3 = 0.005 m3 (F04 section 5).
        assert_eq!(out.quantities.volume_mm3, 5_000_000.0);
        assert!((out.quantities.volume_m3 - 0.005).abs() <= 1e-6 * 0.005);
        assert_eq!(out.quantities.volume_status, QuantityStatus::Current);
        assert_eq!(out.quantities.mass_status, QuantityStatus::Unknown);
        assert_eq!(out.body.width_mm, 1000.0);
        assert_eq!(out.body.length_mm, 500.0);
        assert_eq!(out.authored.thickness_mm, 10.0);
        assert!(!out.footprint.writes().is_empty());
    }

    #[test]
    fn rejections_carry_the_f04_codes_of_the_cases() {
        let ok = rect(1000.0, 500.0);
        assert_eq!(code(&command(ok.clone(), 0.0)), ErrorCode::InvalidArgument); // F04-NEG-04
        assert_eq!(code(&command(ok.clone(), -1.0)), ErrorCode::InvalidArgument); // (negative thickness; the UpdateElement case F04-NEG-14 is not run)
        assert_eq!(
            code(&command(rect(1_000_001.0, 500.0), 10.0)),
            ErrorCode::LimitExceeded
        ); // F04-NEG-05
        assert_eq!(
            code(&command(rect(1_000_000.001, 500.0), 10.0)),
            ErrorCode::LimitExceeded
        ); // F04-NEG-19
        assert_eq!(
            code(&command(ok.clone(), 0.0009)),
            ErrorCode::InvalidArgument
        ); // F04-NEG-27
        assert_eq!(
            code(&command(vec![[0.0; 3], [1000.0, 0.0, 0.0]], 10.0)),
            ErrorCode::InvalidArgument
        ); // F04-NEG-31
        let repeated = vec![
            [0.0, 0.0, 0.0],
            [1000.0, 0.0, 0.0],
            [1000.0, 0.0, 0.0],
            [1000.0, 500.0, 0.0],
            [0.0, 500.0, 0.0],
        ];
        assert_eq!(code(&command(repeated, 10.0)), ErrorCode::InvalidArgument); // F04-NEG-32
        let clockwise = vec![
            [0.0, 0.0, 0.0],
            [0.0, 500.0, 0.0],
            [1000.0, 500.0, 0.0],
            [1000.0, 0.0, 0.0],
        ];
        assert_eq!(code(&command(clockwise, 10.0)), ErrorCode::InvalidArgument); // F04-NEG-28
        let wide = vec![
            [0.0, 0.0, 0.0],
            [750_000.0, 0.0, 0.0],
            [1_500_000.0, 0.0, 0.0],
            [2_250_000.0, 0.0, 0.0],
            [2_250_000.0, 1000.0, 0.0],
            [1_500_000.0, 1000.0, 0.0],
            [750_000.0, 1000.0, 0.0],
            [0.0, 1000.0, 0.0],
        ];
        assert_eq!(code(&command(wide, 10.0)), ErrorCode::LimitExceeded); // F04-NEG-35
    }

    #[test]
    fn the_inclusive_floor_is_the_scaled_geometric_minimum() {
        // F04-POS-17: 0.001 mm is exactly the absolute floor, so it is accepted.
        let out = create_plate(&command(rect(1000.0, 500.0), 0.001), &none).unwrap();
        assert!((out.quantities.volume_m3 - 5e-7).abs() <= 1e-6 * 5e-7);
        // F04-POS-27: the 3 x 4 plate has D = 5, so 5 * 2^-22 is far below 0.001.
        assert!(create_plate(&command(rect(3.0, 4.0), 0.001), &none).is_ok());
        // F04-POS-08: the largest first-scope plate.
        let big = create_plate(&command(rect(1e6, 1e6), 1e6), &none).unwrap();
        assert!((big.quantities.volume_m3 - 1e9).abs() <= 1e-6 * 1e9);
    }

    #[test]
    fn other_valid_contours_are_unsupported_not_guessed() {
        let moved = vec![
            [10.0, 20.0, 30.0],
            [10.0, 1020.0, 30.0],
            [-490.0, 1020.0, 30.0],
            [-490.0, 20.0, 30.0],
        ];
        assert_eq!(code(&command(moved, 10.0)), ErrorCode::UnsupportedGeometry); // F04-POS-18
        let pentagon = vec![
            [0.0, 0.0, 0.0],
            [1000.0, 0.0, 0.0],
            [1200.0, 250.0, 0.0],
            [500.0, 600.0, 0.0],
            [0.0, 500.0, 0.0],
        ];
        assert_eq!(
            code(&command(pentagon, 10.0)),
            ErrorCode::UnsupportedGeometry
        ); // F04-POS-20 (F04 would accept with status unsupported)
    }

    #[test]
    fn argument_errors_come_before_geometry() {
        let mut c = command(rect(1000.0, 500.0), 10.0);
        c.placement.orientation_xyzw = [0.0, 0.0, 0.0, 0.5];
        assert_eq!(code(&c), ErrorCode::InvalidArgument); // F04-NEG-22
        c.placement.orientation_xyzw = [0.0, 0.0, 0.0, 0.99999995];
        assert!(create_plate(&c, &none).is_ok()); // F04-POS-11
        c.placement.orientation_xyzw = [0.0, 0.0, 0.0, -1.0];
        assert!(create_plate(&c, &none).is_ok()); // F04-POS-12
        c.placement.translation_mm = [1e9, 0.0, 0.0];
        assert!(create_plate(&c, &none).is_ok()); // F04-POS-09
        let s = core::f64::consts::FRAC_1_SQRT_2;
        c.placement.orientation_xyzw = [0.0, 0.0, s, s];
        assert_eq!(code(&c), ErrorCode::UnsupportedGeometry);
        let mut nan = command(rect(1000.0, 500.0), 10.0);
        nan.contour_points_mm[1][0] = f64::NAN;
        assert_eq!(code(&nan), ErrorCode::NonFiniteNumber);
    }

    #[test]
    fn an_owned_id_is_a_duplicate_before_any_geometry() {
        let c = command(rect(1000.0, 500.0), 10.0);
        let taken = |id: &ElementId| *id == c.new_id;
        assert_eq!(
            create_plate(&c, &taken).unwrap_err().code,
            ErrorCode::DuplicateId
        ); // F04-NEG-03
        let mut bad = command(rect(1000.0, 500.0), 0.0);
        bad.new_id = c.new_id;
        assert_eq!(
            create_plate(&bad, &taken).unwrap_err().code,
            ErrorCode::DuplicateId
        );
    }

    #[test]
    fn evaluation_never_panics_on_hostile_contours() {
        for contour in [
            vec![],
            vec![[0.0; 3]],
            vec![[f64::MAX; 3]; 4],
            vec![[f64::NAN; 3]; 4],
            vec![[1e300, 0.0, 0.0], [-1e300, 0.0, 0.0], [0.0, 1e300, 0.0]],
            vec![[0.0; 3]; 1024],
        ] {
            let authored = AuthoredPlate {
                id: ElementId::parse("019a7bf0-0000-7000-8000-000000000005").unwrap(),
                name: "x".into(),
                contour_points_mm: contour,
                thickness_mm: 10.0,
                placement: Placement::IDENTITY,
                material: None,
            };
            assert!(evaluate_plate(&authored).is_err());
        }
    }

    #[test]
    fn the_dimension_front_end_builds_the_origin_rectangle() {
        let contour = origin_rectangle_contour(1000.0, 500.0).unwrap();
        assert_eq!(contour, rect(1000.0, 500.0));
        for (w, l, expected) in [
            (0.0, 5.0, ErrorCode::InvalidArgument),
            (5.0, -1.0, ErrorCode::InvalidArgument),
            (f64::NAN, 5.0, ErrorCode::NonFiniteNumber),
            (5.0, f64::INFINITY, ErrorCode::NonFiniteNumber),
        ] {
            assert_eq!(origin_rectangle_contour(w, l).unwrap_err().code, expected);
        }
    }

    #[test]
    fn the_same_arguments_fingerprint_the_same_and_round_trip_through_authored_state() {
        let c = command(rect(1000.0, 500.0), 10.0);
        let out = create_plate(&c, &none).unwrap();
        assert_eq!(out.authored.to_command(), c);
        assert_eq!(
            out.fingerprint,
            out.authored.to_command().content_fingerprint().unwrap()
        );
    }
}
