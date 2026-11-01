// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim
// Contributor-original synthetic model, camera and pixel expectations, MPL-2.0.

//! Named-view restore, orthographic picking and real depth clipping through the native renderer.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]
use beamcraft_engine::{
    Limits, ViewPin, ViewSnapshot,
    model_view::{ModelView, Projection},
    work_plane::WorkPlane,
};
use beamcraft_model::{digest::Sha256, id::Uuid7};
use beamcraft_ui::{
    camera::Camera,
    render::{Gpu, Renderer, Uploaded, wait},
    scene::Scene,
    stats::Stats,
};

fn definition() -> ModelView {
    ModelView {
        id: Uuid7::parse("019a7bf0-0000-7000-8000-000000000123").unwrap(),
        name: "Plan".into(),
        plane: WorkPlane::default(),
        projection: Projection::Orthographic,
        span_mm: 2000.,
        depth: None,
        render_mode: beamcraft_engine::RenderMode::Shaded,
        grids_visible: false,
        folder: String::new(),
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
fn orthographic_rays_are_parallel_and_camera_round_trips_at_arbitrary_orientations() {
    let mut v = definition();
    v.rotation_rad = [0.3, 0.8];
    let mut camera = Camera::from_model_view(&v, [0.; 3]).unwrap();
    camera.navigate([0.2, -0.17], [10., -20.], 0.2).unwrap();
    let (plane, span) = camera.model_frame([0.; 3]).unwrap();
    v.plane = plane;
    v.span_mm = span;
    v.rotation_rad = [0.; 2];
    let restored = Camera::from_model_view(&v, [0.; 3])
        .unwrap()
        .view(1.5)
        .unwrap();
    let original = camera.view(1.5).unwrap();
    for p in [[0.; 3], [100., 200., 300.], [-100., 10., -50.]] {
        for (a, b) in original.project(p).into_iter().zip(restored.project(p)) {
            assert!((a - b).abs() < 1e-5);
        }
    }
    let ray_a = beamcraft_ui::edit::ray_through(&restored, [-0.4, 0.2], [0.; 3]).unwrap();
    let ray_b = beamcraft_ui::edit::ray_through(&restored, [0.7, -0.6], [0.; 3]).unwrap();
    assert_eq!(ray_a.direction, ray_b.direction);
    assert_ne!(ray_a.origin, ray_b.origin);
    let hit = v.plane.ray_point(ray_a.origin, ray_a.direction).unwrap();
    let [x, y, _] = restored.project(hit.map(|n| n as f32));
    assert!((x + 0.4).abs() < 1e-5 && (y - 0.2).abs() < 1e-5);
    for span in [1., 20_000_000.] {
        v.span_mm = span;
        let camera = Camera::from_model_view(&v, [0.; 3]).unwrap();
        assert!((camera.model_frame([0.; 3]).unwrap().1 - span).abs() < 0.01);
    }
}

#[test]
fn property_form_round_trips_through_the_real_cli_and_restores_a_working_camera() {
    use beamcraft_ui::{
        command,
        model_views::{Form, parse_reply},
    };
    use std::{path::Path, process::Command, time::Duration};
    let cli = Path::new(env!("CARGO_BIN_EXE_beamcraft-ui"))
        .parent()
        .unwrap()
        .join(format!("beamcraft{}", std::env::consts::EXE_SUFFIX));
    assert!(
        cli.is_file(),
        "build beamcraft-cli before the view integration tests"
    );
    let store =
        Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("view-form-{}", std::process::id()));
    std::fs::create_dir(&store).unwrap();
    struct Cleanup(std::path::PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    let _cleanup = Cleanup(store.clone());
    let output = Command::new(&cli)
        .args([
            "create-plate",
            "--store",
            store.to_str().unwrap(),
            "--width",
            "1000",
            "--length",
            "500",
            "--thickness",
            "10",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    let form = Form {
        name: "UI section".into(),
        rotation: ["20".into(), "30".into()],
        span: "5000".into(),
        clipped: true,
        depth: ["100".into(), "200".into()],
        folder: "Sections/Frame".into(),
        ..Default::default()
    };
    let mut args = vec![
        "create-model-view".into(),
        "--store".into(),
        store.to_str().unwrap().into(),
        "--three-points".into(),
        "0,0,0,1000,0,0,0,0,1000".into(),
    ];
    args.extend(form.property_args());
    let output = command::run(&cli, &args, Duration::from_secs(30), &Stats::default()).unwrap();
    let collection = parse_reply(output.code, &output.stdout).unwrap();
    let view = &collection.views[0];
    assert_eq!(view.name, form.name);
    assert_eq!(view.folder, form.folder);
    assert_eq!(view.depth, Some([100., 200.]));
    let camera = Camera::from_model_view(view, [0.; 3]).unwrap();
    assert_eq!(camera.view(1.5).unwrap().projection[0], 1.);
    assert_eq!(view.plane.normal(), [0., -1., 0.]);
    let output = command::run(
        &cli,
        &[
            "model-views".into(),
            "--store".into(),
            store.to_str().unwrap().into(),
        ],
        Duration::from_secs(30),
        &Stats::default(),
    )
    .unwrap();
    assert_eq!(
        parse_reply(output.code, &output.stdout).unwrap().views,
        collection.views
    );
}
#[test]
fn native_gpu_clips_the_depth_slab_and_work_area_without_changing_the_snapshot() {
    let gpu = match pollster::block_on(Gpu::headless()) {
        Ok(gpu) => gpu,
        Err(beamcraft_ui::Error::NoAdapter) => {
            eprintln!("SKIP named-view GPU: no native adapter");
            return;
        }
        Err(error) => panic!("{error}"),
    };
    let pin = ViewPin::new(Sha256::digest(b"model-view-pixel-test").unwrap(), 1);
    let snapshot = ViewSnapshot::build(
        pin,
        beamcraft_engine::synthetic::plate_scene(1, false).unwrap(),
        &Limits::default(),
    )
    .unwrap();
    let token = snapshot.token().clone();
    let scene = Scene::from_snapshot(&snapshot, beamcraft_engine::Representation::Full).unwrap();
    let stats = Stats::default();
    let renderer = Renderer::new(&gpu.adapter, &gpu.device, 96, 96, &stats).unwrap();
    let uploaded = Uploaded::new(&gpu.device, &gpu.queue, &scene, &stats).unwrap();
    let bounds = snapshot.engineering_bounds().unwrap();
    let [ax, ay, az] = bounds.min;
    let [bx, by, bz] = bounds.max;
    let center = [(ax + bx) / 2., (ay + by) / 2., (az + bz) / 2.];
    let mut v = definition();
    v.plane = WorkPlane::new(center, [1., 0., 0.], [0., 1., 0.]).unwrap();
    v.span_mm = (bx - ax).max(by - ay) * 1.5;
    let render = |v: &ModelView| {
        let camera = Camera::from_model_view(v, scene.origin_mm)
            .unwrap()
            .view(1.)
            .unwrap();
        let scope = gpu.device.push_error_scope(wgpu::ErrorFilter::Validation);
        let index = renderer.draw(
            &gpu.device,
            &gpu.queue,
            &uploaded,
            camera,
            &stats,
            beamcraft_engine::RenderMode::Shaded,
        );
        wait(&gpu.device, index).unwrap();
        let (pixels, row) = renderer.readback(&gpu.device, &gpu.queue).unwrap();
        assert!(pollster::block_on(scope.pop()).is_none());
        let pixel = |x: usize, y: usize| {
            pixels[y * row as usize + x * 4..y * row as usize + x * 4 + 4].to_vec()
        };
        (pixel(48, 48), pixel(0, 0))
    };
    let (center_pixel, background) = render(&v);
    assert_ne!(center_pixel, background);
    v.plane = WorkPlane::new(
        [center[0], center[1], bz + 1000.],
        [1., 0., 0.],
        [0., 1., 0.],
    )
    .unwrap();
    v.depth = Some([10., 10.]);
    let (center_pixel, background) = render(&v);
    assert_eq!(center_pixel, background);
    v.depth = None;
    v.plane = WorkPlane::new(center, [1., 0., 0.], [0., 1., 0.]).unwrap();
    v.work_area = Some(beamcraft_engine::Aabb {
        min: [bx + 100., by + 100., bz + 100.],
        max: [bx + 200., by + 200., bz + 200.],
    });
    let (center_pixel, background) = render(&v);
    assert_eq!(center_pixel, background);
    assert_eq!(snapshot.token(), &token);
}
