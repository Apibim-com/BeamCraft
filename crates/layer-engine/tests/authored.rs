// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim
// Contributor-original generated synthetic fixtures, MPL-2.0; covered by ATTRIBUTION.md's
// crates/layer-engine/** row. Expectations follow F04 sections 2-4/8 and F06 sections 2/4.

//! U02 real-backend authoring regressions with original in-memory fixtures.

#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use beamcraft_layer_engine::{authored::*, *};
use beamcraft_model::{
    digest::{ContentDigest, Sha256},
    error::ErrorCode,
};
use openusd::sdf::AbstractData;
use std::{
    collections::BTreeSet,
    time::{Duration, Instant},
};

const PLATE: &str = "0199e7a0-0000-7000-8000-000000000003";
const MEMBER: &str = "0199e7a0-0000-7000-8000-000000000004";
const ACTOR: &str = "0199e7a0-0000-7000-8000-000000000005";
const OP: &str = "0199e7a0-0000-7000-8000-000000000006";
fn digest(bytes: &[u8]) -> ContentDigest {
    Sha256::digest(bytes).unwrap()
}
fn control() -> Control {
    Control {
        cancellation: Cancellation::default(),
        deadline: Instant::now() + Duration::from_secs(10),
    }
}
fn asset(name: &str, text: impl AsRef<[u8]>) -> Asset {
    Asset {
        name: name.into(),
        digest: digest(text.as_ref()),
        bytes: text.as_ref().to_vec(),
    }
}
fn layer(body: &str) -> String {
    format!("#usda 1.0\n(metersPerUnit=0.001\nupAxis=\"Z\")\n{body}\n")
}
fn request(assets: Vec<Asset>, owned: &[&str]) -> OpenRequest {
    let mut request = OpenRequest {
        identity: Identity {
            project: "0199e7a0-0000-7000-8000-000000000001".into(),
            workspace: "0199e7a0-0000-7000-8000-000000000002".into(),
            revision: digest(b"pinned U02 base"),
            generation: 42,
        },
        root: "root.usda".into(),
        assets,
        owned_layers: owned.iter().map(|s| (*s).into()).collect(),
        manifest: digest(b"placeholder"),
        backend: BACKEND,
        profile: Profile::StaticLocal,
        budgets: Budgets::default(),
    };
    request.manifest = request.manifest_digest().unwrap();
    request
}
fn initial() -> OpenRequest {
    request(
        vec![
            asset(
                "root.usda",
                "#usda 1.0\n(metersPerUnit=0.001\nupAxis=\"Z\"\nsubLayers=[@authored.usda@,@derived.usda@,@weak.usda@]\ncustomLayerData={string future=\"keep\"})\n",
            ),
            asset(
                "authored.usda",
                layer(
                    "def Scope \"Unknown\" { custom double future:factor=1.125\ncustom double future:absent\ncustom double future:blocked=None\ncustom rel future:link=</Unknown> }",
                ),
            ),
            asset("derived.usda", layer("")),
            asset("weak.usda", layer("")),
        ],
        &["root.usda", "authored.usda", "derived.usda", "weak.usda"],
    )
}
fn targets() -> EditTargets {
    EditTargets {
        authored: "authored.usda".into(),
        derived: "derived.usda".into(),
    }
}
fn open(input: OpenRequest) -> (Author, Opened) {
    Author::open(input, targets(), schema_identity().unwrap(), &control()).unwrap()
}
fn pose() -> Placement {
    Placement {
        translation_mm: [12.25, -42.125, 0.0],
        orientation_xyzw: [0.0, 0.0, 0.0, 1.0],
    }
}
fn resources() -> Vec<Resource> {
    vec![
        Resource::new(Definition::RectangleProfile {
            width_mm: 100.0,
            height_mm: 50.0,
        })
        .unwrap(),
        Resource::new(Definition::Material {
            density_kg_m3: Some(1000.0),
        })
        .unwrap(),
    ]
}
fn elements() -> Vec<Change> {
    let r = resources();
    vec![
        Change::Create(Element {
            id: PLATE.into(),
            name: "Plate Ω".into(),
            shape: Shape::Plate {
                contour_mm: vec![
                    [0.0, 0.0, 0.0],
                    [1000.0, 0.0, 0.0],
                    [1000.0, 500.0, 0.0],
                    [0.0, 500.0, 0.0],
                ],
                thickness_mm: 10.123456789012345,
            },
            placement: pose(),
            material: Some(r[1].digest),
        }),
        Change::Create(Element {
            id: MEMBER.into(),
            name: "Member".into(),
            shape: Shape::Member {
                kind: MemberKind::Beam,
                axis_mm: [[0.0, 0.0, 0.0], [0.0, 0.0, 2000.0]],
                profile: r[0].digest,
                roll_rad: Some(0.125),
            },
            placement: pose(),
            material: None,
        }),
    ]
}
fn mesh() -> Mesh {
    // Original synthetic topology transfer fixture; not a certified engineering solid.
    Mesh {
        id: PLATE.into(),
        shape_key: digest(b"engine-supplied-shape-key"),
        points_mm: vec![
            [0.0, 0.0, 0.0],
            [1000.0, 0.0, 0.0],
            [1000.0, 500.0, 0.0],
            [0.0, 500.0, 0.0],
        ],
        face_counts: vec![4],
        indices: vec![0, 1, 2, 3],
    }
}
fn batch(context: &Context, changes: Vec<Change>) -> Prepare {
    Prepare {
        context: context.clone(),
        op_id: OP.into(),
        actor: ACTOR.into(),
        schema: schema_identity().unwrap(),
        resources: Vec::new(),
        changes,
    }
}
fn prepare_initial(author: &mut Author, context: &Context) -> Prepared {
    let mut input = batch(context, elements());
    input.resources = resources();
    input.changes.push(Change::Mesh(mesh()));
    author.prepare(input, &control()).unwrap()
}
fn populated() -> (Author, Opened) {
    let (mut a, o) = open(initial());
    let prepared = prepare_initial(&mut a, &o.context);
    let installed = a
        .install(
            &o.context,
            &prepared.token,
            &prepared.op_id,
            Decision::Accepted,
            &control(),
        )
        .unwrap();
    (a, installed)
}
fn path(id: &str) -> String {
    format!("/World/Steel/E_{}", id.replace('-', ""))
}
fn snapshot(author: &Author, context: &Context) -> Vec<Prim> {
    let mut result = Vec::new();
    let mut cursor = None;
    loop {
        let page = author.read(context, cursor.as_ref(), &control()).unwrap();
        result.extend(page.items);
        cursor = page.next;
        if cursor.is_none() {
            break;
        }
    }
    result
}
fn value<'a>(prims: &'a [Prim], id: &str, name: &str) -> Option<&'a Value> {
    prims
        .iter()
        .find(|p| p.element_id.as_deref() == Some(id))
        .unwrap()
        .properties
        .iter()
        .find(|p| p.name == name)
        .and_then(|p| p.value.as_ref())
}

