// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! P01 headless commands: `create-plate` and `plate-inspect`. They call the engine API through
//! the adapter of `beamcraft_cli::plate` and print one JSON object (same error shape as the other
//! commands; exit 0 success, 2 rejection, 3 backend unavailable, 4 outcome unknown).

use beamcraft_cli::definition::{
    DefineRequest, Defined, DefinitionRecord, define, definition_path,
};
use beamcraft_cli::edit::{UpdateRequest, UpdatedElement, update_element};
use beamcraft_cli::member::{CreateMemberRequest, create_member};
use beamcraft_cli::plate::{
    CreateRequest, Created, Inspection, PlateError, QuantityReport, RevisionSummary, create_plate,
    inspect,
};
use beamcraft_engine::command::{Quantities, origin_rectangle_contour};
use beamcraft_engine::footprint::{Entry, Footprint, Target};
use beamcraft_engine::quantity::Freshness;
use beamcraft_engine::query::committed::MemberRecord;
use beamcraft_engine::query::committed::PlateRecord;
use beamcraft_host::{
    ids::IdSource,
    revision::{NoFault, RevisionStore, Uuid7},
    storage::BlobStore,
};
use beamcraft_model::command::{
    CreatePlate, CreateStraightMember, ElementPatch, MAX_HOLES, MemberKind, Placement,
    UpdateElement,
};
use beamcraft_model::digest::ContentDigest;
use beamcraft_model::error::ErrorCode;
use beamcraft_model::id::{ElementId, OpId, ProjectId, RevisionId, WorkspaceId};
use beamcraft_protocol::json::Json;
use std::collections::BTreeMap;
use std::ffi::OsString;

type Run<T> = Result<T, PlateError>;

fn invalid(message: &'static str) -> PlateError {
    PlateError {
        code: ErrorCode::InvalidArgument,
        message,
    }
}

/// Runs one plate command and returns the JSON result and the process exit code.
pub fn execute(args: &[OsString]) -> (Json, u8) {
    match run(args) {
        Ok(json) => (json, 0),
        Err(e) => (
            Json::object([
                (
                    "outcome",
                    Json::text(if e.code == ErrorCode::OutcomeUnknown {
                        "outcome_unknown"
                    } else {
                        "rejected"
                    }),
                ),
                ("code", Json::text(e.code.as_str())),
                ("message", Json::text(e.message)),
                ("transport_code", Json::Null),
                ("observed_context", Json::Null),
                ("diagnostics", Json::Array(vec![])),
            ]),
            match e.code {
                ErrorCode::OutcomeUnknown => 4,
                ErrorCode::BackendUnavailable => 3,
                _ => 2,
            },
        ),
    }
}

const CREATE_OPTIONS: &[&str] = &[
    "--store",
    "--width",
    "--length",
    "--thickness",
    "--name",
    "--translation",
    "--material-digest",
    "--hole-rect",
    "--hole-round",
    "--hole-segments",
    "--new-id",
    "--op-id",
    "--revision-id",
    "--actor-id",
    "--project-id",
    "--workspace-id",
];

const DEFINE_MATERIAL_OPTIONS: &[&str] = &[
    "--store",
    "--density",
    "--revision-id",
    "--actor-id",
    "--project-id",
    "--workspace-id",
];

const DEFINE_PROFILE_OPTIONS: &[&str] = &[
    "--store",
    "--width",
    "--height",
    "--revision-id",
    "--actor-id",
    "--project-id",
    "--workspace-id",
];

const CREATE_MEMBER_OPTIONS: &[&str] = &[
    "--store",
    "--kind",
    "--start",
    "--end",
    "--profile-digest",
    "--roll",
    "--name",
    "--translation",
    "--material-digest",
    "--new-id",
    "--op-id",
    "--revision-id",
    "--actor-id",
    "--project-id",
    "--workspace-id",
];

const UPDATE_OPTIONS: &[&str] = &[
    "--store",
    "--id",
    "--name",
    "--material-digest",
    "--thickness",
    "--op-id",
    "--revision-id",
    "--actor-id",
    "--project-id",
    "--workspace-id",
];

