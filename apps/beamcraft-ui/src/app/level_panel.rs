// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! The levels bench: the levels of the displayed revision, and one form that creates a level or
//! edits the selected one through the engine commands. The list is the pinned `definitions`
//! reply, so it follows every revision, undo and redo included.

use super::App;
use crate::levels::{self, LevelForm};
use crate::theme;
use crate::tools::{FormError, LevelChoice, ToolError};
use egui::Ui;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, TryRecvError};

/// What a running command will report when it succeeds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Gesture {
    Create,
    Update,
    Delete,
}

impl Gesture {
    const fn done(self) -> &'static str {
        match self {
            Self::Create => "Level created.",
            Self::Update => "Level updated.",
            Self::Delete => "Level deleted.",
        }
    }
}

type Job = Receiver<Result<(), ToolError>>;

/// The state of the levels bench.
#[derive(Default)]
pub(super) struct LevelPanel {
    form: LevelForm,
    /// The identity of the level being edited; a new level otherwise.
    selected: Option<String>,
    job: Option<(Gesture, Job)>,
    pub(super) note: Option<Result<String, String>>,
}

impl LevelPanel {
    pub(super) const fn running(&self) -> bool {
        self.job.is_some()
    }

    pub(super) fn selected_id(&self) -> Option<&str> {
        self.selected.as_deref()
    }

    fn select(&mut self, level: &LevelChoice) {
        self.form = LevelForm::of(level);
        self.selected = Some(level.id.clone());
        self.note = None;
    }

    fn clear(&mut self) {
        self.form = LevelForm::default();
        self.selected = None;
    }
}

impl App {
    fn selected_level(&self) -> Option<LevelChoice> {
        let id = self.level_panel.selected.as_ref()?;
        self.tools
            .definitions
            .levels
            .iter()
            .find(|level| &level.id == id)
            .cloned()
    }

    pub(super) fn level_panel(&mut self, ui: &mut Ui) {
        // An undo can remove the level being edited.
        let selected = self.selected_level();
        if selected.is_none() && self.level_panel.selected.is_some() {
            self.level_panel.clear();
        }
        let busy = self.other_command_running();
        crate::kit::type_box(ui, theme::Icon::Table, "Levels");
        let all = self.tools.definitions.levels.clone();
        if all.is_empty() {
            ui.label("No levels yet. Enter a name and an elevation below.");
        }
        // The form below keeps its height; a selected level adds the Delete button.
        let footer = if selected.is_some() { 240.0 } else { 200.0 };
        let mut picked = None;
        egui::ScrollArea::vertical()
            .id_salt("level-list")
            .max_height((ui.available_height() - footer).max(0.0))
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for level in levels::sorted(&all) {
                    let chosen = selected.as_ref().is_some_and(|s| s.id == level.id);
                    let row = format!(
                        "{}   {:+} mm   {} el.",
                        level.name, level.elevation_mm, level.elements
                    );
                    if ui.selectable_label(chosen, row).clicked() {
                        picked = Some(level.clone());
                    }
                }
            });
        if let Some(level) = picked {
            self.level_panel.select(&level);
        }
        ui.separator();
        ui.add_enabled_ui(!busy, |ui| {
            crate::kit::prop_row(ui, "Name", |ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut self.level_panel.form.name)
                        .hint_text("Level name")
                        .char_limit(256),
                );
            });
            crate::kit::prop_row(ui, "Elevation (mm)", |ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut self.level_panel.form.elevation)
                        .hint_text("0")
                        .char_limit(64),
                );
            });
            ui.small("A new elevation moves no element.");
            if let Some(level) = &selected
                && ui.button("Delete level").clicked()
            {
                let prepared =
                    levels::prepare_delete(level, &self.source.store, self.source.actor.as_deref());
                self.start_level(Gesture::Delete, prepared);
            }
        });
        let selected = self.selected_level();
        let enabled = selected
            .as_ref()
            .is_none_or(|level| self.level_panel.form.differs(level));
        let done = crate::kit::actions(
            ui,
            if selected.is_some() {
                "Apply changes"
            } else {
                "Create level"
            },
            enabled,
            if selected.is_some() {
                "New level"
            } else {
                "Close"
            },
            busy,
        );
        if done.primary {
            let store = self.source.store.clone();
            let actor = self.source.actor.clone();
            let form = &self.level_panel.form;
            let (gesture, prepared) = match &selected {
                Some(level) => (
                    Gesture::Update,
                    form.prepare_update(level, &store, actor.as_deref()),
                ),
                None => (
                    Gesture::Create,
                    form.prepare_create(&store, actor.as_deref()),
                ),
            };
            self.start_level(gesture, prepared);
        }
        if done.cancel {
            self.level_panel.note = None;
            if selected.is_some() {
                self.level_panel.clear();
            } else {
                self.set_bench(super::workbench::Bench::Select);
            }
        }
    }

    fn start_level(&mut self, gesture: Gesture, prepared: Result<Vec<String>, FormError>) {
        if self.other_command_running() {
            return;
        }
        let args = match prepared {
            Ok(args) => args,
            Err(error) => {
                self.level_panel.note = Some(Err(error.to_string()));
                return;
            }
        };
        let cli = self.source.cli.clone();
        let stats = Arc::clone(&self.stats);
        let (tx, rx) = mpsc::channel();
        match std::thread::Builder::new()
            .name("beamcraft-level".into())
            .spawn(move || {
                let _ = tx.send(levels::commit(&cli, &args, &stats));
            }) {
            Ok(_) => {
                self.level_panel.job = Some((gesture, rx));
                self.level_panel.note = None;
            }
            Err(_) => {
                self.level_panel.note = Some(Err("The level command could not be started.".into()));
            }
        }
    }

    pub(super) fn poll_levels(&mut self) {
        let Some((gesture, rx)) = &self.level_panel.job else {
            return;
        };
        let gesture = *gesture;
        let result = match rx.try_recv() {
            Ok(result) => result,
            Err(TryRecvError::Empty) => return,
            Err(TryRecvError::Disconnected) => Err(ToolError::local(
                "CLI_FAILED",
                "The level command ended without a result.",
            )),
        };
        self.level_panel.job = None;
        match result {
            Ok(()) => {
                if gesture != Gesture::Update {
                    self.level_panel.clear();
                }
                self.level_panel.note = Some(Ok(gesture.done().to_owned()));
                self.reload();
                self.refresh_status();
            }
            Err(error) => {
                if error.code == "OUTCOME_UNKNOWN" {
                    self.reload();
                    self.refresh_status();
                }
                self.level_panel.note = Some(Err(error.to_string()));
            }
        }
    }
}