#[test]
fn author_install_reopen_standard_mesh_relationships_and_exact_unknown_opinions() {
    let (mut a, o) = open(initial());
    let before = a.export(&o.context, &control()).unwrap();
    let p = prepare_initial(&mut a, &o.context);
    assert_eq!(
        a.export(&o.context, &control())
            .unwrap()
            .iter()
            .map(|a| &a.bytes)
            .collect::<Vec<_>>(),
        before.iter().map(|a| &a.bytes).collect::<Vec<_>>()
    );
    assert_eq!(o.context.identity().generation, 42);
    assert_eq!(p.validation.profile, "u02-structural-v1");
    let new = a
        .install(
            &o.context,
            &p.token,
            &p.op_id,
            Decision::Accepted,
            &control(),
        )
        .unwrap();
    assert_eq!(new.context.identity().generation, 43);
    assert_eq!(new.meters_per_unit, 0.001);
    assert_eq!(new.up_axis, "Z");
    assert_eq!(
        a.read(&o.context, None, &control()).unwrap_err().code,
        ErrorCode::StaleWorkspace
    );
    let all = snapshot(&a, &new.context);
    assert_eq!(
        value(&all, PLATE, "steel:plate:thicknessMm"),
        Some(&Value::Number(10.123456789012345))
    );
    assert_eq!(
        value(&all, MEMBER, "steel:member:rollRad"),
        Some(&Value::Number(0.125))
    );
    assert!(
        all.iter()
            .find(|p| p.element_id.as_deref() == Some(PLATE))
            .unwrap()
            .relationships
            .iter()
            .any(|r| r.name == "bim:material" && r.targets.len() == 1)
    );
    let exported = a.export(&new.context, &control()).unwrap();
    let own = exported.iter().find(|a| a.name == "authored.usda").unwrap();
    let parsed = openusd::usda::parse(std::str::from_utf8(&own.bytes).unwrap()).unwrap();
    assert!(parsed.has_spec(&openusd::sdf::Path::new("/Unknown.future:link").unwrap()));
    assert!(parsed.has_spec(
        &openusd::sdf::Path::new(&format!("{}.bim:axisPointsMm", path(MEMBER))).unwrap()
    ));
    assert!(
        !parsed.has_spec(&openusd::sdf::Path::new(&format!("{}/Geometry", path(PLATE))).unwrap())
    );
    let bin = exported.iter().find(|a| a.name == "derived.usda").unwrap();
    let parsed = openusd::usda::parse(std::str::from_utf8(&bin.bytes).unwrap()).unwrap();
    let mesh_path = openusd::sdf::Path::new(&format!("{}/Geometry.points", path(PLATE))).unwrap();
    assert_eq!(
        parsed.get_field(&mesh_path, "typeName").unwrap().as_ref(),
        &openusd::sdf::Value::token("point3f[]")
    );
    let req = request(
        exported,
        &["root.usda", "authored.usda", "derived.usda", "weak.usda"],
    );
    assert!(validate_layers(req.clone(), schema_identity().unwrap(), &control()).is_ok());
    let (fresh, opened) = open(req);
    assert_eq!(snapshot(&fresh, &opened.context), all);
}