fn run(args: &[OsString]) -> Run<Json> {
    let args: Vec<&str> = args
        .iter()
        .map(|a| {
            a.to_str()
                .filter(|s| s.len() <= 4096)
                .ok_or_else(|| invalid("argument is not bounded UTF-8"))
        })
        .collect::<Run<_>>()?;
    let (command, tail) = args
        .split_first()
        .ok_or_else(|| invalid("missing command"))?;
    let allowed: &[&str] = match *command {
        "create-plate" => CREATE_OPTIONS,
        "create-member" => CREATE_MEMBER_OPTIONS,
        "update-element" => UPDATE_OPTIONS,
        "define-material" => DEFINE_MATERIAL_OPTIONS,
        "define-profile" => DEFINE_PROFILE_OPTIONS,
        "plate-inspect" | "definitions" => &["--store"],
        _ => return Err(invalid("unknown command")),
    };
    if !tail.len().is_multiple_of(2) || tail.len() > 40 {
        return Err(invalid("options are KEY VALUE pairs"));
    }
    let mut options = BTreeMap::new();
    for pair in tail.as_chunks::<2>().0 {
        let [key, value] = pair;
        if !allowed.contains(key) || options.insert(*key, *value).is_some() {
            return Err(invalid("unknown or repeated option"));
        }
    }
    let store_root = options
        .get("--store")
        .ok_or_else(|| invalid("--store is required"))?;
    let blobs = BlobStore::new(store_root).map_err(|e| PlateError {
        code: e.code(),
        message: "store root is not usable",
    })?;
    let store = RevisionStore::new(blobs);
    match *command {
        "create-plate" => create(&store, &options),
        "create-member" => create_member_command(&store, &options),
        "update-element" => update_command(&store, &options),
        "define-material" | "define-profile" => define_command(&store, command, &options),
        "definitions" => definitions_json(&inspect(&store)?),
        _ => Ok(inspection_json(&inspect(&store)?)?),
    }
}

fn float(options: &BTreeMap<&str, &str>, key: &'static str) -> Run<f64> {
    options
        .get(key)
        .ok_or_else(|| invalid("a required dimension is missing"))?
        .parse::<f64>()
        .map_err(|_| invalid("a number is malformed"))
}

fn translation(text: &str) -> Run<[f64; 3]> {
    let parts: Vec<f64> = text
        .split(',')
        .map(|p| {
            p.parse::<f64>()
                .map_err(|_| invalid("translation is not three numbers"))
        })
        .collect::<Run<_>>()?;
    parts
        .try_into()
        .map_err(|_| invalid("translation is not three numbers"))
}

fn id_error(error: beamcraft_model::id::IdError) -> PlateError {
    PlateError {
        code: error.code(),
        message: "identity is not a full canonical UUIDv7",
    }
}

fn digest_option(text: &str) -> Run<ContentDigest> {
    text.parse::<ContentDigest>()
        .map_err(|_| invalid("a digest is not 64 lower-case hexadecimal characters"))
}

fn define_command(
    store: &RevisionStore,
    command: &str,
    options: &BTreeMap<&str, &str>,
) -> Run<Json> {
    let mut source = IdSource::system();
    let mut fresh = || {
        source.next_id().map_err(|_| PlateError {
            code: ErrorCode::BackendUnavailable,
            message: "no identity could be generated",
        })
    };
    let given = |key: &str| options.get(key).copied();
    let definition = if command == "define-material" {
        DefinitionRecord::Material {
            density_kg_m3: given("--density")
                .map(|text| {
                    text.parse::<f64>()
                        .map_err(|_| invalid("a number is malformed"))
                })
                .transpose()?,
        }
    } else {
        DefinitionRecord::RectangleProfile {
            width_mm: float(options, "--width")?,
            height_mm: float(options, "--height")?,
        }
    };
    let revision_id = match given("--revision-id") {
        Some(text) => RevisionId::parse(text).map_err(id_error)?,
        None => RevisionId::from_uuid(fresh()?),
    };
    let actor = match given("--actor-id") {
        Some(text) => Uuid7::parse(text).map_err(PlateError::from)?,
        None => Uuid7::parse(&fresh()?.to_string()).map_err(PlateError::from)?,
    };
    let project = match given("--project-id") {
        Some(text) => ProjectId::parse(text).map_err(id_error)?,
        None => ProjectId::from_uuid(fresh()?),
    };
    let workspace = match given("--workspace-id") {
        Some(text) => WorkspaceId::parse(text).map_err(id_error)?,
        None => WorkspaceId::from_uuid(fresh()?),
    };
    let defined = define(
        store,
        &DefineRequest {
            revision_id,
            op_id: OpId::from_uuid(fresh()?),
            actor,
            project,
            workspace,
            definition,
        },
        &mut NoFault,
    )?;
    defined_json(&defined)
}

