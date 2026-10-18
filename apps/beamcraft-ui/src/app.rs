// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! The eframe window: a side panel with the revision, counts, units and adapter, and the viewport
//! with orbit (primary drag), pan (secondary drag), zoom (wheel), fit (F) and reload (R).

use crate::camera::Camera;
use crate::render::{Renderer, Uploaded};
use crate::scene::Scene;
use crate::{Error, check_font, source};
use beamcraft_engine::{Representation, ViewSnapshot};
use std::path::{Path, PathBuf};
use std::time::Instant;

/// Where the window reads its model from.
#[derive(Debug, Clone)]
pub struct Source {
    /// The published store.
    pub store: PathBuf,
    /// The `beamcraft` CLI that exports the snapshot.
    pub cli: PathBuf,
}

struct Loaded {
    snapshot: ViewSnapshot,
    scene: Scene,
    uploaded: Uploaded,
}

/// The window state.
pub struct App {
    state: eframe::egui_wgpu::RenderState,
    source: Source,
    renderer: Renderer,
    texture: egui::TextureId,
    camera: Camera,
    loaded: Option<Loaded>,
    error: Option<String>,
    text: bool,
    adapter: String,
    last: Instant,
    fps: f32,
}

/// Inter from the admitted asset directory, validated before egui can panic on it.
fn inter(ctx: &egui::Context) -> bool {
    let candidates = [
        Path::new("assets/fonts/inter/Inter-Regular.ttf").to_path_buf(),
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/fonts/inter/Inter-Regular.ttf"),
    ];
    let bytes = candidates.iter().find(|p| p.is_file()).map(|path| {
        use std::io::Read;
        let file = std::fs::File::open(path)?;
        let mut bytes = Vec::new();
        file.take(4 * 1024 * 1024 + 1).read_to_end(&mut bytes)?;
        if bytes.is_empty() || bytes.len() > 4 * 1024 * 1024 {
            return Err(std::io::Error::other("Inter size limit"));
        }
        check_font(&bytes).map_err(std::io::Error::other)?;
        Ok(bytes)
    });
    match bytes {
        Some(Ok(bytes)) => {
            let mut fonts = egui::FontDefinitions::empty();
            fonts
                .font_data
                .insert("Inter".into(), egui::FontData::from_owned(bytes).into());
            for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
                fonts.families.insert(family, vec!["Inter".into()]);
            }
            ctx.set_fonts(fonts);
            true
        }
        _ => {
            eprintln!("Inter is unavailable; the window runs without text.");
            ctx.set_fonts(egui::FontDefinitions::empty());
            false
        }
    }
}

/// A contributor-original 32 x 32 window icon (an I-section silhouette), drawn in code. It
/// replaces eframe's built-in logo, which is not a BeamCraft mark.
pub fn icon() -> egui::IconData {
    let mut rgba = Vec::with_capacity(32 * 32 * 4);
    for y in 0..32u32 {
        for x in 0..32u32 {
            let flange = (4..10).contains(&y) || (22..28).contains(&y);
            let web = (13..19).contains(&x) && (10..22).contains(&y);
            let inside = (4..28).contains(&x) && (flange || web);
            rgba.extend_from_slice(if inside {
                &[46, 140, 209, 255]
            } else {
                &[0, 0, 0, 0]
            });
        }
    }
    egui::IconData {
        rgba,
        width: 32,
        height: 32,
    }
}

impl App {
    /// Creates the window state and loads the accepted snapshot.
    pub fn new(cc: &eframe::CreationContext<'_>, source: Source) -> Result<Self, Error> {
        let state = cc
            .wgpu_render_state
            .clone()
            .ok_or(Error::Input("eframe requires wgpu"))?;
        let text = inter(&cc.egui_ctx);
        let renderer = Renderer::new(&state.adapter, &state.device, 1280, 800)?;
        let texture = state.renderer.write().register_native_texture(
            &state.device,
            &renderer.view,
            wgpu::FilterMode::Linear,
        );
        let info = state.adapter.get_info();
        let adapter = format!(
            "{} / {:?} / {:?}",
            info.name, info.device_type, info.backend
        );
        let mut app = Self {
            state,
            source,
            renderer,
            texture,
            camera: Camera::new([0.0; 3], 1000.0)?,
            loaded: None,
            error: None,
            text,
            adapter,
            last: Instant::now(),
            fps: 0.0,
        };
        app.reload();
        Ok(app)
    }

    fn reload(&mut self) {
        let result = (|| -> Result<Loaded, Error> {
            let snapshot = source::load(&self.source.cli, &self.source.store)?;
            let scene = Scene::from_snapshot(&snapshot, Representation::Full)?;
            let uploaded = Uploaded::new(&self.state.device, &self.state.queue, &scene)?;
            Ok(Loaded {
                snapshot,
                scene,
                uploaded,
            })
        })();
        match result {
            Ok(loaded) => {
                let (target, extent) = loaded.scene.framing();
                if let Err(e) = self.camera.fit(target, extent) {
                    self.error = Some(e.to_string());
                }
                self.loaded = Some(loaded);
                self.error = None;
            }
            Err(e) => self.error = Some(e.to_string()),
        }
    }