#[test]
fn twenty_seeded_permutations_produce_identical_owned_bytes_and_digests() {
    let mut expected = None;
    for seed in 0..20 {
        let (mut a, o) = open(initial());
        let mut changes = elements();
        changes.push(Change::Mesh(mesh()));
        let mut r = resources();
        if seed % 2 == 1 {
            r.reverse();
        }
        changes.rotate_left(seed % 3);
        if seed % 4 == 0 {
            changes.reverse();
        }
        let mut input = batch(&o.context, changes);
        input.resources = r;
        let p = a.prepare(input, &control()).unwrap();
        let bytes = p
            .layers
            .iter()
            .map(|a| (a.name.clone(), a.digest, a.bytes.clone()))
            .collect::<Vec<_>>();
        if let Some(expected) = &expected {
            assert_eq!(expected, &bytes);
        } else {
            expected = Some(bytes);
        }
        let o = a
            .install(
                &o.context,
                &p.token,
                &p.op_id,
                Decision::Accepted,
                &control(),
            )
            .unwrap();
        let req = request(
            a.export(&o.context, &control()).unwrap(),
            &["root.usda", "authored.usda", "derived.usda", "weak.usda"],
        );
        let (_, o) = open(req);
        assert!(o.prim_count >= 8);
    }
}