fn create(store: &RevisionStore, options: &BTreeMap<&str, &str>) -> Run<Json> {
    let mut source = IdSource::system();
    let mut fresh = || {
        source.next_id().map_err(|_| PlateError {
            code: ErrorCode::BackendUnavailable,
            message: "no identity could be generated",
        })
    };
    let given = |key: &str| options.get(key).copied();
    let new_id = match given("--new-id") {
        Some(text) => ElementId::parse(text).map_err(id_error)?,
        None => ElementId::from_uuid(fresh()?),
    };
    let op_id = match given("--op-id") {
        Some(text) => OpId::parse(text).map_err(id_error)?,
        None => OpId::from_uuid(fresh()?),
    };
    let revision_id = match given("--revision-id") {
        Some(text) => RevisionId::parse(text).map_err(id_error)?,
        None => RevisionId::from_uuid(fresh()?),
    };
    let actor = match given("--actor-id") {
        Some(text) => Uuid7::parse(text).map_err(PlateError::from)?,
        None => Uuid7::parse(&fresh()?.to_string()).map_err(PlateError::from)?,
    };
    let project = match given("--project-id") {
        Some(text) => ProjectId::parse(text).map_err(id_error)?,
        None => ProjectId::from_uuid(fresh()?),
    };
    let workspace = match given("--workspace-id") {
        Some(text) => WorkspaceId::parse(text).map_err(id_error)?,
        None => WorkspaceId::from_uuid(fresh()?),
    };
    let width = float(options, "--width")?;
    let length = float(options, "--length")?;
    let thickness = float(options, "--thickness")?;
    let contour = origin_rectangle_contour(width, length).map_err(PlateError::from)?;
    let name = given("--name").unwrap_or("Plate").to_owned();
    let command = CreatePlate {
        new_id,
        name,
        contour_points_mm: contour,
        thickness_mm: thickness,
        placement: Placement {
            translation_mm: given("--translation").map_or(Ok([0.0; 3]), translation)?,
            ..Placement::IDENTITY
        },
        material_digest: given("--material-digest").map(digest_option).transpose()?,
        holes_mm: holes(options)?,
    };
    let created = create_plate(
        store,
        &CreateRequest {
            op_id,
            revision_id,
            actor,
            project,
            workspace,
            command,
        },
        &mut NoFault,
    )?;
    created_json(&created, op_id)
}

/// The through holes of `create-plate`: `--hole-rect "X,Y,W,H;..."` (axis-aligned rectangles from
/// the corner `X,Y`) and `--hole-round "X,Y,D;..."` (a regular polygon of `--hole-segments`
/// sides, default 32, inscribed in the circle of centre `X,Y` and diameter `D`). Both are only
/// generators: the model stores the clockwise polygons, so a "round" hole is that polygon and its
/// volume is the polygon's, exactly. The polygon vertices use the platform sin and cos here, at
/// the input edge, and are authored as the doubles they round to.
fn holes(options: &BTreeMap<&str, &str>) -> Run<Vec<Vec<[f64; 2]>>> {
    let given = |key: &str| options.get(key).copied();
    let numbers = |text: &str, count: usize| -> Run<Vec<f64>> {
        let values: Vec<f64> = text
            .split(',')
            .map(|p| {
                p.trim()
                    .parse::<f64>()
                    .map_err(|_| invalid("a hole value is not a number"))
            })
            .collect::<Run<_>>()?;
        if values.len() == count {
            Ok(values)
        } else {
            Err(invalid("a hole has the wrong number of values"))
        }
    };
    let segments = match given("--hole-segments") {
        Some(text) => text
            .parse::<usize>()
            .ok()
            .filter(|n| (8..=256).contains(n))
            .ok_or_else(|| invalid("--hole-segments is 8 to 256"))?,
        None => 32,
    };
    // Non-finite values stay the kernel's `NON_FINITE_NUMBER`; only finite non-positive sizes are
    // refused here, because a reversed polygon would otherwise pass as a valid clockwise hole.
    let size_ok = |sizes: &[f64]| sizes.iter().all(|v| !v.is_finite() || *v > 0.0);
    let mut result = Vec::new();
    if let Some(list) = given("--hole-rect") {
        for item in list.split(';').take(MAX_HOLES.saturating_add(1)) {
            if let [x, y, w, h] = numbers(item, 4)?[..] {
                if !size_ok(&[w, h]) {
                    return Err(invalid("a hole width and height must be positive"));
                }
                result.push(vec![[x, y], [x, y + h], [x + w, y + h], [x + w, y]]);
            }
        }
    }
    if let Some(list) = given("--hole-round") {
        for item in list.split(';').take(MAX_HOLES.saturating_add(1)) {
            if let [x, y, d] = numbers(item, 3)?[..] {
                if !size_ok(&[d]) {
                    return Err(invalid("a hole diameter must be positive"));
                }
                let r = d / 2.0;
                result.push(
                    (0..segments)
                        .map(|k| {
                            let theta = -std::f64::consts::TAU * (k as f64) / (segments as f64);
                            [x + r * theta.cos(), y + r * theta.sin()]
                        })
                        .collect(),
                );
            }
        }
    }
    Ok(result)
}

