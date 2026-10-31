// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! End-to-end tests of the V02 view snapshot and query contract with the real kernel.
//!
//! Every scene is built from genuine `beamcraft_kernel` plate results. No test fakes a mesh.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use beamcraft_engine::steel::synthetic::{SyntheticEdit, apply_edit, plate_scene};
use beamcraft_engine::{
    ChangeKind, ElementId, ElementKind, KindFilter, Limits, Representation, SceneElement,
    Selection, SnapshotHistory, SnapshotToken, ViewCode, ViewError, ViewPin, ViewSettings,
    ViewSnapshot,
};
use beamcraft_geom::Translation3;
use beamcraft_kernel::{steel::RectangularPlate, steel::rectangular_plate};
use beamcraft_model::digest::Sha256;

fn pin(generation: u64) -> ViewPin {
    ViewPin::new(Sha256::digest(b"v02-test-revision").unwrap(), generation)
}

fn id(n: u64) -> ElementId {
    ElementId::parse(&format!("01990000-0000-7000-8000-{n:012x}")).unwrap()
}

fn plate(w: f64, l: f64, t: f64) -> beamcraft_kernel::steel::PlateResult {
    rectangular_plate(RectangularPlate {
        width_mm: w,
        length_mm: l,
        thickness_mm: t,
    })
    .unwrap()
}

fn at(x: f64, y: f64, z: f64) -> Translation3 {
    Translation3::new(x, y, z).unwrap()
}

fn snap(scene: Vec<SceneElement>, generation: u64) -> ViewSnapshot {
    ViewSnapshot::build(pin(generation), scene, &Limits::default()).unwrap()
}

fn code<T: std::fmt::Debug>(r: Result<T, beamcraft_engine::ViewError>) -> ViewCode {
    r.unwrap_err().code()
}

fn hiding(kinds: &[ElementKind]) -> ViewSettings {
    ViewSettings {
        hidden_kinds: kinds.iter().fold(KindFilter::SHOW_ALL, |f, k| f.hiding(*k)),
        representation: Representation::Full,
    }
}

#[test]
fn snapshot_is_ordered_deterministic_and_pinned() {
    let mut scene = plate_scene(12, false).unwrap();
    let a = snap(scene.clone(), 7);
    scene.reverse();
    let b = snap(scene, 7);
    assert_eq!(a, b, "input order never matters");
    assert_eq!(a.token(), b.token());
    assert_eq!(a.pin(), pin(7));
    assert_eq!(a.len(), 12);
    assert!(a.elements().windows(2).all(|w| w[0].id < w[1].id));
    for (i, e) in a.elements().iter().enumerate() {
        assert_eq!(e.index as usize, i);
        assert_eq!(a.element_at(e.index).unwrap().id, e.id);
        assert_eq!(a.element(&e.id).unwrap().index, e.index);
    }
    assert!(a.element(&id(9_999)).is_none());
    assert!(a.element_at(12).is_none());
    let other_generation = snap(plate_scene(12, false).unwrap(), 8);
    assert_ne!(a.token(), other_generation.token());
    let other_revision = ViewSnapshot::build(
        ViewPin::new(Sha256::digest(b"other").unwrap(), 7),
        plate_scene(12, false).unwrap(),
        &Limits::default(),
    )
    .unwrap();
    assert_ne!(a.token(), other_revision.token());
    let text = a.token().as_str();
    assert_eq!(text.len(), 68);
    assert_eq!(SnapshotToken::parse(text).unwrap(), *a.token());
}

#[test]
fn bounds_come_from_kernel_vertices_and_placement() {
    let s = snap(plate_scene(2, false).unwrap(), 1);
    let first = s.element_bounds(&id(0)).unwrap();
    assert_eq!(first.min, [0.0, 0.0, 0.0]);
    assert_eq!(first.max, [65.0, 70.0, 6.0]);
    // Element 1 is 71 x 73 x 8 at (160, 0, 0).
    let second = s.element_bounds(&id(1)).unwrap();
    assert_eq!(second.min, [160.0, 0.0, 0.0]);
    assert_eq!(second.max, [231.0, 73.0, 8.0]);
    let all = s.engineering_bounds().unwrap();
    assert_eq!(all.min, [0.0, 0.0, 0.0]);
    assert_eq!(all.max, [231.0, 73.0, 8.0]);
    assert_eq!(code(s.element_bounds(&id(5))), ViewCode::UnknownElement);
}

