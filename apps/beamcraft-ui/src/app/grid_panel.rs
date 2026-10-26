// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! The insertion form invokes `create-grid`; display endpoints arrive with the pinned snapshot.

use super::{App, workbench::Bench};
use crate::grid::{FamilyForm, GridScene};
use crate::theme::{self, Kind, Palette};
use crate::tools::ToolError;
use egui::Ui;
use std::sync::Arc;
use std::sync::mpsc::{self, TryRecvError};

fn field(ui: &mut Ui, label: &str, value: &mut String, hint: &str) {
    ui.label(theme::bold(label, 12.0));
    ui.add(
        egui::TextEdit::singleline(value)
            .hint_text(hint)
            .desired_width(ui.available_width())
            .char_limit(4096),
    );
}

fn family(ui: &mut Ui, title: &str, form: &mut FamilyForm, hint: &str) {
    egui::CollapsingHeader::new(title)
        .default_open(true)
        .show(ui, |ui| {
            field(
                ui,
                "Coordinates (mm)",
                &mut form.expression,
                "Start coordinate, then spacings",
            );
            ui.label(
                egui::RichText::new(
                    "First position, then positive spacings; N*D repeats a spacing.",
                )
                .small(),
            );
            field(ui, "Labels", &mut form.labels, hint);
            ui.label(egui::RichText::new("Leave blank for automatic labels.").small());
            egui::CollapsingHeader::new("Line limits").show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.set_width(116.0);
                        field(ui, "Line start (mm)", &mut form.span[0], "Automatic");
                    });
                    ui.vertical(|ui| {
                        ui.set_width(116.0);
                        field(ui, "Line end (mm)", &mut form.span[1], "Automatic");
                    });
                });
            });
        });
}

impl App {
    pub(super) fn grid_framing(&self, summary: &crate::scene::SceneSummary) -> ([f32; 3], f32) {
        if self.grids_visible {
            self.grid_scene.framing(summary)
        } else {
            summary.framing()
        }
    }

    pub(super) fn apply_grids(&mut self, grids: Result<GridScene, ToolError>) {
        match grids {
            Ok(scene) => self.grid_scene = scene,
            Err(error) => {
                self.grid_scene = GridScene::default();
                self.grid_note = Some(Err(error.to_string()));
            }
        }
    }

    pub(super) fn grid_panel(&mut self, ui: &mut Ui) {
        let busy = self.other_command_running();
        ui.label("Horizontal project axes. Levels are managed separately.");
        ui.add_space(8.0);
        let footer = if self.grid_note.is_some() {
            180.0
        } else {
            120.0
        };
        egui::ScrollArea::vertical()
            .id_salt("grid-form")
            .max_height((ui.available_height() - footer).max(0.0))
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.add_enabled_ui(!busy, |ui| {
                    field(ui, "Name", &mut self.grid_form.name, "Grid name");
                    family(ui, "U family · X positions", &mut self.grid_form.u, "1 2 3");
                    family(ui, "V family · Y positions", &mut self.grid_form.v, "A B C");
                    egui::CollapsingHeader::new("Position")
                        .default_open(false)
                        .show(ui, |ui| {
                            field(ui, "Origin X (mm)", &mut self.grid_form.origin[0], "0");
                            field(ui, "Origin Y (mm)", &mut self.grid_form.origin[1], "0");
                            field(ui, "Rotation (degrees)", &mut self.grid_form.rotation, "0");
                        });
                });
            });
        ui.checkbox(&mut self.grids_visible, "Show grids");
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if theme::button(ui, Kind::Primary, "Create grid", !busy).clicked() {
                self.start_grid();
            }
            if theme::button(ui, Kind::Secondary, "Cancel", !busy).clicked() {
                self.set_bench(Bench::Select);
            }
        });
        if self.grid_job.is_some() {
            ui.spinner();
            ui.label("Creating grid…");
        }
        if let Some(note) = &self.grid_note {
            match note {
                Ok(message) => {
                    ui.label(message);
                }
                Err(message) => {
                    ui.colored_label(Palette::current(ui).error, message);
                }
            }
        }
    }

    fn start_grid(&mut self) {
        if self.other_command_running() {
            return;
        }
        let args = match self
            .grid_form
            .prepare(&self.source.store, self.source.actor.as_deref())
        {
            Ok(args) => args,
            Err(error) => {
                self.grid_note = Some(Err(error.to_string()));
                return;
            }
        };
        let cli = self.source.cli.clone();
        let stats = Arc::clone(&self.stats);
        let (tx, rx) = mpsc::channel();
        match std::thread::Builder::new()
            .name("beamcraft-create-grid".into())
            .spawn(move || {
                let _ = tx.send(crate::grid::commit(&cli, &args, &stats));
            }) {
            Ok(_) => {
                self.grid_job = Some(rx);
                self.grid_note = None;
            }
            Err(_) => self.grid_note = Some(Err("The grid command could not be started.".into())),
        }
    }

    pub(super) fn poll_grid(&mut self) {
        let Some(rx) = &self.grid_job else { return };
        let result = match rx.try_recv() {
            Ok(result) => result,
            Err(TryRecvError::Empty) => return,
            Err(TryRecvError::Disconnected) => Err(ToolError::local(
                "CLI_FAILED",
                "The grid command ended without a result.",
            )),
        };
        self.grid_job = None;
        match result {
            Ok(_) => {
                self.grids_visible = true;
                self.grid_note = Some(Ok("Grid created.".into()));
                self.reload();
            }
            Err(error) => {
                if error.code == "OUTCOME_UNKNOWN" {
                    self.reload();
                }
                self.grid_note = Some(Err(error.to_string()));
            }
        }
    }
}