fn update_command(store: &RevisionStore, options: &BTreeMap<&str, &str>) -> Run<Json> {
    let mut source = IdSource::system();
    let mut fresh = || {
        source.next_id().map_err(|_| PlateError {
            code: ErrorCode::BackendUnavailable,
            message: "no identity could be generated",
        })
    };
    let given = |key: &str| options.get(key).copied();
    let target_id = ElementId::parse(given("--id").ok_or_else(|| invalid("--id is required"))?)
        .map_err(id_error)?;
    let op_id = match given("--op-id") {
        Some(text) => OpId::parse(text).map_err(id_error)?,
        None => OpId::from_uuid(fresh()?),
    };
    let revision_id = match given("--revision-id") {
        Some(text) => RevisionId::parse(text).map_err(id_error)?,
        None => RevisionId::from_uuid(fresh()?),
    };
    let actor = match given("--actor-id") {
        Some(text) => Uuid7::parse(text).map_err(PlateError::from)?,
        None => Uuid7::parse(&fresh()?.to_string()).map_err(PlateError::from)?,
    };
    let project = match given("--project-id") {
        Some(text) => ProjectId::parse(text).map_err(id_error)?,
        None => ProjectId::from_uuid(fresh()?),
    };
    let workspace = match given("--workspace-id") {
        Some(text) => WorkspaceId::parse(text).map_err(id_error)?,
        None => WorkspaceId::from_uuid(fresh()?),
    };
    let patch = ElementPatch {
        name: given("--name").map(str::to_owned),
        material: match given("--material-digest") {
            None => None,
            Some("none") => Some(None),
            Some(text) => Some(Some(digest_option(text)?)),
        },
        thickness_mm: given("--thickness")
            .map(|text| {
                text.parse::<f64>()
                    .map_err(|_| invalid("a number is malformed"))
            })
            .transpose()?,
    };
    let updated = update_element(
        store,
        &UpdateRequest {
            op_id,
            revision_id,
            actor,
            project,
            workspace,
            command: UpdateElement { target_id, patch },
        },
        &mut NoFault,
    )?;
    let element = match &updated.element {
        UpdatedElement::Plate(record) => plate_json(record)?,
        UpdatedElement::Member(record) => member_json(record)?,
    };
    Ok(Json::object([
        (
            "outcome",
            Json::text(if updated.replayed {
                "element_update_replayed"
            } else {
                "element_updated"
            }),
        ),
        ("op_id", Json::text(op_id.to_string())),
        (
            "effect",
            Json::object([
                ("name", Json::Bool(updated.effect.name)),
                ("material", Json::Bool(updated.effect.material)),
                ("shape", Json::Bool(updated.effect.shape)),
            ]),
        ),
        (
            "changed_layers",
            Json::Array(updated.changed_layers.iter().map(Json::text).collect()),
        ),
        ("element", element),
        ("revision", revision_json(&updated.revision)),
        ("footprint", footprint_json(updated.footprint.as_ref())),
    ]))
}