#[test]
fn identical_blobs_are_stored_once_and_one_mesh_serves_both_representations() {
    let same = |n: u64| {
        SceneElement::new(
            id(n),
            ElementKind::Plate,
            at(n as f64 * 200.0, 0.0, 0.0),
            plate(100.0, 80.0, 10.0),
        )
    };
    let s = snap(vec![same(1), same(2), same(3)], 1);
    assert_eq!(s.meshes().len(), 1);
    for e in s.elements() {
        assert_eq!(e.full.mesh_digest, e.simplified.mesh_digest);
        assert_eq!(e.full.mesh_slot, e.simplified.mesh_slot);
        assert!(e.full.exact && e.simplified.exact, "same mesh is exact");
    }
    let mesh = s.mesh(&s.elements()[0].full.mesh_digest).unwrap();
    assert_eq!(mesh.bytes().len(), 392);
    assert_eq!(Sha256::digest(mesh.bytes()).unwrap(), mesh.digest());
    assert!(s.mesh(&Sha256::digest(b"nope").unwrap()).is_none());
    assert!(s.mesh_at(1).is_none());
}

#[test]
fn a_distinct_simplified_mesh_is_flagged_not_exact() {
    let scene = vec![
        SceneElement::new(
            id(1),
            ElementKind::Plate,
            at(0.0, 0.0, 0.0),
            plate(100.0, 80.0, 10.0),
        )
        .with_simplified(plate(100.0, 80.0, 5.0)),
    ];
    let s = snap(scene, 1);
    let e = &s.elements()[0];
    assert_ne!(e.full.mesh_digest, e.simplified.mesh_digest);
    assert!(e.full.exact);
    assert!(!e.simplified.exact);
    assert_eq!(s.meshes().len(), 2);
    assert!(Representation::Full.feeds_engineering());
    assert!(!Representation::Simplified.feeds_engineering());
}

#[test]
fn view_settings_project_without_touching_the_snapshot_or_engineering_data() {
    let s = snap(plate_scene(20, true).unwrap(), 3);
    let token = s.token().clone();
    let bounds_before = s.engineering_bounds();
    let full = s.project(&ViewSettings::default()).unwrap();
    assert_eq!(full.len(), 20);
    assert_eq!(full.hidden_count(), 0);
    assert!(full.all_exact());
    assert_eq!(full.total_triangles(), 20 * 12);

    // Every fifth element is a `support` stand-in; hide that kind.
    let filtered = s.project(&hiding(&[ElementKind::Support])).unwrap();
    assert_eq!(filtered.len(), 16);
    assert_eq!(filtered.hidden_count(), 4);
    assert!(
        filtered
            .items()
            .iter()
            .all(|p| s.element(&p.id).unwrap().kind == ElementKind::Plate)
    );
    assert!(filtered.shows(ElementKind::Plate) && !filtered.shows(ElementKind::Support));
    assert!(filtered.required_meshes().len() < full.required_meshes().len());

    // Stand-in simplified meshes exist on every third element.
    let simplified = s
        .project(&ViewSettings {
            hidden_kinds: KindFilter::SHOW_ALL,
            representation: Representation::Simplified,
        })
        .unwrap();
    assert_eq!(simplified.len(), 20);
    assert!(!simplified.all_exact());
    let inexact = simplified.items().iter().filter(|p| !p.exact).count();
    assert_eq!(inexact, 6, "ordinals 2, 5, 8, 11, 14, 17");
    assert!(
        simplified
            .items()
            .iter()
            .all(|p| p.representation == Representation::Simplified)
    );
    assert_ne!(full.required_meshes(), simplified.required_meshes());

    // Settings are not revision data: token, snapshot and engineering results are unchanged.
    assert_eq!(*full.token(), token);
    assert_eq!(*filtered.token(), token);
    assert_eq!(*simplified.token(), token);
    assert_eq!(s.token(), &token);
    assert_eq!(s.engineering_bounds(), bounds_before);
    for e in s.elements() {
        assert!(s.engineering_representation(&e.id).unwrap().exact);
        assert_eq!(s.engineering_representation(&e.id).unwrap(), &e.full);
    }
    let none = s.project(&hiding(&ElementKind::ALL)).unwrap();
    assert!(none.is_empty() && none.bounds().is_none());
    assert_eq!(none.required_mesh_bytes(), Some(0));
    assert_eq!(s.engineering_bounds(), bounds_before);
    assert!(full.required_mesh_bytes().unwrap() >= 392);
}

