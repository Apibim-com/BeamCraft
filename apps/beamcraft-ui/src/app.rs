// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! The eframe window: a side panel with the revision, counts, units and adapter, and the viewport
//! with orbit (primary drag), pan (secondary drag), zoom (wheel), fit (F) and reload (R).

use crate::camera::{Camera, View};
use crate::command::Selected;
use crate::edit::{EditKind, Edited, Editor, Epoch, Target, ndc_of, ray_through};
use crate::props::{Properties, State};
use crate::publish::{Phase, Published, Publisher, Status};
use crate::render::{Renderer, Uploaded};
use crate::scene::Scene;
use crate::source::Reloaded;
use crate::tools::{Choice, Created, MAX_TEXT, MemberKind, ToolError, ToolKind, Tools};
use crate::{Error, check_font, command, source};
use beamcraft_engine::ViewSnapshot;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, TryRecvError};
use std::time::Instant;

/// Where the window reads its model from.
#[derive(Debug, Clone)]
pub struct Source {
    /// The published store.
    pub store: PathBuf,
    /// The `beamcraft` CLI that exports the snapshot.
    pub cli: PathBuf,
    /// The personal workspace directory when the window was opened on one (`--workspace`): it is
    /// then also `store`, and undo and redo act on it.
    pub workspace: Option<PathBuf>,
    /// The published store the workspace was forked from (`--published`), the target of Publish.
    pub published: Option<PathBuf>,
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
    tools: Tools,
    job: Option<Receiver<Result<Created, ToolError>>>,
    reload_job: Option<Receiver<Reloaded>>,
    reload_queued: bool,
    /// The result of the last Preview, Cancel or Commit.
    tool_note: Option<Result<String, String>>,
    /// Why the last definitions refresh failed; kept apart from `tool_note` so neither hides the other.
    refresh_error: Option<String>,
    /// The scene framing the camera was last fitted to (centre, extent, aspect): a reload that
    /// reports the same framing keeps the user's view instead of fitting again.
    framed: Option<([f32; 3], f32, f32)>,
    editor: Editor,
    edit_job: Option<Receiver<Result<Edited, ToolError>>>,
    select_job: Option<SelectJob>,
    select_epoch: Epoch,
    /// The result of the last select, edit, undo or redo.
    edit_note: Option<Result<String, String>>,
    props: Properties,
    props_job: Option<Receiver<Result<Edited, ToolError>>>,
    /// The result of the last property Preview, Cancel or Commit.
    props_note: Option<Result<String, String>>,
    publisher: Publisher,
    publish_job: Option<Receiver<Result<Published, ToolError>>>,
    status_job: Option<Receiver<Result<Status, ToolError>>>,
    status_queued: bool,
}

/// A lookup in flight: the epoch it was asked under and its receiver.
type SelectJob = (u64, Receiver<Result<Option<Selected>, ToolError>>);

const RED: egui::Color32 = egui::Color32::from_rgb(230, 80, 60);

/// A drop-down of store definitions; `required` removes the "none" entry.
fn choice_combo(
    ui: &mut egui::Ui,
    id: &str,
    label: &str,
    current: &mut Option<String>,
    choices: &[Choice],
    required: bool,
) {
    let selected = current
        .as_ref()
        .map(|digest| {
            choices.iter().find(|c| &c.digest == digest).map_or_else(
                || format!("unlisted definition {}", digest.get(..8).unwrap_or(digest)),
                |c| c.label.clone(),
            )
        })
        .unwrap_or_else(|| if required { "choose…" } else { "none" }.to_owned());
    // The label sits left of the list (as with the text fields): drawn below it, it read as
    // the label of the next drop-down.
    ui.horizontal(|ui| {
        ui.label(label);
        egui::ComboBox::from_id_salt(id)
            .width(200.0)
            .selected_text(selected)
            .show_ui(ui, |ui| {
                if !required {
                    ui.selectable_value(current, None, "none");
                }
                for choice in choices {
                    ui.selectable_value(current, Some(choice.digest.clone()), &choice.label);
                }
            });
    });
}

fn point_fields(ui: &mut egui::Ui, point: &mut [String; 3]) {
    ui.horizontal(|ui| {
        for (axis, text) in ["X", "Y", "Z"].iter().zip(point.iter_mut()) {
            ui.label(*axis);
            ui.add(
                egui::TextEdit::singleline(text)
                    .desired_width(64.0)
                    .char_limit(MAX_TEXT),
            );
        }
    });
}

/// The twelve edges of a box as pairs of corner indices (bit 0 = x, bit 1 = y, bit 2 = z).
const BOX_EDGES: [(usize, usize); 12] = [
    (0, 1),
    (2, 3),
    (4, 5),
    (6, 7),
    (0, 2),
    (1, 3),
    (4, 6),
    (5, 7),
    (0, 4),
    (1, 5),
    (2, 6),
    (3, 7),
];

