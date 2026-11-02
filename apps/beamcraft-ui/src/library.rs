// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! The model-library pieces shared by the start page and the in-model Menu.
//!
//! `NewModelForm` is the new-model form: the name, the location, the template and the
//! `create-project` command. It owns its fields, its worker and its note, so any window can host
//! it; the host draws the frame and the header around it. The next free name in the location is
//! read with `session::project::next_project_name` on a worker thread; creating a project runs
//! `create-project` in a separate process, read with `crate::start::parse_project`.

use crate::command;
use crate::start::{DEADLINE, MAX_FIELD, ProjectView, parse_project, project_create_args, refusal};
use crate::stats::Stats;
use crate::theme::{self, Kind, Palette};
use crate::tools::ToolError;
use session::project::next_project_name;
use std::path::Path;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, TryRecvError};
use std::time::Duration;

/// Initial editable model name; translate with the UI when other languages are added.
pub(crate) const DEFAULT_MODEL_NAME: &str = "New Model";

/// A one-line text field as wide as the row, at most `limit` characters.
pub(crate) fn field(ui: &mut egui::Ui, text: &mut String, limit: usize) -> egui::Response {
    ui.add(
        egui::TextEdit::singleline(text)
            .desired_width(ui.available_width())
            .min_size(egui::vec2(0.0, 36.0))
            .align(egui::Align2::LEFT_CENTER)
            .margin(egui::Margin::symmetric(12, 8))
            .char_limit(limit),
    )
}