#[test]
fn projection_tables_map_instances_and_triangles_back_to_ids() {
    let s = snap(plate_scene(10, false).unwrap(), 1);
    let p = s.project(&hiding(&[ElementKind::Support])).unwrap();
    assert_eq!(p.len(), 8);
    let mut expected_first = 0;
    for (i, item) in p.items().iter().enumerate() {
        assert_eq!(item.instance_index as usize, i);
        assert_eq!(item.first_triangle, expected_first);
        expected_first += item.triangle_count;
        assert_eq!(p.instance_of(&item.id), Some(item.instance_index));
        assert_eq!(
            p.element_for_instance(item.instance_index).unwrap(),
            item.id
        );
        assert_eq!(
            p.element_for_triangle(item.first_triangle).unwrap(),
            item.id
        );
        assert_eq!(
            p.element_for_triangle(item.first_triangle + item.triangle_count - 1)
                .unwrap(),
            item.id
        );
    }
    assert_eq!(p.total_triangles(), expected_first);
    // Hidden elements (ordinals 4 and 9) have no slot.
    assert_eq!(p.instance_of(&id(4)), None);
    assert_eq!(code(p.element_for_instance(8)), ViewCode::ArgumentInvalid);
    assert_eq!(
        code(p.element_for_triangle(p.total_triangles())),
        ViewCode::ArgumentInvalid
    );
    assert_eq!(
        code(p.element_for_triangle(u32::MAX)),
        ViewCode::ArgumentInvalid
    );
    let empty = s
        .project(&hiding(&[ElementKind::Plate, ElementKind::Support]))
        .unwrap();
    assert_eq!(
        code(empty.element_for_triangle(0)),
        ViewCode::ArgumentInvalid
    );
}

#[test]
fn list_elements_pages_in_id_order() {
    let s = snap(plate_scene(10, false).unwrap(), 1);
    let first = s.list_elements(None, 4).unwrap();
    assert_eq!(first.entries.len(), 4);
    let cursor = first.next_cursor.unwrap();
    assert_eq!(cursor, id(3));
    let second = s.list_elements(Some(&cursor), 4).unwrap();
    assert_eq!(second.entries.first().unwrap().id, id(4));
    let last = s
        .list_elements(Some(&second.next_cursor.unwrap()), 4)
        .unwrap();
    assert_eq!(last.entries.len(), 2);
    assert!(last.next_cursor.is_none());
    let exact = s.list_elements(None, 10).unwrap();
    assert!(exact.next_cursor.is_none());
    let beyond = s.list_elements(Some(&id(50)), 4).unwrap();
    assert!(beyond.entries.is_empty() && beyond.next_cursor.is_none());
    assert_eq!(code(s.list_elements(None, 0)), ViewCode::ArgumentInvalid);
    assert_eq!(
        code(s.list_elements(None, 1_025)),
        ViewCode::ArgumentInvalid
    );
    assert!(s.list_elements(None, 1_024).is_ok());
}