/// Draws the selection as the projected box of the engine's world bounds of the element.
fn draw_selection(
    painter: &egui::Painter,
    rect: egui::Rect,
    view: &View,
    loaded: &Loaded,
    id: &str,
) {
    let Ok(id) = beamcraft_engine::ElementId::parse(id) else {
        return;
    };
    let Ok(bounds) = loaded.snapshot.element_bounds(&id) else {
        return;
    };
    let origin = loaded.scene.origin_mm;
    let screen: Vec<Option<egui::Pos2>> = (0..8usize)
        .map(|corner| {
            let pick = |axis: usize| {
                let on = corner >> axis & 1 == 1;
                let edge = if on {
                    bounds.max.get(axis)
                } else {
                    bounds.min.get(axis)
                };
                edge.copied().unwrap_or(0.0) - origin.get(axis).copied().unwrap_or(0.0)
            };
            let [x, y, z] = view.project([pick(0) as f32, pick(1) as f32, pick(2) as f32]);
            ((0.0..1.0).contains(&z) && x.is_finite() && y.is_finite()).then(|| {
                egui::pos2(
                    rect.min.x + (x * 0.5 + 0.5) * rect.width(),
                    rect.min.y + (0.5 - y * 0.5) * rect.height(),
                )
            })
        })
        .collect();
    let stroke = egui::Stroke::new(2.0, egui::Color32::from_rgb(255, 190, 40));
    for (a, b) in BOX_EDGES {
        if let (Some(Some(p)), Some(Some(q))) = (screen.get(a), screen.get(b)) {
            painter.line_segment([*p, *q], stroke);
        }
    }
}

fn field(ui: &mut egui::Ui, label: &str, text: &mut String, width: f32) {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.add(
            egui::TextEdit::singleline(text)
                .desired_width(width)
                .char_limit(MAX_TEXT),
        );
    });
}

/// The offscreen render target size in device pixels: the viewport with its longest side clamped
/// to the renderer's 4096 limit, scaled as a whole. A target that keeps the viewport's aspect
/// ratio can be drawn over the whole panel without stretching the model (a wide window used to
/// widen the picture by the clamped-away part).
fn target_size(size: [f32; 2], scale: f32) -> [u32; 2] {
    let longest = size.iter().fold(0.0f32, |a, v| a.max(*v)) * scale;
    let factor = (4096.0 / longest).min(1.0);
    [
        ((size[0] * scale * factor) as u32).clamp(1, 4096),
        ((size[1] * scale * factor) as u32).clamp(1, 4096),
    ]
}

/// True when the camera should follow the framing of a reloaded scene: nothing was framed yet,
/// or the bounds or the viewport changed. An unchanged scene keeps the user's view, so a Commit
/// or `R` that altered no bounds does not jump back to the whole model.
fn needs_refit(
    framed: Option<([f32; 3], f32, f32)>,
    target: [f32; 3],
    extent: f32,
    aspect: f32,
) -> bool {
    match framed {
        Some((old_target, old_extent, old_aspect)) => {
            old_target != target || old_extent != extent || old_aspect != aspect
        }
        None => true,
    }
}

/// The volume part of a commit note: cubic millimetres below a thousandth of a cubic metre,
/// cubic metres from there, so a small plate does not read as `0.000 m3`.
fn volume_note(volume_mm3: f64) -> String {
    if volume_mm3 < 1e6 {
        format!(", volume {volume_mm3:.0} mm3")
    } else {
        format!(", volume {:.3} m3", volume_mm3 / 1e9)
    }
}

