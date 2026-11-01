// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! Project-browser view workflows. All persisted writes use the headless CLI.

use super::{App, workbench::Bench};
use crate::{
    camera::Camera,
    model_views::{BrowserTab, Construction, Form},
    tools::ToolError,
};
use beamcraft_engine::model_view::{ModelView, Projection};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::mpsc::{Receiver, TryRecvError},
    time::Duration,
};

#[derive(Default)]
pub(super) struct Views {
    pub tab: BrowserTab,
    pub collection: Vec<ModelView>,
    pub active: Option<String>,
    pub form: Form,
    pub editing: bool,
    pub job: Option<Receiver<Result<crate::model_views::Collection, ToolError>>>,
    pub loaded: bool,
    pub activate_result: bool,
    pub activation_id: Option<String>,
    pub note: Option<Result<String, String>>,
    pub points: Vec<[f64; 3]>,
    pub picking: bool,
    pub surface_ray: Option<crate::edit::Ray>,
    pub hover: Option<([f64; 3], Option<beamcraft_engine::query::pick::SnapKind>)>,
    pub cameras: BTreeMap<String, Camera>,
    pub default_camera: Option<Camera>,
    pub default_display: Option<(beamcraft_engine::RenderMode, bool)>,
    pub delete: Option<String>,
    pub only_favorites: bool,
    pub filtered: Option<crate::render::Uploaded>,
    pub filter_key: Option<(String, beamcraft_engine::KindFilter)>,
    pub origin: Option<[f64; 3]>,
    pub windows: BTreeMap<String, ViewWindow>,
    pub open: Vec<String>,
    pub spatial_camera: Option<Camera>,
    pub tile_windows: bool,
    pub thumbnails: bool,
    pub quick_access: bool,
    pub clipboard: Option<(ModelView, bool)>,
    pub destination_folder: String,
    pub folders: Vec<String>,
    pub new_folder: bool,
    pub selected: BTreeSet<String>,
    pub selection_anchor: Option<String>,
    pub(super) previews: BTreeMap<String, Preview>,
    preview_renderer: Option<crate::render::Renderer>,
    preview_budget: bool,
}
pub(super) struct Preview {
    token: String,
    properties: ModelView,
    texture: egui::TextureHandle,
}
pub(super) struct ViewWindow {
    renderer: crate::render::Renderer,
    texture: egui::TextureId,
    camera: Camera,
    origin: [f64; 3],
    token: String,
    filtered: Option<crate::render::Uploaded>,
    properties: ModelView,
    dock: Option<&'static str>,
}
fn text(ui: &mut egui::Ui, label: &str, value: &mut String) {
    ui.label(label);
    ui.add(
        egui::TextEdit::singleline(value)
            .char_limit(512)
            .desired_width(ui.available_width()),
    );
}
fn triple([x, y, z]: [f64; 3]) -> String {
    format!("{x},{y},{z}")
}