#[test]
fn deltas_resend_a_mesh_only_when_the_shape_changed() {
    let mut scene = plate_scene(6, false).unwrap();
    let mut history = SnapshotHistory::new(4);
    let base = snap(scene.clone(), 1);
    history.push(base.clone());
    assert_eq!(history.len(), 1);

    // Nothing changed: an empty delta.
    let none = history.changes_since(base.token()).unwrap();
    assert!(none.changes.is_empty() && none.new_meshes.is_empty());
    assert_eq!(none.from, none.to);

    // Placement only: one changed entry, no mesh resent.
    apply_edit(&mut scene, SyntheticEdit::MoveFirst, false).unwrap();
    let moved = snap(scene.clone(), 2);
    history.push(moved.clone());
    let d = history.changes_since(base.token()).unwrap();
    assert_eq!(d.changes.len(), 1);
    assert_eq!(d.changes[0].kind, ChangeKind::Changed);
    assert!(d.changes[0].fields.placement && !d.changes[0].fields.shape);
    assert!(
        d.new_meshes.is_empty(),
        "placement-only edit resends no blob"
    );
    assert_eq!(d.to, *moved.token());
    assert_eq!(d.to_pin, pin(2));

    // Shape change and additions.
    apply_edit(&mut scene, SyntheticEdit::ResizeFirst, false).unwrap();
    apply_edit(&mut scene, SyntheticEdit::AddOne, false).unwrap();
    apply_edit(&mut scene, SyntheticEdit::RemoveLast, false).unwrap();
    apply_edit(&mut scene, SyntheticEdit::AddOne, false).unwrap();
    apply_edit(&mut scene, SyntheticEdit::AddOne, false).unwrap();
    let edited = snap(scene.clone(), 3);
    history.push(edited.clone());
    let d = history.changes_since(base.token()).unwrap();
    let kinds: Vec<_> = d.changes.iter().map(|c| (c.id, c.kind)).collect();
    assert_eq!(
        kinds,
        vec![
            (id(0), ChangeKind::Changed),
            (id(6), ChangeKind::Added),
            (id(7), ChangeKind::Added)
        ]
    );
    assert!(d.changes[0].fields.shape && d.changes[0].fields.placement);
    assert!(d.changes[1].entry.is_some() && d.changes[1].fields == Default::default());
    assert!(!d.new_meshes.is_empty());
    assert!(
        d.new_meshes
            .iter()
            .all(|m| edited.mesh(m).is_some() && base.mesh(m).is_none())
    );
    assert!(d.new_meshes.windows(2).all(|w| w[0] < w[1]));

    // Removal.
    let mut fewer = scene.clone();
    apply_edit(&mut fewer, SyntheticEdit::RemoveLast, false).unwrap();
    let removed = snap(fewer, 4);
    history.push(removed.clone());
    let d = history.changes_since(edited.token()).unwrap();
    assert_eq!(d.changes.len(), 1);
    assert_eq!(d.changes[0].kind, ChangeKind::Removed);
    assert_eq!(d.changes[0].id, id(7));
    assert!(d.changes[0].entry.is_none());
    assert!(d.new_meshes.is_empty());
    assert_eq!(history.latest().unwrap().token(), removed.token());
}

#[test]
fn a_token_the_history_cannot_answer_is_stale() {
    let mut history = SnapshotHistory::new(2);
    let empty_token = SnapshotToken::parse(&format!("vs1-{}", "0".repeat(64))).unwrap();
    assert!(history.is_empty());
    assert_eq!(
        code(history.changes_since(&empty_token)),
        ViewCode::StaleToken
    );
    let scene = plate_scene(3, false).unwrap();
    let (a, b, c) = (
        snap(scene.clone(), 1),
        snap(scene.clone(), 2),
        snap(scene, 3),
    );
    history.push(a.clone());
    history.push(a.clone());
    assert_eq!(history.len(), 1, "the latest token is not stored twice");
    history.push(b.clone());
    history.push(c);
    assert_eq!(history.len(), 2, "capacity evicts the oldest");
    assert_eq!(code(history.changes_since(a.token())), ViewCode::StaleToken);
    assert_eq!(
        code(history.changes_since(&empty_token)),
        ViewCode::StaleToken
    );
    assert!(history.changes_since(b.token()).unwrap().changes.is_empty());
    let mut one = SnapshotHistory::new(0);
    one.push(a);
    one.push(b);
    assert_eq!(one.len(), 1, "capacity is clamped to at least one");
    let mut many = SnapshotHistory::new(1_000);
    for g in 0..20 {
        many.push(snap(plate_scene(1, false).unwrap(), g));
    }
    assert_eq!(many.len(), 8, "capacity is clamped to the maximum");
}