fn create_member_command(store: &RevisionStore, options: &BTreeMap<&str, &str>) -> Run<Json> {
    let mut source = IdSource::system();
    let mut fresh = || {
        source.next_id().map_err(|_| PlateError {
            code: ErrorCode::BackendUnavailable,
            message: "no identity could be generated",
        })
    };
    let given = |key: &str| options.get(key).copied();
    let new_id = match given("--new-id") {
        Some(text) => ElementId::parse(text).map_err(id_error)?,
        None => ElementId::from_uuid(fresh()?),
    };
    let op_id = match given("--op-id") {
        Some(text) => OpId::parse(text).map_err(id_error)?,
        None => OpId::from_uuid(fresh()?),
    };
    let revision_id = match given("--revision-id") {
        Some(text) => RevisionId::parse(text).map_err(id_error)?,
        None => RevisionId::from_uuid(fresh()?),
    };
    let actor = match given("--actor-id") {
        Some(text) => Uuid7::parse(text).map_err(PlateError::from)?,
        None => Uuid7::parse(&fresh()?.to_string()).map_err(PlateError::from)?,
    };
    let project = match given("--project-id") {
        Some(text) => ProjectId::parse(text).map_err(id_error)?,
        None => ProjectId::from_uuid(fresh()?),
    };
    let workspace = match given("--workspace-id") {
        Some(text) => WorkspaceId::parse(text).map_err(id_error)?,
        None => WorkspaceId::from_uuid(fresh()?),
    };
    let kind = match given("--kind") {
        Some("beam") | None => MemberKind::Beam,
        Some("column") => MemberKind::Column,
        Some(_) => return Err(invalid("--kind is beam or column")),
    };
    let point = |key: &'static str| -> Run<[f64; 3]> {
        translation(
            options
                .get(key)
                .ok_or_else(|| invalid("--start and --end are required"))?,
        )
    };
    let command = CreateStraightMember {
        new_id,
        kind,
        name: given("--name").unwrap_or(kind.as_str()).to_owned(),
        axis_points_mm: [point("--start")?, point("--end")?],
        profile_digest: digest_option(
            given("--profile-digest").ok_or_else(|| invalid("--profile-digest is required"))?,
        )?,
        placement: Placement {
            translation_mm: given("--translation").map_or(Ok([0.0; 3]), translation)?,
            ..Placement::IDENTITY
        },
        roll_rad: given("--roll")
            .map(|text| {
                text.parse::<f64>()
                    .map_err(|_| invalid("a number is malformed"))
            })
            .transpose()?,
        material_digest: given("--material-digest").map(digest_option).transpose()?,
    };
    let created = create_member(
        store,
        &CreateMemberRequest {
            op_id,
            revision_id,
            actor,
            project,
            workspace,
            command,
        },
        &mut NoFault,
    )?;
    Ok(Json::object([
        (
            "outcome",
            Json::text(if created.replayed {
                "member_replayed"
            } else {
                "member_created"
            }),
        ),
        ("op_id", Json::text(op_id.to_string())),
        ("member", member_json(&created.member)?),
        ("revision", revision_json(&created.revision)),
        ("footprint", footprint_json(created.footprint.as_ref())),
    ]))
}

fn member_json(record: &MemberRecord) -> Run<Json> {
    let mesh = &record.body.solid;
    let a = &record.authored;
    Ok(Json::object([
        ("element_id", Json::text(a.id.to_string())),
        ("prim_path", Json::text(a.id.prim_path())),
        ("kind", Json::text(a.kind.as_str())),
        ("name", Json::text(a.name.clone())),
        ("profile_digest", Json::text(a.profile.to_string())),
        (
            "material_digest",
            a.material.map_or(Json::Null, |d| Json::text(d.to_string())),
        ),
        (
            "section_mm",
            Json::object([
                ("width", num(record.body.width_mm)?),
                ("height", num(record.body.height_mm)?),
            ]),
        ),
        (
            "axis_points_mm",
            Json::Array(vec![
                triple(a.axis_points_mm[0])?,
                triple(a.axis_points_mm[1])?,
            ]),
        ),
        ("roll_rad", a.roll_rad.map_or(Ok(Json::Null), num)?),
        (
            "placement",
            Json::object([
                ("translation_mm", triple(a.placement.translation_mm)?),
                (
                    "orientation_xyzw",
                    Json::Array(
                        a.placement
                            .orientation_xyzw
                            .into_iter()
                            .map(num)
                            .collect::<Run<Vec<_>>>()?,
                    ),
                ),
            ]),
        ),
        ("quantities", quantities_json(&record.quantities)?),
        ("mesh_digest", Json::text(mesh.digest().to_string())),
        ("vertices", Json::uint(mesh.vertices().len() as u64)),
        ("triangles", Json::uint(mesh.triangles().len() as u64)),
    ]))
}