/// Inter from the admitted asset directory, validated before egui can panic on it.
fn inter(ctx: &egui::Context) -> bool {
    // CWD (an extracted package root), the source tree, then next to the executable: the
    // win/linux package puts assets beside it, the macOS .app bundle under its Resources
    // directory (Q02 packaging).
    let mut candidates = vec![
        Path::new("assets/fonts/inter/Inter-Regular.ttf").to_path_buf(),
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/fonts/inter/Inter-Regular.ttf"),
    ];
    if let Ok(exe) = std::env::current_exe()
        && let Some(dir) = exe.parent()
    {
        candidates.push(dir.join("assets/fonts/inter/Inter-Regular.ttf"));
        candidates.push(dir.join("../Resources/assets/fonts/inter/Inter-Regular.ttf"));
    }
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
        let publisher = Publisher::new(source.published.clone(), source.workspace.clone());
        let mut app = Self {
            state,
            source,
            renderer,
            texture,
            camera: Camera::new([0.0; 3], 1000.0, 1.6)?,
            loaded: None,
            error: None,
            text,
            adapter,
            last: Instant::now(),
            fps: 0.0,
            tools: Tools::default(),
            job: None,
            reload_job: None,
            reload_queued: false,
            tool_note: None,
            refresh_error: None,
            framed: None,
            editor: Editor::default(),
            edit_job: None,
            select_job: None,
            select_epoch: Epoch::default(),
            edit_note: None,
            props: Properties::default(),
            props_job: None,
            props_note: None,
            publisher,
            publish_job: None,
            status_job: None,
            status_queued: false,
        };
        app.reload();
        app.refresh_status();
        Ok(app)
    }

    /// Starts a refresh on a worker thread (the CLI queries and the scene build); the result is
    /// applied by `poll_reload`. A refresh asked for while one runs is queued, so it still reads
    /// the store after whatever just changed it.
    fn reload(&mut self) {
        if self.reload_job.is_some() {
            self.reload_queued = true;
            return;
        }
        let (cli, store) = (self.source.cli.clone(), self.source.store.clone());
        let (tx, rx) = std::sync::mpsc::channel();
        let spawned = std::thread::Builder::new()
            .name("beamcraft-reload".into())
            .spawn(move || {
                let _ = tx.send(source::reload(&cli, &store));
            });
        match spawned {
            Ok(_) => self.reload_job = Some(rx),
            Err(_) => self.error = Some("the reload thread could not be started".into()),
        }
    }

    fn poll_reload(&mut self) {
        let Some(rx) = &self.reload_job else { return };
        let reloaded = match rx.try_recv() {
            Ok(reloaded) => reloaded,
            Err(TryRecvError::Empty) => return,
            Err(TryRecvError::Disconnected) => {
                self.reload_job = None;
                self.error = Some("the reload thread ended without a result".into());
                return;
            }
        };
        self.reload_job = None;
        // The listing carries the head it was read from; an unreadable listing leaves it unknown.
        self.props.observe_head(
            reloaded
                .definitions
                .as_ref()
                .ok()
                .and_then(|d| d.head.clone()),
        );
        self.refresh_error = self.tools.apply_definitions(reloaded.definitions);
        let result = reloaded.scene.and_then(|(snapshot, scene)| {
            let uploaded = Uploaded::new(&self.state.device, &self.state.queue, &scene)?;
            Ok(Loaded {
                snapshot,
                scene,
                uploaded,
            })
        });
        match result {
            Ok(loaded) => {
                let (target, extent) = loaded.scene.framing();
                let aspect = self.aspect();
                // A scene the camera cannot frame keeps the previous scene and shows why. An
                // unchanged framing keeps the user's camera and the last rendered frame.
                if needs_refit(self.framed, target, extent, aspect) {
                    match self.camera.fit(target, extent, aspect) {
                        Ok(()) => {
                            self.loaded = Some(loaded);
                            self.framed = Some((target, extent, aspect));
                            self.error = None;
                        }
                        Err(e) => self.error = Some(e.to_string()),
                    }
                } else {
                    self.loaded = Some(loaded);
                    self.error = None;
                }
            }
            // A store with no revision yet is a normal state, not an error: the definitions
            // listing treats the same NOT_FOUND the same way.
            Err(e) if source::is_no_revision(&e) => {
                self.loaded = None;
                self.framed = None;
                self.error = None;
            }
            Err(e) => self.error = Some(e.to_string()),
        }
        if std::mem::take(&mut self.reload_queued) {
            self.reload();
        }
    }
    fn aspect(&self) -> f32 {
        (self.renderer.width as f32 / self.renderer.height as f32).clamp(0.05, 20.0)
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

    fn panel(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            self.info(ui);
            ui.separator();
            self.tools_panel(ui);
            ui.separator();
            self.edit_panel(ui);
            ui.separator();
            self.props_panel(ui);
            ui.separator();
            self.publish_panel(ui);
        });
    }

    fn target(&self) -> Target<'_> {
        Target {
            store: &self.source.store,
            workspace: self.source.workspace.as_deref(),
        }
    }

    /// Starts a click selection on a worker: the engine's `pick` along the ray, then the details
    /// of the element it hit.
    fn start_select(&mut self, ray: crate::edit::Ray) {
        if self.select_job.is_some() {
            return;
        }
        let (cli, store) = (self.source.cli.clone(), self.source.store.clone());
        let (tx, rx) = std::sync::mpsc::channel();
        self.select_job = Some((self.select_epoch.advance(), rx));
        let spawned = std::thread::Builder::new()
            .name("beamcraft-pick".into())
            .spawn(move || {
                let _ = tx.send(command::select_at(&cli, &store, &ray));
            });
        if spawned.is_err() {
            self.select_job = None;
            self.edit_note = Some(Err("the pick thread could not be started".into()));
        }
    }

    /// Drops any lookup in flight: its result is no longer wanted (Esc, an accepted edit).
    fn invalidate_select(&mut self) {
        self.select_epoch.advance();
        self.select_job = None;
    }

    fn poll_select(&mut self) {
        let Some((asked, rx)) = &self.select_job else {
            return;
        };
        let asked = *asked;
        let result = match rx.try_recv() {
            Ok(result) => result,
            Err(TryRecvError::Empty) => return,
            Err(TryRecvError::Disconnected) => Err(ToolError {
                code: "CLI_FAILED".into(),
                message: "the pick thread ended without a result".into(),
            }),
        };
        self.select_job = None;
        if !self.select_epoch.is_current(asked) {
            return;
        }
        match result {
            Ok(None) => {
                self.editor.deselect();
                self.props.clear();
                self.edit_note = None;
            }
            Ok(Some(Selected { id, info, details })) => {
                match info {
                    Ok(info) => {
                        self.editor.select(info);
                        self.edit_note = None;
                    }
                    Err(e) => {
                        self.editor.select_id(id.clone());
                        self.edit_note = Some(Err(format!("details: {e}")));
                    }
                }
                if self.props.selected() != Some(id.as_str()) {
                    self.props_note = None;
                }
                self.props.load(&id, details);
            }
            Err(e) => self.edit_note = Some(Err(e.to_string())),
        }
    }

    fn start_edit(&mut self, direct: Option<EditKind>) {
        if self.other_command_running() {
            self.edit_note = Some(Err(
                "another command is running; wait for it to finish.".into()
            ));
            return;
        }
        let target = Target {
            store: &self.source.store,
            workspace: self.source.workspace.as_deref(),
        };
        let edit = match direct {
            Some(kind) => match self.editor.begin_direct(kind, target) {
                Ok(edit) => edit,
                Err(e) => {
                    self.edit_note = Some(Err(e.to_string()));
                    return;
                }
            },
            None => self.editor.begin_commit(target),
        };
        let Some(edit) = edit else { return };
        let cli = self.source.cli.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        self.edit_job = Some(rx);
        self.edit_note = None;
        let spawned = std::thread::Builder::new()
            .name("beamcraft-edit".into())
            .spawn(move || {
                let _ = tx.send(command::commit_edit(&cli, &edit));
            });
        if spawned.is_err() {
            self.edit_job = None;
            self.editor.finish();
            self.edit_note = Some(Err("the edit thread could not be started".into()));
        }
    }

    fn poll_edit(&mut self) {
        let Some(rx) = &self.edit_job else { return };
        let result = match rx.try_recv() {
            Ok(result) => result,
            Err(TryRecvError::Empty) => return,
            Err(TryRecvError::Disconnected) => Err(ToolError {
                code: "CLI_FAILED".into(),
                message: "the edit thread ended without a result".into(),
            }),
        };
        self.edit_job = None;
        self.editor.finish();
        match result {
            Ok(done) => {
                // A lookup that started before this edit describes the element as it was.
                self.invalidate_select();
                self.reload();
                let subject = match done.kind {
                    EditKind::Undo | EditKind::Redo => "the workspace".to_owned(),
                    _ => done.element_id.clone().unwrap_or_default(),
                };
                self.edit_note = Some(Ok(format!(
                    "{} {} (accepted revision {}, sequence {}).",
                    done.kind.done(),
                    subject,
                    done.manifest_digest
                        .get(..12)
                        .unwrap_or(&done.manifest_digest),
                    done.sequence
                )));
                match done.kind {
                    EditKind::Delete => {
                        self.editor.deselect();
                        self.props.clear();
                    }
                    // The element may have moved, or the undo may have removed it: ask again.
                    _ => self.refresh_selection(),
                }
                self.refresh_status();
            }
            Err(e) => {
                if e.code == "OUTCOME_UNKNOWN" {
                    self.invalidate_select();
                    self.reload();
                    self.refresh_selection();
                    self.refresh_status();
                }
                self.edit_note = Some(Err(e.to_string()));
            }
        }
    }

    /// Re-reads the selected element from the accepted revision (it may have moved or gone).
    fn refresh_selection(&mut self) {
        let Some(id) = self.editor.selected().map(|s| s.id.clone()) else {
            return;
        };
        let (cli, store) = (self.source.cli.clone(), self.source.store.clone());
        let (tx, rx) = std::sync::mpsc::channel();
        self.select_job = Some((self.select_epoch.advance(), rx));
        let spawned = std::thread::Builder::new()
            .name("beamcraft-info".into())
            .spawn(move || {
                // An element the revision no longer holds clears the selection; any other
                // failure keeps it selected without details, so the move form is emptied
                // rather than left with the position from before the edit.
                let reply = match command::element_details(&cli, &store, &id) {
                    Err(e) if e.code == "NOT_FOUND" => None,
                    details => Some(Selected::from_details(id.clone(), details)),
                };
                let _ = tx.send(Ok(reply));
            });
        if spawned.is_err() {
            self.select_job = None;
        }
    }

    /// True while a command that writes to the store (or publishes it) is running: the property,
    /// publish and edit actions never run at the same time.
    fn other_command_running(&self) -> bool {
        self.props_job.is_some()
            || self.publish_job.is_some()
            || self.edit_job.is_some()
            || self.job.is_some()
    }

    /// Starts the committed property edit on a worker: the same `update-element` the CLI runs.
    fn start_property_commit(&mut self) {
        if self.other_command_running() {
            self.props_note = Some(Err(
                "another command is running; wait for it to finish.".into()
            ));
            return;
        }
        let Some(edit) = self.props.begin_commit(&self.source.store) else {
            return;
        };
        let cli = self.source.cli.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        self.props_job = Some(rx);
        self.props_note = None;
        let spawned = std::thread::Builder::new()
            .name("beamcraft-properties".into())
            .spawn(move || {
                let _ = tx.send(command::commit_property(&cli, &edit));
            });
        if spawned.is_err() {
            self.props_job = None;
            self.props.finish();
            self.props_note = Some(Err("the command thread could not be started".into()));
        }
    }

    fn poll_props(&mut self) {
        let Some(rx) = &self.props_job else { return };
        let result = match rx.try_recv() {
            Ok(result) => result,
            Err(TryRecvError::Empty) => return,
            Err(TryRecvError::Disconnected) => Err(ToolError {
                code: "CLI_FAILED".into(),
                message: "the command thread ended without a result".into(),
            }),
        };
        self.props_job = None;
        self.props.finish();
        match result {
            Ok(done) => {
                self.invalidate_select();
                self.reload();
                self.props_note = Some(Ok(format!(
                    "Updated {} (accepted revision {}, sequence {}).",
                    done.element_id.as_deref().unwrap_or("the element"),
                    done.manifest_digest
                        .get(..12)
                        .unwrap_or(&done.manifest_digest),
                    done.sequence
                )));
                self.refresh_selection();
                self.refresh_status();
            }
            Err(e) => {
                if e.code == "OUTCOME_UNKNOWN" {
                    self.invalidate_select();
                    self.reload();
                    self.refresh_selection();
                    self.refresh_status();
                }
                self.props_note = Some(Err(e.to_string()));
            }
        }
    }

    /// Reads the workspace status on a worker when the window can publish.
    fn refresh_status(&mut self) {
        let Ok((published, workspace)) = self.publisher.target() else {
            return;
        };
        if self.status_job.is_some() {
            self.status_queued = true;
            return;
        }
        let (published, workspace) = (published.to_path_buf(), workspace.to_path_buf());
        let cli = self.source.cli.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        let spawned = std::thread::Builder::new()
            .name("beamcraft-status".into())
            .spawn(move || {
                let _ = tx.send(command::workspace_status(&cli, &published, &workspace));
            });
        if spawned.is_ok() {
            self.status_job = Some(rx);
        }
    }

    fn poll_status(&mut self) {
        let Some(rx) = &self.status_job else { return };
        let result = match rx.try_recv() {
            Ok(result) => result,
            Err(TryRecvError::Empty) => return,
            Err(TryRecvError::Disconnected) => Err(ToolError {
                code: "CLI_FAILED".into(),
                message: "the status thread ended without a result".into(),
            }),
        };
        self.status_job = None;
        self.publisher.set_status(result);
        if std::mem::take(&mut self.status_queued) {
            self.refresh_status();
        }
    }

    /// Runs the confirmed `workspace-publish` on a worker.
    fn start_publish(&mut self) {
        if self.other_command_running() {
            self.publisher.cancel();
            self.publisher.note = Some(Err(
                "another command is running; wait for it to finish.".into()
            ));
            return;
        }
        let Some(args) = self.publisher.confirm() else {
            return;
        };
        let cli = self.source.cli.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        self.publish_job = Some(rx);
        let spawned = std::thread::Builder::new()
            .name("beamcraft-publish".into())
            .spawn(move || {
                let _ = tx.send(command::publish(&cli, &args));
            });
        if spawned.is_err() {
            self.publish_job = None;
            self.publisher.finish(Err(ToolError::local(
                "CLI_UNAVAILABLE",
                "the publish thread could not be started",
            )));
        }
    }

    fn poll_publish(&mut self) {
        let Some(rx) = &self.publish_job else { return };
        let result = match rx.try_recv() {
            Ok(result) => result,
            Err(TryRecvError::Empty) => return,
            Err(TryRecvError::Disconnected) => Err(ToolError {
                code: "CLI_FAILED".into(),
                message: "the publish thread ended without a result".into(),
            }),
        };
        self.publish_job = None;
        self.publisher.finish(result);
        self.refresh_status();
    }

    /// Re-reads the selected element and the head, so a stale reading becomes current.
    fn reread(&mut self) {
        self.invalidate_select();
        self.reload();
        self.refresh_selection();
    }

    fn props_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("Properties");
        let Some(id) = self.props.selected().map(str::to_owned) else {
            ui.label("Select a plate or straight member to see its properties.");
            return;
        };
        ui.label(format!("Element {id}"));
        let state = self.props.state();
        match &state {
            Some(State::Current) if self.props.head_known() => {
                ui.label("State: current");
            }
            Some(State::Current) => {
                ui.label("State: current (the head revision could not be checked)");
            }
            Some(other) => {
                ui.colored_label(RED, format!("State: {}", other.label()));
            }
            None => {}
        }
        let editable = matches!(state, Some(State::Current));
        let busy = self.props.busy() || self.other_command_running();
        if let Some(details) = self.props.details().cloned() {
            ui.label(format!(
                "{} read from revision {}.",
                details.kind,
                details.revision.label()
            ));
            egui::Grid::new("props-readonly").show(ui, |ui| {
                for (label, value) in details.read_only_rows() {
                    ui.label(label);
                    ui.label(value);
                    ui.end_row();
                }
            });
            let materials = self.tools.definitions.materials.clone();
            ui.add_enabled_ui(editable && !busy, |ui| {
                ui.label("Editable, in mm:");
                field(ui, "Name", &mut self.props.form.name, 140.0);
                choice_combo(
                    ui,
                    "props-material",
                    "Material",
                    &mut self.props.form.material,
                    &materials,
                    false,
                );
                if details.is_plate() {
                    field(ui, "Thickness", &mut self.props.form.thickness, 80.0);
                }
                ui.label("Position (absolute):");
                point_fields(ui, &mut self.props.form.translation);
            });
            if !editable {
                ui.label("Editing is off until the properties are current.");
            }
        } else if matches!(state, Some(State::Unknown { .. })) {
            ui.label("No property value is shown: none could be read.");
        }
        let store = self.source.store.clone();
        let can_commit = self.props.can_commit(&store);
        let staged = self.props.staged().is_some();
        let (mut preview, mut commit, mut cancel, mut reread) = (false, false, false, false);
        ui.horizontal(|ui| {
            preview = ui
                .add_enabled(editable && !busy, egui::Button::new("Preview"))
                .clicked();
            commit = ui
                .add_enabled(can_commit && !busy, egui::Button::new("Commit"))
                .clicked();
            cancel = ui
                .add_enabled(staged && !busy, egui::Button::new("Cancel"))
                .clicked();
            reread = ui
                .add_enabled(!busy, egui::Button::new("Re-read"))
                .clicked();
        });
        if preview {
            match self.props.preview(&store) {
                Ok(_) => self.props_note = None,
                Err(e) => {
                    self.props.cancel();
                    self.props_note = Some(Err(e.to_string()));
                }
            }
        }
        if commit {
            self.start_property_commit();
        }
        if cancel {
            self.props.cancel();
            self.props_note = Some(Ok("Cancelled; nothing was changed.".into()));
        }
        if reread {
            self.reread();
        }
        if let Some(edit) = self.props.staged() {
            ui.label(format!(
                "Preview: Commit runs `beamcraft` with exactly these {} arguments, one per line (not a shell command line). It changes: {}.",
                edit.args.len(),
                edit.changed.join(", ")
            ));
            ui.add(egui::Label::new(egui::RichText::new(edit.argument_list()).monospace()).wrap());
            if !self.props.can_commit(&store) {
                ui.colored_label(
                    RED,
                    "The parameters or the revision changed: preview again.",
                );
            }
        }
        if self.props.busy() {
            ui.label("Running…");
        }
        match &self.props_note {
            Some(Ok(message)) => {
                ui.label(message);
            }
            Some(Err(message)) => {
                ui.colored_label(RED, message);
            }
            None => {}
        }
    }

    fn publish_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("Publish");
        match self.publisher.target() {
            Err(why) => {
                ui.label(why);
            }
            Ok((published, workspace)) => {
                ui.label(format!(
                    "Workspace {} to published store {}.",
                    workspace.display(),
                    published.display()
                ));
                if let Some(status) = &self.publisher.status {
                    ui.label(status.summary());
                }
                let enabled =
                    self.publisher.phase() == Phase::Idle && !self.other_command_running();
                if ui
                    .add_enabled(enabled, egui::Button::new("Publish…"))
                    .clicked()
                    && let Err(why) = self.publisher.request()
                {
                    self.publisher.note = Some(Err(why.into()));
                }
                if self.publisher.phase() == Phase::Running {
                    ui.label("Publishing…");
                }
            }
        }
        match &self.publisher.note {
            Some(Ok(message)) => {
                ui.label(message);
            }
            Some(Err(message)) => {
                ui.colored_label(RED, message);
            }
            None => {}
        }
    }

    /// The confirm dialog of Publish: nothing runs until Publish is pressed in it.
    fn publish_dialog(&mut self, ctx: &egui::Context) {
        if self.publisher.phase() != Phase::Confirming {
            return;
        }
        let list = self
            .publisher
            .args()
            .map(|args| {
                args.iter()
                    .map(|a| crate::json_string(a))
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .unwrap_or_default();
        let (mut confirm, mut cancel) = (false, false);
        egui::Window::new("Publish the workspace?")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.label(
                    "The draft head becomes the next published revision. The engine refuses it when the published head moved since the workspace was forked or when there is nothing to publish; there is no merge or conflict resolution.",
                );
                ui.label("Runs `beamcraft` with exactly these arguments, one per line:");
                ui.add(egui::Label::new(egui::RichText::new(list).monospace()).wrap());
                ui.horizontal(|ui| {
                    confirm = ui.button("Publish").clicked();
                    cancel = ui.button("Cancel").clicked();
                });
            });
        if confirm {
            self.start_publish();
        } else if cancel {
            self.publisher.cancel();
        }
    }

    fn edit_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("Edit");
        ui.label("Click an element in the view to select it. Esc clears the selection.");
        match self.editor.selected().cloned() {
            Some(info) => {
                ui.label(format!(
                    "Selected: {} {} ({})",
                    info.kind, info.name, info.id
                ));
            }
            None => {
                ui.label("Nothing selected.");
            }
        }
        let selected = self.editor.selected().is_some();
        let mut action = self.editor.action();
        ui.add_enabled_ui(selected, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut action, EditKind::Move, "Move");
                ui.selectable_value(&mut action, EditKind::Copy, "Copy");
                ui.selectable_value(&mut action, EditKind::Delete, "Delete");
            });
            match action {
                EditKind::Move => {
                    ui.label("New position in mm (absolute):");
                    point_fields(ui, &mut self.editor.move_to);
                }
                EditKind::Copy => {
                    ui.label("Offset of the copy in mm:");
                    point_fields(ui, &mut self.editor.copy_offset);
                }
                _ => {
                    ui.label("Deletes the element (refused while another element links to it).");
                }
            }
        });
        // Another action needs its own preview.
        self.editor.set_action(action);
        let busy = self.editor.busy();
        let target = self.target();
        let can_commit = self.editor.can_commit(target);
        let staged = self.editor.staged().is_some();
        let (mut preview, mut commit, mut cancel) = (false, false, false);
        ui.horizontal(|ui| {
            preview = ui
                .add_enabled(selected && !busy, egui::Button::new("Preview"))
                .clicked();
            commit = ui
                .add_enabled(can_commit, egui::Button::new("Commit"))
                .clicked();
            cancel = ui
                .add_enabled(staged && !busy, egui::Button::new("Cancel"))
                .clicked();
        });
        if preview {
            let action = self.editor.action();
            let target = Target {
                store: &self.source.store,
                workspace: self.source.workspace.as_deref(),
            };
            match self.editor.preview(action, target) {
                Ok(_) => self.edit_note = None,
                Err(e) => {
                    self.editor.cancel();
                    self.edit_note = Some(Err(e.to_string()));
                }
            }
        }
        if commit {
            self.start_edit(None);
        }
        if cancel {
            self.editor.cancel();
            self.edit_note = Some(Ok("Cancelled; nothing was changed.".into()));
        }
        if let Some(edit) = self.editor.staged() {
            ui.label(format!(
                "Preview: Commit runs `beamcraft` with exactly these {} arguments, one per line (not a shell command line).",
                edit.args.len()
            ));
            ui.add(egui::Label::new(egui::RichText::new(edit.argument_list()).monospace()).wrap());
            if !self.editor.can_commit(self.target()) {
                ui.colored_label(RED, "The parameters changed: preview again.");
            }
        }
        let has_workspace = self.source.workspace.is_some();
        let (mut undo, mut redo) = (false, false);
        ui.horizontal(|ui| {
            undo = ui
                .add_enabled(has_workspace && !busy, egui::Button::new("Undo"))
                .clicked();
            redo = ui
                .add_enabled(has_workspace && !busy, egui::Button::new("Redo"))
                .clicked();
        });
        if undo {
            self.start_edit(Some(EditKind::Undo));
        }
        if redo {
            self.start_edit(Some(EditKind::Redo));
        }
        if !has_workspace {
            ui.label(
                "Undo and redo need a personal workspace: open the window with --workspace DIR.",
            );
        }
        if busy {
            ui.label("Running…");
        }
        if self.select_job.is_some() {
            ui.label("Looking up the element…");
        }
        match &self.edit_note {
            Some(Ok(message)) => {
                ui.label(message);
            }
            Some(Err(message)) => {
                ui.colored_label(RED, message);
            }
            None => {}
        }
    }

    fn start_commit(&mut self) {
        if self.other_command_running() {
            self.tool_note = Some(Err(
                "another command is running; wait for it to finish.".into()
            ));
            return;
        }
        let Some(prepared) = self.tools.begin_commit(&self.source.store) else {
            return;
        };
        let cli = self.source.cli.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        self.job = Some(rx);
        self.tool_note = None;
        let spawned = std::thread::Builder::new()
            .name("beamcraft-command".into())
            .spawn(move || {
                let _ = tx.send(command::commit(&cli, &prepared));
            });
        if spawned.is_err() {
            self.job = None;
            self.tools.finish();
            self.tool_note = Some(Err("the command thread could not be started".into()));
        }
    }

    fn poll_commit(&mut self) {
        let Some(rx) = &self.job else { return };
        let result = match rx.try_recv() {
            Ok(result) => result,
            Err(TryRecvError::Empty) => return,
            Err(TryRecvError::Disconnected) => Err(ToolError {
                code: "CLI_FAILED".into(),
                message: "the command thread ended without a result".into(),
            }),
        };
        self.job = None;
        self.tools.finish();
        match result {
            Ok(created) => {
                self.reload();
                let what = match created.kind {
                    ToolKind::Plate => "plate",
                    ToolKind::Member => "member",
                };
                let volume = created.volume_mm3.map_or(String::new(), volume_note);
                self.tool_note = Some(Ok(format!(
                    "Created {what} {} ({}{}). Accepted revision {} (sequence {}).",
                    created.element_id,
                    if created.name.is_empty() {
                        "unnamed"
                    } else {
                        &created.name
                    },
                    volume,
                    created
                        .manifest_digest
                        .get(..12)
                        .unwrap_or(&created.manifest_digest),
                    created.sequence
                )));
            }
            Err(e) => {
                if e.code == "OUTCOME_UNKNOWN" {
                    self.reload();
                }
                self.tool_note = Some(Err(e.to_string()));
            }
        }
    }

    fn tools_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("Create");
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.tools.active, ToolKind::Plate, "Plate");
            ui.selectable_value(&mut self.tools.active, ToolKind::Member, "Member");
        });
        ui.label("Dimensions in mm.");
        let materials = self.tools.definitions.materials.clone();
        let profiles = self.tools.definitions.profiles.clone();
        match self.tools.active {
            ToolKind::Plate => {
                let form = &mut self.tools.plate;
                field(ui, "Width", &mut form.width, 80.0);
                field(ui, "Length", &mut form.length, 80.0);
                field(ui, "Thickness", &mut form.thickness, 80.0);
                field(ui, "Name", &mut form.name, 140.0);
                choice_combo(
                    ui,
                    "plate-material",
                    "Material",
                    &mut form.material,
                    &materials,
                    false,
                );
            }
            ToolKind::Member => {
                let form = &mut self.tools.member;
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut form.kind, MemberKind::Beam, "Beam");
                    ui.selectable_value(&mut form.kind, MemberKind::Column, "Column");
                });
                for (label, point) in [("Start", &mut form.start), ("End", &mut form.end)] {
                    ui.horizontal(|ui| {
                        ui.label(label);
                        for (axis, text) in ["X", "Y", "Z"].iter().zip(point.iter_mut()) {
                            ui.label(*axis);
                            ui.add(
                                egui::TextEdit::singleline(text)
                                    .desired_width(56.0)
                                    .char_limit(MAX_TEXT),
                            );
                        }
                    });
                }
                field(ui, "Name", &mut form.name, 140.0);
                choice_combo(
                    ui,
                    "member-profile",
                    "Profile",
                    &mut form.profile,
                    &profiles,
                    true,
                );
                choice_combo(
                    ui,
                    "member-material",
                    "Material",
                    &mut form.material,
                    &materials,
                    false,
                );
                if profiles.is_empty() {
                    ui.label("The store has no profile definition yet (beamcraft define-profile).");
                }
            }
        }
        let store = self.source.store.clone();
        let busy = self.tools.busy();
        ui.horizontal(|ui| {
            if ui
                .add_enabled(!busy, egui::Button::new("Preview"))
                .clicked()
            {
                match self.tools.preview(&store) {
                    Ok(_) => self.tool_note = None,
                    Err(e) => {
                        self.tools.cancel();
                        self.tool_note = Some(Err(e.to_string()));
                    }
                }
            }
            let can_commit = self.tools.can_commit(&store);
            if ui
                .add_enabled(can_commit, egui::Button::new("Commit"))
                .clicked()
            {
                self.start_commit();
            }
            if ui
                .add_enabled(
                    self.tools.staged().is_some() && !busy,
                    egui::Button::new("Cancel"),
                )
                .clicked()
            {
                self.tools.cancel();
                self.tool_note = Some(Ok("Cancelled; nothing was published.".into()));
            }
        });
        if let Some(staged) = self.tools.staged() {
            let current = self.tools.can_commit(&store);
            ui.label(format!(
                "Preview: Commit runs `beamcraft` with exactly these {} arguments, one per line (not a shell command line). The engine builds the shape then; there is no geometry preview.",
                staged.args.len()
            ));
            ui.add(
                egui::Label::new(egui::RichText::new(staged.argument_list()).monospace()).wrap(),
            );
            if !current {
                ui.colored_label(RED, "The parameters changed: preview again.");
            }
        }
        if busy {
            ui.label("Running…");
        }
        if self.reload_job.is_some() {
            ui.label("Refreshing the model and definitions…");
        }
        if let Some(problem) = &self.refresh_error {
            ui.colored_label(RED, problem);
        }
        match &self.tool_note {
            Some(Ok(message)) => {
                ui.label(message);
            }
            Some(Err(message)) => {
                ui.colored_label(RED, message);
            }
            None => {}
        }
    }

    fn info(&self, ui: &mut egui::Ui) {
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
        let wants_text = ui.ctx().egui_wants_keyboard_input();
        let (fit, reload) = ui.input(|i| {
            (
                !wants_text && i.key_pressed(egui::Key::F),
                !wants_text && i.key_pressed(egui::Key::R),
            )
        });
        self.poll_commit();
        self.poll_edit();
        self.poll_props();
        self.poll_publish();
        self.poll_status();
        self.poll_select();
        self.poll_reload();
        if !wants_text && ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            // Esc closes the publish dialog first; it clears the selection only when none is open.
            if self.publisher.phase() == Phase::Confirming {
                self.publisher.cancel();
            } else {
                self.invalidate_select();
                self.editor.deselect();
                self.props.clear();
            }
        }
        if reload {
            self.reload();
        }
        if fit && let Some(l) = &self.loaded {
            let (target, extent) = l.scene.framing();
            if let Err(e) = self.camera.fit(target, extent, self.aspect()) {
                self.error = Some(e.to_string());
            }
        }
        if self.text {
            // A fixed width: the 3D view must not shift sideways when a tab or a chosen
            // definition name is wider, and the coordinate rows must fit at the default size.
            egui::Panel::left("info")
                .exact_size(320.0)
                .show(ui, |ui| self.panel(ui));
            self.publish_dialog(&ui.ctx().clone());
        }
        egui::CentralPanel::default().show(ui, |ui| {
            let size = ui.available_size();
            let scale = ui.ctx().pixels_per_point();
            let [width, height] = target_size([size.x, size.y], scale);
            if let Err(e) = self.resize(width, height) {
                self.error = Some(e.to_string());
            }
            let response =
                ui.add(egui::Image::new((self.texture, size)).sense(egui::Sense::click_and_drag()));
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
            let aspect = self.aspect();
            let mut picked = None;
            match self
                .camera
                .navigate(orbit, pan, zoom)
                .and_then(|()| self.camera.view(aspect))
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
                    if let Some(l) = &self.loaded {
                        if let Some(selected) = self.editor.selected() {
                            let painter = ui.painter_at(response.rect);
                            draw_selection(&painter, response.rect, &view, l, &selected.id);
                        }
                        // A click (not a drag) picks along the ray through the pixel.
                        if response.clicked_by(egui::PointerButton::Primary)
                            && let Some(pos) = response.interact_pointer_pos()
                            && let Some(ndc) = ndc_of(
                                [pos.x, pos.y],
                                [response.rect.min.x, response.rect.min.y],
                                [response.rect.width(), response.rect.height()],
                            )
                            && let Some(ray) = ray_through(&view, ndc, l.scene.origin_mm)
                        {
                            picked = Some(ray);
                        }
                    }
                }
                Err(e) => self.error = Some(e.to_string()),
            }
            if let Some(ray) = picked {
                self.start_select(ray);
            }
        });
        ui.ctx().request_repaint();
    }
}

