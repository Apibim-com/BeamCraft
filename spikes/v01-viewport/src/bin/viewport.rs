// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim
//! Owner-run native eframe app; runtime Inter only, no embedded font or third-party artwork.
use beamcraft_v01_viewport_spike::{
    Error,
    camera::Camera,
    render::{Renderer, Strategy, Uploaded},
    scene,
};
use std::{path::Path, time::Instant};
struct App {
    device: wgpu::Device,
    queue: wgpu::Queue,
    renderer: Renderer,
    data: Uploaded,
    camera: Camera,
    texture: egui::TextureId,
    count: u32,
    strategy: Strategy,
    text: bool,
    adapter: String,
    error: Option<String>,
    last: Instant,
    fps: f32,
}
fn inter(ctx: &egui::Context) -> bool {
    let repo_path = Path::new("assets/fonts/inter/Inter-Regular.ttf");
    let spike_path = Path::new("../../assets/fonts/inter/Inter-Regular.ttf");
    let path = if repo_path.is_file() {
        repo_path
    } else {
        spike_path
    };
    // A bounded read, even if a local file is replaced while reading it.
    let bytes = (|| -> std::io::Result<Vec<u8>> {
        use std::io::Read;
        let file = std::fs::File::open(path)?;
        let mut bytes = Vec::new();
        file.take(4 * 1024 * 1024 + 1).read_to_end(&mut bytes)?;
        if bytes.is_empty() || bytes.len() > 4 * 1024 * 1024 {
            return Err(std::io::Error::other("Inter size limit"));
        }
        Ok(bytes)
    })();
    match bytes {
        Ok(bytes) => {
            let mut fonts = egui::FontDefinitions::empty();
            fonts
                .font_data
                .insert("Inter".into(), egui::FontData::from_owned(bytes).into());
            fonts
                .families
                .insert(egui::FontFamily::Proportional, vec!["Inter".into()]);
            fonts
                .families
                .insert(egui::FontFamily::Monospace, vec!["Inter".into()]);
            ctx.set_fonts(fonts);
            true
        }
        Err(e) => {
            eprintln!(
                "Inter unavailable ({e}); running without text. Camera mouse controls and keyboard counts remain available."
            );
            ctx.set_fonts(egui::FontDefinitions::empty());
            false
        }
    }
}
impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> Result<Self, Error> {
        let state = cc
            .wgpu_render_state
            .as_ref()
            .ok_or(Error::Input("eframe requires wgpu"))?;
        let text = inter(&cc.egui_ctx);
        let scene = scene::build(10_000)?;
        let camera = Camera::new(scene.target, scene.extent)?;
        let data = Uploaded::new(&state.device, &state.queue, &scene, Strategy::Instanced)?;
        let renderer = Renderer::new(&state.adapter, &state.device, 1024, 768)?;
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
        eprintln!("Window adapter: {adapter}");
        Ok(Self {
            device: state.device.clone(),
            queue: state.queue.clone(),
            renderer,
            data,
            camera,
            texture,
            count: 10_000,
            strategy: Strategy::Instanced,
            text,
            adapter,
            error: None,
            last: Instant::now(),
            fps: 0.0,
        })
    }
    fn rebuild(&mut self, count: u32, strategy: Strategy) -> Result<(), Error> {
        let scene = scene::build(count)?;
        let data = Uploaded::new(&self.device, &self.queue, &scene, strategy)?;
        let camera = Camera::new(scene.target, scene.extent)?;
        self.data = data;
        self.camera = camera;
        self.count = count;
        self.strategy = strategy;
        Ok(())
    }
}
impl eframe::App for App {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if ctx.input(|i| i.viewport().close_requested()) && craft_quit::quit() {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
        }
    }
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let now = Instant::now();
        let dt = now.duration_since(self.last).as_secs_f32();
        self.last = now;
        if dt > 0.0 {
            self.fps = self.fps * 0.9 + 0.1 / dt;
        }
        let mut count = self.count;
        let mut strategy = self.strategy;
        if self.text {
            ui.horizontal(|ui| {
                ui.label(format!(
                    "{:.1} FPS | {} objects | {}x MSAA",
                    self.fps, self.count, self.renderer.samples
                ));
                for n in [1_000, 10_000, 100_000] {
                    if ui.selectable_label(count == n, format!("{n}")).clicked() {
                        count = n;
                    }
                }
                if ui
                    .selectable_label(strategy == Strategy::Instanced, "Instanced")
                    .clicked()
                {
                    strategy = Strategy::Instanced;
                }
                if ui
                    .selectable_label(strategy == Strategy::Merged, "Merged")
                    .clicked()
                {
                    strategy = Strategy::Merged;
                }
            });
            ui.label(&self.adapter);
            ui.label("Drag: orbit | secondary drag: pan | wheel: zoom | 1/2/3: count | M: strategy | R: reset");
            if let Some(error) = &self.error {
                ui.colored_label(egui::Color32::RED, error);
            }
        }
        let (one, two, three, toggle, reset) = ui.input(|i| {
            (
                i.key_pressed(egui::Key::Num1),
                i.key_pressed(egui::Key::Num2),
                i.key_pressed(egui::Key::Num3),
                i.key_pressed(egui::Key::M),
                i.key_pressed(egui::Key::R),
            )
        });
        if one {
            count = 1_000;
        }
        if two {
            count = 10_000;
        }
        if three {
            count = 100_000;
        }
        if toggle {
            strategy = if self.strategy == Strategy::Instanced {
                Strategy::Merged
            } else {
                Strategy::Instanced
            };
        }
        if (count != self.count || strategy != self.strategy || reset)
            && let Err(e) = self.rebuild(count, strategy)
        {
            eprintln!("{e}");
            self.error = Some(e.to_string());
        }
        let size = ui.available_size();
        let scale = (size.x / 1024.0).min(size.y / 768.0).max(0.01);
        let response = ui.add(
            egui::Image::new((self.texture, egui::vec2(1024.0 * scale, 768.0 * scale)))
                .sense(egui::Sense::drag()),
        );
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
        match self
            .camera
            .navigate(orbit, pan, zoom)
            .and_then(|()| self.camera.view(1024.0 / 768.0))
        {
            Ok(view) => {
                self.renderer
                    .draw(&self.device, &self.queue, &self.data, view);
            }
            Err(e) => {
                self.error = Some(e.to_string());
            }
        }
        ui.ctx().request_repaint();
    }
}
fn main() -> eframe::Result {
    let mut setup = eframe::egui_wgpu::WgpuSetupCreateNew::without_display_handle();
    setup.instance_descriptor.backends =
        wgpu::Backends::DX12 | wgpu::Backends::VULKAN | wgpu::Backends::METAL;
    let options = eframe::NativeOptions {
        // Transparent procedural pixel; never display eframe's embedded logo as our brand.
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 900.0])
            .with_icon(egui::IconData {
                rgba: vec![0, 0, 0, 0],
                width: 1,
                height: 1,
            }),
        renderer: eframe::Renderer::Wgpu,
        wgpu_options: eframe::WgpuConfiguration {
            wgpu_setup: setup.into(),
            ..Default::default()
        },
        ..Default::default()
    };
    eframe::run_native(
        "BeamCraft V01 viewport spike",
        options,
        Box::new(|cc| Ok(Box::new(App::new(cc)?))),
    )
}