impl App {
    fn view_thumbnail(
        &mut self,
        ctx: &egui::Context,
        definition: &ModelView,
    ) -> Option<egui::TextureId> {
        let loaded = self.loaded.as_ref()?;
        let id = definition.id.to_string();
        let token = loaded.snapshot.token().as_str();
        if let Some(preview) = self.views.previews.get(&id)
            && preview.token == token
            && &preview.properties == definition
        {
            return Some(preview.texture.id());
        }
        if !std::mem::take(&mut self.views.preview_budget) {
            ctx.request_repaint();
            return None;
        }
        let result = (|| {
            if self.views.preview_renderer.is_none() {
                self.views.preview_renderer = Some(crate::render::Renderer::with_samples(
                    &self.state.adapter,
                    &self.state.device,
                    160,
                    100,
                    Some(1),
                    &self.stats,
                )?);
            }
            let renderer = self
                .views
                .preview_renderer
                .as_mut()
                .ok_or(crate::Error::Input("preview renderer"))?;
            let camera =
                Camera::from_model_view(definition, loaded.summary.origin_mm)?.view(1.6)?;
            let filtered = if definition.hidden_kinds == beamcraft_engine::KindFilter::SHOW_ALL {
                None
            } else {
                let scene = crate::scene::Scene::from_snapshot_filtered(
                    &loaded.snapshot,
                    beamcraft_engine::Representation::Full,
                    &loaded.uploaded.resident(),
                    definition.hidden_kinds,
                )?;
                Some(crate::render::Uploaded::upload(
                    &self.state.device,
                    &self.state.queue,
                    &scene,
                    Some(&loaded.uploaded),
                )?)
            };
            renderer.background = self.renderer.background;
            renderer.draw(
                &self.state.device,
                &self.state.queue,
                filtered.as_ref().unwrap_or(&loaded.uploaded),
                camera,
                &self.stats,
                definition.render_mode,
            );
            let (pixels, stride) = renderer.readback(&self.state.device, &self.state.queue)?;
            let mut packed = Vec::with_capacity(160 * 100 * 4);
            for line in pixels.chunks(stride as usize).take(100) {
                packed.extend_from_slice(
                    line.get(..160 * 4)
                        .ok_or(crate::Error::Input("preview row"))?,
                );
            }
            Ok::<_, crate::Error>(egui::ColorImage::from_rgba_unmultiplied(
                [160, 100],
                &packed,
            ))
        })();
        match result {
            Ok(image) => {
                let texture = ctx.load_texture(
                    format!("view-preview-{id}"),
                    image,
                    egui::TextureOptions::LINEAR,
                );
                let texture_id = texture.id();
                self.views.previews.insert(
                    id,
                    Preview {
                        token: token.into(),
                        properties: definition.clone(),
                        texture,
                    },
                );
                Some(texture_id)
            }
            Err(error) => {
                self.views.note = Some(Err(error.to_string()));
                None
            }
        }
    }
    pub(super) fn selected_is_visible(&self) -> bool {
        let Some(view) = self.views.active.as_ref().and_then(|id| {
            self.views
                .collection
                .iter()
                .find(|v| v.id.to_string() == *id)
        }) else {
            return true;
        };
        view.opacity > 0.
            && self
                .editor
                .selected()
                .and_then(|s| beamcraft_engine::ElementId::parse(&s.id).ok())
                .and_then(|id| self.loaded.as_ref().and_then(|l| l.snapshot.element(&id)))
                .is_some_and(|e| view.hidden_kinds.is_visible(e.kind))
    }
    pub(super) fn view_shortcuts(&mut self, ctx: &egui::Context) {
        if self.views.picking
            && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
        {
            self.cancel_view_picking();
            self.views.editing = self.views.form.id.is_some();
            return;
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::P)) {
            self.toggle_view_plane();
        }
        if ctx.egui_wants_keyboard_input() {
            return;
        }
        if self.views.picking
            && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Backspace))
        {
            if self.views.points.pop().is_none() {
                self.views.surface_ray = None;
            }
            self.views.note = None;
        }
        let next = ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::Tab));
        if next && !self.views.open.is_empty() {
            let index = self
                .views
                .active
                .as_ref()
                .and_then(|id| self.views.open.iter().position(|v| v == id))
                .map_or(0, |i| i.saturating_add(1) % self.views.open.len());
            if let Some(id) = self.views.open.get(index).cloned() {
                self.activate_view(Some(&id));
            }
        }
    }
    pub(super) fn toggle_view_plane(&mut self) {
        if let Some(camera) = self.views.spatial_camera.take() {
            self.camera = camera;
            return;
        }
        let plane = self
            .views
            .active
            .as_ref()
            .and_then(|id| {
                self.views
                    .collection
                    .iter()
                    .find(|v| v.id.to_string() == *id)
            })
            .map(|v| v.plane)
            .unwrap_or(self.tools.plane.unwrap_or_default());
        let origin = self
            .loaded
            .as_ref()
            .map_or([0.; 3], |l| l.summary.origin_mm);
        if let Ok((_, span)) = self.camera.model_frame(origin)
            && let Ok(camera) = self.camera.aligned(plane, origin, span)
        {
            self.views.spatial_camera = Some(self.camera);
            self.camera = camera;
        }
    }
    pub(super) fn open_view_window(&mut self, id: &str) {
        if self.views.windows.contains_key(id) {
            return;
        }
        if self.views.windows.len() >= 8
            || (!self.views.open.iter().any(|v| v == id) && self.views.open.len() >= 9)
        {
            self.views.note = Some(Err(
                "Close a view window before opening another (nine viewports maximum).".into(),
            ));
            return;
        }
        let Some(view) = self
            .views
            .collection
            .iter()
            .find(|v| v.id.to_string() == id)
        else {
            return;
        };
        let Some(loaded) = &self.loaded else {
            return;
        };
        let renderer = crate::render::Renderer::with_samples(
            &self.state.adapter,
            &self.state.device,
            480,
            320,
            self.source.samples,
            &self.stats,
        );
        let result = renderer.and_then(|renderer| {
            let camera = Camera::from_model_view(view, loaded.summary.origin_mm)?;
            let texture = self.state.renderer.write().register_native_texture(
                &self.state.device,
                &renderer.view,
                wgpu::FilterMode::Linear,
            );
            Ok(ViewWindow {
                renderer,
                texture,
                camera,
                origin: loaded.summary.origin_mm,
                token: String::new(),
                filtered: None,
                properties: view.clone(),
                dock: None,
            })
        });
        match result {
            Ok(window) => {
                self.views.windows.insert(id.into(), window);
                if !self.views.open.iter().any(|v| v == id) {
                    self.views.open.push(id.into());
                }
            }
            Err(error) => self.views.note = Some(Err(error.to_string())),
        }
    }
    pub(super) fn show_view_windows(&mut self, ctx: &egui::Context) {
        let Some(loaded) = &self.loaded else {
            return;
        };
        let mut close = Vec::new();
        let tiled = std::mem::take(&mut self.views.tile_windows);
        for (i, (id, window)) in self.views.windows.iter_mut().enumerate() {
            let Some(definition) = self
                .views
                .collection
                .iter()
                .find(|v| v.id.to_string() == *id)
            else {
                close.push(id.clone());
                continue;
            };
            if &window.properties != definition
                && let Ok(camera) = Camera::from_model_view(definition, loaded.summary.origin_mm)
            {
                window.camera = camera;
                window.origin = loaded.summary.origin_mm;
                window.properties = definition.clone();
            }
            if window.origin != loaded.summary.origin_mm {
                window
                    .camera
                    .rebase_origin(window.origin, loaded.summary.origin_mm);
                window.origin = loaded.summary.origin_mm;
            }
            let token = format!(
                "{}:{:?}",
                loaded.snapshot.token().as_str(),
                definition.hidden_kinds
            );
            if window.token != token {
                if definition.hidden_kinds == beamcraft_engine::KindFilter::SHOW_ALL {
                    window.filtered = None;
                } else {
                    let result = crate::scene::Scene::from_snapshot_filtered(
                        &loaded.snapshot,
                        beamcraft_engine::Representation::Full,
                        &loaded.uploaded.resident(),
                        definition.hidden_kinds,
                    )
                    .and_then(|scene| {
                        crate::render::Uploaded::upload(
                            &self.state.device,
                            &self.state.queue,
                            &scene,
                            Some(&loaded.uploaded),
                        )
                    });
                    match result {
                        Ok(uploaded) => window.filtered = Some(uploaded),
                        Err(error) => self.views.note = Some(Err(error.to_string())),
                    }
                }
                window.token = token;
            }
            let mut open = true;
            let mut frame = egui::Window::new(&definition.name)
                .id(egui::Id::unique(("model-view-window", id)))
                .open(&mut open)
                .default_size(egui::vec2(480., 320.));
            if tiled {
                frame = frame
                    .current_pos(egui::pos2(
                        270. + (i % 2) as f32 * 430.,
                        160. + (i / 2) as f32 * 270.,
                    ))
                    .fixed_size(egui::vec2(420., 250.));
            }
            if let Some(dock) = window.dock {
                let area = ctx.content_rect();
                let width = (area.width() - 580.).max(200.);
                let height = (area.height() - 250.).max(160.);
                let (x, y, w, h) = match dock {
                    "left" => (260., 180., width / 2., height),
                    "right" => (260. + width / 2., 180., width / 2., height),
                    "top" => (260., 180., width, height / 2.),
                    _ => (260., 180. + height / 2., width, height / 2.),
                };
                frame = frame
                    .fixed_pos(egui::pos2(x, y))
                    .fixed_size(egui::vec2(w, h));
            }
            frame.show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.menu_button("Dock", |ui| {
                        for side in ["left", "right", "top", "bottom"] {
                            if ui.button(side).clicked() {
                                window.dock = Some(side);
                                ui.close();
                            }
                        }
                        if ui.button("Float window").clicked() {
                            window.dock = None;
                            ui.close();
                        }
                    });
                    if ui.small_button("Fit").clicked() {
                        let (target, extent) = loaded.summary.framing();
                        if let Err(error) = window.camera.fit(target, extent, 1.5) {
                            self.views.note = Some(Err(error.to_string()));
                        }
                    }
                });
                let size = ui.available_size().max(egui::vec2(64., 64.));
                let response = ui.allocate_response(size, egui::Sense::click_and_drag());
                let scale = ctx.pixels_per_point();
                let [w, h] = super::target_size([size.x, size.y], scale);
                match window
                    .renderer
                    .resize(&self.state.device, w, h, &self.stats)
                {
                    Ok(true) => self
                        .state
                        .renderer
                        .write()
                        .update_egui_texture_from_wgpu_texture(
                            &self.state.device,
                            &window.renderer.view,
                            wgpu::FilterMode::Linear,
                            window.texture,
                        ),
                    Ok(false) => {}
                    Err(error) => {
                        self.views.note = Some(Err(error.to_string()));
                        return;
                    }
                }
                let aspect = size.x / size.y;
                let (orbit, pan, zoom) = ui.input(|input| {
                    super::navigation(
                        self.preset,
                        input.pointer.delta(),
                        response.dragged_by(egui::PointerButton::Primary),
                        response.dragged_by(egui::PointerButton::Secondary),
                        response.dragged_by(egui::PointerButton::Middle),
                        input.modifiers.ctrl,
                        input.modifiers.shift,
                        window.camera.distance(),
                        response.hovered(),
                        input.smooth_scroll_delta.y,
                        input.zoom_delta(),
                        false,
                        false,
                        self.invert_zoom,
                    )
                });
                if let Err(error) = window.camera.navigate(orbit, pan, zoom) {
                    self.views.note = Some(Err(error.to_string()));
                }
                if let Ok(view) = window.camera.view(aspect) {
                    window.renderer.background = self.renderer.background;
                    let selected =
                        if definition.render_mode == beamcraft_engine::RenderMode::SelectedOnly {
                            self.editor
                                .selected()
                                .and_then(|s| beamcraft_engine::ElementId::parse(&s.id).ok())
                                .filter(|id| {
                                    loaded
                                        .snapshot
                                        .element(id)
                                        .is_some_and(|e| definition.hidden_kinds.is_visible(e.kind))
                                })
                                .and_then(|id| {
                                    crate::scene::SelectedDraw::of(
                                        &loaded.snapshot,
                                        &id,
                                        loaded.summary.origin_mm,
                                    )
                                })
                        } else {
                            None
                        };
                    if let Some(selected) = selected {
                        window.renderer.draw_one(
                            &self.state.device,
                            &self.state.queue,
                            window.filtered.as_ref().unwrap_or(&loaded.uploaded),
                            view,
                            &self.stats,
                            &selected.digest,
                            &selected.instance,
                        );
                    } else {
                        window.renderer.draw(
                            &self.state.device,
                            &self.state.queue,
                            window.filtered.as_ref().unwrap_or(&loaded.uploaded),
                            view,
                            &self.stats,
                            definition.render_mode,
                        );
                    }
                    ui.painter().image(
                        window.texture,
                        response.rect,
                        egui::Rect::from_min_max(egui::pos2(0., 0.), egui::pos2(1., 1.)),
                        egui::Color32::WHITE,
                    );
                    if definition.grids_visible {
                        self.grid_scene.paint(
                            ui,
                            response.rect,
                            &view,
                            loaded.summary.origin_mm,
                            None,
                        );
                    }
                }
            });
            if !open {
                close.push(id.clone());
            }
        }
        for id in close {
            if self.views.active.as_deref() != Some(id.as_str()) {
                self.views.open.retain(|v| v != &id);
            }
            if let Some(window) = self.views.windows.remove(&id) {
                self.state.renderer.write().free_texture(&window.texture);
            }
        }
    }
    pub(super) fn refresh_view_display(&mut self) {
        let Some(loaded) = &self.loaded else {
            return;
        };
        let origin = loaded.summary.origin_mm;
        if let Some(old) = self.views.origin
            && old != origin
            && self.views.active.is_some()
        {
            self.camera.rebase_origin(old, origin);
            for camera in self.views.cameras.values_mut() {
                camera.rebase_origin(old, origin);
            }
        }
        self.views.origin = Some(origin);
        let filter = self
            .views
            .active
            .as_ref()
            .and_then(|id| {
                self.views
                    .collection
                    .iter()
                    .find(|v| v.id.to_string() == *id)
            })
            .map_or(beamcraft_engine::KindFilter::SHOW_ALL, |v| v.hidden_kinds);
        if filter == beamcraft_engine::KindFilter::SHOW_ALL {
            self.views.filtered = None;
            self.views.filter_key = None;
            return;
        }
        let key = (loaded.snapshot.token().as_str().to_owned(), filter);
        if self.views.filter_key.as_ref() == Some(&key) {
            return;
        }
        let result = crate::scene::Scene::from_snapshot_filtered(
            &loaded.snapshot,
            beamcraft_engine::Representation::Full,
            &loaded.uploaded.resident(),
            filter,
        )
        .and_then(|scene| {
            crate::render::Uploaded::upload(
                &self.state.device,
                &self.state.queue,
                &scene,
                Some(&loaded.uploaded),
            )
        });
        match result {
            Ok(uploaded) => {
                self.views.filtered = Some(uploaded);
                self.views.filter_key = Some(key);
            }
            Err(error) => self.views.note = Some(Err(error.to_string())),
        }
    }
    pub(super) fn view_command(&mut self, command: &str, extra: Vec<String>, activate: bool) {
        if self.views.job.is_some() {
            return;
        }
        let mut args = vec![
            command.to_owned(),
            "--store".into(),
            self.source.store.to_string_lossy().into_owned(),
        ];
        args.extend(extra);
        if command == "create-model-view" {
            let reserved = self
                .views
                .collection
                .iter()
                .filter(|v| v.temporary && Some(v.id.to_string()) != self.views.form.id)
                .count();
            args.extend(["--reserved-views".into(), reserved.to_string()]);
        }
        let (tx, rx) = std::sync::mpsc::channel();
        let (cli, stats) = (self.source.cli.clone(), std::sync::Arc::clone(&self.stats));
        match std::thread::Builder::new()
            .name("model-views".into())
            .spawn(move || {
                let result = crate::command::run(&cli, &args, Duration::from_secs(30), &stats)
                    .and_then(|output| {
                        crate::model_views::parse_reply(output.code, &output.stdout)
                    });
                let _ = tx.send(result);
            }) {
            Ok(_) => {
                self.views.job = Some(rx);
                self.views.activate_result = activate;
                self.views.activation_id = if activate {
                    self.views.form.id.clone().or_else(|| {
                        (command != "create-model-view")
                            .then(|| self.views.active.clone())
                            .flatten()
                    })
                } else {
                    None
                };
                self.views.note = None;
            }
            Err(_) => self.views.note = Some(Err("Could not start the view worker; retry.".into())),
        }
    }
    pub(super) fn poll_views(&mut self) {
        if !self.views.loaded {
            self.views.loaded = true;
            self.view_command("model-views", Vec::new(), false);
        }
        let Some(job) = &self.views.job else {
            return;
        };
        let result = match job.try_recv() {
            Ok(result) => result,
            Err(TryRecvError::Empty) => return,
            Err(TryRecvError::Disconnected) => Err(ToolError::local(
                "BACKEND_UNAVAILABLE",
                "View worker stopped; retry.",
            )),
        };
        self.views.job = None;
        match result {
            Ok(collection) => {
                self.views.folders = collection.folders;
                let mut views = collection.views;
                // Capture the result before appending unrelated session-only views. The target
                // belongs to the submitted request, not a form the user may since have opened.
                let id = if self.views.activate_result {
                    self.views
                        .activation_id
                        .take()
                        .or_else(|| {
                            views
                                .iter()
                                .rev()
                                .find(|v| !self.views.collection.iter().any(|old| old.id == v.id))
                                .map(|v| v.id.to_string())
                        })
                        .or_else(|| self.views.active.clone())
                } else {
                    None
                };
                // Temporary views live only in this session, including across later CLI replies.
                for view in self.views.collection.iter().filter(|v| v.temporary) {
                    if !views.iter().any(|other| other.id == view.id) {
                        views.push(view.clone());
                    }
                }
                if let Err(error) = beamcraft_engine::model_view::validate_collection(&views) {
                    self.views.note = Some(Err(format!(
                        "The loaded collection conflicts with session views: {error}. Rename or delete the conflicting temporary views, then refresh the list."
                    )));
                    return;
                }
                self.views.collection = views;
                self.views.previews.retain(|id, _| {
                    self.views
                        .collection
                        .iter()
                        .any(|v| v.id.to_string() == *id)
                });
                self.views.open.retain(|id| {
                    self.views
                        .collection
                        .iter()
                        .any(|v| v.id.to_string() == *id)
                });
                self.views.cameras.retain(|id, _| {
                    self.views
                        .collection
                        .iter()
                        .any(|v| v.id.to_string() == *id)
                });
                if self.views.form.id.as_ref().is_some_and(|id| {
                    !self
                        .views
                        .collection
                        .iter()
                        .any(|v| v.id.to_string() == *id)
                }) {
                    self.views.form = Form::default();
                    self.views.editing = false;
                }
                self.views.selected.retain(|id| {
                    self.views
                        .collection
                        .iter()
                        .any(|v| v.id.to_string() == *id)
                });
                self.views.note = Some(Ok("View collection updated.".into()));
                if let Some(id) = id {
                    self.activate_view(Some(&id));
                } else if self.views.active.as_ref().is_some_and(|id| {
                    !self
                        .views
                        .collection
                        .iter()
                        .any(|v| v.id.to_string() == *id)
                }) {
                    self.activate_view(None);
                }
            }
            Err(error) => self.views.note = Some(Err(error.to_string())),
        }
    }
    pub(super) fn begin_view(&mut self, construction: Construction) {
        if self.other_command_running() || self.views.job.is_some() {
            self.views.note = Some(Err("Wait for the current command to finish.".into()));
            return;
        }
        self.set_bench(Bench::Select);
        let construction = if construction == Construction::Explicit {
            Construction::ThreePoints
        } else {
            construction
        };
        self.views.form = Form {
            construction,
            ..Form::default()
        };
        let origin = self
            .loaded
            .as_ref()
            .map_or([0.; 3], |l| l.summary.origin_mm);
        if let Ok((_, span)) = self.camera.model_frame(origin) {
            self.views.form.span = span.to_string();
        }
        let mut n = 1usize;
        while self
            .views
            .collection
            .iter()
            .any(|v| v.name == self.views.form.name)
            && n < 257
        {
            n = n.saturating_add(1);
            self.views.form.name = format!("New view {n}");
        }
        if let Some(grid) = self.grid_edit.as_ref() {
            self.views.form.selector = grid.id.clone();
        } else if let Some(grid) = self.grid_scene.query_grids.first() {
            self.views.form.selector = grid.id.to_string();
        }
        self.views.editing = true;
        self.views.tab = BrowserTab::Views;
        self.start_view_picking();
    }
    fn start_view_picking(&mut self) {
        self.views.points.clear();
        self.views.hover = None;
        self.views.surface_ray = None;
        self.views.picking = self.views.form.construction.picks_geometry();
        self.views.note = None;
    }
    pub(super) fn cancel_view_picking(&mut self) {
        self.views.picking = false;
        self.views.points.clear();
        self.views.hover = None;
        self.views.surface_ray = None;
        self.views.note = None;
    }
    pub(super) fn view_pick_prompt(&self) -> &'static str {
        if matches!(
            self.views.form.construction,
            Construction::Face | Construction::FaceEdge
        ) && self.views.surface_ray.is_none()
        {
            return "Pick a planar surface";
        }
        match (self.views.form.construction, self.views.points.len()) {
            (Construction::FaceEdge, 0) => "Pick the start of the surface edge",
            (Construction::FaceEdge, _) => "Pick the end of the surface edge",
            (_, 0) => "Pick the first point",
            (_, 1) => "Pick the second point",
            _ => "Pick the third point",
        }
    }
    pub(super) fn activate_view(&mut self, id: Option<&str>) {
        if let Some(old) = &self.views.active {
            if id != Some(old.as_str())
                && self
                    .views
                    .collection
                    .iter()
                    .any(|v| v.id.to_string() == *old)
            {
                self.views.cameras.insert(old.clone(), self.camera);
            }
        } else {
            self.views.default_camera = Some(self.camera);
            self.views.default_display = Some((self.mode, self.grids_visible));
        }
        let Some(id) = id else {
            self.views.active = None;
            if let Some(camera) = self.views.default_camera {
                self.camera = camera;
            }
            if let Some((mode, grids)) = self.views.default_display {
                self.mode = mode;
                self.grids_visible = grids;
            }
            self.install_plane(None);
            return;
        };
        let Some(view) = self
            .views
            .collection
            .iter()
            .find(|v| v.id.to_string() == id)
            .cloned()
        else {
            return;
        };
        let origin = self
            .loaded
            .as_ref()
            .map_or([0.; 3], |l| l.summary.origin_mm);
        if !self.views.open.iter().any(|v| v == id) && self.views.open.len() >= 9 {
            self.views.note = Some(Err(
                "Close a view before opening another (nine open views maximum).".into(),
            ));
            return;
        }
        let camera = self
            .views
            .cameras
            .get(id)
            .copied()
            .map(Ok)
            .unwrap_or_else(|| Camera::from_model_view(&view, origin));
        match camera {
            Ok(camera) => {
                self.camera = camera;
                self.mode = view.render_mode;
                self.grids_visible = view.grids_visible;
                self.views.active = Some(id.into());
                self.views.form = Form::from_view(&view);
                if !self.views.open.iter().any(|v| v == id) {
                    self.views.open.push(id.into());
                }
                self.views.spatial_camera = None;
                self.install_plane(Some(view.plane));
                self.views.picking = false;
            }
            Err(error) => self.views.note = Some(Err(error.to_string())),
        }
    }
    pub(super) fn refresh_current_view(&mut self, all: bool, redraw: bool) {
        self.plate_hover = None;
        self.snap_hover = None;
        self.selection_edges = Default::default();
        if redraw {
            self.reload();
        }
        if all {
            for window in self.views.windows.values_mut() {
                window.token.clear();
            }
        }
    }
    pub(super) fn orient_view(&mut self, side: &str) {
        let origin = self
            .loaded
            .as_ref()
            .map_or([0.; 3], |l| l.summary.origin_mm);
        let (target, span) = match self.camera.model_frame(origin) {
            Ok(v) => v,
            Err(_) => return,
        };
        let o = target.origin();
        let (x, y) = match side {
            "top" => ([1., 0., 0.], [0., 1., 0.]),
            "bottom" => ([1., 0., 0.], [0., -1., 0.]),
            "front" => ([1., 0., 0.], [0., 0., 1.]),
            "back" => ([-1., 0., 0.], [0., 0., 1.]),
            "left" => ([0., -1., 0.], [0., 0., 1.]),
            "right" => ([0., 1., 0.], [0., 0., 1.]),
            _ => return,
        };
        if let Ok(plane) = beamcraft_engine::work_plane::WorkPlane::new(o, x, y)
            && let Ok(camera) = self.camera.aligned(plane, origin, span)
        {
            self.views.spatial_camera = Some(self.camera);
            self.camera = camera;
        }
    }
    fn submit_view(&mut self) {
        let mode = self.views.form.construction;
        let count = if mode == Construction::ThreePoints {
            3
        } else {
            2
        };
        if mode.picks_geometry()
            && (if mode == Construction::Face {
                self.views.surface_ray.is_none()
            } else {
                self.views.points.len() < count
                    || (mode == Construction::FaceEdge && self.views.surface_ray.is_none())
            })
        {
            self.views.picking = true;
            self.views.note = None;
            return;
        }
        let form = &self.views.form;
        let batch = form.id.is_none()
            && (matches!(
                form.construction,
                Construction::PartViews | Construction::AssemblyViews
            ) || (form.construction == Construction::Grid && form.batch != "single")
                || (form.construction == Construction::Level && form.batch != "single"));
        if !batch
            && self.views.collection.iter().any(|v| {
                Some(v.id.to_string()) != self.views.form.id
                    && v.name.to_lowercase() == self.views.form.name.trim().to_lowercase()
            })
        {
            self.views.note = Some(Err(
                "A view with this name already exists. Choose another name.".into(),
            ));
            return;
        }
        let mut args = self.views.form.property_args();
        let construction = self.views.form.construction;
        if let Some(id) = &self.views.form.id {
            let temporary = self
                .views
                .collection
                .iter()
                .any(|v| v.id.to_string() == *id && v.temporary);
            args.extend([
                if temporary { "--new-id" } else { "--id" }.into(),
                id.clone(),
                "--plane".into(),
                self.views.form.plane.clone(),
            ]);
            self.views.cameras.remove(id);
            self.view_command(
                if temporary {
                    "create-model-view"
                } else {
                    "update-model-view"
                },
                args,
                true,
            );
            return;
        }
        let pair = match construction {
            Construction::XY | Construction::XZ | Construction::YZ => {
                args.extend(["--offset".into(), self.views.form.offset.clone()]);
                (
                    "--parallel",
                    match construction {
                        Construction::XY => "xy",
                        Construction::XZ => "xz",
                        _ => "yz",
                    }
                    .into(),
                )
            }
            Construction::TwoPoints => {
                args.extend([
                    "--work-plane".into(),
                    self.tools.plane.unwrap_or_default().expression(),
                ]);
                (
                    "--two-points",
                    self.views
                        .form
                        .points
                        .iter()
                        .take(2)
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(","),
                )
            }
            Construction::ThreePoints => ("--three-points", self.views.form.points.join(",")),
            Construction::WorkPlane => {
                ("--plane", self.tools.plane.unwrap_or_default().expression())
            }
            Construction::Explicit => ("--plane", self.views.form.plane.clone()),
            Construction::Grid => {
                if self.views.form.batch == "single" {
                    args.extend(["--axis".into(), self.views.form.axis.clone()]);
                } else if self.views.form.batch == "families" {
                    for (index, family) in ["u", "v"].into_iter().enumerate() {
                        if let (Some(mode), Some(prefix), Some(source)) = (
                            self.views.form.families.get(index),
                            self.views.form.prefixes.get(index),
                            self.views.form.property_sources.get(index),
                        ) {
                            args.extend([
                                format!("--{family}-axes"),
                                mode.clone(),
                                format!("--{family}-prefix"),
                                prefix.clone(),
                            ]);
                            if !source.is_empty() {
                                args.extend([format!("--{family}-properties"), source.clone()]);
                            }
                        }
                    }
                } else {
                    args.extend(["--axes".into(), self.views.form.batch.clone()]);
                }
                ("--grid", self.views.form.selector.clone())
            }
            Construction::Level => {
                if self.views.form.batch == "single" {
                    ("--level", self.views.form.selector.clone())
                } else {
                    (
                        "--levels",
                        if self.views.form.batch == "families" {
                            "all".into()
                        } else {
                            self.views.form.batch.clone()
                        },
                    )
                }
            }
            Construction::Face | Construction::FaceEdge => {
                let Some(ray) = self.views.surface_ray else {
                    self.views.note = Some(Err("Pick a model surface first.".into()));
                    return;
                };
                let edge = if construction == Construction::FaceEdge {
                    let [a, b] = self.views.points.as_slice() else {
                        self.views.note =
                            Some(Err("Pick the start and end of the surface edge.".into()));
                        return;
                    };
                    Some([*a, *b])
                } else {
                    None
                };
                let Some(loaded) = &self.loaded else {
                    self.views.note = Some(Err("Load a model first.".into()));
                    return;
                };
                let view = self.views.active.as_ref().and_then(|id| {
                    self.views
                        .collection
                        .iter()
                        .find(|v| v.id.to_string() == *id)
                });
                match beamcraft_engine::model_view::surface_plane_in_view(
                    &loaded.snapshot,
                    view,
                    ray.origin,
                    ray.direction,
                    edge,
                ) {
                    Ok(plane) => ("--plane", plane.expression()),
                    Err(error) => {
                        self.views.note = Some(Err(error.to_string()));
                        return;
                    }
                }
            }
            mode => {
                let Some(selected) = self.editor.selected() else {
                    self.views.note = Some(Err("Select a part before creating its views.".into()));
                    return;
                };
                args.extend([
                    "--side".into(),
                    match mode {
                        Construction::PartViews | Construction::AssemblyViews => "four",
                        Construction::PartFront => "front",
                        Construction::PartBack => "back",
                        Construction::PartTop => "top",
                        Construction::PartBottom => "bottom",
                        _ => "3d",
                    }
                    .into(),
                ]);
                if mode == Construction::Undeformed {
                    args.extend(["--undeformed".into(), "true".into()]);
                }
                (
                    if matches!(mode, Construction::AssemblyViews | Construction::Assembly3D) {
                        "--group"
                    } else {
                        "--element"
                    },
                    selected.id.clone(),
                )
            }
        };
        args.extend([pair.0.into(), pair.1]);
        self.view_command("create-model-view", args, true);
    }
    pub(super) fn view_pick(&mut self, ray: crate::edit::Ray) {
        if self.views.job.is_some() {
            return;
        }
        let mode = self.views.form.construction;
        if matches!(mode, Construction::Face | Construction::FaceEdge)
            && self.views.surface_ray.is_none()
        {
            let result = self
                .loaded
                .as_ref()
                .ok_or_else(|| "Load a model first.".to_owned())
                .and_then(|l| {
                    beamcraft_engine::model_view::surface_plane_in_view(
                        &l.snapshot,
                        self.views.active.as_ref().and_then(|id| {
                            self.views
                                .collection
                                .iter()
                                .find(|v| v.id.to_string() == *id)
                        }),
                        ray.origin,
                        ray.direction,
                        None,
                    )
                    .map_err(|e| e.to_string())
                });
            match result {
                Ok(plane) => {
                    self.views.surface_ray = Some(ray);
                    self.views.form.plane = plane.expression();
                    self.views.note = None;
                    if mode == Construction::Face {
                        self.views.picking = false;
                        self.submit_view();
                    } else {
                        self.views.note = Some(Ok(
                            "Pick the two endpoints of an edge on this surface.".into(),
                        ));
                    }
                }
                Err(error) => self.views.note = Some(Err(error)),
            }
            return;
        }
        let point = match self.resolve_view_point(ray) {
            Ok((point, _)) => point,
            Err(error) => {
                self.views.note = Some(Err(error.to_string()));
                return;
            }
        };
        let count = if mode == Construction::ThreePoints {
            3
        } else {
            2
        };
        if self.views.points.len() >= count {
            return;
        }
        let mut points = self.views.points.clone();
        points.push(point);
        let plane = if points.len() == count {
            let result = match (mode, points.as_slice()) {
                (Construction::TwoPoints, [a, b]) => {
                    beamcraft_engine::model_view::two_points_in_plane(
                        *a,
                        *b,
                        self.tools.plane.unwrap_or_default(),
                    )
                }
                (Construction::ThreePoints, [a, b, c]) => {
                    beamcraft_engine::model_view::three_points(*a, *b, *c)
                }
                (Construction::FaceEdge, [a, b]) => {
                    let Some(loaded) = &self.loaded else {
                        return;
                    };
                    let Some(surface) = self.views.surface_ray else {
                        return;
                    };
                    let view = self.views.active.as_ref().and_then(|id| {
                        self.views
                            .collection
                            .iter()
                            .find(|v| v.id.to_string() == *id)
                    });
                    beamcraft_engine::model_view::surface_plane_in_view(
                        &loaded.snapshot,
                        view,
                        surface.origin,
                        surface.direction,
                        Some([*a, *b]),
                    )
                }
                _ => return,
            };
            match result {
                Ok(plane) => Some(plane),
                Err(error) => {
                    self.views.note = Some(Err(format!("{error}. Pick a different point.")));
                    return;
                }
            }
        } else {
            None
        };
        self.views.points = points;
        for (slot, point) in self.views.form.points.iter_mut().zip(&self.views.points) {
            *slot = triple(*point);
        }
        self.views.note = None;
        if let Some(plane) = plane {
            self.views.form.plane = plane.expression();
            self.views.picking = false;
            self.views.hover = None;
            self.submit_view();
        }
    }
    fn resolve_view_point(
        &self,
        ray: crate::edit::Ray,
    ) -> Result<([f64; 3], Option<beamcraft_engine::query::pick::SnapKind>), ToolError> {
        if self.views.form.construction == Construction::FaceEdge {
            let plane = beamcraft_engine::work_plane::WorkPlane::parse(&self.views.form.plane)
                .map_err(|e| ToolError::local("INVALID_ARGUMENT", e.to_string()))?;
            self.point_hit_on_plane(ray, plane)
        } else {
            self.view_point_hit(ray)
        }
    }
    pub(super) fn paint_view_picking(
        &mut self,
        ui: &mut egui::Ui,
        response: &egui::Response,
        view: &crate::camera::View,
    ) {
        if !self.views.picking {
            self.views.hover = None;
            return;
        }
        let origin = self
            .loaded
            .as_ref()
            .map_or([0.; 3], |l| l.summary.origin_mm);
        let awaiting_face = matches!(
            self.views.form.construction,
            Construction::Face | Construction::FaceEdge
        ) && self.views.surface_ray.is_none();
        self.views.hover = if awaiting_face {
            None
        } else {
            response
                .hover_pos()
                .and_then(|pos| {
                    crate::edit::ndc_of(
                        [pos.x, pos.y],
                        [response.rect.left(), response.rect.top()],
                        [response.rect.width(), response.rect.height()],
                    )
                })
                .and_then(|ndc| crate::edit::ray_through(view, ndc, origin))
                .and_then(|ray| self.resolve_view_point(ray).ok())
        };
        if response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
        }
        let rect = response.rect;
        let painter = ui.painter_at(rect);
        let palette = crate::theme::Palette::current(ui);
        let project = |point: [f64; 3]| {
            let local = std::array::from_fn(|i| {
                (point.get(i).copied().unwrap_or(0.) - origin.get(i).copied().unwrap_or(0.)) as f32
            });
            let [x, y, z] = view.project(local);
            (x.is_finite() && y.is_finite() && (0.0..=1.0).contains(&z)).then(|| {
                egui::pos2(
                    rect.left() + (x + 1.) * rect.width() / 2.,
                    rect.top() + (1. - y) * rect.height() / 2.,
                )
            })
        };
        let stroke = egui::Stroke::new(2., palette.link);
        for pair in self.views.points.windows(2) {
            if let [a, b] = pair
                && let (Some(a), Some(b)) = (project(*a), project(*b))
            {
                painter.line_segment([a, b], stroke);
            }
        }
        for (index, point) in self.views.points.iter().enumerate() {
            if let Some(pos) = project(*point) {
                painter.circle_filled(pos, 4., palette.link);
                painter.text(
                    pos + egui::vec2(8., -8.),
                    egui::Align2::LEFT_BOTTOM,
                    (index + 1).to_string(),
                    egui::FontId::proportional(12.),
                    palette.link,
                );
            }
        }
        if let Some((point, kind)) = self.views.hover
            && let Some(pos) = project(point)
        {
            if let Some(kind) = kind {
                super::switches::snap_marker(
                    &painter,
                    egui::Rect::from_center_size(pos, egui::vec2(16., 16.)),
                    kind,
                    palette.link,
                );
                use beamcraft_engine::query::pick::SnapKind;
                let label = match kind {
                    SnapKind::Midpoint => "Midpoint",
                    SnapKind::Center => "Axis center",
                    SnapKind::GridIntersection => "Grid intersection",
                    SnapKind::Edge | SnapKind::Axis | SnapKind::GridLine => "On line",
                    _ => "Endpoint",
                };
                painter.text(
                    pos + egui::vec2(12., -12.),
                    egui::Align2::LEFT_BOTTOM,
                    label,
                    egui::FontId::proportional(12.),
                    palette.link,
                );
            } else {
                painter.circle_stroke(pos, 5., stroke);
            }
            if let Some(last) = self.views.points.last().and_then(|p| project(*p)) {
                painter.line_segment([last, pos], stroke);
            }
            if self.views.form.construction == Construction::ThreePoints
                && self.views.points.len() == 2
                && let Some(first) = self.views.points.first().and_then(|p| project(*p))
            {
                painter.line_segment(
                    [pos, first],
                    egui::Stroke::new(1., palette.link.gamma_multiply(0.5)),
                );
            }
        }
        let text = format!(
            "{} · {}\nEsc: cancel  ·  Backspace: undo point",
            self.views.form.construction.label(),
            self.view_pick_prompt()
        );
        let galley = painter.layout(
            text,
            egui::FontId::proportional(13.),
            palette.text,
            (rect.width() - 40.).max(40.),
        );
        let position = rect.left_top() + egui::vec2(12., 12.);
        painter.rect_filled(
            egui::Rect::from_min_size(position, galley.size() + egui::vec2(20., 14.)),
            4,
            palette.surface,
        );
        painter.galley(position + egui::vec2(10., 7.), galley, palette.text);
    }
    pub(super) fn view_properties(&mut self, ui: &mut egui::Ui) {
        let mut pick = false;
        let mut appearance_all = false;
        let busy = self.views.job.is_some();
        let mut property_source = String::new();
        let model_bounds = self
            .loaded
            .as_ref()
            .and_then(|l| l.snapshot.engineering_bounds());
        let selection_bounds = self
            .editor
            .selected()
            .and_then(|s| beamcraft_engine::ElementId::parse(&s.id).ok())
            .and_then(|id| {
                self.loaded
                    .as_ref()
                    .and_then(|l| l.snapshot.element_bounds(&id).ok())
            });
        let grid_choices: Vec<_> = self
            .grid_scene
            .query_grids
            .iter()
            .map(|g| (g.id.to_string(), g.name.clone()))
            .collect();
        let previous_mode = self.views.form.construction;
        let picked_count = self.views.points.len();
        let sources: Vec<_> = self
            .views
            .collection
            .iter()
            .map(|v| (v.id.to_string(), v.name.clone()))
            .collect();
        ui.add_enabled_ui(!busy, |ui| {
            let form = &mut self.views.form;
            text(ui, "Name", &mut form.name);
            text(ui, "Folder (use / for subfolders)", &mut form.folder);
            ui.horizontal(|ui| {
                ui.checkbox(&mut form.favorite, "Favorite");
                ui.checkbox(&mut form.temporary, "Temporary");
            });
            if form.id.is_none() {
                egui::ComboBox::from_id_salt("view-property-source")
                    .selected_text("Copy display settings from...")
                    .show_ui(ui, |ui| {
                        for (id, name) in &sources {
                            ui.selectable_value(&mut property_source, id.clone(), name);
                        }
                    });
                egui::ComboBox::from_id_salt("view-construction")
                    .selected_text(form.construction.label())
                    .show_ui(ui, |ui| {
                        for (mode, label) in Construction::ALL {
                            ui.selectable_value(&mut form.construction, mode, label);
                        }
                    });
                match form.construction {
                    Construction::XY | Construction::XZ | Construction::YZ => {
                        text(ui, "Plane offset (mm)", &mut form.offset)
                    }
                    Construction::TwoPoints | Construction::ThreePoints => {
                        let count = if form.construction == Construction::TwoPoints {
                            2
                        } else {
                            3
                        };
                        ui.label(format!(
                            "{picked_count} of {count} points picked in the model"
                        ));
                        ui.small(
                            "Click points in the viewport. Model and grid snaps are available.",
                        );
                        pick = ui.button("Restart point selection").clicked();
                    }
                    Construction::Grid => {
                        let selected = grid_choices
                            .iter()
                            .find(|(id, _)| id == &form.selector)
                            .map_or("Choose a model grid", |(_, name)| name.as_str());
                        egui::ComboBox::from_id_salt("view-grid-source")
                            .selected_text(selected)
                            .show_ui(ui, |ui| {
                                for (id, name) in &grid_choices {
                                    ui.selectable_value(&mut form.selector, id.clone(), name);
                                }
                            });
                        egui::ComboBox::from_id_salt("view-grid-batch")
                            .selected_text(&form.batch)
                            .show_ui(ui, |ui| {
                                for v in ["families", "all", "u", "v", "first", "last", "single"] {
                                    ui.selectable_value(&mut form.batch, v.into(), v);
                                }
                            });
                        if form.batch == "single" {
                            let axes = self
                                .grid_scene
                                .query_grids
                                .iter()
                                .find(|g| g.id.to_string() == form.selector)
                                .and_then(|g| beamcraft_engine::grid_query::segments(g).ok())
                                .unwrap_or_default();
                            egui::ComboBox::from_id_salt("view-grid-axis")
                                .selected_text(if form.axis.is_empty() {
                                    "Choose an axis"
                                } else {
                                    &form.axis
                                })
                                .show_ui(ui, |ui| {
                                    for axis in axes {
                                        ui.selectable_value(
                                            &mut form.axis,
                                            axis.label.clone(),
                                            &axis.label,
                                        );
                                    }
                                });
                        } else if form.batch == "families" {
                            for (index, family) in ["U", "V"].into_iter().enumerate() {
                                ui.separator();
                                ui.label(format!("{family} family"));
                                if let Some(mode) = form.families.get_mut(index) {
                                    egui::ComboBox::from_id_salt(("view-family", family))
                                        .selected_text(mode.as_str())
                                        .show_ui(ui, |ui| {
                                            for value in ["none", "first", "last", "all"] {
                                                ui.selectable_value(mode, value.into(), value);
                                            }
                                        });
                                }
                                if let Some(prefix) = form.prefixes.get_mut(index) {
                                    text(ui, "Name prefix", prefix);
                                }
                                if let Some(source) = form.property_sources.get_mut(index) {
                                    let label = sources
                                        .iter()
                                        .find(|(id, _)| id == source)
                                        .map_or("Current form", |(_, name)| name.as_str());
                                    egui::ComboBox::from_id_salt(("family-properties", family))
                                        .selected_text(label)
                                        .show_ui(ui, |ui| {
                                            ui.selectable_value(
                                                source,
                                                String::new(),
                                                "Current form",
                                            );
                                            for (id, name) in &sources {
                                                ui.selectable_value(source, id.clone(), name);
                                            }
                                        });
                                }
                            }
                        }
                    }
                    Construction::Level => {
                        egui::ComboBox::from_id_salt("level-view-batch")
                            .selected_text(if form.batch == "families" {
                                "all"
                            } else {
                                &form.batch
                            })
                            .show_ui(ui, |ui| {
                                for mode in ["single", "all", "first", "last"] {
                                    ui.selectable_value(&mut form.batch, mode.into(), mode);
                                }
                            });
                        if form.batch == "single" {
                            text(ui, "Level name or identity", &mut form.selector);
                        }
                    }
                    Construction::Face | Construction::FaceEdge => {
                        pick = ui.button("Pick surface in model").clicked();
                    }
                    Construction::WorkPlane => {
                        ui.label("Uses the current work plane.");
                    }
                    Construction::Explicit => {
                        ui.label("Uses the source view's plane.");
                        if ui.button("Pick a different plane").clicked() {
                            form.construction = Construction::ThreePoints;
                            pick = true;
                        }
                    }
                    _ => {
                        ui.label("Select the target part, then Create view.");
                    }
                }
            } else {
                ui.collapsing("View plane", |ui| {
                    if ui.button("Pick plane by three points").clicked() {
                        form.construction = Construction::ThreePoints;
                        pick = true;
                    }
                    if ui.button("Use current work plane").clicked() {
                        form.plane = self.tools.plane.unwrap_or_default().expression();
                    }
                });
            }
            ui.separator();
            ui.label("Projection");
            ui.horizontal(|ui| {
                ui.selectable_value(
                    &mut form.projection,
                    Projection::Orthographic,
                    "Orthographic",
                );
                ui.selectable_value(&mut form.projection, Projection::Perspective, "Perspective");
            });
            ui.label("Camera rotation (degrees)");
            let [z, x] = &mut form.rotation;
            text(ui, "About view Z", z);
            text(ui, "About view X", x);
            text(ui, "Visible height (mm)", &mut form.span);
            ui.checkbox(&mut form.clipped, "Limit view depth");
            if form.clipped {
                let [above, below] = &mut form.depth;
                text(ui, "Towards viewer (mm)", above);
                text(ui, "Behind plane (mm)", below);
            }
            ui.label(if form.area == "none" {
                "Work area: entire model"
            } else {
                "Work area: clipped"
            });
            ui.horizontal(|ui| {
                let mut area = None;
                if ui
                    .add_enabled(model_bounds.is_some(), egui::Button::new("Fit model"))
                    .clicked()
                {
                    area = model_bounds;
                }
                if ui
                    .add_enabled(
                        selection_bounds.is_some(),
                        egui::Button::new("Fit selection"),
                    )
                    .clicked()
                {
                    area = selection_bounds;
                }
                if let Some(bounds) = area {
                    form.area = bounds
                        .min
                        .into_iter()
                        .chain(bounds.max)
                        .map(|v| v.to_string())
                        .collect::<Vec<_>>()
                        .join(",");
                }
                if ui.button("Clear").clicked() {
                    form.area = "none".into();
                }
            });
            ui.checkbox(&mut form.grids, "Show grids");
            ui.checkbox(&mut form.override_color, "Override model colour");
            if form.override_color {
                ui.color_edit_button_rgb(&mut form.color);
            }
            ui.add(egui::Slider::new(&mut form.opacity, 0.0..=1.0).text("Opacity"));
            appearance_all = ui.button("Apply colour and opacity to all views").clicked();
            egui::ComboBox::from_id_salt("view-render-mode")
                .selected_text(form.mode.as_token())
                .show_ui(ui, |ui| {
                    for mode in beamcraft_engine::RenderMode::ALL {
                        ui.selectable_value(&mut form.mode, mode, mode.as_token());
                    }
                });
            ui.collapsing("Object visibility", |ui| {
                let mut filter =
                    beamcraft_engine::KindFilter::parse_list(&form.hidden).unwrap_or_default();
                for kind in beamcraft_engine::ElementKind::ALL {
                    let mut visible = filter.is_visible(kind);
                    if ui.checkbox(&mut visible, kind.as_token()).changed() {
                        filter = if visible {
                            filter.showing(kind)
                        } else {
                            filter.hiding(kind)
                        };
                    }
                }
                form.hidden = filter
                    .hidden_kinds()
                    .iter()
                    .map(|k| k.as_token())
                    .collect::<Vec<_>>()
                    .join(",");
            });
        });
        if let Some(view) = self
            .views
            .collection
            .iter()
            .find(|v| v.id.to_string() == property_source)
        {
            self.views.form.apply_display_properties(view);
        }
        if appearance_all {
            let form = &self.views.form;
            let color = if form.override_color {
                form.color
                    .iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            } else {
                "default".into()
            };
            let opacity = form.opacity.to_string();
            for view in self.views.collection.iter_mut().filter(|v| v.temporary) {
                view.color = form.override_color.then(|| form.color.map(f64::from));
                view.opacity = f64::from(form.opacity);
            }
            self.views.cameras.clear();
            self.view_command(
                "update-model-view",
                vec![
                    "--all".into(),
                    "true".into(),
                    "--color".into(),
                    color,
                    "--opacity".into(),
                    opacity,
                ],
                true,
            );
        }
        if busy {
            ui.spinner();
        }
        if self.views.picking {
            ui.label(self.view_pick_prompt());
            ui.small("Esc cancels. Backspace removes the last point.");
        }
        if pick || self.views.form.construction != previous_mode {
            if self.views.form.construction == Construction::Explicit {
                self.views.form.construction = Construction::ThreePoints;
            }
            self.start_view_picking();
        }
    }
    pub(super) fn view_property_actions(&mut self, ui: &mut egui::Ui) {
        let busy = self.views.job.is_some();
        let mut submit = false;
        ui.add_enabled_ui(!busy, |ui| {
            ui.horizontal(|ui| {
                submit = ui
                    .button(if self.views.picking {
                        "Pick in model"
                    } else if self.views.form.id.is_some() {
                        "Apply properties"
                    } else {
                        "Create view"
                    })
                    .clicked();
                if ui.button("Reset").clicked()
                    && let Some(view) = self
                        .views
                        .collection
                        .iter()
                        .find(|v| Some(v.id.to_string()) == self.views.form.id)
                {
                    self.views.form = Form::from_view(view);
                }
                if ui.button("Close").clicked() {
                    self.views.editing = false;
                    self.cancel_view_picking();
                }
            });
            if self.views.form.id.is_some() && ui.button("Save current camera").clicked() {
                let origin = self
                    .loaded
                    .as_ref()
                    .map_or([0.; 3], |l| l.summary.origin_mm);
                match self.camera.model_frame(origin) {
                    Ok((plane, span)) => {
                        self.views.form.plane = plane.expression();
                        self.views.form.rotation = ["0".into(), "0".into()];
                        self.views.form.span = span.to_string();
                        submit = true;
                    }
                    Err(error) => self.views.note = Some(Err(error.to_string())),
                }
            }
        });
        if busy {
            ui.spinner();
        }
        if submit {
            self.submit_view();
        }
        self.view_note(ui);
    }
    fn view_note(&self, ui: &mut egui::Ui) {
        if let Some(note) = &self.views.note {
            match note {
                Ok(text) => {
                    ui.label(text);
                }
                Err(text) => {
                    ui.colored_label(crate::theme::Palette::current(ui).error, text);
                }
            }
        }
    }
    pub(super) fn view_browser_contents(&mut self, ui: &mut egui::Ui) {
        self.views.preview_budget = true;
        ui.horizontal(|ui| {
            for (tab, label) in [
                (BrowserTab::Views, "Views"),
                (BrowserTab::Drawings, "Drawings"),
                (BrowserTab::Schedules, "Schedules"),
            ] {
                ui.selectable_value(&mut self.views.tab, tab, label);
            }
        });
        ui.separator();
        if self.views.tab != BrowserTab::Views {
            ui.label(if self.views.tab == BrowserTab::Drawings {
                "No drawing documents in this browser yet."
            } else {
                "No schedules in this browser yet."
            });
            return;
        }
        ui.add(
            egui::TextEdit::singleline(&mut self.browser_search)
                .hint_text("Find views")
                .char_limit(128)
                .desired_width(ui.available_width()),
        );
        ui.horizontal(|ui| {
            ui.menu_button("New view", |ui| {
                for (mode, label) in Construction::ALL {
                    if super::workbench::icon_menu_item(
                        ui,
                        super::workbench::view_construction_icon(mode),
                        label,
                    )
                    .clicked()
                    {
                        self.begin_view(mode);
                        ui.close();
                    }
                }
            });
            if ui.button("Refresh list").clicked() {
                self.view_command("model-views", Vec::new(), false);
            }
        });
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.views.quick_access, "Quick access");
            ui.checkbox(&mut self.views.thumbnails, "Previews");
        });
        ui.checkbox(&mut self.views.only_favorites, "Favorites only");
        if !self.views.selected.is_empty() {
            ui.horizontal(|ui| {
                ui.label(format!("{} selected", self.views.selected.len()));
                if ui.button("Open selected").clicked() {
                    for id in self.views.selected.clone() {
                        self.open_view_window(&id);
                    }
                }
            });
        }
        if ui.small_button("New folder").clicked() {
            self.views.new_folder = true;
        }
        if self.views.new_folder {
            text(ui, "Folder path", &mut self.views.destination_folder);
            ui.horizontal(|ui| {
                if ui.button("Create folder").clicked() {
                    self.views.new_folder = false;
                    self.view_command(
                        "create-view-folder",
                        vec!["--folder".into(), self.views.destination_folder.clone()],
                        false,
                    );
                }
                if ui.button("Cancel").clicked() {
                    self.views.new_folder = false;
                }
            });
        }
        if self.views.clipboard.is_some() {
            text(ui, "Paste into folder", &mut self.views.destination_folder);
            if ui.button("Paste view").clicked()
                && let Some((view, cut)) = self.views.clipboard.take()
            {
                self.views.form = Form::from_view(&view);
                self.views.form.folder = self.views.destination_folder.clone();
                if !cut {
                    self.views.form.id = None;
                    self.views.form.name.push_str(" copy");
                    self.views.form.construction = Construction::Explicit;
                }
                self.submit_view();
            }
        }
        let needle = self.browser_search.trim().to_lowercase();
        if ui
            .selectable_label(self.views.active.is_none(), "{3D}")
            .clicked()
        {
            self.activate_view(None);
        }
        let mut groups: BTreeMap<String, Vec<ModelView>> = BTreeMap::new();
        for view in &self.views.collection {
            if (!self.views.only_favorites || view.favorite)
                && (!self.views.quick_access
                    || view.favorite
                    || view.temporary
                    || self.views.open.contains(&view.id.to_string()))
                && format!("{} {}", view.name, view.folder)
                    .to_lowercase()
                    .contains(&needle)
            {
                groups
                    .entry(view.folder.clone())
                    .or_default()
                    .push(view.clone());
            }
        }
        if !self.views.quick_access && !self.views.only_favorites {
            for folder in &self.views.folders {
                if folder.to_lowercase().contains(&needle) {
                    groups.entry(folder.clone()).or_default();
                }
            }
        }
        egui::ScrollArea::vertical().show(ui, |ui| {
            for (folder, mut views) in groups {
                views.sort_by_key(|v| v.name.to_lowercase());
                let group = egui::CollapsingHeader::new(if folder.is_empty() {
                    "Model views"
                } else {
                    &folder
                })
                .default_open(true)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        if views.is_empty()
                            && !folder.is_empty()
                            && ui.small_button("Delete empty folder").clicked()
                        {
                            self.view_command(
                                "delete-view-folder",
                                vec!["--folder".into(), folder.clone()],
                                false,
                            );
                        }
                        if ui.small_button("Open folder").clicked() {
                            for view in &views {
                                self.open_view_window(&view.id.to_string());
                            }
                        }
                        if ui.small_button("Close folder").clicked() {
                            for view in &views {
                                let id = view.id.to_string();
                                if let Some(window) = self.views.windows.remove(&id) {
                                    self.state.renderer.write().free_texture(&window.texture);
                                }
                                self.views.open.retain(|v| v != &id);
                                if self.views.active.as_deref() == Some(id.as_str()) {
                                    self.activate_view(None);
                                }
                            }
                        }
                    });
                    let visible_order: Vec<String> =
                        views.iter().map(|v| v.id.to_string()).collect();
                    for view in views {
                        let id = view.id.to_string();
                        if self.views.thumbnails {
                            if let Some(texture) = self.view_thumbnail(ui.ctx(), &view) {
                                ui.image((texture, egui::vec2(180., 112.5)));
                            } else {
                                ui.small("Preparing preview...");
                            }
                        }
                        let response = ui.add(
                            egui::Button::selectable(
                                self.views.selected.contains(&id)
                                    || self.views.active.as_deref() == Some(&id),
                                format!(
                                    "{}{}{}",
                                    if view.favorite { "* " } else { "" },
                                    view.name,
                                    if view.temporary { " (temporary)" } else { "" }
                                ),
                            )
                            .sense(egui::Sense::click_and_drag()),
                        );
                        response.dnd_set_drag_payload(id.clone());
                        if response.clicked() {
                            let modifiers = ui.input(|i| i.modifiers);
                            if modifiers.shift {
                                let a = self.views.selection_anchor.as_ref().and_then(|anchor| {
                                    visible_order.iter().position(|v| v == anchor)
                                });
                                let b = visible_order.iter().position(|v| v == &id);
                                if let (Some(a), Some(b)) = (a, b)
                                    && let Some(range) = visible_order.get(a.min(b)..=a.max(b))
                                {
                                    self.views.selected.extend(range.iter().cloned());
                                }
                            } else if modifiers.ctrl {
                                if !self.views.selected.remove(&id) {
                                    self.views.selected.insert(id.clone());
                                }
                            } else {
                                self.views.selected.clear();
                                self.views.selected.insert(id.clone());
                                self.activate_view(Some(&id));
                            }
                            self.views.selection_anchor = Some(id.clone());
                        }
                        if response.double_clicked() {
                            self.set_bench(Bench::Select);
                            self.views.form = Form::from_view(&view);
                            self.views.editing = true;
                        }
                        if self.views.active.as_deref() == Some(id.as_str()) {
                            ui.small("Active");
                        } else if self.views.open.contains(&id) {
                            ui.small("Open");
                        }
                        response.context_menu(|ui| {
                            if ui.button("Open / activate").clicked() {
                                self.activate_view(Some(&id));
                                ui.close();
                            }
                            if ui.button("Open in another window").clicked() {
                                self.open_view_window(&id);
                                ui.close();
                            }
                            if ui.button("Properties / rename").clicked() {
                                self.set_bench(Bench::Select);
                                self.views.form = Form::from_view(&view);
                                self.views.editing = true;
                                ui.close();
                            }
                            if ui
                                .button(if view.favorite {
                                    "Remove favorite"
                                } else {
                                    "Add favorite"
                                })
                                .clicked()
                            {
                                self.views.form = Form::from_view(&view);
                                self.views.form.favorite = !view.favorite;
                                self.submit_view();
                                ui.close();
                            }
                            if ui.button("Duplicate").clicked() {
                                self.views.form = Form::from_view(&view);
                                self.views.form.id = None;
                                self.views.form.name.push_str(" copy");
                                self.views.form.construction = Construction::Explicit;
                                self.views.editing = true;
                                ui.close();
                            }
                            if ui.button("Cut to folder").clicked() {
                                self.views.clipboard = Some((view.clone(), true));
                                ui.close();
                            }
                            if ui.button("Copy to folder").clicked() {
                                self.views.clipboard = Some((view.clone(), false));
                                ui.close();
                            }
                            if ui.button("Close view").clicked() {
                                if self.views.active.as_deref() == Some(&id) {
                                    self.activate_view(None);
                                }
                                self.views.cameras.remove(&id);
                                self.views.open.retain(|v| v != &id);
                                if let Some(window) = self.views.windows.remove(&id) {
                                    self.state.renderer.write().free_texture(&window.texture);
                                }
                                ui.close();
                            }
                            if ui.button("Delete view").clicked() {
                                self.views.delete = Some(id.clone());
                                ui.close();
                            }
                        });
                    }
                });
                if let Some(id) = group.header_response.dnd_release_payload::<String>()
                    && let Some(view) = self
                        .views
                        .collection
                        .iter()
                        .find(|v| v.id.to_string() == *id)
                        .cloned()
                {
                    self.views.form = Form::from_view(&view);
                    self.views.form.folder = folder;
                    self.submit_view();
                }
            }
        });
        if let Some(id) = self.views.delete.clone() {
            ui.separator();
            ui.label("Delete this view? The model is unchanged.");
            ui.horizontal(|ui| {
                if ui.button("Delete view").clicked() {
                    let temporary = self
                        .views
                        .collection
                        .iter()
                        .any(|v| v.temporary && v.id.to_string() == id);
                    self.views
                        .collection
                        .retain(|v| !(v.temporary && v.id.to_string() == id));
                    self.views.delete = None;
                    if temporary {
                        if self.views.active.as_deref() == Some(id.as_str()) {
                            self.activate_view(None);
                        }
                        self.views.open.retain(|v| v != &id);
                    } else {
                        self.view_command("delete-model-view", vec!["--id".into(), id], false);
                    }
                }
                if ui.button("Keep view").clicked() {
                    self.views.delete = None;
                }
            });
        }
        self.view_note(ui);
    }
}
