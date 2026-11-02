// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! The work area of the model window: the view pane drawn in the central panel, its navigation
//! and the input it hands to the tools. The active pane is the one selection, picking, snapping
//! and every tool work in; the accessors here read and write its display state.

use super::{App, View, annotation_panel, draw_selection, navigation, target_size, workbench};
use crate::edit::{ndc_of, ray_through};
use beamcraft_engine::RenderMode;
use std::time::Instant;

/// What a drawn pane hands to the input step.
pub(super) struct PaneDraw {
    /// The pane drawn.
    pub key: u64,
    /// The response of its image.
    pub response: egui::Response,
    /// The camera view it was drawn with.
    pub view: View,
}

impl App {
    /// Width over height of the active pane's image; 1.6 without a pane.
    pub(super) fn aspect(&self) -> f32 {
        self.views.active_pane().map_or(1.6, |p| p.target.aspect())
    }

    /// The render mode of the active pane.
    pub(super) fn active_mode(&self) -> RenderMode {
        self.views
            .active_pane()
            .map_or(RenderMode::Shaded, |p| p.mode)
    }

    /// Sets the render mode of the active pane; switching never reloads the scene.
    pub(super) fn set_active_mode(&mut self, mode: RenderMode) {
        if let Some(pane) = self.views.active_pane_mut() {
            pane.mode = mode;
        }
    }

    /// Whether the active pane shows its grid guides.
    pub(super) fn grids_shown(&self) -> bool {
        self.views.active_pane().is_some_and(|p| p.grids)
    }

    /// Shows or hides the grid guides of the active pane.
    pub(super) fn set_grids_shown(&mut self, on: bool) {
        if let Some(pane) = self.views.active_pane_mut() {
            pane.grids = on;
        }
    }

    /// Drops the ghost preview of every pane.
    pub(super) fn clear_ghost(&mut self) {
        for pane in &mut self.views.panes {
            if let Some(renderer) = pane.target.renderer_mut() {
                renderer.set_ghost(&self.state.device, &self.state.queue, None);
            }
        }
    }

    /// Hands the ghost preview to the active pane's renderer, the pane the pointer places in.
    pub(super) fn set_active_ghost(&mut self, ghost: Option<&crate::scene::GhostMesh>) {
        if let Some(renderer) = self
            .views
            .panes
            .last_mut()
            .and_then(|p| p.target.renderer_mut())
        {
            renderer.set_ghost(&self.state.device, &self.state.queue, ghost);
        }
    }