fn num(value: f64) -> Run<Json> {
    Json::number(value).map_err(|_| PlateError {
        code: ErrorCode::NonFiniteNumber,
        message: "a result is not finite",
    })
}

fn triple(values: [f64; 3]) -> Run<Json> {
    Ok(Json::Array(
        values.into_iter().map(num).collect::<Run<Vec<_>>>()?,
    ))
}

fn quantities_json(q: &Quantities) -> Run<Json> {
    let optional = |value: Option<f64>| value.map_or(Ok(Json::Null), num);
    Ok(Json::object([
        (
            "volume",
            Json::object([
                ("mm3", num(q.volume_mm3)?),
                ("m3", num(q.volume_m3)?),
                ("status", Json::text(q.volume_status.as_str())),
                ("algorithm", Json::text(q.algorithm)),
            ]),
        ),
        (
            "mass",
            Json::object([
                ("kg", optional(q.mass_kg)?),
                ("density_kg_m3", optional(q.density_kg_m3)?),
                ("status", Json::text(q.mass_status.as_str())),
            ]),
        ),
    ]))
}

fn definition_json(digest: &ContentDigest, record: &DefinitionRecord) -> Run<Json> {
    let mut fields = vec![
        ("digest", Json::text(digest.to_string())),
        ("prim_path", Json::text(definition_path(digest))),
        ("kind", Json::text(record.kind())),
        ("units", Json::text("mm")),
        ("schema_version", Json::text("0.1.0")),
    ];
    match *record {
        DefinitionRecord::Material { density_kg_m3 } => {
            fields.push(("density_kg_m3", density_kg_m3.map_or(Ok(Json::Null), num)?))
        }
        DefinitionRecord::RectangleProfile {
            width_mm,
            height_mm,
        } => {
            fields.push(("width_mm", num(width_mm)?));
            fields.push(("height_mm", num(height_mm)?));
        }
    }
    Ok(Json::Object(
        fields
            .into_iter()
            .map(|(key, value)| (key.to_owned(), value))
            .collect(),
    ))
}

fn defined_json(defined: &Defined) -> Run<Json> {
    Ok(Json::object([
        (
            "outcome",
            Json::text(if defined.already_defined {
                "definition_exists"
            } else {
                "definition_created"
            }),
        ),
        (
            "definition",
            definition_json(&defined.digest, &defined.record)?,
        ),
        ("revision", revision_json(&defined.revision)),
    ]))
}

fn definitions_json(inspection: &Inspection) -> Run<Json> {
    let definitions = inspection
        .definitions
        .iter()
        .map(|(digest, record)| {
            let used_by: Vec<Json> = inspection
                .plates
                .iter()
                .filter(|p| p.authored.material == Some(*digest))
                .map(|p| Json::text(p.authored.id.to_string()))
                .collect();
            let Json::Object(mut object) = definition_json(digest, record)? else {
                return Err(invalid("definition result is not an object"));
            };
            object.insert("used_by".into(), Json::Array(used_by));
            Ok(Json::Object(object))
        })
        .collect::<Run<Vec<_>>>()?;
    Ok(Json::object([
        ("outcome", Json::text("definitions")),
        ("revision", revision_json(&inspection.revision)),
        ("definition_count", Json::uint(definitions.len() as u64)),
        ("definitions", Json::Array(definitions)),
    ]))
}

