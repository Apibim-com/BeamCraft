// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! Bounded local model-view collection, separate from immutable model revisions.

use crate::plate_cmd::invalid;
use beamcraft_cli::plate::PlateError;
use beamcraft_engine::{
    RenderMode,
    model_view::{self, ModelView, Projection},
    work_plane::WorkPlane,
};
use beamcraft_host::{ids::IdSource, revision::RevisionStore, storage::BlobStore};
use beamcraft_json::Json;
use beamcraft_layer_engine::{Value, authored::view_layer};
use beamcraft_model::{ErrorCode, id::Uuid7};
use std::{
    collections::BTreeMap,
    ffi::OsString,
    fs,
    io::{Read, Write},
    path::Path,
};

const FILE: &str = "views.usda";

pub fn handles(command: &OsString) -> bool {
    [
        "model-views",
        "create-model-view",
        "update-model-view",
        "delete-model-view",
        "create-view-folder",
        "delete-view-folder",
    ]
    .iter()
    .any(|c| command == c)
}
pub fn execute(args: &[OsString]) -> (Json, u8) {
    crate::report::report(run(args))
}
fn failure(code: ErrorCode, message: &'static str) -> PlateError {
    PlateError {
        code,
        message,
        rule_id: None,
    }
}
fn io_error(_: std::io::Error) -> PlateError {
    failure(
        ErrorCode::BackendUnavailable,
        "local view collection could not be read or saved",
    )
}
fn number(n: f64) -> Json {
    Json::Number(n.to_string())
}
fn encode(view: &ModelView) -> Json {
    Json::object([
        ("id", Json::text(view.id.to_string())),
        ("name", Json::text(&view.name)),
        ("plane", Json::text(view.plane.expression())),
        ("projection", Json::text(view.projection.as_str())),
        ("span_mm", number(view.span_mm)),
        (
            "depth_mm",
            view.depth.map_or(Json::Null, |v| {
                Json::Array(v.into_iter().map(number).collect())
            }),
        ),
        ("render_mode", Json::text(view.render_mode.as_token())),
        ("grids_visible", Json::Bool(view.grids_visible)),
        (
            "rotation_rad",
            Json::Array(view.rotation_rad.into_iter().map(number).collect()),
        ),
        (
            "color",
            view.color.map_or(Json::Null, |c| {
                Json::Array(c.into_iter().map(number).collect())
            }),
        ),
        ("opacity", number(view.opacity)),
        ("folder", Json::text(&view.folder)),
        ("favorite", Json::Bool(view.favorite)),
        ("temporary", Json::Bool(view.temporary)),
        (
            "hidden_kinds",
            Json::text(
                view.hidden_kinds
                    .hidden_kinds()
                    .iter()
                    .map(|k| k.as_token())
                    .collect::<Vec<_>>()
                    .join(","),
            ),
        ),
        (
            "work_area",
            view.work_area.map_or(Json::Null, |b| {
                Json::Array(b.min.into_iter().chain(b.max).map(number).collect())
            }),
        ),
    ])
}
fn decode(json: &Json) -> Result<ModelView, PlateError> {
    let shape = || invalid("view collection contains an invalid view");
    json.closed(&[
        "id",
        "name",
        "plane",
        "projection",
        "span_mm",
        "depth_mm",
        "render_mode",
        "grids_visible",
        "folder",
        "favorite",
        "temporary",
        "hidden_kinds",
        "work_area",
        "rotation_rad",
        "color",
        "opacity",
    ])
    .map_err(|_| shape())?;
    let text = |key| json.get(key).and_then(Json::string).map_err(|_| shape());
    let depth = match json.get("depth_mm").map_err(|_| shape())? {
        Json::Null => None,
        value => {
            let [a, b] = value.array().map_err(|_| shape())? else {
                return Err(shape());
            };
            Some([
                a.float().map_err(|_| shape())?,
                b.float().map_err(|_| shape())?,
            ])
        }
    };
    let area = match json.get("work_area").map_err(|_| shape())? {
        Json::Null => None,
        j => {
            let values = j
                .array()
                .map_err(|_| shape())?
                .iter()
                .map(Json::float)
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| shape())?;
            let [a, b, c, d, e, f] = values.as_slice() else {
                return Err(shape());
            };
            Some(beamcraft_engine::Aabb {
                min: [*a, *b, *c],
                max: [*d, *e, *f],
            })
        }
    };
    let [z, x] = json
        .get("rotation_rad")
        .and_then(Json::array)
        .map_err(|_| shape())?
    else {
        return Err(shape());
    };
    let color = match json.get("color").map_err(|_| shape())? {
        Json::Null => None,
        v => {
            let [r, g, b] = v.array().map_err(|_| shape())? else {
                return Err(shape());
            };
            Some([
                r.float().map_err(|_| shape())?,
                g.float().map_err(|_| shape())?,
                b.float().map_err(|_| shape())?,
            ])
        }
    };
    let view = ModelView {
        id: Uuid7::parse(text("id")?).map_err(|_| shape())?,
        name: text("name")?.into(),
        plane: WorkPlane::parse(text("plane")?)?,
        projection: Projection::parse(text("projection")?)?,
        span_mm: json
            .get("span_mm")
            .and_then(Json::float)
            .map_err(|_| shape())?,
        depth,
        render_mode: RenderMode::parse(text("render_mode")?).ok_or_else(shape)?,
        grids_visible: json
            .get("grids_visible")
            .and_then(Json::boolean)
            .map_err(|_| shape())?,
        folder: text("folder")?.into(),
        favorite: json
            .get("favorite")
            .and_then(Json::boolean)
            .map_err(|_| shape())?,
        temporary: json
            .get("temporary")
            .and_then(Json::boolean)
            .map_err(|_| shape())?,
        hidden_kinds: model_view::hidden_kinds(text("hidden_kinds")?).map_err(|_| shape())?,
        work_area: area,
        rotation_rad: [
            z.float().map_err(|_| shape())?,
            x.float().map_err(|_| shape())?,
        ],
        color,
        opacity: json
            .get("opacity")
            .and_then(Json::float)
            .map_err(|_| shape())?,
    };
    view.validate()?;
    Ok(view)
}
fn regular_file(path: &Path) -> Result<(), PlateError> {
    match fs::symlink_metadata(path) {
        Ok(m) if m.is_file() && !m.file_type().is_symlink() => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        _ => Err(invalid("view collection path must be a regular file")),
    }
}
fn read_collection(root: &Path, project: &str) -> Result<view_layer::Collection, PlateError> {
    let path = root.join(FILE);
    regular_file(&path)?;
    let file = match fs::File::open(path) {
        Ok(file) => file,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(view_layer::Collection {
                folders: Vec::new(),
                records: Vec::new(),
            });
        }
        Err(e) => return Err(io_error(e)),
    };
    let mut bytes = Vec::new();
    file.take(view_layer::MAX_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(io_error)?;
    view_layer::read(&bytes, project).map_err(|e| failure(e.code, e.message))
}
fn from_usd(value: Value) -> Result<Json, PlateError> {
    Ok(match value {
        Value::Block => Json::Null,
        Value::Bool(v) => Json::Bool(v),
        Value::Number(v) if v.is_finite() => number(v),
        Value::Text(v) => Json::text(v),
        Value::Array(v) => Json::Array(v.into_iter().map(from_usd).collect::<Result<_, _>>()?),
        _ => return Err(invalid("unsupported view property")),
    })
}
fn to_usd(value: Json) -> Result<Value, PlateError> {
    Ok(match value {
        Json::Null => Value::Block,
        Json::Bool(v) => Value::Bool(v),
        Json::Number(v) => Value::Number(v.parse().map_err(|_| invalid("invalid view number"))?),
        Json::String(v) => Value::Text(v),
        Json::Array(v) => Value::Array(v.into_iter().map(to_usd).collect::<Result<_, _>>()?),
        _ => return Err(invalid("unsupported view property")),
    })
}
fn load(root: &Path, project: &str) -> Result<Vec<ModelView>, PlateError> {
    let collection = read_collection(root, project)?;
    let views = collection
        .records
        .into_iter()
        .map(|fields| {
            let fields = fields
                .into_iter()
                .map(|(k, v)| Ok((k, from_usd(v)?)))
                .collect::<Result<_, PlateError>>()?;
            decode(&Json::Object(fields))
        })
        .collect::<Result<Vec<_>, PlateError>>()?;
    model_view::validate_collection(&views)?;
    Ok(views)
}
fn folders(root: &Path, project: &str) -> Result<Vec<String>, PlateError> {
    let list = read_collection(root, project)?.folders;
    let mut output = std::collections::BTreeSet::new();
    for folder in list {
        model_view::validate_folder(&folder)?;
        output.insert(folder);
    }
    Ok(output.into_iter().collect())
}
fn save_with_folders(
    root: &Path,
    project: &str,
    views: &[ModelView],
    list: Vec<String>,
) -> Result<(), PlateError> {
    model_view::validate_collection(views)?;
    let mut list: std::collections::BTreeSet<String> = list.into_iter().collect();
    let paths: Vec<_> = list
        .iter()
        .cloned()
        .chain(views.iter().map(|v| v.folder.clone()))
        .collect();
    for folder in paths {
        let mut path = String::new();
        for part in folder.split('/').filter(|p| !p.is_empty()) {
            if !path.is_empty() {
                path.push('/');
            }
            path.push_str(part);
            list.insert(path.clone());
        }
    }
    if list.len() > 512 {
        return Err(failure(
            ErrorCode::LimitExceeded,
            "view folder count exceeds 512",
        ));
    }
    for folder in &list {
        model_view::validate_folder(folder)?;
    }
    let records = views
        .iter()
        .filter(|v| !v.temporary)
        .map(|view| {
            let Json::Object(fields) = encode(view) else {
                return Err(invalid("invalid view record"));
            };
            let properties = fields
                .into_iter()
                .map(|(k, v)| Ok((k, to_usd(v)?)))
                .collect::<Result<_, PlateError>>()?;
            let camera = view.camera_plane()?;
            let [rx, ry, rz] = camera.x();
            let [ux, uy, uz] = camera.y();
            let [nx, ny, nz] = camera.normal();
            let [x, y, z] = camera.origin();
            let distance =
                (view.span_mm / (2. * (std::f64::consts::PI / 8.).tan())).clamp(10., 2e7);
            let distance = if view.projection == Projection::Orthographic {
                distance.max(view.depth.map_or(0., |[above, _]| above + 10.))
            } else {
                distance
            };
            Ok(view_layer::CameraRecord {
                id: view.id,
                properties,
                transform: [
                    rx,
                    ry,
                    rz,
                    0.,
                    ux,
                    uy,
                    uz,
                    0.,
                    nx,
                    ny,
                    nz,
                    0.,
                    x + nx * distance,
                    y + ny * distance,
                    z + nz * distance,
                    1.,
                ],
                projection: view.projection.as_str().into(),
                span_mm: view.span_mm,
            })
        })
        .collect::<Result<Vec<_>, PlateError>>()?;
    let bytes = view_layer::write(project, &list.into_iter().collect::<Vec<_>>(), &records)
        .map_err(|e| failure(e.code, e.message))?;
    // Verify the serialized candidate through the normal reader before replacing a good layer.
    view_layer::read(&bytes, project).map_err(|e| failure(e.code, e.message))?;
    let temp = root.join(format!(".model-views-{}.tmp", std::process::id()));
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)
        .map_err(io_error)?;
    let result = (|| {
        file.write_all(&bytes).map_err(io_error)?;
        file.sync_all().map_err(io_error)?;
        drop(file);
        regular_file(&root.join(FILE))?;
        fs::rename(&temp, root.join(FILE)).map_err(io_error)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}
fn save(root: &Path, project: &str, views: &[ModelView]) -> Result<(), PlateError> {
    save_with_folders(root, project, views, folders(root, project)?)
}
fn scalars(text: &str) -> Result<Vec<f64>, PlateError> {
    if text.len() > 384 {
        return Err(invalid("view coordinate list is too long"));
    }
    text.split(',')
        .take(10)
        .map(|v| {
            v.trim()
                .parse::<f64>()
                .map_err(|_| invalid("view coordinates must be numbers"))
        })
        .collect()
}
fn run(args: &[OsString]) -> Result<Json, PlateError> {
    let args = args
        .iter()
        .map(|s| {
            s.to_str()
                .filter(|v| v.len() <= 4096)
                .ok_or_else(|| invalid("view option is not bounded UTF-8"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let (command, tail) = args
        .split_first()
        .ok_or_else(|| invalid("missing view command"))?;
    if tail.len() > 48 || !tail.len().is_multiple_of(2) {
        return Err(invalid("view options are bounded KEY VALUE pairs"));
    }
    let mut options = BTreeMap::new();
    for [key, value] in tail.as_chunks::<2>().0 {
        if ![
            "--store",
            "--id",
            "--name",
            "--new-id",
            "--plane",
            "--parallel",
            "--offset",
            "--two-points",
            "--three-points",
            "--grid",
            "--axis",
            "--axes",
            "--u-axes",
            "--v-axes",
            "--u-prefix",
            "--v-prefix",
            "--u-properties",
            "--v-properties",
            "--projection",
            "--span",
            "--depth",
            "--render-mode",
            "--grids",
            "--folder",
            "--favorite",
            "--temporary",
            "--reserved-views",
            "--hidden-kinds",
            "--work-area",
            "--element",
            "--side",
            "--level",
            "--levels",
            "--ray-origin",
            "--ray-direction",
            "--edge",
            "--rotation",
            "--color",
            "--opacity",
            "--all",
            "--work-plane",
            "--group",
            "--undeformed",
        ]
        .contains(key)
            || options.insert(*key, *value).is_some()
        {
            return Err(invalid("unknown or repeated view option"));
        }
    }
    let required = |key| {
        options
            .get(key)
            .copied()
            .ok_or_else(|| invalid("required view option is missing"))
    };
    let root = Path::new(required("--store")?);
    let store = RevisionStore::new(
        BlobStore::new(root).map_err(|e| failure(e.code(), "view store is not usable"))?,
    );
    let revision = store.open()?;
    let project = revision.manifest().content().project().as_str();
    let reserved = options
        .get("--reserved-views")
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|_| invalid("reserved-views must be an integer"))
        })
        .transpose()?
        .unwrap_or(0);
    if reserved > model_view::MAX_VIEWS
        || (options.contains_key("--reserved-views") && *command != "create-model-view")
    {
        return Err(invalid(
            "reserved-views applies only to creation and cannot exceed 256",
        ));
    }
    let write_views = |views: &[ModelView]| {
        if views.len().saturating_add(reserved) > model_view::MAX_VIEWS {
            return Err(failure(
                ErrorCode::LimitExceeded,
                "view collection including session views exceeds 256",
            ));
        }
        save(root, project, views)
    };

    if *command == "model-views" {
        if options.len() != 1 {
            return Err(invalid("model-views accepts only store"));
        }
        return Ok(reply(load(root, project)?, folders(root, project)?));
    }
    let lock_path = root.join("model-views.lock");
    regular_file(&lock_path)?;
    let lock = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(lock_path)
        .map_err(io_error)?;
    lock.try_lock().map_err(|_| {
        failure(
            ErrorCode::BackendUnavailable,
            "view collection is busy; retry",
        )
    })?;
    let mut views = load(root, project)?;
    if *command == "create-view-folder" || *command == "delete-view-folder" {
        if options.keys().any(|k| !["--store", "--folder"].contains(k)) {
            return Err(invalid("folder command accepts only store and folder"));
        }
        let folder = required("--folder")?;
        model_view::validate_folder(folder)?;
        if folder.is_empty() {
            return Err(invalid("root folder cannot be created or deleted"));
        }
        let mut list = folders(root, project)?;
        if *command == "create-view-folder" {
            if list.iter().any(|f| f == folder) {
                return Err(failure(ErrorCode::DuplicateId, "folder already exists"));
            }
            list.push(folder.into());
        } else {
            if !list.iter().any(|f| f == folder) {
                return Err(failure(ErrorCode::NotFound, "view folder was not found"));
            }
            if views
                .iter()
                .any(|v| v.folder == folder || v.folder.starts_with(&format!("{folder}/")))
            {
                return Err(failure(
                    ErrorCode::DependencyConflict,
                    "move the folder's views before deleting it",
                ));
            }
            list.retain(|f| f != folder && !f.starts_with(&format!("{folder}/")));
        }
        save_with_folders(root, project, &views, list)?;
        return Ok(reply(views, folders(root, project)?));
    }
    if options.contains_key("--all") {
        if *command != "update-model-view"
            || required("--all")? != "true"
            || options
                .keys()
                .any(|k| !["--store", "--all", "--color", "--opacity"].contains(k))
        {
            return Err(invalid("all only updates display color and opacity"));
        }
        for view in &mut views {
            if let Some(value) = options.get("--opacity") {
                view.opacity = value
                    .parse()
                    .map_err(|_| invalid("opacity must be a number"))?;
            }
            if let Some(value) = options.get("--color") {
                view.color = if *value == "default" {
                    None
                } else {
                    let values = scalars(value)?;
                    let [r, g, b] = values.as_slice() else {
                        return Err(invalid("color needs R,G,B"));
                    };
                    Some([*r, *g, *b])
                };
            }
        }
        write_views(&views)?;
        return Ok(reply(views, folders(root, project)?));
    }
    if *command == "delete-model-view" {
        if options.keys().any(|k| !["--store", "--id"].contains(k)) {
            return Err(invalid("delete accepts only store and id"));
        }
        let id = required("--id")?;
        let index = views
            .iter()
            .position(|v| v.id.to_string() == id)
            .ok_or_else(|| failure(ErrorCode::NotFound, "view was not found"))?;
        views.remove(index);
    } else {
        let existing = if *command == "update-model-view" {
            let id = required("--id")?;
            Some(
                views
                    .iter()
                    .position(|v| v.id.to_string() == id)
                    .ok_or_else(|| failure(ErrorCode::NotFound, "view was not found"))?,
            )
        } else {
            None
        };
        let old = existing.and_then(|i| views.get(i)).cloned();
        let modes = [
            "--plane",
            "--parallel",
            "--two-points",
            "--three-points",
            "--grid",
            "--element",
            "--group",
            "--level",
            "--levels",
            "--ray-origin",
        ]
        .iter()
        .filter(|k| options.contains_key(**k))
        .count();
        if options.contains_key("--work-plane") && !options.contains_key("--two-points") {
            return Err(invalid("work-plane applies only to two-point sections"));
        }
        if (options.contains_key("--side") || options.contains_key("--undeformed"))
            && !options.contains_key("--element")
            && !options.contains_key("--group")
        {
            return Err(invalid(
                "side and undeformed need a part or assembly target",
            ));
        }
        if options.contains_key("--undeformed") && options.contains_key("--group") {
            return Err(invalid("undeformed needs a member, not an assembly"));
        }
        if (options.contains_key("--ray-direction") || options.contains_key("--edge"))
            && !options.contains_key("--ray-origin")
        {
            return Err(invalid("ray direction and edge need a surface ray"));
        }
        if modes > 1 {
            return Err(invalid("choose one view construction mode"));
        }
        if options.contains_key("--offset") && !options.contains_key("--parallel") {
            return Err(invalid("offset needs parallel mode"));
        }
        if options.keys().any(|k| {
            [
                "--axis",
                "--axes",
                "--u-axes",
                "--v-axes",
                "--u-prefix",
                "--v-prefix",
                "--u-properties",
                "--v-properties",
            ]
            .contains(k)
        }) && !options.contains_key("--grid")
        {
            return Err(invalid("axis needs grid mode"));
        }
        if existing.is_some()
            && options.keys().any(|k| {
                [
                    "--new-id",
                    "--levels",
                    "--axes",
                    "--u-axes",
                    "--v-axes",
                    "--u-prefix",
                    "--v-prefix",
                    "--u-properties",
                    "--v-properties",
                ]
                .contains(k)
            })
        {
            return Err(invalid(
                "update cannot change identity or create an axis batch",
            ));
        }
        if existing.is_none() && options.contains_key("--id") {
            return Err(invalid("create uses new-id, not id"));
        }
        let parse = |key| {
            required(key)?
                .parse::<f64>()
                .map_err(|_| invalid("view size must be a number"))
        };
        let plane = if let Some(text) = options.get("--plane") {
            WorkPlane::parse(text)?
        } else if let Some(name) = options.get("--parallel") {
            WorkPlane::parallel(
                name,
                if options.contains_key("--offset") {
                    parse("--offset")?
                } else {
                    0.
                },
            )?
        } else if let Some(text) = options.get("--two-points") {
            let points = scalars(text)?;
            let [a, b, c, d, e, f] = points.as_slice() else {
                return Err(invalid("two points need six coordinates"));
            };
            match options.get("--work-plane") {
                Some(input) => model_view::two_points_in_plane(
                    [*a, *b, *c],
                    [*d, *e, *f],
                    WorkPlane::parse(input)?,
                )?,
                None => model_view::two_points([*a, *b, *c], [*d, *e, *f])?,
            }
        } else if let Some(text) = options.get("--three-points") {
            let points = scalars(text)?;
            let [a, b, c, d, e, f, g, h, i] = points.as_slice() else {
                return Err(invalid("three points need nine coordinates"));
            };
            model_view::three_points([*a, *b, *c], [*d, *e, *f], [*g, *h, *i])?
        } else {
            old.as_ref().map_or(WorkPlane::default(), |v| v.plane)
        };
        let depth = match options.get("--depth") {
            Some(&"none") => None,
            Some(text) => {
                let values = scalars(text)?;
                let [a, b] = values.as_slice() else {
                    return Err(invalid("depth needs above,below or none"));
                };
                Some([*a, *b])
            }
            None => old.as_ref().and_then(|v| v.depth),
        };
        let boolean = |key, default| match options.get(key) {
            Some(&"true") => Ok(true),
            Some(&"false") => Ok(false),
            Some(_) => Err(invalid("view flags must be true or false")),
            None => Ok(default),
        };
        let area = match options.get("--work-area") {
            Some(&"none") => None,
            Some(value) => {
                let values = scalars(value)?;
                let [a, b, c, d, e, f] = values.as_slice() else {
                    return Err(invalid("work area needs min XYZ and max XYZ"));
                };
                Some(beamcraft_engine::Aabb {
                    min: [*a, *b, *c],
                    max: [*d, *e, *f],
                })
            }
            None => old.as_ref().and_then(|v| v.work_area),
        };
        let rotation_rad = match options.get("--rotation") {
            Some(value) => {
                let values = scalars(value)?;
                let [z, x] = values.as_slice() else {
                    return Err(invalid("rotation needs Z,X degrees"));
                };
                [z.to_radians(), x.to_radians()]
            }
            None => old.as_ref().map_or([0.; 2], |v| v.rotation_rad),
        };
        let color = match options.get("--color") {
            Some(&"default") => None,
            Some(value) => {
                let values = scalars(value)?;
                let [r, g, b] = values.as_slice() else {
                    return Err(invalid("color needs R,G,B in 0..1 or default"));
                };
                Some([*r, *g, *b])
            }
            None => old.as_ref().and_then(|v| v.color),
        };
        let mut next = ModelView {
            id: match &old {
                Some(v) => v.id,
                None => match options.get("--new-id") {
                    Some(id) => Uuid7::parse(id).map_err(|_| invalid("invalid view identity"))?,
                    None => IdSource::system()
                        .next_id()
                        .map_err(|_| invalid("view identity is unavailable"))?,
                },
            },
            name: options
                .get("--name")
                .map(|s| s.to_string())
                .or_else(|| old.as_ref().map(|v| v.name.clone()))
                .ok_or_else(|| invalid("name is required"))?,
            plane,
            projection: options
                .get("--projection")
                .map(|p| Projection::parse(p))
                .transpose()?
                .unwrap_or(
                    old.as_ref()
                        .map_or(Projection::Orthographic, |v| v.projection),
                ),
            span_mm: if options.contains_key("--span") {
                parse("--span")?
            } else {
                old.as_ref().map_or(10000., |v| v.span_mm)
            },
            depth,
            render_mode: match options.get("--render-mode") {
                Some(t) => RenderMode::parse(t).ok_or_else(|| invalid("unknown render mode"))?,
                None => old.as_ref().map_or(RenderMode::Shaded, |v| v.render_mode),
            },
            folder: options
                .get("--folder")
                .map(|s| s.to_string())
                .unwrap_or_else(|| old.as_ref().map_or(String::new(), |v| v.folder.clone())),
            favorite: boolean("--favorite", old.as_ref().is_some_and(|v| v.favorite))?,
            temporary: boolean("--temporary", old.as_ref().is_some_and(|v| v.temporary))?,
            hidden_kinds: match options.get("--hidden-kinds") {
                Some(text) => {
                    model_view::hidden_kinds(text).map_err(|_| invalid("unknown hidden kind"))?
                }
                None => old
                    .as_ref()
                    .map_or(beamcraft_engine::KindFilter::SHOW_ALL, |v| v.hidden_kinds),
            },
            work_area: area,
            rotation_rad,
            color,
            opacity: if options.contains_key("--opacity") {
                parse("--opacity")?
            } else {
                old.as_ref().map_or(1., |v| v.opacity)
            },
            grids_visible: match options.get("--grids") {
                Some(&"true") => true,
                Some(&"false") => false,
                Some(_) => return Err(invalid("grids must be true or false")),
                None => old.as_ref().is_none_or(|v| v.grids_visible),
            },
        };
        if let Some(mode) = options.get("--levels") {
            if existing.is_some() || options.contains_key("--new-id") {
                return Err(invalid("level batch needs generated identities"));
            }
            let inspection = beamcraft_cli::plate::inspect_revision(&revision)?;
            let mut levels: Vec<_> = inspection.levels.levels().map(|(_, level)| level).collect();
            levels.sort_by(|a, b| {
                a.elevation_mm
                    .abs()
                    .total_cmp(&b.elevation_mm.abs())
                    .then_with(|| a.name.cmp(&b.name))
            });
            match *mode {
                "all" => {}
                "first" => levels.truncate(1),
                "last" => {
                    levels.reverse();
                    levels.truncate(1);
                }
                _ => return Err(invalid("levels must be all, first or last")),
            }
            if views.len().saturating_add(levels.len()) > model_view::MAX_VIEWS {
                return Err(failure(
                    ErrorCode::LimitExceeded,
                    "level batch exceeds the view limit",
                ));
            }
            let prefix = next.name.clone();
            let mut ids = IdSource::system();
            for level in levels {
                next.id = ids
                    .next_id()
                    .map_err(|_| invalid("view identity unavailable"))?;
                next.plane = WorkPlane::parallel("xy", level.elevation_mm)?;
                let base = format!("{prefix} {}", level.name);
                next.name = base.clone();
                let mut serial = 1usize;
                while views
                    .iter()
                    .any(|v| v.name.to_lowercase() == next.name.to_lowercase())
                {
                    serial = serial.saturating_add(1);
                    next.name = format!("{base} ({serial})");
                }
                views.push(next.clone());
            }
            write_views(&views)?;
            return Ok(reply(views, folders(root, project)?));
        }
        if let Some(selector) = options.get("--level") {
            let inspection = beamcraft_cli::plate::inspect_revision(&revision)?;
            let mut levels = inspection
                .levels
                .levels()
                .filter(|(id, l)| id.to_string() == *selector || l.name == *selector);
            let (_, level) = levels
                .next()
                .ok_or_else(|| invalid("level was not found"))?;
            if levels.next().is_some() {
                return Err(invalid("level name is ambiguous; use its identity"));
            }
            next.plane = WorkPlane::parallel("xy", level.elevation_mm)?;
        }
        if let Some(element) = options.get("--element").or_else(|| options.get("--group")) {
            let inspection = beamcraft_cli::plate::inspect_revision(&revision)?;
            let id = beamcraft_model::id::ElementId::parse(element)
                .map_err(|_| invalid("invalid element identity"))?;
            let mut parts = vec![id];
            if options.contains_key("--group") {
                let target = inspection
                    .elements
                    .get(&id)
                    .ok_or_else(|| invalid("assembly target was not found"))?;
                let root = if target.kind == "assembly" {
                    id
                } else {
                    target
                        .part_of
                        .ok_or_else(|| invalid("selected part has no assembly"))?
                };
                let mut children = BTreeMap::<_, Vec<_>>::new();
                for (child, record) in &inspection.elements {
                    if let Some(parent) = record.part_of {
                        children
                            .entry(parent)
                            .or_default()
                            .push((*child, record.kind == "assembly"));
                    }
                }
                let mut pending = std::collections::VecDeque::from([root]);
                let mut seen = std::collections::BTreeSet::new();
                parts.clear();
                while let Some(parent) = pending.pop_front() {
                    if !seen.insert(parent) {
                        return Err(invalid("assembly view contains a cycle"));
                    }
                    if seen.len() > 65_536 {
                        return Err(failure(
                            ErrorCode::LimitExceeded,
                            "assembly view exceeds its budget",
                        ));
                    }
                    if let Some(children) = children.get(&parent) {
                        for (child, assembly) in children {
                            if *assembly {
                                pending.push_back(*child);
                            } else if inspection.snapshot.element(child).is_some() {
                                parts.push(*child);
                            }
                        }
                    }
                }
                parts.sort();
            }
            if options.contains_key("--undeformed") {
                if required("--undeformed")? != "true" {
                    return Err(invalid("undeformed must be true"));
                }
                let member = inspection
                    .members
                    .iter()
                    .find(|m| m.authored.id == id)
                    .ok_or_else(|| {
                        failure(
                            ErrorCode::UnsupportedProperty,
                            "undeformed inspection applies to members",
                        )
                    })?;
                if member.authored.axis_points_mm.len() != 2 {
                    return Err(failure(
                        ErrorCode::UnsupportedProperty,
                        "this member has no supported undeformed representation",
                    ));
                }
            }
            let side = options.get("--side").copied().unwrap_or("3d");
            if side == "four" {
                if existing.is_some() || options.contains_key("--new-id") {
                    return Err(invalid("four views need new generated identities"));
                }
                let prefix = next.name.clone();
                let mut ids = IdSource::system();
                for side in ["front", "top", "end", "3d"] {
                    let (plane, span) =
                        model_view::group_plane(&inspection.snapshot, &parts, side)?;
                    next.id = ids
                        .next_id()
                        .map_err(|_| invalid("view identity unavailable"))?;
                    next.name = format!("{prefix} {side}");
                    next.plane = plane;
                    next.span_mm = span;
                    next.projection = if side == "3d" {
                        Projection::Perspective
                    } else {
                        Projection::Orthographic
                    };
                    views.push(next.clone());
                }
                write_views(&views)?;
                return Ok(reply(views, folders(root, project)?));
            }
            let (plane, span) = model_view::group_plane(&inspection.snapshot, &parts, side)?;
            next.plane = plane;
            next.span_mm = span;
        }
        if let Some(origin) = options.get("--ray-origin") {
            let inspection = beamcraft_cli::plate::inspect_revision(&revision)?;
            let triple = |key| -> Result<[f64; 3], PlateError> {
                scalars(key)?
                    .try_into()
                    .map_err(|_| invalid("ray needs XYZ triples"))
            };
            let edge = options
                .get("--edge")
                .map(|s| {
                    let v = scalars(s)?;
                    let [a, b, c, d, e, f] = v.as_slice() else {
                        return Err(invalid("edge needs two points"));
                    };
                    Ok([[*a, *b, *c], [*d, *e, *f]])
                })
                .transpose()?;
            next.plane = model_view::surface_plane(
                &inspection.snapshot,
                triple(origin)?,
                triple(required("--ray-direction")?)?,
                edge,
            )?;
        }
        if let Some(selector) = options.get("--grid") {
            let grids = beamcraft_cli::grid::grids_at(&store, revision.info().sequence())?;
            let grid = beamcraft_engine::grid_query::resolve_grid(&grids, selector)?;
            if options.keys().any(|k| {
                [
                    "--axes",
                    "--u-axes",
                    "--v-axes",
                    "--u-prefix",
                    "--v-prefix",
                    "--u-properties",
                    "--v-properties",
                ]
                .contains(k)
            }) {
                if options.contains_key("--axis") || options.contains_key("--new-id") {
                    return Err(invalid("axis batch cannot specify axis or new-id"));
                }
                if options.contains_key("--axes")
                    && (options.contains_key("--u-axes") || options.contains_key("--v-axes"))
                {
                    return Err(invalid(
                        "choose a batch shortcut or separate U/V family modes",
                    ));
                }
                let (u, v) = match options.get("--axes").copied().unwrap_or("all") {
                    "all" => ("all", "all"),
                    "u" => ("all", "none"),
                    "v" => ("none", "all"),
                    "first" => ("first", "first"),
                    "last" => ("last", "last"),
                    _ => return Err(invalid("axes must be all, u, v, first or last")),
                };
                let lines = model_view::grid_views(
                    grid,
                    options.get("--u-axes").copied().unwrap_or(u),
                    options.get("--v-axes").copied().unwrap_or(v),
                )?;
                if views.len().saturating_add(lines.len()) > model_view::MAX_VIEWS {
                    return Err(failure(
                        ErrorCode::LimitExceeded,
                        "axis batch exceeds the view limit",
                    ));
                }
                let sources = ["--u-properties", "--v-properties"]
                    .map(|key| {
                        options
                            .get(key)
                            .map(|selector| {
                                views
                                    .iter()
                                    .find(|v| v.id.to_string() == *selector || v.name == *selector)
                                    .cloned()
                                    .ok_or_else(|| invalid("view property source was not found"))
                            })
                            .transpose()
                    })
                    .into_iter()
                    .collect::<Result<Vec<_>, PlateError>>()?;
                let prefix = next.name.clone();
                let mut ids = IdSource::system();
                for line in lines {
                    let mut view = next.clone();
                    if let Some(Some(source)) = sources.get(usize::from(line.family)) {
                        view.apply_display_properties(source);
                    }
                    view.id = ids
                        .next_id()
                        .map_err(|_| invalid("view identity unavailable"))?;
                    view.plane = line.plane;
                    let key = if line.family == 0 {
                        "--u-prefix"
                    } else {
                        "--v-prefix"
                    };
                    let prefix = options.get(key).copied().unwrap_or(&prefix);
                    let base = format!(
                        "{prefix}{}{}",
                        if prefix.is_empty() { "" } else { " " },
                        line.label
                    );
                    view.name = base.clone();
                    let mut serial = 1usize;
                    while views
                        .iter()
                        .any(|v| v.name.to_lowercase() == view.name.to_lowercase())
                    {
                        serial = serial.saturating_add(1);
                        view.name = format!("{base} ({serial})");
                    }
                    views.push(view);
                }
            } else {
                next.plane = model_view::grid_axis(grid, required("--axis")?)?;
                if let Some(i) = existing {
                    if let Some(v) = views.get_mut(i) {
                        *v = next;
                    }
                } else {
                    views.push(next);
                }
            }
        } else if let Some(i) = existing {
            if let Some(v) = views.get_mut(i) {
                *v = next;
            }
        } else {
            views.push(next);
        }
    }
    write_views(&views)?;
    Ok(reply(views, folders(root, project)?))
}
fn reply(views: Vec<ModelView>, folders: Vec<String>) -> Json {
    Json::object([
        (
            "folders",
            Json::Array(folders.into_iter().map(Json::text).collect()),
        ),
        ("outcome", Json::text("model_views")),
        ("views", Json::Array(views.iter().map(encode).collect())),
    ])
}
