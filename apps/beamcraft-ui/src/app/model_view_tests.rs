// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim
// Contributor-original synthetic scenes, views and UI interaction evidence, MPL-2.0.

//! Windowless controller and real GPU layout acceptance for the view browser.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]
use super::{
    tests::{deliver, headless, refreshed},
    workbench::RibbonTab,
};
use beamcraft_engine::{
    model_view::{ModelView, Projection},
    work_plane::WorkPlane,
};

struct InputFrame {
    labels: Vec<(String, egui::Pos2)>,
    viewport: egui::Rect,
}
fn input_frame(
    app: &mut super::App,
    ctx: &egui::Context,
    mut events: Vec<egui::Event>,
    modifiers: egui::Modifiers,
) -> InputFrame {
    events.insert(0, egui::Event::ModifiersChanged(modifiers));
    let mut native = eframe::Frame::_new_kittest();
    let output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1400., 900.),
            )),
            events,
            ..Default::default()
        },
        |ui| eframe::App::ui(app, ui, &mut native),
    );
    let labels = output
        .shapes
        .iter()
        .filter_map(|s| match &s.shape {
            egui::Shape::Text(text) => Some((
                text.galley.text().to_owned(),
                text.pos + text.galley.size() / 2.,
            )),
            _ => None,
        })
        .collect();
    let viewport = output.shapes.iter().find_map(|s| match &s.shape {
        egui::Shape::Rect(rect) if rect.fill_texture_id() == app.texture => Some(rect.rect),
        egui::Shape::Mesh(mesh) if mesh.texture_id == app.texture => Some(
            mesh.vertices
                .iter()
                .fold(egui::Rect::NOTHING, |mut rect, v| {
                    rect.extend_with(v.pos);
                    rect
                }),
        ),
        _ => None,
    });
    output.drop_without_applying_deltas();
    InputFrame {
        labels,
        viewport: viewport.expect("viewport image must be rendered"),
    }
}
fn click(app: &mut super::App, ctx: &egui::Context, pos: egui::Pos2) -> InputFrame {
    input_frame(
        app,
        ctx,
        vec![
            egui::Event::PointerMoved(pos),
            egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
        ],
        egui::Modifiers::NONE,
    );
    input_frame(
        app,
        ctx,
        vec![egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        }],
        egui::Modifiers::NONE,
    )
}
fn invoke_view_menu(app: &mut super::App, ctx: &egui::Context, name: &str) {
    let frame = input_frame(app, ctx, vec![], egui::Modifiers::NONE);
    let button = frame
        .labels
        .iter()
        .find(|(text, _)| text == "Create view")
        .unwrap()
        .1;
    click(app, ctx, button);
    let frame = input_frame(app, ctx, vec![], egui::Modifiers::NONE);
    let item = frame
        .labels
        .iter()
        .find(|(text, _)| text == name)
        .unwrap_or_else(|| panic!("missing {name}: {:?}", frame.labels))
        .1;
    click(app, ctx, item);
    input_frame(app, ctx, vec![], egui::Modifiers::NONE);
}
fn click_world(app: &mut super::App, ctx: &egui::Context, point: [f64; 3]) {
    let frame = input_frame(app, ctx, vec![], egui::Modifiers::NONE);
    let origin = app.loaded.as_ref().unwrap().summary.origin_mm;
    let camera = app.camera.view(frame.viewport.aspect_ratio()).unwrap();
    let local = std::array::from_fn(|i| (point[i] - origin[i]) as f32);
    let [x, y, z] = camera.project(local);
    assert!((0.0..=1.0).contains(&z));
    let pos = egui::pos2(
        frame.viewport.left() + (x + 1.) * frame.viewport.width() / 2.,
        frame.viewport.top() + (1. - y) * frame.viewport.height() / 2.,
    );
    assert!(frame.viewport.contains(pos));
    click(app, ctx, pos);
}
fn key_frame(
    app: &mut super::App,
    ctx: &egui::Context,
    key: egui::Key,
    modifiers: egui::Modifiers,
) {
    input_frame(
        app,
        ctx,
        vec![egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
        }],
        modifiers,
    );
    input_frame(
        app,
        ctx,
        vec![egui::Event::Key {
            key,
            physical_key: None,
            pressed: false,
            repeat: false,
            modifiers,
        }],
        egui::Modifiers::NONE,
    );
}

