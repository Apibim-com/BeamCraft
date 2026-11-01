// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! A bounded, standalone local presentation layer. No engineering revision or schema mutation.

use super::edit;
use crate::*;
use openusd::{gf, sdf};
use std::collections::BTreeMap;

/// Maximum size of the standalone presentation layer.
pub const MAX_BYTES: usize = 1_048_576;

/// One local camera and its closed set of application presentation attributes.
pub struct CameraRecord {
    /// Full UUIDv7, used in the camera prim path.
    pub id: beamcraft_model::id::Uuid7,
    /// Row-major USD camera-to-world matrix; translation is the last row, millimetres.
    pub transform: [f64; 16],
    /// Orthographic or perspective projection.
    pub projection: String,
    /// Vertical span at the target, millimetres.
    pub span_mm: f64,
    /// Scalar/flat-array application properties, separately typed in USD.
    pub properties: BTreeMap<String, Value>,
}

/// Decoded local presentation data; domain admission remains the caller's responsibility.
pub struct Collection {
    /// Browser paths, independent of camera prim paths.
    pub folders: Vec<String>,
    /// Each camera's application properties, in deterministic prim order.
    pub records: Vec<BTreeMap<String, Value>>,
}

const FIELDS: &[&str] = &[
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
];

fn value(v: &Value) -> Result<(&'static str, sdf::Value), Diagnostic> {
    Ok(match v {
        Value::Block => ("double[]", sdf::Value::DoubleVec(Vec::new())),
        Value::Text(s) if s.len() <= 2048 && !s.chars().any(char::is_control) => {
            ("string", sdf::Value::String(s.clone()))
        }
        Value::Bool(b) => ("bool", sdf::Value::Bool(*b)),
        Value::Number(n) if n.is_finite() => ("double", sdf::Value::Double(*n)),
        Value::Array(a) if a.len() <= 16 => {
            let numbers = a
                .iter()
                .map(|n| match n {
                    Value::Number(n) if n.is_finite() => Ok(*n),
                    _ => Err(corrupt()),
                })
                .collect::<Result<Vec<_>, _>>()?;
            ("double[]", sdf::Value::DoubleVec(numbers))
        }
        _ => return Err(corrupt()),
    })
}

/// Encode native USD Camera prims with explicit application properties, under a 1 MiB cap.
pub fn write(
    project: &str,
    folders: &[String],
    records: &[CameraRecord],
) -> Result<Vec<u8>, Diagnostic> {
    if !uuid7(project) || records.len() > 256 || folders.len() > 512 {
        return Err(limit());
    }
    let mut data = sdf::Data::default();
    edit::prim(&mut data, "/", None, false)?;
    edit::field(&mut data, "/", "metersPerUnit", sdf::Value::Double(0.001))?;
    edit::field(&mut data, "/", "upAxis", sdf::Value::token("Z"))?;
    edit::prim(&mut data, "/Views", Some("Scope"), true)?;
    edit::attr(
        &mut data,
        "/Views",
        "bim:viewFormat",
        "string",
        sdf::Value::String("local-views/1".into()),
    )?;
    edit::attr(
        &mut data,
        "/Views",
        "bim:project",
        "string",
        sdf::Value::String(project.into()),
    )?;
    if folders
        .iter()
        .any(|s| s.len() > 512 || s.chars().any(char::is_control))
    {
        return Err(limit());
    }
    edit::attr(
        &mut data,
        "/Views",
        "bim:folders",
        "string[]",
        sdf::Value::StringVec(folders.to_vec()),
    )?;
    let mut ids = BTreeSet::new();
    for record in records {
        if !ids.insert(record.id)
            || record.properties.len() != FIELDS.len()
            || record
                .properties
                .keys()
                .any(|k| !FIELDS.contains(&k.as_str()))
            || record.transform.iter().any(|n| !n.is_finite())
            || !record.span_mm.is_finite()
            || !(1.0..=20_000_000.).contains(&record.span_mm)
            || !["orthographic", "perspective"].contains(&record.projection.as_str())
        {
            return Err(corrupt());
        }
        let path = format!("/Views/v_{}", record.id.to_string().replace('-', ""));
        edit::prim(&mut data, &path, Some("Camera"), true)?;
        edit::attr(
            &mut data,
            &path,
            "xformOp:transform",
            "matrix4d",
            sdf::Value::Matrix4d(gf::Matrix4d(record.transform)),
        )?;
        edit::attr(
            &mut data,
            &path,
            "xformOpOrder",
            "token[]",
            sdf::Value::TokenVec(vec!["xformOp:transform".into()]),
        )?;
        edit::attr(
            &mut data,
            &path,
            "projection",
            "token",
            sdf::Value::token(&record.projection),
        )?;
        // USD Camera apertures and focal length use tenths of a world unit. At 45 degrees,
        // perspective target distance is span/(2*tan(pi/8)); the engine camera uses the same.
        let aperture = if record.projection == "orthographic" {
            record.span_mm * 10.
        } else {
            20.955
        };
        edit::attr(
            &mut data,
            &path,
            "verticalAperture",
            "float",
            sdf::Value::Float(aperture as f32),
        )?;
        edit::attr(
            &mut data,
            &path,
            "horizontalAperture",
            "float",
            sdf::Value::Float(aperture as f32),
        )?;
        edit::attr(
            &mut data,
            &path,
            "focalLength",
            "float",
            sdf::Value::Float((20.955 / (2. * (std::f64::consts::PI / 8.).tan())) as f32),
        )?;
        edit::attr(
            &mut data,
            &path,
            "clippingRange",
            "float2",
            sdf::Value::Vec2f(gf::Vec2f {
                x: 0.1,
                y: 110_000_000.,
            }),
        )?;
        for (key, v) in &record.properties {
            let (ty, v) = value(v)?;
            edit::attr(&mut data, &path, &format!("bim:view:{key}"), ty, v)?;
        }
    }
    edit::serialize(&data, MAX_BYTES)
}