fn plate_json(record: &PlateRecord) -> Run<Json> {
    let mesh = &record.body.solid;
    Ok(Json::object([
        ("element_id", Json::text(record.authored.id.to_string())),
        ("prim_path", Json::text(record.authored.id.prim_path())),
        ("kind", Json::text("plate")),
        ("name", Json::text(record.authored.name.clone())),
        (
            "material_digest",
            record
                .authored
                .material
                .map_or(Json::Null, |d| Json::text(d.to_string())),
        ),
        (
            "dimensions_mm",
            Json::object([
                ("width", num(record.body.width_mm)?),
                ("length", num(record.body.length_mm)?),
                ("thickness", num(record.body.thickness_mm)?),
            ]),
        ),
        (
            "placement",
            Json::object([
                (
                    "translation_mm",
                    triple(record.authored.placement.translation_mm)?,
                ),
                (
                    "orientation_xyzw",
                    Json::Array(
                        record
                            .authored
                            .placement
                            .orientation_xyzw
                            .into_iter()
                            .map(num)
                            .collect::<Run<Vec<_>>>()?,
                    ),
                ),
            ]),
        ),
        ("quantities", quantities_json(&record.quantities)?),
        ("mesh_digest", Json::text(mesh.digest().to_string())),
        ("vertices", Json::uint(mesh.vertices().len() as u64)),
        ("triangles", Json::uint(mesh.triangles().len() as u64)),
        (
            "hole_count",
            Json::uint(record.authored.holes_mm.len() as u64),
        ),
        (
            "holes_mm",
            Json::Array(
                record
                    .authored
                    .holes_mm
                    .iter()
                    .map(|hole| {
                        Ok(Json::Array(
                            hole.iter()
                                .map(|[x, y]| Ok(Json::Array(vec![num(*x)?, num(*y)?])))
                                .collect::<Run<Vec<_>>>()?,
                        ))
                    })
                    .collect::<Run<Vec<_>>>()?,
            ),
        ),
    ]))
}

fn revision_json(revision: &RevisionSummary) -> Json {
    let details = revision.details.as_ref().map_or(Json::Null, |d| {
        Json::object([
            ("project_id", Json::text(d.project.clone())),
            ("workspace_id", Json::text(d.workspace.clone())),
            ("op_id", Json::text(d.op.clone())),
            ("workspace_generation", Json::text(d.generation.to_string())),
            (
                "index",
                d.index.map_or(Json::Null, |(structure, prims)| {
                    Json::object([
                        ("structure_digest", Json::text(structure.to_string())),
                        ("prim_count", Json::text(prims.to_string())),
                    ])
                }),
            ),
            (
                "layers",
                Json::Array(
                    d.layers
                        .iter()
                        .map(|l| {
                            Json::object([
                                ("role", Json::text(l.role)),
                                ("name", Json::text(l.name.clone())),
                                ("digest", Json::text(l.digest.to_string())),
                                ("size", Json::text(l.size.to_string())),
                            ])
                        })
                        .collect(),
                ),
            ),
        ])
    });
    Json::object([
        ("revision_id", Json::text(revision.revision_id.clone())),
        ("sequence", Json::text(revision.sequence.to_string())),
        (
            "manifest_digest",
            Json::text(revision.manifest_digest.to_string()),
        ),
        ("manifest", details),
    ])
}

fn footprint_json(footprint: Option<&Footprint>) -> Json {
    let entries = |set: &std::collections::BTreeSet<Entry>| {
        Json::Array(
            set.iter()
                .map(|e| {
                    let (kind, id) = match e.target {
                        Target::Element(id) => ("element", id.to_string()),
                        Target::Definition(digest) => ("definition", digest.to_string()),
                    };
                    Json::object([
                        ("target_kind", Json::text(kind)),
                        ("target", Json::text(id)),
                        ("group", Json::text(e.group.as_str())),
                    ])
                })
                .collect(),
        )
    };
    footprint.map_or(Json::Null, |f| {
        Json::object([
            ("reads", entries(f.reads())),
            ("writes", entries(f.writes())),
        ])
    })
}

fn created_json(created: &Created, op_id: OpId) -> Run<Json> {
    Ok(Json::object([
        (
            "outcome",
            Json::text(if created.replayed {
                "plate_replayed"
            } else {
                "plate_created"
            }),
        ),
        ("op_id", Json::text(op_id.to_string())),
        ("plate", plate_json(&created.plate)?),
        ("revision", revision_json(&created.revision)),
        ("footprint", footprint_json(created.footprint.as_ref())),
    ]))
}