#[test]
fn ribbon_point_views_pick_real_viewport_geometry_without_coordinate_entry() {
    let mut app =
        headless().expect("native/software adapter required for point-view input acceptance");
    let cli = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join(format!("beamcraft{}", std::env::consts::EXE_SUFFIX));
    assert!(
        cli.is_file(),
        "build beamcraft-cli before view input acceptance"
    );
    let root = std::env::temp_dir().join(format!(
        "bc-view-input-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&root).unwrap();
    struct Cleanup(std::path::PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    let _cleanup = Cleanup(root.clone());
    let run = |command: &str| {
        let output = std::process::Command::new(&cli)
            .args([command, "--store", root.to_str().unwrap()])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stdout)
        );
        beamcraft_json::decode(&output.stdout, &beamcraft_json::Limits::DEFAULT).unwrap()
    };
    let output = std::process::Command::new(&cli)
        .args([
            "create-plate",
            "--store",
            root.to_str().unwrap(),
            "--width",
            "1000",
            "--length",
            "500",
            "--thickness",
            "10",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let revision = run("open-revision");
    app.source.store = root.clone();
    app.source.cli = cli.clone();
    app.views.loaded = true;
    let reloaded = crate::source::reload(
        &cli,
        &app.source.store,
        &crate::source::Known::default(),
        &app.stats,
    );
    deliver(&mut app, reloaded);
    let token = app.loaded.as_ref().unwrap().snapshot.token().clone();
    let origin = app.loaded.as_ref().unwrap().summary.origin_mm;
    let mut base = view("Input frame", 20);
    base.plane = WorkPlane::new([500., 250., 0.], [1., 0., 0.], [0., 1., 0.]).unwrap();
    base.span_mm = 1800.;
    app.camera = crate::camera::Camera::from_model_view(&base, origin).unwrap();
    app.ribbon_tab = RibbonTab::View;
    let ctx = egui::Context::default();
    crate::theme::install(&ctx);
    assert!(super::inter(&ctx));
    for _ in 0..2 {
        input_frame(&mut app, &ctx, vec![], egui::Modifiers::NONE);
    }
    invoke_view_menu(&mut app, &ctx, "Two points");
    assert!(
        app.views.picking,
        "ribbon command must start picking immediately"
    );
    let frame = input_frame(&mut app, &ctx, vec![], egui::Modifiers::NONE);
    assert!(
        frame
            .labels
            .iter()
            .any(|(text, _)| text.contains("Pick the first point"))
    );
    assert!(
        !frame
            .labels
            .iter()
            .any(|(text, _)| text.contains("X,Y,Z") || text.contains("unit X/Y"))
    );
    click_world(&mut app, &ctx, [0., 0., 10.]);
    assert_eq!(app.views.points.len(), 1);
    assert!(app.views.picking);
    assert_eq!(app.views.points[0], [0., 0., 10.]);
    click_world(&mut app, &ctx, [0., 0., 10.]);
    assert_eq!(
        app.views.points.len(),
        1,
        "a coincident second point stays in the picking tool"
    );
    assert!(app.views.job.is_none());
    click_world(&mut app, &ctx, [1000., 0., 10.]);
    let wait = |app: &mut super::App| {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
        while app.views.job.is_some() && std::time::Instant::now() < deadline {
            app.poll_views();
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert!(app.views.job.is_none());
        assert!(
            app.views.note.as_ref().is_some_and(Result::is_ok),
            "{:?}",
            app.views.note
        );
    };
    wait(&mut app);
    assert_eq!(app.views.collection.len(), 1);
    let section = app.views.collection[0].clone();
    assert_eq!(section.plane.origin(), [0., 0., 10.]);
    assert_eq!(section.plane.x(), [1., 0., 0.]);
    app.camera.navigate([0.3, 0.2], [0.; 2], 0.).unwrap();
    let spatial = app.camera.view(1.).unwrap().back;
    key_frame(&mut app, &ctx, egui::Key::P, egui::Modifiers::CTRL);
    let [nx, ny, nz] = section.plane.normal();
    assert_eq!(
        app.camera.view(1.).unwrap().back,
        [nx as f32, ny as f32, nz as f32, 0.]
    );
    assert_eq!(app.tools.plane, Some(section.plane));
    key_frame(&mut app, &ctx, egui::Key::P, egui::Modifiers::CTRL);
    assert_eq!(app.camera.view(1.).unwrap().back, spatial);
    app.activate_view(None);
    base.rotation_rad = [0.2, 0.4];
    app.camera = crate::camera::Camera::from_model_view(&base, origin).unwrap();
    invoke_view_menu(&mut app, &ctx, "Three points");
    assert!(app.views.picking);
    let before_plane = app.tools.plane;
    key_frame(&mut app, &ctx, egui::Key::P, egui::Modifiers::CTRL);
    assert!(app.views.picking && app.views.points.is_empty());
    assert_eq!(app.tools.plane, before_plane);
    assert_eq!(app.camera.view(1.).unwrap().back, [0., 0., 1., 0.]);
    key_frame(&mut app, &ctx, egui::Key::P, egui::Modifiers::CTRL);

    for point in [[0., 0., 10.], [1000., 0., 10.]] {
        click_world(&mut app, &ctx, point);
    }
    assert_eq!(app.views.points.len(), 2);
    click_world(&mut app, &ctx, [0., 0., 10.]);
    assert_eq!(
        app.views.points.len(),
        2,
        "a collinear third point can be re-picked"
    );
    click_world(&mut app, &ctx, [1000., 500., 10.]);
    wait(&mut app);
    assert_eq!(app.views.collection.len(), 2);
    assert_eq!(app.views.collection[1].plane.normal(), [0., 0., 1.]);
    app.activate_view(None);
    app.camera = crate::camera::Camera::from_model_view(&base, origin).unwrap();
    invoke_view_menu(&mut app, &ctx, "Two points");
    click_world(&mut app, &ctx, [0., 0., 10.]);
    key_frame(&mut app, &ctx, egui::Key::Escape, egui::Modifiers::NONE);
    assert!(!app.views.picking && app.views.points.is_empty());
    assert_eq!(app.views.collection.len(), 2);
    assert_eq!(app.loaded.as_ref().unwrap().snapshot.token(), &token);
    assert_eq!(run("open-revision"), revision);
}

fn view(name: &str, n: u8) -> ModelView {
    ModelView {
        id: beamcraft_model::id::Uuid7::parse(&format!("019a7bf0-0000-7000-8000-{n:012x}"))
            .unwrap(),
        name: name.into(),
        plane: WorkPlane::default(),
        projection: Projection::Orthographic,
        span_mm: 10000.,
        depth: None,
        render_mode: beamcraft_engine::RenderMode::Shaded,
        grids_visible: true,
        folder: "Plans".into(),
        favorite: n == 1,
        temporary: false,
        hidden_kinds: beamcraft_engine::KindFilter::SHOW_ALL,
        work_area: None,
        rotation_rad: [0.; 2],
        color: None,
        opacity: 1.,
    }
}

#[test]
fn named_view_controller_restores_planes_cameras_and_independent_windows() {
    let mut app = headless().expect("native/software adapter required for view acceptance");
    deliver(&mut app, refreshed(2));
    app.views.loaded = true;
    let token = app.loaded.as_ref().unwrap().snapshot.token().clone();
    let a = view("Ground plan", 1);
    let mut b = view("Section", 2);
    b.plane = WorkPlane::parallel("xz", 500.).unwrap();
    b.depth = Some([100., 200.]);
    app.views.collection = vec![a.clone(), b.clone()];
    app.activate_view(Some(&a.id.to_string()));
    let plane = app.tools.plane.unwrap();
    assert_eq!(plane, a.plane);
    app.camera.navigate([0.; 2], [100., 200.], 0.2).unwrap();
    let saved = app.camera.view(1.5).unwrap().eye;
    app.activate_view(Some(&b.id.to_string()));
    assert_eq!(app.tools.plane, Some(b.plane));
    app.activate_view(Some(&a.id.to_string()));
    assert_eq!(app.camera.view(1.5).unwrap().eye, saved);
    app.open_view_window(&b.id.to_string());
    assert_eq!(app.views.windows.len(), 1);
    let ctx = egui::Context::default();
    super::inter(&ctx);
    for _ in 0..2 {
        let output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1400., 900.),
                )),
                ..Default::default()
            },
            |ui| {
                app.project_browser(ui);
                app.show_view_windows(ui.ctx());
            },
        );
        output.drop_without_applying_deltas();
    }
    assert_eq!(app.loaded.as_ref().unwrap().snapshot.token(), &token);
    app.activate_view(None);
    assert!(app.tools.plane.is_none());
    app.views.thumbnails = true;
    let output = ctx.run_ui(egui::RawInput::default(), |ui| app.project_browser(ui));
    output.drop_without_applying_deltas();
    assert!(!app.views.previews.is_empty());
    // A pending create must activate its result even when another temporary view is retained.
    let mut temporary = view("Temporary", 3);
    temporary.temporary = true;
    let new = view("New result", 4);
    app.views.collection.push(temporary.clone());
    let (tx, rx) = std::sync::mpsc::channel();
    app.views.job = Some(rx);
    app.views.activate_result = true;
    app.views.activation_id = None;
    tx.send(Ok(crate::model_views::Collection {
        views: vec![a, b, new.clone()],
        folders: vec![],
    }))
    .unwrap();
    app.poll_views();
    assert_eq!(app.views.active, Some(new.id.to_string()));
    assert!(app.views.collection.iter().any(|v| v.id == temporary.id));
    for n in 5..=12 {
        let next = view(&format!("Window {n}"), n);
        let id = next.id.to_string();
        app.views.collection.push(next);
        app.activate_view(Some(&id));
    }
    assert_eq!(app.views.open.len(), 9);
    assert!(app.views.note.as_ref().unwrap().is_err());
}

