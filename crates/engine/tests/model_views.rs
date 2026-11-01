// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim
// Contributor-original synthetic frames and views, MPL-2.0.

//! Pure model-view admission and construction regression coverage.

#![allow(clippy::unwrap_used)]
use beamcraft_engine::{
    model_view::{self, ModelView, Projection},
    work_plane::WorkPlane,
};

fn view() -> ModelView {
    ModelView {
        id: beamcraft_model::id::Uuid7::parse("019a7bf0-0000-7000-8000-000000000123").unwrap(),
        name: "Plan".into(),
        plane: WorkPlane::parallel("xy", 3000.).unwrap(),
        projection: Projection::Orthographic,
        span_mm: 10000.,
        depth: Some([100., 200.]),
        render_mode: beamcraft_engine::RenderMode::Shaded,
        grids_visible: true,
        folder: "Plans/Levels".into(),
        favorite: false,
        temporary: false,
        hidden_kinds: beamcraft_engine::KindFilter::SHOW_ALL,
        work_area: None,
        rotation_rad: [0.; 2],
        color: None,
        opacity: 1.,
    }
}
#[test]
fn planes_preserve_3d_coordinates_and_refuse_degenerate_points() {
    let plane =
        model_view::three_points([10., 20., 30.], [11., 20., 30.], [10., 20., 33.]).unwrap();
    assert_eq!(plane.normal(), [0., -1., 0.]);
    assert_eq!(plane.origin(), [10., 20., 30.]);
    assert!(model_view::three_points([0.; 3], [1., 0., 0.], [2., 0., 0.]).is_err());
    assert!(model_view::two_points([0.; 3], [0., 0., 10.]).is_err());
    assert!(model_view::two_points([0.; 3], [f64::NAN, 0., 0.]).is_err());
}
#[test]
fn depth_work_area_and_names_are_checked_without_model_changes() {
    let mut v = view();
    assert!(v.contains([0., 0., 2800.]));
    assert!(v.contains([0., 0., 3100.]));
    assert!(!v.contains([0., 0., 3101.]));
    assert!(!v.contains([0., 0., 2799.]));
    v.work_area = Some(beamcraft_engine::Aabb {
        min: [0., 0., 0.],
        max: [100., 100., 10000.],
    });
    assert!(!v.contains([-1., 0., 3000.]));
    v.depth = Some([f64::INFINITY, 0.]);
    assert!(v.validate().is_err());
    v.depth = None;
    v.name = " bad".into();
    assert!(v.validate().is_err());
    v.name = "Plan".into();
    v.folder = "a/../b".into();
    assert!(v.validate().is_err());
    let a = view();
    let mut b = a.clone();
    b.name = "Other".into();
    assert!(model_view::validate_collection(&[a, b]).is_err());
}

#[test]
fn visible_pick_obeys_depth_and_kind_filters_and_camera_rotation_keeps_input_plane() {
    use beamcraft_engine::{Limits, ViewPin, ViewSnapshot, query::pick::SelectionFilter};
    let pin = ViewPin::new(
        beamcraft_model::digest::Sha256::digest(b"view-pick").unwrap(),
        1,
    );
    let snapshot = ViewSnapshot::build(
        pin,
        beamcraft_engine::synthetic::plate_scene(1, false).unwrap(),
        &Limits::default(),
    )
    .unwrap();
    let bounds = snapshot.engineering_bounds().unwrap();
    let [ax, ay, _] = bounds.min;
    let [bx, by, bz] = bounds.max;
    let ray = [(ax + bx) / 2., (ay + by) / 2., bz + 1000.];
    let mut v = view();
    v.depth = None;
    assert!(
        model_view::pick_visible(&snapshot, &v, ray, [0., 0., -1.], SelectionFilter::ALL)
            .unwrap()
            .is_some()
    );
    assert!(
        model_view::surface_plane_in_view(&snapshot, Some(&v), ray, [0., 0., -1.], None).is_ok()
    );
    v.hidden_kinds =
        beamcraft_engine::KindFilter::SHOW_ALL.hiding(beamcraft_engine::ElementKind::Plate);
    assert!(
        model_view::pick_visible(&snapshot, &v, ray, [0., 0., -1.], SelectionFilter::ALL)
            .unwrap()
            .is_none()
    );
    assert!(
        model_view::surface_plane_in_view(&snapshot, Some(&v), ray, [0., 0., -1.], None).is_err()
    );
    v.hidden_kinds = beamcraft_engine::KindFilter::SHOW_ALL;
    v.depth = Some([1., 1.]);
    assert!(
        model_view::pick_visible(&snapshot, &v, ray, [0., 0., -1.], SelectionFilter::ALL)
            .unwrap()
            .is_none()
    );
    let plane = v.plane;
    v.rotation_rad = [0.5, 0.7];
    let camera_plane = v.camera_plane().unwrap();
    assert_eq!(v.plane, plane);
    assert_ne!(camera_plane.normal(), plane.normal());
    let section = model_view::two_points_in_plane(
        [0.; 3],
        [100., 0., 0.],
        WorkPlane::parallel("xz", 0.).unwrap(),
    )
    .unwrap();
    assert_eq!(section.y(), [0., -1., 0.]);
}