/// Read only the supplied local layer, through the normal bounded virtual USD reader.
pub fn read(bytes: &[u8], project: &str) -> Result<Collection, Diagnostic> {
    if bytes.len() > MAX_BYTES {
        return Err(limit());
    }
    let digest = Sha256::digest(bytes).map_err(|_| limit())?;
    let mut request = OpenRequest {
        identity: Identity {
            project: project.into(),
            workspace: project.into(),
            revision: digest,
            generation: 0,
        },
        root: "views.usda".into(),
        assets: vec![Asset {
            name: "views.usda".into(),
            digest,
            bytes: bytes.to_vec(),
        }],
        owned_layers: BTreeSet::from(["views.usda".into()]),
        manifest: digest,
        backend: BACKEND,
        profile: Profile::StaticLocal,
        budgets: Budgets {
            layer_bytes: MAX_BYTES,
            total_bytes: MAX_BYTES,
            layers: 1,
            prims: 257,
            values: 20_000,
            array_items: 512,
            text_bytes: 2048,
            ..Budgets::default()
        },
    };
    // This commits a local, untrusted presentation input, not a verified model revision.
    request.manifest = request.manifest_digest()?;
    let control = Control {
        cancellation: Cancellation::default(),
        deadline: Instant::now() + std::time::Duration::from_secs(30),
    };
    let mut reader = Reader::default();
    let opened = reader.open(request, &control)?;
    let mut cursor = None;
    let mut folders = None;
    let mut records = Vec::new();
    loop {
        let page = reader.read(&opened.context, cursor.as_ref(), &control)?;
        for prim in page.items {
            if !prim.active || !prim.relationships.is_empty() {
                return Err(corrupt());
            }
            let fields: BTreeMap<_, _> = prim
                .properties
                .into_iter()
                .map(|p| Ok((p.name, p.value.ok_or_else(corrupt)?)))
                .collect::<Result<_, Diagnostic>>()?;
            if prim.path == "/Views" {
                if prim.type_name.as_deref() != Some("Scope")
                    || fields.len() != 3
                    || fields.get("bim:project") != Some(&Value::Text(project.into()))
                    || fields.get("bim:viewFormat") != Some(&Value::Text("local-views/1".into()))
                {
                    return Err(corrupt());
                }
                let Some(Value::Array(list)) = fields.get("bim:folders") else {
                    return Err(corrupt());
                };
                folders = Some(
                    list.iter()
                        .map(|v| match v {
                            Value::Text(s) => Ok(s.clone()),
                            _ => Err(corrupt()),
                        })
                        .collect::<Result<Vec<_>, _>>()?,
                );
            } else {
                if prim.type_name.as_deref() != Some("Camera")
                    || !prim.path.starts_with("/Views/v_")
                {
                    return Err(corrupt());
                }
                let mut record = BTreeMap::new();
                for (key, mut v) in fields {
                    if let Some(key) = key.strip_prefix("bim:view:") {
                        if !FIELDS.contains(&key) {
                            return Err(corrupt());
                        }
                        if matches!(&v, Value::Array(a) if a.is_empty())
                            && ["depth_mm", "color", "work_area"].contains(&key)
                        {
                            v = Value::Block;
                        }
                        record.insert(key.into(), v);
                    } else if ![
                        "xformOp:transform",
                        "xformOpOrder",
                        "projection",
                        "verticalAperture",
                        "horizontalAperture",
                        "focalLength",
                        "clippingRange",
                    ]
                    .contains(&key.as_str())
                    {
                        return Err(corrupt());
                    }
                }
                if record.len() != FIELDS.len() {
                    return Err(corrupt());
                }
                records.push(record);
            }
        }
        cursor = page.next;
        if cursor.is_none() {
            break;
        }
    }
    Ok(Collection {
        folders: folders.ok_or_else(corrupt)?,
        records,
    })
}