#[test]
fn selection_resolves_ids_hints_and_survives_view_changes() {
    let s = snap(plate_scene(10, false).unwrap(), 1);
    let token = s.token().clone();
    let sel = Selection::from_ids(&s, &[id(3), id(1), id(3)]).unwrap();
    assert_eq!(sel.len(), 2);
    assert!(sel.contains(&id(1)) && !sel.contains(&id(2)));
    assert_eq!(sel.ids().copied().collect::<Vec<_>>(), vec![id(1), id(3)]);
    assert_eq!(sel.token(), &token);
    assert!(Selection::from_ids(&s, &[]).unwrap().is_empty());

    let p = s.project(&hiding(&[ElementKind::Support])).unwrap();
    assert_eq!(sel.instances(&p).unwrap(), vec![1, 3]);
    let hidden = Selection::from_ids(&s, &[id(4), id(0)]).unwrap();
    assert_eq!(
        hidden.instances(&p).unwrap(),
        vec![0],
        "hidden elements have no slot"
    );
    // Display hints resolve to ids through the projection and are then discarded.
    let by_instance = Selection::from_instances(&p, &[2, 2, 5]).unwrap();
    assert_eq!(
        by_instance.ids().copied().collect::<Vec<_>>(),
        vec![id(2), id(6)]
    );
    let by_triangle = Selection::from_triangles(&p, &[0, 11, 12, 95]).unwrap();
    assert_eq!(
        by_triangle.ids().copied().collect::<Vec<_>>(),
        vec![id(0), id(1), id(8)]
    );
    // Selecting changes no key.
    assert_eq!(s.token(), &token);

    // A newer snapshot: the selection is rebased, never silently reused.
    let mut scene = plate_scene(10, false).unwrap();
    apply_edit(&mut scene, SyntheticEdit::RemoveLast, false).unwrap();
    let newer = snap(scene, 2);
    let newer_view = newer.project(&ViewSettings::default()).unwrap();
    assert_eq!(code(sel.instances(&newer_view)), ViewCode::StaleToken);
    let tail = Selection::from_ids(&s, &[id(1), id(9)]).unwrap();
    let (rebased, dropped) = tail.rebased(&newer).unwrap();
    assert_eq!(dropped, 1);
    assert_eq!(rebased.ids().copied().collect::<Vec<_>>(), vec![id(1)]);
    assert_eq!(rebased.token(), newer.token());
    assert_eq!(rebased.instances(&newer_view).unwrap(), vec![1]);
}

#[test]
fn repeated_resizes_each_thicken_the_first_plate_by_two_millimetres() {
    let mut scene = plate_scene(3, false).unwrap();
    let mut previous = snap(scene.clone(), 1);
    // The first plate is 65 x 70 x 6; width and length stay, thickness grows by 2 each time.
    for (step, thickness) in [8.0, 10.0, 12.0].into_iter().enumerate() {
        apply_edit(&mut scene, SyntheticEdit::ResizeFirst, false).unwrap();
        let next = snap(scene.clone(), 2 + step as u64);
        let bounds = next.element_bounds(&id(0)).unwrap();
        assert_eq!(bounds.min, [0.0, 0.0, 0.0]);
        assert_eq!(bounds.max, [65.0, 70.0, thickness], "step {step}");
        let delta = beamcraft_engine::diff(&previous, &next);
        assert_eq!(delta.changes.len(), 1, "step {step} must be a shape change");
        assert!(delta.changes[0].fields.shape);
        assert_eq!(delta.new_meshes.len(), 1);
        previous = next;
    }
}