#[test]
fn absence_block_removal_reveal_weaker_defaults_without_flattening() {
    let (a, o) = populated();
    let mut assets = a.export(&o.context, &control()).unwrap();
    let weak = assets.iter_mut().find(|a| a.name == "weak.usda").unwrap();
    *weak = asset(
        "weak.usda",
        layer(&format!(
            "over \"World\" {{ over \"Steel\" {{ over \"E_{}\" {{ custom double steel:member:rollRad=0.75 }} }} }}",
            MEMBER.replace('-', "")
        )),
    );
    let weak_before = weak.bytes.clone();
    let (mut a, mut o) = open(request(
        assets,
        &["root.usda", "authored.usda", "derived.usda", "weak.usda"],
    ));
    for (opinion, expected, spec_exists) in [
        (Opinion::Absent, Some(Value::Number(0.75)), true),
        (Opinion::Block, Some(Value::Block), true),
        (Opinion::Remove, Some(Value::Number(0.75)), false),
        (Opinion::Value(0.125), Some(Value::Number(0.125)), true),
    ] {
        let expected_local = opinion.clone();
        let p = a
            .prepare(
                batch(
                    &o.context,
                    vec![Change::Roll {
                        id: MEMBER.into(),
                        opinion,
                    }],
                ),
                &control(),
            )
            .unwrap();
        let local = p
            .opinions
            .iter()
            .find(|v| v.element_id == MEMBER && v.attribute == "steel:member:rollRad")
            .unwrap();
        let expected_local = match expected_local {
            Opinion::Value(n) => Opinion::Value(Value::Number(n)),
            Opinion::Absent => Opinion::Absent,
            Opinion::Block => Opinion::Block,
            Opinion::Remove => Opinion::Remove,
        };
        assert_eq!(local.opinion, expected_local);
        o = a
            .install(
                &o.context,
                &p.token,
                &p.op_id,
                Decision::Accepted,
                &control(),
            )
            .unwrap();
        assert_eq!(
            value(&snapshot(&a, &o.context), MEMBER, "steel:member:rollRad"),
            expected.as_ref()
        );
        let assets = a.export(&o.context, &control()).unwrap();
        assert_eq!(
            assets.iter().find(|a| a.name == "weak.usda").unwrap().bytes,
            weak_before
        );
        let d = openusd::usda::parse(
            std::str::from_utf8(
                &assets
                    .iter()
                    .find(|a| a.name == "authored.usda")
                    .unwrap()
                    .bytes,
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(
            d.has_spec(
                &openusd::sdf::Path::new(&format!("{}.steel:member:rollRad", path(MEMBER)))
                    .unwrap()
            ),
            spec_exists
        );
    }
}

#[test]
fn foreign_reference_overrides_and_foreign_ancestor_writes_fail_atomically() {
    let mut input = initial();
    let foreign = layer("def Xform \"Source\" { custom double future:value=23 }");
    input.assets.push(asset("foreign.usda", foreign));
    let body = layer(&format!(
        "over \"World\" {{ over \"Steel\" {{ def Xform \"E_{}\" (references=@foreign.usda@</Source>) {{ custom string bim:id=\"{PLATE}\" }} }} }}",
        PLATE.replace('-', "")
    ));
    *input
        .assets
        .iter_mut()
        .find(|a| a.name == "authored.usda")
        .unwrap() = asset("authored.usda", body);
    input.manifest = input.manifest_digest().unwrap();
    let (mut a, o) = open(input);
    let before = a.export(&o.context, &control()).unwrap();
    for change in [
        Change::Label {
            id: PLATE.into(),
            name: "Attempt".into(),
        },
        Change::Thickness {
            id: PLATE.into(),
            mm: 10.0,
        },
        Change::Material {
            id: PLATE.into(),
            opinion: MaterialOpinion::Empty,
        },
        Change::Mesh(mesh()),
    ] {
        assert_eq!(
            a.prepare(batch(&o.context, vec![change]), &control())
                .unwrap_err()
                .code,
            ErrorCode::ForeignReadOnly
        );
    }
    assert_eq!(
        a.export(&o.context, &control())
            .unwrap()
            .iter()
            .map(|a| &a.bytes)
            .collect::<Vec<_>>(),
        before.iter().map(|a| &a.bytes).collect::<Vec<_>>()
    );
    // A foreign ancestor also forbids creating a new child in our stronger layer.
    let mut req = initial();
    req.assets.push(asset(
        "foreign.usda",
        layer("over \"World\" { over \"Steel\" {} }"),
    ));
    let root = req
        .assets
        .iter_mut()
        .find(|a| a.name == "root.usda")
        .unwrap();
    *root = asset(
        "root.usda",
        String::from_utf8(root.bytes.clone())
            .unwrap()
            .replace("@weak.usda@]", "@weak.usda@,@foreign.usda@]"),
    );
    req.manifest = req.manifest_digest().unwrap();
    let (mut a, o) = open(req);
    assert_eq!(
        a.prepare(batch(&o.context, elements()), &control())
            .unwrap_err()
            .code,
        ErrorCode::ForeignReadOnly
    );
}

#[test]
fn mismatched_schema_pin_corruption_and_nonfinite_never_change_live_state() {
    let (mut a, o) = populated();
    let before = snapshot(&a, &o.context);
    let mut input = batch(
        &o.context,
        vec![Change::Label {
            id: PLATE.into(),
            name: "after".into(),
        }],
    );
    input.schema.digest = digest(b"wrong schema");
    assert_eq!(
        a.prepare(input, &control()).unwrap_err().code,
        ErrorCode::UnsupportedVersion
    );
    for mm in [
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        0.0,
        -1.0,
        0.0009,
        1_000_000.1,
    ] {
        let code = if !mm.is_finite() {
            ErrorCode::NonFiniteNumber
        } else if mm > 1e6 {
            ErrorCode::LimitExceeded
        } else {
            ErrorCode::InvalidArgument
        };
        assert_eq!(
            a.prepare(
                batch(
                    &o.context,
                    vec![Change::Thickness {
                        id: PLATE.into(),
                        mm
                    }]
                ),
                &control()
            )
            .unwrap_err()
            .code,
            code
        );
        assert_eq!(snapshot(&a, &o.context), before);
    }
    let mut req = initial();
    req.assets[1].bytes.push(0);
    assert_eq!(
        validate_layers(req, schema_identity().unwrap(), &control())
            .unwrap_err()
            .code,
        ErrorCode::IntegrityError
    );
    let mut req = initial();
    req.backend.version = "0.9.0";
    req.manifest = req.manifest_digest().unwrap();
    assert_eq!(
        validate_layers(req, schema_identity().unwrap(), &control())
            .unwrap_err()
            .code,
        ErrorCode::UnsupportedVersion
    );
    let mut req = initial();
    req.assets[1] = asset("authored.usda", layer("def {"));
    req.manifest = req.manifest_digest().unwrap();
    assert!(validate_layers(req, schema_identity().unwrap(), &control()).is_err());
}

#[test]
fn candidate_decision_cancellation_wrong_owner_stale_token_and_generation_gates() {
    let (mut a, o) = open(initial());
    let p = prepare_initial(&mut a, &o.context);
    assert_eq!(
        a.prepare(batch(&o.context, elements()), &control())
            .unwrap_err()
            .code,
        ErrorCode::DependencyConflict
    );
    for decision in [Decision::Accepted, Decision::Unknown] {
        assert_eq!(
            a.discard(&o.context, &p.token, &p.op_id, decision, &control())
                .unwrap_err()
                .code,
            ErrorCode::OutcomeUnknown
        );
    }
    assert_eq!(
        a.install(
            &o.context,
            &p.token,
            &p.op_id,
            Decision::Unknown,
            &control()
        )
        .unwrap_err()
        .code,
        ErrorCode::OutcomeUnknown
    );
    let cancelled = control();
    cancelled.cancellation.cancel();
    assert_eq!(
        a.install(
            &o.context,
            &p.token,
            &p.op_id,
            Decision::Accepted,
            &cancelled
        )
        .unwrap_err()
        .code,
        ErrorCode::OutcomeUnknown
    );
    assert_eq!(snapshot(&a, &o.context).len(), 1);
    let (mut other, other_o) = open(initial());
    assert_eq!(
        other
            .discard(
                &other_o.context,
                &p.token,
                &p.op_id,
                Decision::Undecided,
                &control()
            )
            .unwrap_err()
            .code,
        ErrorCode::StaleWorkspace
    );
    a.discard(
        &o.context,
        &p.token,
        &p.op_id,
        Decision::Rejected,
        &control(),
    )
    .unwrap();
    assert_eq!(
        a.install(
            &o.context,
            &p.token,
            &p.op_id,
            Decision::Accepted,
            &control()
        )
        .unwrap_err()
        .code,
        ErrorCode::StaleWorkspace
    );
    assert_eq!(
        a.prepare(batch(&o.context, elements()), &cancelled)
            .unwrap_err()
            .code,
        ErrorCode::Cancelled
    );
    let p = prepare_initial(&mut a, &o.context);
    let new = a
        .install(
            &o.context,
            &p.token,
            &p.op_id,
            Decision::Accepted,
            &control(),
        )
        .unwrap();
    assert_eq!(new.context.identity().generation, 43);
    let mut req = initial();
    req.identity.generation = u64::MAX;
    req.manifest = req.manifest_digest().unwrap();
    let (mut a, o) = open(req);
    let mut input = batch(&o.context, elements());
    input.resources = resources();
    assert_eq!(
        a.prepare(input, &control()).unwrap_err().code,
        ErrorCode::LimitExceeded
    );
}

#[test]
fn hundred_serialized_changes_invalidate_contexts_and_derive_semantic_groups() {
    let (mut a, mut o) = populated();
    for n in 0..100 {
        let id = format!("0199e7a0-0000-7000-8000-{:012x}", n + 100);
        let mut input = batch(
            &o.context,
            vec![Change::Label {
                id: PLATE.into(),
                name: format!("Plate {n}"),
            }],
        );
        input.op_id = id;
        let p = a.prepare(input, &control()).unwrap();
        assert!(p.writes.contains(&Footprint {
            identity: PLATE.into(),
            group: Group::Label
        }));
        assert!(!p.writes.contains(&Footprint {
            identity: PLATE.into(),
            group: Group::ShapeInputs
        }));
        o = a
            .install(
                &o.context,
                &p.token,
                &p.op_id,
                Decision::Accepted,
                &control(),
            )
            .unwrap();
    }
    assert_eq!(o.context.identity().generation, 143);
    let p = a
        .prepare(
            batch(
                &o.context,
                vec![Change::Thickness {
                    id: PLATE.into(),
                    mm: 12.0,
                }],
            ),
            &control(),
        )
        .unwrap();
    assert!(p.reads.contains(&Footprint {
        identity: PLATE.into(),
        group: Group::ShapeInputs
    }));
    assert!(p.writes.contains(&Footprint {
        identity: PLATE.into(),
        group: Group::Derived
    }));
    o = a
        .install(
            &o.context,
            &p.token,
            &p.op_id,
            Decision::Accepted,
            &control(),
        )
        .unwrap();
    assert!(
        !snapshot(&a, &o.context)
            .iter()
            .any(|p| p.path.ends_with("/Geometry"))
    );
}

#[test]
fn duplicate_keys_wrong_kind_bad_mesh_and_missing_or_forged_definitions_fail() {
    let (mut a, o) = populated();
    let c = Change::Label {
        id: PLATE.into(),
        name: "rename".into(),
    };
    assert_eq!(
        a.prepare(batch(&o.context, vec![c.clone(), c]), &control())
            .unwrap_err()
            .code,
        ErrorCode::MalformedCommand
    );
    assert_eq!(
        a.prepare(
            batch(
                &o.context,
                vec![Change::Roll {
                    id: PLATE.into(),
                    opinion: Opinion::Value(0.0)
                }]
            ),
            &control()
        )
        .unwrap_err()
        .code,
        ErrorCode::UnsupportedProperty
    );
    let mut bad = mesh();
    bad.indices[0] = -1;
    assert_eq!(
        a.prepare(batch(&o.context, vec![Change::Mesh(bad)]), &control())
            .unwrap_err()
            .code,
        ErrorCode::InvalidArgument
    );
    let mut bad = mesh();
    bad.points_mm[0][0] = f32::NAN;
    assert_eq!(
        a.prepare(batch(&o.context, vec![Change::Mesh(bad)]), &control())
            .unwrap_err()
            .code,
        ErrorCode::NonFiniteNumber
    );
    assert_eq!(
        a.prepare(
            batch(
                &o.context,
                vec![Change::Material {
                    id: PLATE.into(),
                    opinion: MaterialOpinion::Target(digest(b"missing"))
                }]
            ),
            &control()
        )
        .unwrap_err()
        .code,
        ErrorCode::DanglingRelationship
    );
    let (mut a, o) = open(initial());
    let mut input = batch(&o.context, elements());
    input.resources = resources();
    input.resources[0].digest = digest(b"forged");
    assert_eq!(
        a.prepare(input, &control()).unwrap_err().code,
        ErrorCode::IntegrityError
    );
    let mut req = initial();
    req.owned_layers = BTreeSet::from(["root.usda".into()]);
    req.manifest = req.manifest_digest().unwrap();
    assert_eq!(
        Author::open(req, targets(), schema_identity().unwrap(), &control())
            .err()
            .unwrap()
            .code,
        ErrorCode::ForeignReadOnly
    );
}

#[test]
fn material_empty_remove_profile_pin_and_loaded_unknown_schema_preservation() {
    let (mut a, mut o) = populated();
    for opinion in [MaterialOpinion::Empty, MaterialOpinion::Remove] {
        let p = a
            .prepare(
                batch(
                    &o.context,
                    vec![Change::Material {
                        id: PLATE.into(),
                        opinion: opinion.clone(),
                    }],
                ),
                &control(),
            )
            .unwrap();
        let local = p
            .opinions
            .iter()
            .find(|p| p.attribute == "bim:material")
            .unwrap();
        assert_eq!(
            local.opinion,
            if opinion == MaterialOpinion::Empty {
                Opinion::Value(Value::Array(vec![]))
            } else {
                Opinion::Remove
            }
        );
        o = a
            .install(
                &o.context,
                &p.token,
                &p.op_id,
                Decision::Accepted,
                &control(),
            )
            .unwrap();
    }
    let r = Resource::new(Definition::RectangleProfile {
        width_mm: 120.0,
        height_mm: 60.0,
    })
    .unwrap();
    let mut input = batch(
        &o.context,
        vec![Change::Profile {
            id: MEMBER.into(),
            digest: r.digest,
        }],
    );
    input.resources = vec![r];
    let p = a.prepare(input, &control()).unwrap();
    assert!(p.reads.contains(&Footprint {
        identity: resources()[0].digest.to_string(),
        group: Group::Relationship
    }));
    o = a
        .install(
            &o.context,
            &p.token,
            &p.op_id,
            Decision::Accepted,
            &control(),
        )
        .unwrap();
    let mut assets = a.export(&o.context, &control()).unwrap();
    let authored = assets
        .iter_mut()
        .find(|a| a.name == "authored.usda")
        .unwrap();
    // A newer schema is inert on load and cannot be silently migrated by editing it.
    let text = String::from_utf8(authored.bytes.clone()).unwrap().replace(
        "bim:schemaVersion = \"0.1.0\"",
        "bim:schemaVersion = \"0.9.0\"",
    );
    *authored = asset("authored.usda", text);
    let (mut a, o) = open(request(
        assets,
        &["root.usda", "authored.usda", "derived.usda", "weak.usda"],
    ));
    assert_eq!(
        a.prepare(
            batch(
                &o.context,
                vec![Change::Label {
                    id: PLATE.into(),
                    name: "attempt".into()
                }]
            ),
            &control()
        )
        .unwrap_err()
        .code,
        ErrorCode::UnsupportedVersion
    );
}

#[test]
fn validator_rejects_schema_markers_changed_definition_bytes_and_loaded_bad_mesh() {
    let (a, o) = populated();
    let base = a.export(&o.context, &control()).unwrap();
    let mut assets = base.clone();
    let own = assets
        .iter_mut()
        .find(|a| a.name == "authored.usda")
        .unwrap();
    *own = asset(
        "authored.usda",
        String::from_utf8(own.bytes.clone()).unwrap().replace(
            "steel:profile:widthMm = 100.0",
            "steel:profile:widthMm = 101.0",
        ),
    );
    assert_eq!(
        validate_layers(
            request(
                assets,
                &["root.usda", "authored.usda", "derived.usda", "weak.usda"]
            ),
            schema_identity().unwrap(),
            &control()
        )
        .unwrap_err()
        .code,
        ErrorCode::IntegrityError
    );
    let mut assets = base.clone();
    let derived = assets
        .iter_mut()
        .find(|a| a.name == "derived.usda")
        .unwrap();
    *derived = asset(
        "derived.usda",
        String::from_utf8(derived.bytes.clone())
            .unwrap()
            .replace("[0, 1, 2, 3]", "[0, 1, 2, 999]"),
    );
    assert_eq!(
        validate_layers(
            request(
                assets,
                &["root.usda", "authored.usda", "derived.usda", "weak.usda"]
            ),
            schema_identity().unwrap(),
            &control()
        )
        .unwrap_err()
        .code,
        ErrorCode::InvalidArgument
    );
    let mut assets = base;
    let root = assets.iter_mut().find(|a| a.name == "root.usda").unwrap();
    *root = asset(
        "root.usda",
        String::from_utf8(root.bytes.clone()).unwrap().replace(
            &schema_identity().unwrap().digest.to_string(),
            &digest(b"incorrect schema").to_string(),
        ),
    );
    assert_eq!(
        validate_layers(
            request(
                assets,
                &["root.usda", "authored.usda", "derived.usda", "weak.usda"]
            ),
            schema_identity().unwrap(),
            &control()
        )
        .unwrap_err()
        .code,
        ErrorCode::UnsupportedVersion
    );
}

#[test]
fn changed_input_bounds_are_inclusive_and_rejections_keep_the_entire_patch_atomic() {
    let (mut a, mut o) = populated();
    let before = snapshot(&a, &o.context);
    let bad = vec![
        Change::Label {
            id: PLATE.into(),
            name: "would be valid".into(),
        },
        Change::Thickness {
            id: PLATE.into(),
            mm: f64::INFINITY,
        },
    ];
    assert_eq!(
        a.prepare(batch(&o.context, bad), &control())
            .unwrap_err()
            .code,
        ErrorCode::NonFiniteNumber
    );
    assert_eq!(snapshot(&a, &o.context), before);
    let p = a
        .prepare(
            batch(
                &o.context,
                vec![Change::Thickness {
                    id: PLATE.into(),
                    mm: 0.001,
                }],
            ),
            &control(),
        )
        .unwrap();
    o = a
        .install(
            &o.context,
            &p.token,
            &p.op_id,
            Decision::Accepted,
            &control(),
        )
        .unwrap();
    let mut bad = mesh();
    bad.points_mm = vec![[0.0; 3]; 342];
    assert_eq!(
        a.prepare(batch(&o.context, vec![Change::Mesh(bad)]), &control())
            .unwrap_err()
            .code,
        ErrorCode::LimitExceeded
    );
    let p = a
        .prepare(
            batch(
                &o.context,
                vec![Change::Label {
                    id: PLATE.into(),
                    name: "x".repeat(256),
                }],
            ),
            &control(),
        )
        .unwrap();
    a.discard(
        &o.context,
        &p.token,
        &p.op_id,
        Decision::Undecided,
        &control(),
    )
    .unwrap();
    assert_eq!(
        a.prepare(
            batch(
                &o.context,
                vec![Change::Label {
                    id: PLATE.into(),
                    name: "x".repeat(257)
                }]
            ),
            &control()
        )
        .unwrap_err()
        .code,
        ErrorCode::LimitExceeded
    );
    assert_eq!(
        a.install(
            &o.context,
            &p.token,
            &p.op_id,
            Decision::Accepted,
            &control()
        )
        .unwrap_err()
        .code,
        ErrorCode::StaleWorkspace
    );
}