#[test]
fn view_browser_properties_and_tabs_render_in_both_themes() {
    let mut app = headless().expect("native/software adapter required for view layout acceptance");
    deliver(&mut app, refreshed(2));
    app.views.loaded = true;
    let mut a = view("Ground plan", 1);
    a.span_mm = 300.;
    let bounds = app
        .loaded
        .as_ref()
        .unwrap()
        .snapshot
        .engineering_bounds()
        .unwrap();
    let [ax, ay, az] = bounds.min;
    let [bx, by, bz] = bounds.max;
    a.plane = WorkPlane::new(
        [(ax + bx) / 2., (ay + by) / 2., (az + bz) / 2.],
        [1., 0., 0.],
        [0., 1., 0.],
    )
    .unwrap();
    let mut b = view("Section A", 2);
    b.folder = "Sections".into();
    app.views.collection = vec![a.clone(), b.clone()];
    app.views.folders = vec!["Plans".into(), "Sections".into()];
    app.activate_view(Some(&a.id.to_string()));
    app.views.editing = true;
    app.ribbon_tab = RibbonTab::View;
    let ctx = egui::Context::default();
    crate::theme::install(&ctx);
    assert!(super::inter(&ctx));
    for (theme, name) in [
        (egui::ThemePreference::Light, "light"),
        (egui::ThemePreference::Dark, "dark"),
    ] {
        ctx.set_theme(theme);
        let output_target = crate::render::Renderer::with_samples(
            &app.state.adapter,
            &app.state.device,
            1400,
            900,
            Some(1),
            &app.stats,
        )
        .unwrap();
        for pass in 0..4 {
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1400., 900.),
                    )),
                    ..Default::default()
                },
                |ui| {
                    app.header(ui);
                    app.ribbon(ui);
                    app.status_bar(ui);
                    app.inspector(ui);
                    app.project_browser(ui);
                    egui::CentralPanel::default().show(ui, |ui| {
                        if let Some(loaded) = &app.loaded {
                            let camera = app.camera.view(1.5).unwrap();
                            app.renderer.draw(
                                &app.state.device,
                                &app.state.queue,
                                &loaded.uploaded,
                                camera,
                                &app.stats,
                                app.mode,
                            );
                            ui.image((app.texture, ui.available_size()));
                        }
                    });
                },
            );
            let labels: Vec<String> = output
                .shapes
                .iter()
                .filter_map(|s| {
                    if let egui::Shape::Text(t) = &s.shape {
                        Some(t.galley.text().into())
                    } else {
                        None
                    }
                })
                .collect();
            for label in [
                "Views",
                "Drawings",
                "Schedules",
                "Ground plan",
                "View properties",
            ] {
                assert!(
                    labels
                        .iter()
                        .any(|text| text == label || text.contains(label)),
                    "missing {label}: {labels:?}"
                );
            }
            let screen = eframe::egui_wgpu::ScreenDescriptor {
                size_in_pixels: [1400, 900],
                pixels_per_point: output.pixels_per_point,
            };
            let jobs = ctx.tessellate(output.shapes, output.pixels_per_point);
            let mut renderer = app.state.renderer.write();
            for (id, deltas) in &output.textures_delta.set {
                for delta in deltas {
                    renderer.update_texture(&app.state.device, &app.state.queue, *id, delta);
                }
            }
            let mut encoder = app.state.device.create_command_encoder(&Default::default());
            let mut callbacks = renderer.update_buffers(
                &app.state.device,
                &app.state.queue,
                &mut encoder,
                &jobs,
                &screen,
            );
            {
                let pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &output_target.view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    ..Default::default()
                });
                renderer.render(&mut pass.forget_lifetime(), &jobs, &screen);
            }
            callbacks.push(encoder.finish());
            let index = app.state.queue.submit(callbacks);
            crate::render::wait(&app.state.device, index).unwrap();
            for id in &output.textures_delta.free {
                renderer.free_texture(id);
            }
            output.textures_delta.clear();
            drop(renderer);
            if pass == 3
                && let Ok(directory) = std::env::var("BEAMCRAFT_VIEW_QA_DIR")
            {
                let (pixels, row) = output_target
                    .readback(&app.state.device, &app.state.queue)
                    .unwrap();
                let mut packed = Vec::new();
                for line in pixels.chunks(row as usize).take(900) {
                    packed.extend_from_slice(&line[..1400 * 4]);
                }
                let path = std::path::Path::new(&directory).join(format!("model-views-{name}.png"));
                image::save_buffer(path, &packed, 1400, 900, image::ColorType::Rgba8).unwrap();
            }
        }
    }
}