    /// Draws pane `key` over `body` (points): resizes its target, navigates its camera from
    /// `response` (the zoom keys only when `keys` is given), renders with its mode, the view's
    /// tint and clip and the scene's hidden kinds, then paints the grid guides, annotations and
    /// selection edges. `None` when the pane is gone or its camera makes no view (the error goes
    /// to `self.error`).
    pub(super) fn draw_pane(
        &mut self,
        ui: &mut egui::Ui,
        key: u64,
        body: egui::Rect,
        response: egui::Response,
        keys: Option<(bool, bool)>,
    ) -> Option<PaneDraw> {
        // Everything read from the views is taken before the pane is borrowed mutably.
        let shown = self
            .views
            .panes
            .iter()
            .find(|p| p.key == key)?
            .view
            .and_then(|id| self.views.book.get(&id));
        // The model view tints the model (V12); the free 3D view does not.
        let tint = crate::views::tint_of(shown);
        // The view adds its depth slab and work area to the display cut (V12).
        let cut = crate::views::display_clip(self.cut.as_ref(), shown);
        let scale = ui.ctx().pixels_per_point();
        let [width, height] = target_size([body.width(), body.height()], scale);
        let dark = ui.ctx().theme() == egui::Theme::Dark;
        let pane = self.views.panes.iter_mut().find(|p| p.key == key)?;
        pane.body = body;
        if pane.target.size() != Some([width, height]) {
            let started = Instant::now();
            // Only the targets follow the size; the shader and the pipeline stay.
            match pane
                .target
                .resize(&self.state, width, height, self.source.samples, &self.stats)
            {
                Ok(_) => self.stats.resize_ns.add_duration(started.elapsed()),
                Err(e) => self.error = Some(e.to_string()),
            }
        }
        if let Some(texture) = pane.target.texture() {
            ui.painter().image(
                texture,
                body,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );
        }
        let hovered = response.hovered();
        let (delta, shift, ctrl, scroll_y, pinch) = ui.input(|i| {
            (
                i.pointer.delta(),
                i.modifiers.shift,
                i.modifiers.ctrl,
                if hovered {
                    i.smooth_scroll_delta.y
                } else {
                    0.0
                },
                if hovered { i.zoom_delta() } else { 1.0 },
            )
        });
        let (zoom_in, zoom_out) = keys.unwrap_or((false, false));
        let (orbit, pan, zoom) = navigation(
            self.preset,
            delta,
            response.dragged_by(egui::PointerButton::Primary),
            response.dragged_by(egui::PointerButton::Secondary),
            response.dragged_by(egui::PointerButton::Middle),
            ctrl,
            shift,
            pane.camera.distance(),
            hovered,
            scroll_y,
            pinch,
            zoom_in,
            zoom_out,
            self.invert_zoom,
        );
        let aspect = pane.target.aspect();
        // Zoom toward the cursor: keep the point under the pointer still while zooming. A
        // hostile cursor or an extreme distance falls back to the view centre.
        let pan = if zoom == 0.0 {
            pan
        } else {
            // hover_pos (not interact_pointer_pos): egui fills the latter only while a
            // button is down on this widget, so wheel and pinch zoom need the hover point.
            let ndc = response.hover_pos().and_then(|pos| {
                ndc_of(
                    [pos.x, pos.y],
                    [response.rect.min.x, response.rect.min.y],
                    [response.rect.width(), response.rect.height()],
                )
            });
            match ndc.map(|n| pane.camera.cursor_zoom_pan(n, aspect, zoom)) {
                Some(Ok(extra)) => [pan[0] + extra[0], pan[1] + extra[1]],
                _ => pan,
            }
        };
        let mut view = match pane
            .camera
            .navigate(orbit, pan, zoom)
            .and_then(|()| pane.camera.view(aspect))
        {
            Ok(view) => view,
            Err(e) => {
                self.error = Some(e.to_string());
                return None;
            }
        };
        view.tint = tint;
        let rgb = if dark {
            [0x17, 0x21, 0x2B]
        } else {
            [0xED, 0xF1, 0xF4]
        };
        let [r, g, b] = rgb.map(|c: u8| f64::from(c) / 255.0);
        if let Some(renderer) = pane.target.renderer_mut() {
            renderer.background = wgpu::Color { r, g, b, a: 1.0 };
        }
        if let Some(l) = &self.loaded {
            // The display cut is uniform state: the scene and its snapshot never change.
            // Planes are model millimetres; the uniform is scene-centred.
            let clip = match &cut {
                Some(cut) => crate::cut_plane::ClipGpu::of(cut, l.summary.origin_mm),
                None => crate::cut_plane::ClipGpu::none(),
            };
            // One loaded scene, five drawing styles: the mode picks the pipeline and never
            // rewrites the snapshot or its token.
            if pane.mode == RenderMode::SelectedOnly {
                // The one engine query runs on a selection or reload change, not per frame;
                // the fields are disjoint, so the cache updates while the scene is borrowed.
                let key = (
                    self.editor
                        .selected()
                        .map(|s| s.id.clone())
                        .unwrap_or_default(),
                    l.summary.origin_mm,
                );
                if pane.selected_only.0 != key {
                    let draw = self
                        .editor
                        .selected()
                        .and_then(|s| beamcraft_engine::ElementId::parse(&s.id).ok())
                        .and_then(|id| {
                            crate::scene::SelectedDraw::of(&l.snapshot, &id, l.summary.origin_mm)
                        });
                    pane.selected_only = (key, draw);
                }
            }
            if let Some(renderer) = pane.target.renderer() {
                match pane.mode {
                    RenderMode::SelectedOnly => match &pane.selected_only.1 {
                        Some(selected) => {
                            renderer.draw_one(
                                &self.state.device,
                                &self.state.queue,
                                &l.uploaded,
                                view,
                                &self.stats,
                                &selected.digest,
                                &selected.instance,
                                &clip,
                            );
                        }
                        // Nothing selected: an empty view, not the whole model.
                        None => {
                            renderer.draw(
                                &self.state.device,
                                &self.state.queue,
                                &l.uploaded,
                                view,
                                &self.stats,
                                RenderMode::SelectedOnly,
                                &clip,
                            );
                        }
                    },
                    mode => {
                        renderer.draw(
                            &self.state.device,
                            &self.state.queue,
                            &l.uploaded,
                            view,
                            &self.stats,
                            mode,
                            &clip,
                        );
                    }
                }
            }
        }
        if let Some(l) = &self.loaded {
            if pane.grids {
                self.grid_scene.paint(
                    ui,
                    response.rect,
                    &view,
                    l.summary.origin_mm,
                    self.grid_edit.as_ref().map(|edit| edit.id.as_str()),
                );
            }
            if let Ok(rows) = self.annotation.rows.as_deref()
                && !rows.is_empty()
            {
                annotation_panel::paint(ui, response.rect, &view, l.summary.origin_mm, rows);
            }
            if let Some(selected) = self.editor.selected() {
                let key = (selected.id.clone(), l.snapshot.token().as_str().to_owned());
                if self.selection_edges.0 != key {
                    let edges = beamcraft_engine::ElementId::parse(&selected.id)
                        .ok()
                        .and_then(|id| {
                            beamcraft_engine::query::feature::element_edges(&l.snapshot, &id).ok()
                        })
                        .unwrap_or_default();
                    self.selection_edges = (key, edges);
                }
                let painter = ui.painter_at(response.rect);
                draw_selection(
                    &painter,
                    response.rect,
                    &view,
                    l.summary.origin_mm,
                    &self.selection_edges.1,
                );
            } else {
                self.selection_edges = Default::default();
            }
        }
        Some(PaneDraw {
            key,
            response,
            view,
        })
    }