#[test]
fn rebasing_respects_the_destination_selection_cap() {
    let scene = plate_scene(4, false).unwrap();
    let roomy = snap(scene.clone(), 1);
    let tight_limits = Limits::new(usize::MAX, usize::MAX, u32::MAX, 1);
    let tight = ViewSnapshot::build(pin(2), scene, &tight_limits).unwrap();
    let two = Selection::from_ids(&roomy, &[id(0), id(1)]).unwrap();
    assert_eq!(
        Selection::from_ids(&tight, &[id(0), id(1)])
            .unwrap_err()
            .code(),
        ViewCode::SelectionLimit
    );
    assert_eq!(code(two.rebased(&tight)), ViewCode::SelectionLimit);
    // Survivors at the cap are accepted; dropped ids do not count against it.
    let one = Selection::from_ids(&roomy, &[id(0)]).unwrap();
    let (kept, dropped) = one.rebased(&tight).unwrap();
    assert_eq!((kept.len(), dropped), (1, 0));
    let mut smaller = plate_scene(4, false).unwrap();
    smaller.retain(|e| e.id() != id(1));
    let smaller = ViewSnapshot::build(pin(3), smaller, &tight_limits).unwrap();
    let (kept, dropped) = two.rebased(&smaller).unwrap();
    assert_eq!((kept.len(), dropped), (1, 1));
}

