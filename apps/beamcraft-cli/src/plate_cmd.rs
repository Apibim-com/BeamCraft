// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! P01 headless commands: `create-plate` and `plate-inspect`. They call the engine API through
//! the adapter of `beamcraft_cli::plate` and print one JSON object (same error shape as the other
//! commands; exit 0 success, 2 rejection, 3 backend unavailable, 4 outcome unknown).

use beamcraft_cli::definition::{
    DefineRequest, Defined, DefinitionRecord, define, definition_path,
};
use beamcraft_cli::plate::{
    CreateRequest, Created, Inspection, PlateError, RevisionSummary, create_plate, inspect,
};
use beamcraft_engine::command::{Quantities, origin_rectangle_contour};
use beamcraft_engine::footprint::{Entry, Footprint, Target};
use beamcraft_engine::query::committed::PlateRecord;
use beamcraft_host::{
    ids::IdSource,
    revision::{NoFault, RevisionStore, Uuid7},
    storage::BlobStore,
};
use beamcraft_model::command::{CreatePlate, Placement};
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
        "define-material" => DEFINE_MATERIAL_OPTIONS,
        "define-profile" => DEFINE_PROFILE_OPTIONS,
        "plate-inspect" | "definitions" => &["--store"],
        _ => return Err(invalid("unknown command")),
    };
    if !tail.len().is_multiple_of(2) || tail.len() > 32 {
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
    let mesh = record.body.result.mesh();
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
        ("mesh_digest", Json::text(mesh.mesh_digest().to_string())),
        ("vertices", Json::uint(mesh.vertices().len() as u64)),
        ("triangles", Json::uint(mesh.triangles().len() as u64)),
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

fn inspection_json(inspection: &Inspection) -> Run<Json> {
    let snapshot = &inspection.snapshot;
    let plates = inspection
        .plates
        .iter()
        .map(plate_json)
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
        (
            "other_element_count",
            Json::uint(inspection.other_elements as u64),
        ),
        ("plates", Json::Array(plates)),
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