    fn resize(&mut self, width: u32, height: u32) -> Result<(), Error> {
        if width == self.renderer.width && height == self.renderer.height {
            return Ok(());
        }
        let renderer = Renderer::new(&self.state.adapter, &self.state.device, width, height)?;
        self.state
            .renderer
            .write()
            .update_egui_texture_from_wgpu_texture(
                &self.state.device,
                &renderer.view,
                wgpu::FilterMode::Linear,
                self.texture,
            );
        self.renderer = renderer;
        Ok(())
    }

    fn panel(&self, ui: &mut egui::Ui) {
        ui.heading("BeamCraft");
        ui.label(format!("{:.0} FPS", self.fps));
        ui.label(&self.adapter);
        ui.label(format!("{}x MSAA", self.renderer.samples));
        ui.separator();
        match &self.loaded {
            Some(l) => {
                let pin = l.snapshot.pin();
                let (meshes, instances) = l.scene.counts();
                let revision = pin.revision().to_string();
                ui.label(format!(
                    "Revision {}… generation {}",
                    revision.get(..12).unwrap_or(&revision),
                    pin.generation()
                ));
                ui.label(format!("Elements: {}", l.scene.elements));
                ui.label(format!("Meshes: {meshes} (instances: {instances})"));
                ui.label(format!("Triangles: {}", l.scene.triangles));
                let size = [0, 1, 2].map(|a| {
                    f64::from(l.scene.max.get(a).copied().unwrap_or(0.0))
                        - f64::from(l.scene.min.get(a).copied().unwrap_or(0.0))
                });
                ui.label(format!(
                    "Bounds: {:.3} x {:.3} x {:.3} m (model units: mm)",
                    size[0] / 1000.0,
                    size[1] / 1000.0,
                    size[2] / 1000.0
                ));
            }
            None => {
                ui.label("No model loaded");
            }
        }
        ui.separator();
        ui.label("Drag: orbit | secondary drag: pan | wheel: zoom | F: fit | R: reload");
        if let Some(error) = &self.error {
            ui.colored_label(egui::Color32::from_rgb(230, 80, 60), error);
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let now = Instant::now();
        let dt = now.duration_since(self.last).as_secs_f32();
        self.last = now;
        if dt > 0.0 {
            self.fps = self.fps * 0.9 + 0.1 / dt;
        }
        let (fit, reload) =
            ui.input(|i| (i.key_pressed(egui::Key::F), i.key_pressed(egui::Key::R)));
        if reload {
            self.reload();
        }
        if fit && let Some(l) = &self.loaded {
            let (target, extent) = l.scene.framing();
            if let Err(e) = self.camera.fit(target, extent) {
                self.error = Some(e.to_string());
            }
        }
        if self.text {
            egui::Panel::left("info").show(ui, |ui| self.panel(ui));
        }
        egui::CentralPanel::default().show(ui, |ui| {
            let size = ui.available_size();
            let scale = ui.ctx().pixels_per_point();
            let width = ((size.x * scale) as u32).clamp(1, 4096);
            let height = ((size.y * scale) as u32).clamp(1, 4096);
            if let Err(e) = self.resize(width, height) {
                self.error = Some(e.to_string());
            }
            let response =
                ui.add(egui::Image::new((self.texture, size)).sense(egui::Sense::drag()));
            let delta = ui.input(|i| i.pointer.delta());
            let orbit = if response.dragged_by(egui::PointerButton::Primary) {
                [delta.x * 0.008, -delta.y * 0.008]
            } else {
                [0.0; 2]
            };
            let pan = if response.dragged_by(egui::PointerButton::Secondary) {
                let factor = self.camera.distance() * 0.0008;
                [-delta.x * factor, delta.y * factor]
            } else {
                [0.0; 2]
            };
            let zoom = if response.hovered() {
                ui.input(|i| i.smooth_scroll_delta.y * 0.002)
            } else {
                0.0
            };
            let aspect = self.renderer.width as f32 / self.renderer.height as f32;
            match self
                .camera
                .navigate(orbit, pan, zoom)
                .and_then(|()| self.camera.view(aspect.clamp(0.05, 20.0)))
            {
                Ok(view) => {
                    if let Some(l) = &self.loaded {
                        self.renderer.draw(
                            &self.state.device,
                            &self.state.queue,
                            &l.uploaded,
                            view,
                        );
                    }
                }
                Err(e) => self.error = Some(e.to_string()),
            }
        });
        ui.ctx().request_repaint();
    }
}
