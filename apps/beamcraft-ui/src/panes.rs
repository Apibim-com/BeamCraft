// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! The view panes of the work area: each one shows a model view (or the free 3D view) through
//! its own camera, in its own render mode, on its own offscreen target. Display state only: a
//! pane never touches the loaded snapshot, and the model is never written.

use crate::camera::Camera;
use beamcraft_engine::RenderMode;
use beamcraft_engine::views::ModelView;
use beamcraft_model::id::Uuid7;

/// A render target that is made on first use: the offscreen renderer and the egui texture that
/// shows it.
pub struct RenderTarget {
    renderer: Option<crate::render::Renderer>,
    texture: Option<egui::TextureId>,
}

impl RenderTarget {
    /// A target with nothing built yet.
    pub(crate) fn empty() -> Self {
        Self {
            renderer: None,
            texture: None,
        }
    }

    /// A target built already: a renderer and the egui texture registered for it.
    pub(crate) fn adopt(renderer: crate::render::Renderer, texture: egui::TextureId) -> Self {
        Self {
            renderer: Some(renderer),
            texture: Some(texture),
        }
    }

    /// The built size in pixels, `None` before the first build.
    pub(crate) fn size(&self) -> Option<[u32; 2]> {
        self.renderer.as_ref().map(|r| [r.width, r.height])
    }

    /// Resizes or creates the offscreen targets, keeping the egui texture in step. Returns the
    /// aspect of the drawn image. A zero size is refused. Records no timing: the caller decides
    /// what a resize counts as.
    pub(crate) fn resize(
        &mut self,
        state: &eframe::egui_wgpu::RenderState,
        width: u32,
        height: u32,
        samples: Option<u32>,
        stats: &crate::stats::Stats,
    ) -> Result<f32, crate::Error> {
        if width == 0 || height == 0 {
            // egui reports a zero-size content rect before the window lays out; wait for real
            // pixels rather than building empty targets.
            return Err(crate::Error::Input("window not laid out yet"));
        }
        if let Some(renderer) = &mut self.renderer {
            if (renderer.width != width || renderer.height != height)
                && renderer.resize(&state.device, width, height, stats)?
                && let Some(texture) = self.texture
            {
                // The targets changed: egui's copy of the texture follows them.
                state
                    .renderer
                    .write()
                    .update_egui_texture_from_wgpu_texture(
                        &state.device,
                        &renderer.view,
                        wgpu::FilterMode::Linear,
                        texture,
                    );
            }
        } else {
            let renderer = crate::render::Renderer::with_samples(
                &state.adapter,
                &state.device,
                width,
                height,
                samples,
                stats,
            )?;
            let texture = state.renderer.write().register_native_texture(
                &state.device,
                &renderer.view,
                wgpu::FilterMode::Linear,
            );
            self.texture = Some(texture);
            self.renderer = Some(renderer);
        }
        Ok(self.aspect())
    }

    /// The renderer, once built.
    pub(crate) fn renderer(&self) -> Option<&crate::render::Renderer> {
        self.renderer.as_ref()
    }

    /// The renderer for background and ghost writes, once built.
    pub(crate) fn renderer_mut(&mut self) -> Option<&mut crate::render::Renderer> {
        self.renderer.as_mut()
    }

    /// The egui texture of the drawn image, once built.
    pub(crate) fn texture(&self) -> Option<egui::TextureId> {
        self.texture
    }

    /// Width over height of the built targets, clamped to 0.05..=20; 1.6 before the first build
    /// (the aspect a new camera starts with).
    pub(crate) fn aspect(&self) -> f32 {
        self.renderer.as_ref().map_or(1.6, |r| {
            (r.width as f32 / r.height as f32).clamp(0.05, 20.0)
        })
    }

    /// Frees the egui texture; freeing twice is harmless.
    pub(crate) fn release(&mut self, state: &eframe::egui_wgpu::RenderState) {
        if let Some(texture) = self.texture.take() {
            state.renderer.write().free_texture(&texture);
        }
    }
}

/// One view pane of the work area: a model view or the free 3D view through its own camera, on
/// its own render target. The work area's `Vec` order is the z-order; the last pane is active.
pub struct ViewPane {
    /// Stable identity for egui ids; never reused within a window.
    pub(crate) key: u64,
    /// The view it shows; `None` is the free 3D view.
    pub(crate) view: Option<Uuid7>,
    /// Its own navigation camera.
    pub(crate) camera: Camera,
    /// Its own render mode, Selected only included.
    pub(crate) mode: RenderMode,
    /// Whether its grid guides show.
    pub(crate) grids: bool,
    /// The selected-only draw and the (selected id, scene origin) it was built for, so the one
    /// engine query runs on a selection or reload change, not per frame.
    pub(crate) selected_only: ((String, [f64; 3]), Option<crate::scene::SelectedDraw>),
    /// The camera Ctrl+P returns to.
    pub(crate) toggled: Option<Camera>,
    /// The free 3D camera and display, kept while this pane shows a view.
    pub(crate) spatial: Option<(Camera, RenderMode, bool)>,
    /// The scene framing the camera was last fitted to (centre, extent, aspect): a reload that
    /// reports the same framing keeps the user's view instead of fitting again.
    pub(crate) framed: Option<([f32; 3], f32, f32)>,
    /// The image rect drawn last frame, in points; 1280 x 800 before the first frame.
    pub(crate) body: egui::Rect,
    /// The title: the view's name, or "3D view"; rewritten whenever `view` changes.
    pub(crate) title: String,
    /// Its offscreen target.
    pub(crate) target: RenderTarget,
}

impl ViewPane {
    /// A pane on `view` (or the free 3D view) with an empty target; needs no GPU. A view brings
    /// its render mode and grid guides; the free 3D view starts shaded with grid guides on.
    pub(crate) fn new(view: Option<&ModelView>, camera: Camera, key: u64) -> Self {
        let mut pane = Self {
            key,
            view: None,
            camera,
            mode: view.map_or(RenderMode::default(), |v| v.render_mode),
            grids: view.is_none_or(|v| v.grids_visible),
            selected_only: ((String::new(), [0.0; 3]), None),
            toggled: None,
            spatial: None,
            framed: None,
            body: egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 800.0)),
            title: String::new(),
            target: RenderTarget::empty(),
        };
        pane.set_view(view);
        pane
    }

    /// Shows `view` (or the free 3D view) and rewrites the title; the only writer of `view`
    /// outside tests.
    pub(crate) fn set_view(&mut self, view: Option<&ModelView>) {
        self.view = view.map(|v| v.id);
        self.title = view.map_or_else(|| "3D view".to_owned(), |v| v.name.clone());
    }
}