fn freshness_json(
    freshness: Freshness,
    stored_value: Option<f64>,
    stored_status: Option<&str>,
) -> Run<Json> {
    Ok(Json::object([
        ("freshness", Json::text(freshness.as_str())),
        ("stored_value", stored_value.map_or(Ok(Json::Null), num)?),
        (
            "stored_status",
            stored_status.map_or(Json::Null, Json::text),
        ),
    ]))
}

/// The stored quantity record of one element against what the engine computes now. A `stale`
/// stored value is shown as a previous value, never as the current one.
fn stored_json(report: &QuantityReport) -> Run<Json> {
    let stored = report.stored.as_ref();
    Ok(Json::object([
        (
            "volume",
            freshness_json(
                report.assessment.volume,
                stored.map(|s| s.volume_mm3),
                stored.map(|s| s.volume_status.as_str()),
            )?,
        ),
        (
            "mass",
            freshness_json(
                report.assessment.mass,
                stored.and_then(|s| s.mass_kg),
                stored.map(|s| s.mass_status.as_str()),
            )?,
        ),
        (
            "expected_shape_signature",
            Json::text(report.expected.shape_signature.to_string()),
        ),
        (
            "expected_mass_signature",
            Json::text(report.expected.mass_signature.to_string()),
        ),
        (
            "stored_shape_signature",
            stored.map_or(Json::Null, |s| Json::text(s.shape_signature.to_string())),
        ),
        (
            "stored_mass_signature",
            stored.map_or(Json::Null, |s| Json::text(s.mass_signature.to_string())),
        ),
    ]))
}

/// The element JSON with its stored-quantity report attached.
fn with_stored(element: Json, id: ElementId, inspection: &Inspection) -> Run<Json> {
    let Json::Object(mut object) = element else {
        return Ok(element);
    };
    if let Some(report) = inspection.quantities.get(&id) {
        object.insert("stored_quantities".into(), stored_json(report)?);
    }
    Ok(Json::Object(object))
}

fn inspection_json(inspection: &Inspection) -> Run<Json> {
    let snapshot = &inspection.snapshot;
    let plates = inspection
        .plates
        .iter()
        .map(|p| with_stored(plate_json(p)?, p.authored.id, inspection))
        .collect::<Run<Vec<_>>>()?;
    let members = inspection
        .members
        .iter()
        .map(|m| with_stored(member_json(m)?, m.authored.id, inspection))
        .collect::<Run<Vec<_>>>()?;
    let meshes = snapshot
        .meshes()
        .iter()
        .map(|m| {
            Json::object([
                ("digest", Json::text(m.digest().to_string())),
                ("bytes", Json::uint(m.bytes().len() as u64)),
                ("vertices", Json::uint(u64::from(m.vertex_count()))),
                ("triangles", Json::uint(u64::from(m.triangle_count()))),
            ])
        })
        .collect();
    let entries = snapshot
        .elements()
        .iter()
        .map(|e| {
            Ok(Json::object([
                ("id", Json::text(e.id.to_string())),
                ("instance", Json::uint(u64::from(e.index))),
                ("mesh_digest", Json::text(e.full.mesh_digest.to_string())),
                ("placement_mm", triple(e.placement_mm)?),
                ("world_min_mm", triple(e.full.world_bounds.min)?),
                ("world_max_mm", triple(e.full.world_bounds.max)?),
            ]))
        })
        .collect::<Run<Vec<_>>>()?;
    Ok(Json::object([
        ("outcome", Json::text("plate_inspection")),
        ("revision", revision_json(&inspection.revision)),
        ("plate_count", Json::uint(inspection.plates.len() as u64)),
        ("member_count", Json::uint(inspection.members.len() as u64)),
        (
            "other_element_count",
            Json::uint(inspection.other_elements as u64),
        ),
        ("plates", Json::Array(plates)),
        ("members", Json::Array(members)),
        (
            "snapshot",
            Json::object([
                ("synthetic", Json::Bool(false)),
                ("token", Json::text(snapshot.token().as_str())),
                (
                    "pin_revision",
                    Json::text(snapshot.pin().revision().to_string()),
                ),
                (
                    "pin_generation",
                    Json::text(snapshot.pin().generation().to_string()),
                ),
                ("elements", Json::Array(entries)),
                ("meshes", Json::Array(meshes)),
            ]),
        ),
    ]))
}