#[cfg(test)]
mod tests {
    use super::{needs_refit, target_size, volume_note};

    #[test]
    fn the_render_target_is_clamped_as_a_whole() {
        // Below the limit the viewport passes through unchanged.
        assert_eq!(target_size([1280.0, 800.0], 1.0), [1280, 800]);
        // A 5400 px window: both sides shrink by the same factor, so a circle stays a circle.
        let [w, h] = target_size([5400.0, 900.0], 1.0);
        assert_eq!(w, 4096);
        assert!((h as f32 / 900.0 - 4096.0 / 5400.0).abs() < 0.01);
        // Simulated Retina (2x) reaches the limit at half the point size.
        let [w, h] = target_size([2560.0, 1300.0], 2.0);
        assert_eq!((w, h), (4096, 2080));
        // A degenerate viewport still yields a one-pixel target.
        assert_eq!(target_size([0.0, 0.0], 1.0), [1, 1]);
    }

    #[test]
    fn an_unchanged_framing_keeps_the_camera() {
        let framing = ([10.0, 20.0, 30.0], 500.0, 1.6);
        assert!(needs_refit(None, framing.0, framing.1, framing.2));
        assert!(!needs_refit(Some(framing), framing.0, framing.1, framing.2));
        // New bounds or a new viewport aspect follow.
        assert!(needs_refit(
            Some(framing),
            [0.0, 20.0, 30.0],
            framing.1,
            framing.2
        ));
        assert!(needs_refit(Some(framing), framing.0, 600.0, framing.2));
        assert!(needs_refit(Some(framing), framing.0, framing.1, 1.0));
    }

    #[test]
    fn a_small_volume_is_not_zero() {
        assert_eq!(volume_note(400_000.0), ", volume 400000 mm3");
        assert_eq!(volume_note(999_999.0), ", volume 999999 mm3");
        assert_eq!(volume_note(1e6), ", volume 0.001 m3");
        assert_eq!(volume_note(1.025e9), ", volume 1.025 m3");
    }
}