/// Shows the system's folder dialog titled `title`, starting at `start`: the chosen folder,
/// `None` when it was cancelled, or the reason it could not be shown or used.
pub(crate) fn browse_folder(title: &str, start: &str) -> Result<Option<String>, String> {
    match crate::dialog::pick_folder(title, Some(Path::new(start))) {
        Ok(Some(dir)) => match dir.to_str() {
            Some(text) => Ok(Some(text.to_owned())),
            None => Err("the chosen folder's path is not UTF-8 text".into()),
        },
        Ok(None) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

/// What the form needs from its host window.
pub(crate) struct Host<'a> {
    /// The CLI that runs `create-project`.
    pub(crate) cli: &'a Path,
    /// The configuration directory replacing the user's, if any.
    pub(crate) config_dir: Option<&'a Path>,
    /// The author identity recorded as the creator.
    pub(crate) actor: Option<&'a str>,
    /// The environment the new model is seeded from.
    pub(crate) environment: Option<&'a str>,
    /// The window's process statistics.
    pub(crate) stats: &'a Arc<Stats>,
}

/// What the user did with the form.
#[derive(Debug)]
pub(crate) enum FormEvent {
    /// Cancel was pressed.
    Cancelled,
    /// `create-project` created this project.
    Created(ProjectView),
}

enum FormReply {
    Suggested {
        parent: String,
        previous: String,
        name: Result<String, ToolError>,
    },
    Created(Result<ProjectView, ToolError>),
}

/// The new-model form: name, location, template, and the create command.
pub(crate) struct NewModelForm {
    name: String,
    suggested: String,
    parent: String,
    note: Option<Result<String, String>>,
    job: Option<Receiver<FormReply>>,
    /// The sending end of a job a test installed with `hold_job`.
    #[cfg(test)]
    sender: Option<std::sync::mpsc::Sender<FormReply>>,
}

impl NewModelForm {
    /// An empty form with the default name and no location.
    pub(crate) fn new() -> Self {
        Self {
            name: DEFAULT_MODEL_NAME.to_owned(),
            suggested: DEFAULT_MODEL_NAME.to_owned(),
            parent: String::new(),
            note: None,
            job: None,
            #[cfg(test)]
            sender: None,
        }
    }

    /// Sets the location to `parent` unless one was chosen or typed already.
    pub(crate) fn seed_parent(&mut self, parent: &str) {
        if self.parent.is_empty() {
            parent.clone_into(&mut self.parent);
        }
    }

    /// The location as typed.
    pub(crate) fn parent(&self) -> &str {
        &self.parent
    }

    /// Whether a suggestion or a create command is running.
    pub(crate) fn busy(&self) -> bool {
        self.job.is_some()
    }

    /// Prepares the form to be shown: clears its note and suggests a name.
    pub(crate) fn show(&mut self) {
        self.note = None;
        self.suggest();
    }

    /// Asks for the next free model name in the location, unless the name was edited, no
    /// location is set or a job is running.
    pub(crate) fn suggest(&mut self) {
        if self.job.is_some() || self.name != self.suggested || self.parent.trim().is_empty() {
            return;
        }
        let parent = self.parent.trim().to_owned();
        let previous = self.name.clone();
        self.run_worker(move || {
            let name = next_project_name(Path::new(&parent), DEFAULT_MODEL_NAME).map_err(refusal);
            FormReply::Suggested {
                parent,
                previous,
                name,
            }
        });
    }

    /// Runs `create-project` for the form's name and location.
    pub(crate) fn create(&mut self, host: &Host<'_>) {
        // A typed location is checked here, so the refusal names the path that was typed.
        let parent = self.parent.trim();
        if !Path::new(parent).is_dir() {
            self.note = Some(Err(format!(
                "The folder \"{parent}\" does not exist. Choose an existing folder with Browse…"
            )));
            return;
        }

        let args = project_create_args(
            host.config_dir,
            self.parent.trim(),
            if self.name == self.suggested {
                DEFAULT_MODEL_NAME
            } else {
                self.name.trim()
            },
            host.actor,
            host.environment,
        );
        let cli = host.cli.to_path_buf();
        let stats = Arc::clone(host.stats);
        self.run_worker(move || {
            FormReply::Created(
                command::run(&cli, &args, DEADLINE, &stats)
                    .and_then(|out| parse_project(out.code, &out.stdout, "project_created")),
            )
        });
    }

    /// Reads the reply of the running job, if it has arrived: a created project is the event,
    /// a suggestion or a failure only changes the form.
    pub(crate) fn poll(&mut self) -> Option<FormEvent> {
        let rx = self.job.as_ref()?;
        let reply = match rx.try_recv() {
            Ok(reply) => reply,
            Err(TryRecvError::Empty) => return None,
            Err(TryRecvError::Disconnected) => {
                self.job = None;
                self.note = Some(Err("the command thread ended without an answer".into()));
                return None;
            }
        };
        self.job = None;
        match reply {
            FormReply::Suggested {
                parent,
                previous,
                name,
            } => {
                if self.parent.trim() == parent
                    && self.name == previous
                    && let Ok(name) = name
                {
                    self.name = name.clone();
                    self.suggested = name;
                }
                None
            }
            FormReply::Created(Ok(project)) => Some(FormEvent::Created(project)),
            FormReply::Created(Err(e)) => {
                self.note = Some(Err(e.to_string()));
                None
            }
        }
    }

    /// Draws the form's fields and buttons, without a frame or a header; `outer_busy` disables
    /// them while the host runs a command of its own.
    pub(crate) fn ui(
        &mut self,
        ui: &mut egui::Ui,
        host: &Host<'_>,
        outer_busy: bool,
    ) -> Option<FormEvent> {
        let mut event = self.poll();
        if self.busy() {
            ui.ctx().request_repaint_after(Duration::from_millis(50));
        }
        let busy = self.busy() || outer_busy;
        let palette = Palette::current(ui);
        ui.label(theme::bold("Name", 14.0));
        field(ui, &mut self.name, 100);
        ui.label(
            egui::RichText::new("Also the name of the model's folder.")
                .small()
                .color(palette.muted),
        );
        ui.add_space(8.0);
        ui.label(theme::bold("Location", 14.0));
        ui.horizontal(|ui| {
            let location = ui.add(
                egui::TextEdit::singleline(&mut self.parent)
                    .hint_text("Choose a folder with Browse…")
                    .desired_width(ui.available_width() - 130.0)
                    .min_size(egui::vec2(0.0, 42.0))
                    .align(egui::Align2::LEFT_CENTER)
                    .margin(egui::Margin::symmetric(12, 8))
                    .char_limit(MAX_FIELD),
            );
            if location.lost_focus() && !outer_busy {
                self.suggest();
            }
            if theme::button(ui, Kind::Secondary, "Browse…", !busy).clicked() {
                match browse_folder("Where to create the model", self.parent.trim()) {
                    Ok(Some(dir)) => {
                        self.parent = dir;
                        self.suggest();
                    }
                    Ok(None) => {}
                    Err(e) => self.note = Some(Err(e)),
                }
            }
        });
        ui.add_space(8.0);
        ui.label(theme::bold("Template", 14.0));
        ui.horizontal(|ui| {
            template_card(ui, "Blank", "An empty model with no grids or levels.", true);
            template_card(ui, "Grid and levels", "Coming with the model tools.", false);
            template_card(
                ui,
                "Your templates",
                "Save any model as a template, later.",
                false,
            );
        });
        ui.add_space(8.0);
        ui.label(theme::bold("Who works on it", 14.0));
        ui.horizontal(|ui| {
            let _ = ui.radio(true, "Just me");
            ui.add_enabled(false, egui::RadioButton::new(false, "A team (coming soon)"));
        });
        ui.add_space(12.0);
        ui.horizontal(|ui| {
            let ready = !self.name.trim().is_empty() && !self.parent.trim().is_empty();
            if theme::button(ui, Kind::Primary, "Create and open", ready && !busy).clicked() {
                self.create(host);
            }
            if theme::button(ui, Kind::Secondary, "Cancel", !busy).clicked() && event.is_none() {
                event = Some(FormEvent::Cancelled);
            }
            if busy {
                ui.spinner();
            }
        });
        match &self.note {
            Some(Ok(note)) => {
                ui.label(egui::RichText::new(note).color(palette.muted));
            }
            Some(Err(error)) => {
                ui.label(egui::RichText::new(error).color(palette.error).strong());
            }
            None => {}
        }
        event
    }

    /// Runs `work` on a worker thread; the form polls for its reply.
    fn run_worker(&mut self, work: impl FnOnce() -> FormReply + Send + 'static) {
        let (tx, rx) = std::sync::mpsc::channel();
        let spawned = std::thread::Builder::new()
            .name("beamcraft-start".into())
            .spawn(move || {
                let _ = tx.send(work());
            });
        if spawned.is_ok() {
            self.job = Some(rx);
            self.note = None;
        } else {
            self.note = Some(Err("the command thread could not be started".into()));
        }
    }

    /// Installs a pending job whose reply the test delivers.
    #[cfg(test)]
    pub(crate) fn hold_job(&mut self) {
        let (tx, rx) = std::sync::mpsc::channel();
        self.job = Some(rx);
        self.sender = Some(tx);
    }

    /// Delivers a created project to the job installed with `hold_job`.
    #[cfg(test)]
    pub(crate) fn deliver_created(&mut self, project: ProjectView) {
        if let Some(tx) = &self.sender {
            let _ = tx.send(FormReply::Created(Ok(project)));
        }
    }

    /// Delivers a name suggestion to the job installed with `hold_job`.
    #[cfg(test)]
    pub(crate) fn deliver_suggested(
        &mut self,
        parent: &str,
        previous: &str,
        name: Result<String, ToolError>,
    ) {
        if let Some(tx) = &self.sender {
            let _ = tx.send(FormReply::Suggested {
                parent: parent.to_owned(),
                previous: previous.to_owned(),
                name,
            });
        }
    }

    /// Ends the job installed with `hold_job` without an answer.
    #[cfg(test)]
    pub(crate) fn disconnect(&mut self) {
        self.sender = None;
    }
}

fn template_card(ui: &mut egui::Ui, title: &str, text: &str, available: bool) {
    let palette = Palette::current(ui);
    let frame = if available {
        theme::card(palette)
    } else {
        egui::Frame::new()
            .fill(palette.head)
            .stroke(egui::Stroke::new(1.0, palette.rule))
            .corner_radius(egui::CornerRadius::same(5))
    };
    frame.show(ui, |ui| {
        ui.vertical(|ui| {
            ui.set_width(230.0);
            let (rect, _) = ui.allocate_exact_size(egui::vec2(230.0, 110.0), egui::Sense::hover());
            if available {
                theme::paint_preview(ui.painter(), rect);
            }
            egui::Frame::new()
                .inner_margin(egui::Margin::symmetric(14, 10))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let color = if available {
                            palette.text
                        } else {
                            palette.muted
                        };
                        ui.label(theme::bold(title, 14.0).color(color));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.add_enabled(available, egui::RadioButton::new(available, ""));
                        });
                    });
                    ui.label(egui::RichText::new(text).small().color(palette.muted));
                });
        });
    });
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

    use super::*;
    use std::path::PathBuf;

    fn host_parts() -> (PathBuf, Arc<Stats>) {
        (
            PathBuf::from("missing-test-cli"),
            Arc::new(Stats::default()),
        )
    }

    #[test]
    fn new_model_entry_suggests_a_name_and_keeps_user_edits_during_the_reply() {
        let mut form = NewModelForm::new();
        form.show();
        assert!(form.job.is_none());
        assert_eq!(form.name, DEFAULT_MODEL_NAME);
        form.hold_job();
        form.name = "My structure".into();
        let parent = form.parent.clone();
        form.deliver_suggested(&parent, DEFAULT_MODEL_NAME, Ok("New model 3".into()));
        assert!(form.poll().is_none());
        assert_eq!(form.name, "My structure");
        assert!(form.job.is_none());
    }

    #[test]
    fn a_suggestion_replaces_an_untouched_name() {
        let mut form = NewModelForm::new();
        form.seed_parent("/m");
        form.seed_parent("/other");
        assert_eq!(form.parent(), "/m");
        form.hold_job();
        form.deliver_suggested("/m", DEFAULT_MODEL_NAME, Ok("New Model 3".into()));
        assert!(form.poll().is_none());
        assert_eq!(form.name, "New Model 3");
        assert_eq!(form.suggested, "New Model 3");
    }

    #[test]
    fn a_missing_location_is_refused_without_a_command() {
        let (cli, stats) = host_parts();
        let host = Host {
            cli: &cli,
            config_dir: None,
            actor: None,
            environment: None,
            stats: &stats,
        };
        let mut form = NewModelForm::new();
        form.parent = std::env::temp_dir()
            .join(format!("beamcraft-library-missing-{}", std::process::id()))
            .to_string_lossy()
            .into_owned();
        form.create(&host);
        assert!(form.job.is_none());
        let note = form.note.clone().unwrap().unwrap_err();
        assert!(note.contains("does not exist"), "{note}");
    }

    #[test]
    fn a_disconnected_worker_is_a_note() {
        let mut form = NewModelForm::new();
        form.hold_job();
        form.disconnect();
        assert!(form.poll().is_none());
        assert!(!form.busy());
        assert_eq!(
            form.note,
            Some(Err("the command thread ended without an answer".into()))
        );
    }

    #[test]
    fn a_created_reply_is_the_event() {
        let mut form = NewModelForm::new();
        form.hold_job();
        form.deliver_created(ProjectView {
            name: "Nave 1".into(),
            path: "/m/Nave 1".into(),
            state: "ready".into(),
            created_ms: Some(0),
            modified_ms: Some(0),
        });
        assert!(
            matches!(form.poll(), Some(FormEvent::Created(project)) if project.path == "/m/Nave 1")
        );
        assert!(!form.busy());
    }

    #[test]
    fn cancel_reports_cancelled() {
        let ctx = egui::Context::default();
        assert!(crate::app::inter(&ctx));
        theme::install(&ctx);
        let (cli, stats) = host_parts();
        let host = Host {
            cli: &cli,
            config_dir: None,
            actor: None,
            environment: None,
            stats: &stats,
        };
        let mut form = NewModelForm::new();
        let mut frame = |events: Vec<egui::Event>| {
            let mut event = None;
            let output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1280.0, 800.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ui| event = form.ui(ui, &host, false),
            );
            let labels: Vec<_> = output
                .shapes
                .iter()
                .filter_map(|shape| {
                    if let egui::Shape::Text(text) = &shape.shape {
                        Some((
                            text.galley.text().to_owned(),
                            text.pos + text.galley.size() / 2.0,
                        ))
                    } else {
                        None
                    }
                })
                .collect();
            output.drop_without_applying_deltas();
            (labels, event)
        };
        let (labels, event) = frame(vec![]);
        assert!(event.is_none());
        let cancel = labels.iter().find(|(text, _)| text == "Cancel").unwrap().1;
        let (_, event) = frame(vec![
            egui::Event::PointerMoved(cancel),
            egui::Event::PointerButton {
                pos: cancel,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
            egui::Event::PointerButton {
                pos: cancel,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            },
        ]);
        assert!(matches!(event, Some(FormEvent::Cancelled)), "{event:?}");
    }
}