#[test]
fn every_closed_error_has_a_negative_test() {
    let scene = plate_scene(4, false).unwrap();

    // VIEW_ELEMENT_LIMIT: above the hard maximum, before any other work.
    let one = scene.first().unwrap().clone();
    let too_many = vec![one; 65_537];
    assert_eq!(
        code(ViewSnapshot::build(pin(1), too_many, &Limits::default())),
        ViewCode::ElementLimit
    );
    // A lowered limit is exact: at the cap is accepted, one above is rejected.
    let limits = Limits::new(4, usize::MAX, u32::MAX, usize::MAX);
    assert!(ViewSnapshot::build(pin(1), scene.clone(), &limits).is_ok());
    assert_eq!(
        code(ViewSnapshot::build(
            pin(1),
            plate_scene(5, false).unwrap(),
            &limits
        )),
        ViewCode::ElementLimit
    );

    // VIEW_DUPLICATE_ELEMENT.
    let mut dup = scene.clone();
    dup.push(scene.first().unwrap().clone());
    assert_eq!(
        code(ViewSnapshot::build(pin(1), dup, &Limits::default())),
        ViewCode::DuplicateElement
    );

    // GEO_BUFFER_LIMIT (mesh bytes): 4 distinct 392-byte blobs; 1,568 fits, 1,567 does not.
    let bytes_ok = Limits::new(usize::MAX, 4 * 392, u32::MAX, usize::MAX);
    let s = ViewSnapshot::build(pin(1), scene.clone(), &bytes_ok).unwrap();
    assert_eq!(s.meshes().len(), 4);
    let bytes_low = Limits::new(usize::MAX, 4 * 392 - 1, u32::MAX, usize::MAX);
    assert_eq!(
        code(ViewSnapshot::build(pin(1), scene.clone(), &bytes_low)),
        ViewCode::BufferLimit
    );

    // GEO_BUFFER_LIMIT (projected triangles): 4 plates draw 48 triangles.
    let tri_ok = Limits::new(usize::MAX, usize::MAX, 48, usize::MAX);
    let s = ViewSnapshot::build(pin(1), scene.clone(), &tri_ok).unwrap();
    assert_eq!(
        s.project(&ViewSettings::default())
            .unwrap()
            .total_triangles(),
        48
    );
    let tri_low = Limits::new(usize::MAX, usize::MAX, 47, usize::MAX);
    let s = ViewSnapshot::build(pin(1), scene.clone(), &tri_low).unwrap();
    assert_eq!(
        code(s.project(&ViewSettings::default())),
        ViewCode::BufferLimit
    );

    // VIEW_SELECTION_LIMIT: at the cap accepted, above rejected, before any lookup.
    let sel_limits = Limits::new(usize::MAX, usize::MAX, u32::MAX, 3);
    let s = ViewSnapshot::build(pin(1), scene.clone(), &sel_limits).unwrap();
    assert!(Selection::from_ids(&s, &[id(0), id(1), id(2)]).is_ok());
    assert_eq!(
        code(Selection::from_ids(&s, &[id(0), id(1), id(2), id(3)])),
        ViewCode::SelectionLimit
    );
    let p = s.project(&ViewSettings::default()).unwrap();
    assert_eq!(
        code(Selection::from_instances(&p, &[0, 0, 0, 0])),
        ViewCode::SelectionLimit
    );
    assert_eq!(
        code(Selection::from_triangles(&p, &[0, 0, 0, 0])),
        ViewCode::SelectionLimit
    );

    // VIEW_UNKNOWN_ELEMENT.
    let s = snap(scene, 1);
    assert_eq!(
        code(Selection::from_ids(&s, &[id(77)])),
        ViewCode::UnknownElement
    );
    assert_eq!(
        code(s.engineering_representation(&id(77))),
        ViewCode::UnknownElement
    );

    // VIEW_ID_INVALID and VIEW_ARGUMENT_INVALID (token, setting).
    assert_eq!(
        code(ElementId::parse("not-an-id").map_err(ViewError::from)),
        ViewCode::IdInvalid
    );
    for bad in [
        "".to_owned(),
        "vs1-".to_owned(),
        format!("vs2-{}", "0".repeat(64)),
        format!("vs1-{}", "0".repeat(63)),
        format!("vs1-{}", "0".repeat(65)),
        format!("vs1-{}", "A".repeat(64)),
        format!("vs1-{}", "g".repeat(64)),
    ] {
        assert_eq!(
            code(SnapshotToken::parse(&bad)),
            ViewCode::ArgumentInvalid,
            "{bad:?}"
        );
    }
    assert_eq!(
        code(KindFilter::parse_list("weld")),
        ViewCode::ArgumentInvalid
    );
    assert!(Representation::parse("coarse").is_none());
    assert_eq!(
        Representation::parse("simplified"),
        Some(Representation::Simplified)
    );
    assert_eq!(code(plate_scene(0, false)), ViewCode::ArgumentInvalid);
    assert_eq!(code(plate_scene(65_537, false)), ViewCode::ElementLimit);

    // GEO_STALE_TOKEN is covered by `a_token_the_history_cannot_answer_is_stale`; GEO_BLOB_INVALID,
    // GEO_TOPOLOGY_LIMIT and the blob-level GEO_BUFFER_LIMIT by the unit tests of `query::mesh`.
    // GEO_NUMERIC_UNCERTAIN is a defensive finite check that validated kernel inputs cannot
    // reach inside `ViewSnapshot::build`; its mapping is covered by the `error` unit tests and
    // the same code is returned, reachably, by the synthetic edit below.
    let mut empty: Vec<SceneElement> = Vec::new();
    for edit in [
        SyntheticEdit::MoveFirst,
        SyntheticEdit::ResizeFirst,
        SyntheticEdit::RemoveLast,
    ] {
        assert_eq!(
            code(apply_edit(&mut empty, edit, false)),
            ViewCode::ArgumentInvalid
        );
    }
    let mut far = vec![SceneElement::new(
        id(1),
        ElementKind::Plate,
        at(1.0e9, 0.0, 0.0),
        plate(10.0, 10.0, 1.0),
    )];
    assert_eq!(
        code(apply_edit(&mut far, SyntheticEdit::MoveFirst, false)),
        ViewCode::NumericUncertain
    );
    assert_eq!(
        SyntheticEdit::parse("move-first"),
        Some(SyntheticEdit::MoveFirst)
    );
    assert_eq!(SyntheticEdit::parse("nope"), None);
}

#[test]
fn the_hard_element_cap_is_accepted_exactly() {
    let one = plate(10.0, 10.0, 1.0);
    let scene: Vec<SceneElement> = (0..65_536u64)
        .map(|n| SceneElement::new(id(n), ElementKind::Plate, at(0.0, 0.0, 0.0), one.clone()))
        .collect();
    let s = snap(scene, 1);
    assert_eq!(s.len(), 65_536);
    assert_eq!(
        s.meshes().len(),
        1,
        "65,536 identical plates share one blob"
    );
    let p = s.project(&ViewSettings::default()).unwrap();
    assert_eq!(p.total_triangles(), 65_536 * 12);
    assert_eq!(
        p.element_for_triangle(p.total_triangles() - 1).unwrap(),
        id(65_535)
    );
}
