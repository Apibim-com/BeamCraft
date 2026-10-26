// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! Grid workbench: command-backed edits and session-only drawing, labels and snap switches.
use super::*;
use crate::grid_tools::GridAction;
use beamcraft_json::{Json, Limits};

impl App {
    pub(super) fn begin_grid_job(&mut self, mut args: Vec<String>, commit: bool) {
        if self.grid_job.is_some() {
            return;
        }
        if commit && let Some(actor) = &self.source.actor {
            args.extend(["--actor-id".into(), actor.clone()]);
        }
        let cli = self.source.cli.clone();
        let stats = Arc::clone(&self.stats);
        let (tx, rx) = std::sync::mpsc::channel();
        match std::thread::Builder::new()
            .name("beamcraft-grid".into())
            .spawn(move || {
                let result = (|| {
                    let output = command::run(&cli, &args, Duration::from_secs(120), &stats)
                        .map_err(|e| e.to_string())?;
                    let json = beamcraft_json::decode(&output.stdout, &Limits::DEFAULT)
                        .map_err(|e| e.to_string())?;
                    if output.code != Some(0) {
                        return Err(format!(
                            "{}: {}",
                            json.get("code")
                                .and_then(Json::string)
                                .unwrap_or("COMMAND_FAILED"),
                            json.get("message")
                                .and_then(Json::string)
                                .unwrap_or("grid command was refused")
                        ));
                    }
                    Ok((commit, json))
                })();
                let _ = tx.send(result);
            }) {
            Ok(_) => self.grid_job = Some(rx),
            Err(e) => self.grids.note = Some(Err(e.to_string())),
        }
    }
    pub(super) fn poll_grids(&mut self) {
        if let Some(rx) = &self.grid_job {
            match rx.try_recv() {
                Err(TryRecvError::Empty) => return,
                result => {
                    self.grid_job = None;
                    match result {
                        Ok(Ok((true, j))) => {
                            self.grids.cancel();
                            self.grids.note = Some(Ok(j
                                .get("outcome")
                                .and_then(Json::string)
                                .unwrap_or("Grid updated")
                                .into()));
                            self.reload();
                        }
                        Ok(Ok((false, j))) => {
                            if let Err(e) = self.grids.apply_listing(&j) {
                                self.grids.note = Some(Err(e));
                            }
                        }
                        Ok(Err(e)) => self.grids.note = Some(Err(e)),
                        Err(_) => {
                            self.grids.note = Some(Err("grid worker ended without a result".into()))
                        }
                    }
                }
            }
        }
        if self.grid_refresh && self.grid_job.is_none() {
            self.grid_refresh = false;
            self.begin_grid_job(
                vec![
                    "list-grids".into(),
                    "--store".into(),
                    self.source.store.to_string_lossy().into_owned(),
                ],
                false,
            );
        }
    }
    pub(super) fn fit_grid_scene(&mut self) {
        if !self.grids_current() || self.grids.grids.is_empty() {
            return;
        }
        let Some(loaded) = &self.loaded else {
            return;
        };
        if !loaded.snapshot.is_empty() {
            self.grid_framing = None;
            return;
        }
        let manifest = loaded.snapshot.pin().revision().to_string();
        if self
            .grid_framing
            .as_ref()
            .is_some_and(|(m, origin)| m == &manifest && *origin == loaded.summary.origin_mm)
        {
            return;
        }
        let mut min = [f64::INFINITY; 2];
        let mut max = [f64::NEG_INFINITY; 2];
        for g in &self.grids.grids {
            for line in beamcraft_engine::grid_query::segments(g).unwrap_or_default() {
                for [x, y] in [
                    beamcraft_engine::grid_query::local_to_world(g, line.start),
                    beamcraft_engine::grid_query::local_to_world(g, line.end),
                ] {
                    let [a, b] = &mut min;
                    *a = a.min(x);
                    *b = b.min(y);
                    let [a, b] = &mut max;
                    *a = a.max(x);
                    *b = b.max(y);
                }
            }
        }
        let [ax, ay] = min;
        let [bx, by] = max;
        let origin = [(ax + bx) / 2., (ay + by) / 2., 0.];
        if let Some(loaded) = &mut self.loaded {
            loaded.summary.origin_mm = origin;
            loaded.summary.min = [-((bx - ax) / 2.) as f32, -((by - ay) / 2.) as f32, 0.];
            loaded.summary.max = [((bx - ax) / 2.) as f32, ((by - ay) / 2.) as f32, 0.];
        }
        self.grid_framing = Some((manifest, origin));
        let extent = (bx - ax).max(by - ay).max(1000.) as f32;
        let _ = self.camera.fit([0.; 3], extent, self.aspect());
    }
    fn grids_current(&self) -> bool {
        self.loaded.as_ref().is_some_and(|l| {
            self.grids.manifest.as_deref() == Some(l.snapshot.pin().revision().to_string().as_str())
        })
    }
    pub(super) fn snap_grid_point(&self, point: [f64; 3]) -> [f64; 3] {
        self.loaded.as_ref().map_or(point, |l| {
            self.grids.snap_point(&l.snapshot, point).unwrap_or(point)
        })
    }
    pub(super) fn place_grid_line(&mut self, ray: crate::edit::Ray) {
        if let Some(point) = ground_point(&ray) {
            let point = self.snap_grid_point(point);
            self.grids.click(point);
        } else {
            self.grids.note = Some(Err("Aim at the ground plane to place a grid line".into()));
        }
    }
    pub(super) fn draw_grids(&self, painter: &egui::Painter, rect: egui::Rect, view: &View) {
        if !self.grids_current() {
            return;
        }
        let Some(loaded) = &self.loaded else {
            return;
        };
        let [ox, oy, oz] = loaded.summary.origin_mm;
        let screen = |[x, y]: [f64; 2]| {
            let [x, y, z] = view.project([(x - ox) as f32, (y - oy) as f32, (-oz) as f32]);
            (x.is_finite() && y.is_finite() && (0.0..1.0).contains(&z)).then(|| {
                egui::pos2(
                    rect.min.x + (x * 0.5 + 0.5) * rect.width(),
                    rect.min.y + (0.5 - y * 0.5) * rect.height(),
                )
            })
        };
        let color = egui::Color32::from_rgb(40, 150, 170);
        for g in &self.grids.grids {
            if self.grids.hidden.contains(&g.id) {
                continue;
            }
            for line in beamcraft_engine::grid_query::segments(g).unwrap_or_default() {
                let a = screen(beamcraft_engine::grid_query::local_to_world(g, line.start));
                let b = screen(beamcraft_engine::grid_query::local_to_world(g, line.end));
                if let (Some(a), Some(b)) = (a, b) {
                    painter.line_segment([a, b], egui::Stroke::new(1., color));
                    if self.grids.labels && self.text {
                        painter.text(
                            a,
                            egui::Align2::LEFT_BOTTOM,
                            &line.label,
                            egui::FontId::proportional(12.),
                            color,
                        );
                    }
                }
            }
        }
        if self.grids.drawing
            && let Some([x, y, _]) = self.grids.first
            && let Some(p) = screen([x, y])
        {
            painter.circle_stroke(p, 5., egui::Stroke::new(2., color));
        }
    }
    pub(super) fn grid_panel(&mut self, ui: &mut egui::Ui) {
        let busy = self.grid_job.is_some()
            || self.tools.busy()
            || self.edit_job.is_some()
            || self.props_job.is_some();
        let mut selected = None;
        egui::ComboBox::from_id_salt("grid-selection")
            .selected_text(
                self.grids
                    .grids
                    .iter()
                    .find(|g| g.id.to_string() == self.grids.form.target)
                    .map_or("Choose grid", |g| g.name.as_str()),
            )
            .show_ui(ui, |ui| {
                for g in &self.grids.grids {
                    if ui
                        .selectable_label(self.grids.form.target == g.id.to_string(), &g.name)
                        .clicked()
                    {
                        selected = Some(g.clone());
                    }
                }
            });
        if let Some(g) = selected {
            self.grids.cancel();
            self.grids.form.select(&g);
            self.grids.form.action = GridAction::Update;
        }
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.grids.labels, "Labels");
            ui.checkbox(&mut self.grids.snap, "Snap");
        });
        ui.add(
            egui::DragValue::new(&mut self.grids.radius_mm)
                .range(0.0..=10000.)
                .prefix("Snap radius ")
                .suffix(" mm"),
        );
        egui::CollapsingHeader::new("Visible grids").show(ui, |ui| {
            for g in &self.grids.grids {
                let mut visible = !self.grids.hidden.contains(&g.id);
                if ui.checkbox(&mut visible, &g.name).changed() {
                    if visible {
                        self.grids.hidden.remove(&g.id);
                    } else {
                        self.grids.hidden.insert(g.id);
                    }
                }
            }
        });
        ui.separator();
        let f = &mut self.grids.form;
        let previous_action = f.action;
        egui::ComboBox::from_id_salt("grid-action")
            .selected_text(f.action.title())
            .show_ui(ui, |ui| {
                for a in GridAction::ALL {
                    ui.selectable_value(&mut f.action, a, a.title());
                }
            });
        let edit = |ui: &mut egui::Ui, label: &str, value: &mut String| {
            ui.label(label);
            ui.add(
                egui::TextEdit::singleline(value)
                    .char_limit(16384)
                    .desired_width(180.),
            );
        };
        match f.action {
            GridAction::Create | GridAction::Update => {
                edit(ui, "Name", &mut f.name);
                edit(ui, "Origin X,Y (mm)", &mut f.origin);
                edit(ui, "Rotation (degrees)", &mut f.rotation);
                for (title, expression, labels, span) in [
                    (
                        "U family",
                        &mut f.u_expression,
                        &mut f.u_labels,
                        &mut f.u_span,
                    ),
                    (
                        "V family",
                        &mut f.v_expression,
                        &mut f.v_labels,
                        &mut f.v_span,
                    ),
                ] {
                    ui.label(title);
                    edit(ui, "Expression (empty = drawn)", expression);
                    edit(ui, "Labels", labels);
                    edit(ui, "Span (optional)", span);
                }
                if f.action == GridAction::Create && f.u_expression.is_empty() {
                    edit(ui, "First line label", &mut f.label);
                    edit(ui, "From X,Y", &mut f.from);
                    edit(ui, "To X,Y", &mut f.to);
                }
            }
            _ => {
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut f.family, "u".into(), "U");
                    ui.selectable_value(&mut f.family, "v".into(), "V");
                });
                if let Some(g) = self
                    .grids
                    .grids
                    .iter()
                    .find(|g| g.id.to_string() == f.target)
                {
                    egui::ComboBox::from_id_salt("grid-axis")
                        .selected_text(if f.axis_id.is_empty() {
                            "Choose line"
                        } else {
                            &f.label
                        })
                        .show_ui(ui, |ui| {
                            for line in
                                beamcraft_engine::grid_query::segments(g).unwrap_or_default()
                            {
                                if ui
                                    .selectable_label(
                                        f.axis_id == line.axis_id.to_string(),
                                        &line.label,
                                    )
                                    .clicked()
                                {
                                    f.axis_id = line.axis_id.to_string();
                                    f.label = line.label;
                                    let [x, y] =
                                        beamcraft_engine::grid_query::local_to_world(g, line.start);
                                    f.from = format!("{x},{y}");
                                    let [x, y] =
                                        beamcraft_engine::grid_query::local_to_world(g, line.end);
                                    f.to = format!("{x},{y}");
                                }
                            }
                        });
                }
                if matches!(f.action, GridAction::Draw | GridAction::Reshape) {
                    edit(ui, "From X,Y", &mut f.from);
                    edit(ui, "To X,Y", &mut f.to);
                }
                if matches!(f.action, GridAction::Draw | GridAction::Relabel) {
                    edit(ui, "Line label", &mut f.label);
                }
                if f.action == GridAction::Relabel {
                    if ui.button("Relabel complete family").clicked() {
                        f.axis_id.clear();
                    }
                    edit(
                        ui,
                        "Family labels",
                        if f.family == "u" {
                            &mut f.u_labels
                        } else {
                            &mut f.v_labels
                        },
                    );
                }
            }
        }
        if f.action != previous_action {
            self.grids.drawing = false;
            self.grids.first = None;
            self.grids.staged = None;
        }
        let draw_available = matches!(f.action, GridAction::Draw | GridAction::Reshape);
        if draw_available
            && ui
                .add_enabled(
                    !busy,
                    egui::Button::new(if self.grids.drawing {
                        "Stop point capture"
                    } else {
                        "Pick two points in view"
                    }),
                )
                .clicked()
        {
            self.grids.drawing = !self.grids.drawing;
            self.grids.first = None;
            self.grids.staged = None;
            self.beam_mode = false;
        }
        let prepared = self.grids.form.prepare(&self.source.store);
        let mut commit = None;
        ui.horizontal(|ui| {
            if ui
                .add_enabled(!busy, egui::Button::new("Preview"))
                .clicked()
            {
                match &prepared {
                    Ok(a) => {
                        self.grids.staged = Some(a.clone());
                        self.grids.note = Some(Ok("Ready to commit the grid edit".into()));
                    }
                    Err(e) => self.grids.note = Some(Err(e.clone())),
                }
            }
            let current = prepared
                .as_ref()
                .ok()
                .is_some_and(|a| self.grids.staged.as_ref() == Some(a));
            if ui
                .add_enabled(!busy && current, egui::Button::new("Commit"))
                .clicked()
            {
                commit = self.grids.staged.clone();
            }
            if ui.add_enabled(!busy, egui::Button::new("Cancel")).clicked() {
                self.grids.cancel();
                self.grids.note = None;
            }
        });
        if self.grids.drawing {
            ui.label(if self.grids.first.is_some() {
                "Click the line end. Nothing is saved until Commit."
            } else {
                "Click the line start on Z = 0."
            });
        }
        if let Some(note) = &self.grids.note {
            match note {
                Ok(n) => {
                    ui.label(n);
                }
                Err(n) => {
                    ui.colored_label(RED, n);
                }
            }
        }
        if let Some(args) = commit {
            self.begin_grid_job(args, true);
        }
    }
}