    /// The active pane's click, hover previews and tool dispatch: a click selects, picks a grid
    /// line or a view point, or places a point of the armed tool. A pane that is not the active
    /// one takes none of them.
    pub(super) fn pane_input(&mut self, ui: &mut egui::Ui, draw: &PaneDraw) {
        if self.views.active_pane().map(|p| p.key) != Some(draw.key) {
            return;
        }
        let response = &draw.response;
        let view = &draw.view;
        let mut picked = None;
        let mut picked_grid = None;
        let finished_creation = self.finish_creation_gesture(response, view);
        // A closure gesture consumes the click before it can append another point.
        if !finished_creation
            && response.clicked_by(egui::PointerButton::Primary)
            && let Some(pos) = response.interact_pointer_pos()
            && let Some(ndc) = ndc_of(
                [pos.x, pos.y],
                [response.rect.min.x, response.rect.min.y],
                [response.rect.width(), response.rect.height()],
            )
            && let Some(ray) = ray_through(
                view,
                ndc,
                self.loaded
                    .as_ref()
                    .map_or([0.0; 3], |l| l.summary.origin_mm),
            )
        {
            // The grids switch (V11) takes grids out of picking; the display switch stays the
            // pane's grid guides.
            if !self.beam_mode
                && !self.plate_mode
                && self.views.picking.is_none()
                && self.grid_line.click.is_none()
                && self.grids_shown()
                && self
                    .selection
                    .is_on(beamcraft_engine::query::pick::SelectionClass::Grids)
            {
                picked_grid = self
                    .grid_scene
                    .hit_test(
                        response.rect,
                        view,
                        self.loaded
                            .as_ref()
                            .map_or([0.0; 3], |l| l.summary.origin_mm),
                        pos,
                    )
                    .map(|grid| grid.id.clone());
            }
            if picked_grid.is_none() {
                picked = Some(ray);
            }
        }
        if self.plate_mode {
            let resolved = response
                .hover_pos()
                .and_then(|pos| {
                    ndc_of(
                        [pos.x, pos.y],
                        [response.rect.min.x, response.rect.min.y],
                        [response.rect.width(), response.rect.height()],
                    )
                })
                .and_then(|ndc| {
                    ray_through(
                        view,
                        ndc,
                        self.loaded
                            .as_ref()
                            .map_or([0.0; 3], |l| l.summary.origin_mm),
                    )
                })
                .and_then(|ray| self.creation_hit(ray, true).ok());
            self.plate_hover = resolved.map(|(point, _)| point);
            self.snap_hover = resolved.and_then(|(_, kind)| kind);
            self.paint_plate_draft(ui, response.rect, view);
        }
        // The ghost preview follows the pointer on the plane through the armed start, parallel
        // to the view; the committed endpoint comes from the click.
        if self.beam_mode {
            match response.hover_pos().and_then(|pos| {
                ndc_of(
                    [pos.x, pos.y],
                    [response.rect.min.x, response.rect.min.y],
                    [response.rect.width(), response.rect.height()],
                )
            }) {
                Some(ndc) => {
                    self.update_ghost(view, ndc);
                    self.paint_member_draft(ui, response.rect, view);
                }
                None => {
                    self.member_cursor = None;
                    self.snap_hover = None;
                }
            }
        }
        if self.views.picking.is_some() {
            let hover = response.hover_pos().and_then(|pos| {
                ndc_of(
                    [pos.x, pos.y],
                    [response.rect.min.x, response.rect.min.y],
                    [response.rect.width(), response.rect.height()],
                )
                .and_then(|ndc| ray_through(view, ndc, self.view_origin()))
            });
            self.paint_view_picking(ui, response.rect, view, hover);
        }
        if let Some(id) = picked_grid {
            if !self.other_command_running() {
                self.select_grid(&id);
            }
        } else if let Some(ray) = picked {
            if self.views.picking.is_some() {
                self.view_click(ray);
            } else if self.plane_pick {
                if let Some(loaded) = &self.loaded {
                    match beamcraft_engine::work_plane::pick_work_plane(
                        &loaded.snapshot,
                        ray.origin,
                        ray.direction,
                    ) {
                        Ok(Some(plane)) => {
                            self.plane_form.set(plane);
                            self.install_plane(Some(plane));
                        }
                        Ok(None) => {
                            self.tool_note = Some(Err("No face was hit; pick a model face.".into()))
                        }
                        Err(e) => self.tool_note = Some(Err(e.to_string())),
                    }
                }
            } else if self.grid_line.click.is_some() {
                self.place_grid_line(ray);
            } else if self.plate_mode {
                self.place_plate(ray);
            } else if self.beam_mode {
                self.place_beam(ray);
            } else if self.bench == workbench::Bench::Annotations {
                self.annotation_click(ray);
            } else if !self.other_command_running() {
                self.start_select(ray);
            }
        }
    }
}
