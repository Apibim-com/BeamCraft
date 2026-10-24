// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! The start window and the start page, shown when the window is opened without a model.
//!
//! The start window asks for the environment and for the author: a name and an e-mail address,
//! saved in the user's settings. The author identity those settings hold is recorded as the actor
//! of every edit made in the window. "Don't show at start" skips the start window next time.
//!
//! The start page lists the recently opened projects, the projects of a folder, and creates a new
//! project from a template. Shared (cloud) projects are not available yet; the tab says so.
//!
//! Everything here runs the same CLI commands a user types (`settings-show`, `settings-set`,
//! `environment-list`, `recent-list`, `recent-add`, `project-list`, `project-create`), each in a
//! separate process on a worker thread; replies are read with [`crate::tools::rejection`] so a
//! refusal is shown with the CLI's code and message. Nothing here reads a store directly.

use crate::command;
use crate::stats::Stats;
use crate::tools::{ToolError, decode, rejection, text};
use beamcraft_json::Json;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, TryRecvError};
use std::time::Duration;

/// How long one start command may run.
const DEADLINE: Duration = Duration::from_secs(60);
/// Longest text in a field.
const MAX_FIELD: usize = 1_024;

/// The author as the settings hold it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Profile {
    /// The author identity (UUIDv7).
    pub actor_id: String,
    /// Display name.
    pub name: String,
    /// E-mail address.
    pub email: String,
}

/// The user's settings as `settings-show` reports them.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SettingsView {
    /// The configuration directory the CLI used.
    pub config_dir: String,
    /// The author, once entered.
    pub profile: Option<Profile>,
    /// The chosen environment id.
    pub environment: Option<String>,
    /// Skip the start window.
    pub skip_launcher: bool,
    /// The folder listed under "All models".
    pub models_dir: Option<String>,
}

/// One environment as `environment-list` reports it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvironmentView {
    /// Its id, `None` when it does not read.
    pub id: Option<String>,
    /// Its title, when declared.
    pub title: Option<String>,
    /// `builtin` or `user`.
    pub origin: String,
    /// Another environment of the same id answers for it.
    pub shadowed: bool,
    /// Why it does not read.
    pub refused: Option<String>,
}

impl EnvironmentView {
    /// The text shown for it in the list.
    pub fn label(&self) -> String {
        match (&self.id, &self.title) {
            (Some(id), Some(title)) if title != id => format!("{title} ({id})"),
            (Some(id), _) => id.clone(),
            (None, _) => "unreadable environment".to_owned(),
        }
    }

    fn usable_id(&self) -> Option<&str> {
        self.id.as_deref().filter(|_| !self.shadowed)
    }
}

/// One project folder as `project-list`, `recent-list` or `project-create` report it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectView {
    /// The folder name.
    pub name: String,
    /// The store directory.
    pub path: String,
    /// `ready`, `empty`, `missing` or the code of an unreadable store.
    pub state: String,
    /// Creation time, Unix milliseconds.
    pub created_ms: Option<u64>,
    /// Time of the last change, Unix milliseconds.
    pub modified_ms: Option<u64>,
}

impl ProjectView {
    /// Whether the project can be opened.
    pub fn ready(&self) -> bool {
        self.state == "ready"
    }
}

fn unreadable() -> ToolError {
    ToolError::local("UNREADABLE_REPLY", "the CLI reply lacks a field")
}

fn reply(exit_code: Option<i32>, stdout: &[u8], outcome: &str) -> Result<Json, ToolError> {
    let reply = match decode(stdout) {
        Ok(reply) => reply,
        Err(error) => {
            return Err(match exit_code {
                Some(0) => error,
                _ => ToolError::local("CLI_FAILED", "the CLI failed without a JSON reply"),
            });
        }
    };
    if let Some(error) = rejection(&reply, exit_code) {
        return Err(error);
    }
    if text(&reply, "outcome").as_deref() != Some(outcome) {
        return Err(ToolError::local("UNREADABLE_REPLY", "unexpected outcome"));
    }
    Ok(reply)
}

fn optional_text(json: &Json, key: &str) -> Result<Option<String>, ToolError> {
    match json.get(key).map_err(|_| unreadable())? {
        Json::Null => Ok(None),
        value => Ok(Some(value.string().map_err(|_| unreadable())?.to_owned())),
    }
}

fn optional_uint(json: &Json, key: &str) -> Option<u64> {
    json.get(key).ok().and_then(|v| v.unsigned().ok())
}

/// Reads the reply of `settings-show` or `settings-set`.
pub fn parse_settings(exit_code: Option<i32>, stdout: &[u8]) -> Result<SettingsView, ToolError> {
    let reply = match reply(exit_code, stdout, "settings") {
        Err(e) if e.code == "UNREADABLE_REPLY" => reply(exit_code, stdout, "settings_saved")?,
        other => other?,
    };
    let settings = reply.get("settings").map_err(|_| unreadable())?;
    let profile = match settings.get("profile").map_err(|_| unreadable())? {
        Json::Null => None,
        p => Some(Profile {
            actor_id: text(p, "actor_id").ok_or_else(unreadable)?,
            name: text(p, "name").ok_or_else(unreadable)?,
            email: text(p, "email").ok_or_else(unreadable)?,
        }),
    };
    Ok(SettingsView {
        config_dir: text(&reply, "config_dir").ok_or_else(unreadable)?,
        profile,
        environment: optional_text(settings, "environment")?,
        skip_launcher: settings
            .get("skip_launcher")
            .and_then(Json::boolean)
            .map_err(|_| unreadable())?,
        models_dir: optional_text(settings, "models_dir")?,
    })
}

/// Reads the reply of `environment-list`.
pub fn parse_environments(
    exit_code: Option<i32>,
    stdout: &[u8],
) -> Result<Vec<EnvironmentView>, ToolError> {
    let reply = reply(exit_code, stdout, "environments")?;
    let list = reply
        .get("environments")
        .and_then(Json::array)
        .map_err(|_| unreadable())?;
    list.iter()
        .map(|e| {
            Ok(EnvironmentView {
                id: optional_text(e, "id")?,
                title: optional_text(e, "title")?,
                origin: text(e, "origin").ok_or_else(unreadable)?,
                shadowed: e
                    .get("shadowed")
                    .and_then(Json::boolean)
                    .map_err(|_| unreadable())?,
                refused: match e.get("refused").map_err(|_| unreadable())? {
                    Json::Null => None,
                    r => Some(format!(
                        "{}: {}",
                        text(r, "code").unwrap_or_default(),
                        text(r, "message").unwrap_or_default()
                    )),
                },
            })
        })
        .collect()
}

fn project_of(json: &Json) -> Result<ProjectView, ToolError> {
    let state = match json.get("state").map_err(|_| unreadable())? {
        Json::String(state) => state.clone(),
        other => text(other, "unreadable").ok_or_else(unreadable)?,
    };
    Ok(ProjectView {
        name: text(json, "name").ok_or_else(unreadable)?,
        path: text(json, "path").ok_or_else(unreadable)?,
        state,
        created_ms: optional_uint(json, "created_ms"),
        modified_ms: optional_uint(json, "modified_ms"),
    })
}

/// Reads the reply of `project-list`.
pub fn parse_projects(
    exit_code: Option<i32>,
    stdout: &[u8],
) -> Result<Vec<ProjectView>, ToolError> {
    let reply = reply(exit_code, stdout, "projects")?;
    reply
        .get("projects")
        .and_then(Json::array)
        .map_err(|_| unreadable())?
        .iter()
        .map(project_of)
        .collect()
}

/// Reads the reply of `recent-list`.
pub fn parse_recent(exit_code: Option<i32>, stdout: &[u8]) -> Result<Vec<ProjectView>, ToolError> {
    let reply = reply(exit_code, stdout, "recent")?;
    reply
        .get("recent")
        .and_then(Json::array)
        .map_err(|_| unreadable())?
        .iter()
        .map(|r| project_of(r.get("project").map_err(|_| unreadable())?))
        .collect()
}

/// Reads the reply of `project-create` or `recent-add`: the project.
pub fn parse_project(
    exit_code: Option<i32>,
    stdout: &[u8],
    outcome: &str,
) -> Result<ProjectView, ToolError> {
    let reply = reply(exit_code, stdout, outcome)?;
    project_of(reply.get("project").map_err(|_| unreadable())?)
}

fn with_config(mut args: Vec<String>, config_dir: Option<&Path>) -> Vec<String> {
    if let Some(dir) = config_dir {
        args.push("--config-dir".into());
        args.push(dir.to_string_lossy().into_owned());
    }
    args
}

/// The `settings-set` arguments that save the start window's choices.
pub fn settings_set_args(
    config_dir: Option<&Path>,
    name: &str,
    email: &str,
    environment: &str,
    skip_launcher: bool,
) -> Vec<String> {
    let args = vec![
        "settings-set".to_owned(),
        "--name".into(),
        name.to_owned(),
        "--email".into(),
        email.to_owned(),
        "--environment".into(),
        environment.to_owned(),
        "--skip-launcher".into(),
        skip_launcher.to_string(),
    ];
    with_config(args, config_dir)
}

/// The `project-create` arguments of the Create tab.
pub fn project_create_args(
    config_dir: Option<&Path>,
    parent: &str,
    name: &str,
    actor_id: Option<&str>,
) -> Vec<String> {
    let mut args = vec![
        "project-create".to_owned(),
        "--parent".into(),
        parent.to_owned(),
        "--name".into(),
        name.to_owned(),
    ];
    if let Some(actor) = actor_id {
        args.push("--actor-id".into());
        args.push(actor.to_owned());
    }
    with_config(args, config_dir)
}

/// A Unix time in milliseconds as `YYYY-MM-DD HH:MM` (UTC).
pub fn format_time(ms: u64) -> String {
    let secs = ms / 1000;
    let days = i64::try_from(secs / 86_400).unwrap_or(0);
    let rem = secs % 86_400;
    // Civil date from days since 1970-01-01 (proleptic Gregorian).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02} {:02}:{:02}",
        rem / 3_600,
        (rem % 3_600) / 60
    )
}

/// The tabs of the start page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    /// Recently opened projects.
    Recent,
    /// The projects of a folder.
    All,
    /// Shared projects (not available yet).
    Shared,
    /// A new project.
    Create,
}

enum Reply {
    Loaded(Result<(SettingsView, Vec<EnvironmentView>, Vec<ProjectView>), ToolError>),
    Saved(Result<(SettingsView, Vec<ProjectView>), ToolError>),
    Listed(Result<Vec<ProjectView>, ToolError>),
    Opened(Result<ProjectView, ToolError>),
}

/// The start window and the start page.
pub struct StartPage {
    cli: PathBuf,
    config_dir: Option<PathBuf>,
    stats: Arc<Stats>,
    job: Option<Receiver<Reply>>,
    launcher: bool,
    loaded: bool,
    settings: SettingsView,
    environments: Vec<EnvironmentView>,
    environment: Option<String>,
    name: String,
    email: String,
    skip: bool,
    tab: Tab,
    recent: Vec<ProjectView>,
    all: Vec<ProjectView>,
    folder: String,
    new_name: String,
    new_parent: String,
    note: Option<Result<String, String>>,
    open: Option<PathBuf>,
}

impl StartPage {
    /// The start page of the CLI `cli`; `config_dir` replaces the user's configuration directory.
    pub fn new(cli: PathBuf, config_dir: Option<PathBuf>, stats: Arc<Stats>) -> Self {
        let mut page = Self {
            cli,
            config_dir,
            stats,
            job: None,
            launcher: true,
            loaded: false,
            settings: SettingsView::default(),
            environments: Vec::new(),
            environment: None,
            name: String::new(),
            email: String::new(),
            skip: false,
            tab: Tab::Recent,
            recent: Vec::new(),
            all: Vec::new(),
            folder: String::new(),
            new_name: String::new(),
            new_parent: String::new(),
            note: None,
            open: None,
        };
        page.load();
        page
    }

    /// The author identity of the saved profile, recorded as the actor of the window's edits.
    pub fn actor_id(&self) -> Option<String> {
        self.settings.profile.as_ref().map(|p| p.actor_id.clone())
    }

    /// The configuration directory the window was started with, if any.
    pub fn config_dir(&self) -> Option<&Path> {
        self.config_dir.as_deref()
    }

    /// The chosen environment id.
    pub fn environment(&self) -> Option<&str> {
        self.settings.environment.as_deref()
    }

    fn spawn(&mut self, work: impl FnOnce(&Path, &Stats) -> Reply + Send + 'static) {
        let (tx, rx) = std::sync::mpsc::channel();
        let cli = self.cli.clone();
        let stats = Arc::clone(&self.stats);
        let spawned = std::thread::Builder::new()
            .name("beamcraft-start".into())
            .spawn(move || {
                let _ = tx.send(work(&cli, &stats));
            });
        if spawned.is_ok() {
            self.job = Some(rx);
            self.note = None;
        } else {
            self.note = Some(Err("the command thread could not be started".into()));
        }
    }

    fn load(&mut self) {
        let config = self.config_dir.clone();
        self.spawn(move |cli, stats| {
            let run = |args: Vec<String>| {
                command::run(cli, &with_config(args, config.as_deref()), DEADLINE, stats)
            };
            let loaded = (|| {
                let out = run(vec!["settings-show".into()])?;
                let settings = parse_settings(out.code, &out.stdout)?;
                let out = run(vec!["environment-list".into()])?;
                let environments = parse_environments(out.code, &out.stdout)?;
                let out = run(vec!["recent-list".into()])?;
                let recent = parse_recent(out.code, &out.stdout)?;
                Ok((settings, environments, recent))
            })();
            Reply::Loaded(loaded)
        });
    }

    fn save_launcher(&mut self) {
        let Some(environment) = self.environment.clone() else {
            self.note = Some(Err("choose an environment".into()));
            return;
        };
        let args = settings_set_args(
            self.config_dir.as_deref(),
            self.name.trim(),
            self.email.trim(),
            &environment,
            self.skip,
        );
        let config = self.config_dir.clone();
        self.spawn(move |cli, stats| {
            let saved = (|| {
                let out = command::run(cli, &args, DEADLINE, stats)?;
                let settings = parse_settings(out.code, &out.stdout)?;
                let out = command::run(
                    cli,
                    &with_config(vec!["recent-list".into()], config.as_deref()),
                    DEADLINE,
                    stats,
                )?;
                Ok((settings, parse_recent(out.code, &out.stdout)?))
            })();
            Reply::Saved(saved)
        });
    }

    fn list_folder(&mut self) {
        let folder = self.folder.trim().to_owned();
        if folder.is_empty() {
            self.note = Some(Err("enter a folder".into()));
            return;
        }
        // The folder is remembered for the next start; the list does not wait for that.
        let remember = with_config(
            vec!["settings-set".into(), "--models-dir".into(), folder.clone()],
            self.config_dir.as_deref(),
        );
        self.spawn(move |cli, stats| {
            let _ = command::run(cli, &remember, DEADLINE, stats);
            let args = vec!["project-list".to_owned(), "--dir".into(), folder];
            Reply::Listed(
                command::run(cli, &args, DEADLINE, stats)
                    .and_then(|out| parse_projects(out.code, &out.stdout)),
            )
        });
    }

    fn open_project(&mut self, path: String) {
        let args = with_config(
            vec!["recent-add".into(), "--store".into(), path],
            self.config_dir.as_deref(),
        );
        self.spawn(move |cli, stats| {
            Reply::Opened(
                command::run(cli, &args, DEADLINE, stats)
                    .and_then(|out| parse_project(out.code, &out.stdout, "recent_added")),
            )
        });
    }

    fn create_project(&mut self) {
        let args = project_create_args(
            self.config_dir.as_deref(),
            self.new_parent.trim(),
            self.new_name.trim(),
            self.actor_id().as_deref(),
        );
        self.spawn(move |cli, stats| {
            Reply::Opened(
                command::run(cli, &args, DEADLINE, stats)
                    .and_then(|out| parse_project(out.code, &out.stdout, "project_created")),
            )
        });
    }

    fn adopt(&mut self, settings: SettingsView) {
        if let Some(profile) = &settings.profile {
            self.name.clone_from(&profile.name);
            self.email.clone_from(&profile.email);
        }
        self.skip = settings.skip_launcher;
        if self.folder.is_empty() {
            self.folder = settings.models_dir.clone().unwrap_or_default();
        }
        if self.new_parent.is_empty() {
            self.new_parent = settings.models_dir.clone().unwrap_or_default();
        }
        self.settings = settings;
    }

    fn poll(&mut self) {
        let Some(rx) = &self.job else { return };
        let reply = match rx.try_recv() {
            Ok(reply) => reply,
            Err(TryRecvError::Empty) => return,
            Err(TryRecvError::Disconnected) => {
                self.job = None;
                self.note = Some(Err("the command thread ended without an answer".into()));
                return;
            }
        };
        self.job = None;
        match reply {
            Reply::Loaded(Ok((settings, environments, recent))) => {
                self.loaded = true;
                self.environments = environments;
                let usable = |id: &str| self.environments.iter().any(|e| e.usable_id() == Some(id));
                let saved = settings.environment.clone().filter(|id| usable(id));
                self.environment = saved.clone().or_else(|| {
                    self.environments
                        .iter()
                        .find_map(|e| e.usable_id().map(str::to_owned))
                });
                self.launcher =
                    !(settings.skip_launcher && settings.profile.is_some() && saved.is_some());
                self.recent = recent;
                self.adopt(settings);
            }
            Reply::Saved(Ok((settings, recent))) => {
                self.recent = recent;
                self.adopt(settings);
                self.launcher = false;
            }
            Reply::Listed(Ok(projects)) => {
                self.note = Some(Ok(format!("{} project(s)", projects.len())));
                self.all = projects;
            }
            Reply::Opened(Ok(project)) => self.open = Some(PathBuf::from(project.path)),
            Reply::Loaded(Err(e)) | Reply::Saved(Err(e)) => self.note = Some(Err(e.to_string())),
            Reply::Listed(Err(e)) | Reply::Opened(Err(e)) => {
                self.note = Some(Err(e.to_string()));
            }
        }
    }

    /// Draws the page and returns the store to open, once one was chosen or created.
    pub fn ui(&mut self, ui: &mut egui::Ui) -> Option<PathBuf> {
        self.poll();
        let busy = self.job.is_some();
        if busy {
            ui.ctx().request_repaint_after(Duration::from_millis(50));
        }
        egui::CentralPanel::default().show(ui, |ui| {
            if !self.loaded {
                ui.heading("BeamCraft");
                if busy {
                    ui.spinner();
                } else if ui.button("Retry").clicked() {
                    self.load();
                }
            } else if self.launcher {
                self.launcher_ui(ui, busy);
            } else {
                self.home_ui(ui, busy);
            }
            match &self.note {
                Some(Ok(note)) => {
                    ui.label(note);
                }
                Some(Err(error)) => {
                    ui.colored_label(egui::Color32::from_rgb(230, 80, 60), error);
                }
                None => {}
            }
        });
        self.open.take()
    }

    /// The start window: a compact card in the middle of the window.
    fn launcher_ui(&mut self, ui: &mut egui::Ui, busy: bool) {
        const WIDTH: f32 = 440.0;
        let margin = ((ui.available_width() - WIDTH) / 2.0).max(0.0);
        ui.add_space((ui.available_height() * 0.2).min(160.0));
        ui.horizontal(|ui| {
            ui.add_space(margin);
            egui::Frame::group(ui.style())
                .inner_margin(16.0)
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.set_width(WIDTH);
                        self.launcher_form(ui, busy);
                    });
                });
        });
    }

    fn launcher_form(&mut self, ui: &mut egui::Ui, busy: bool) {
        ui.vertical_centered(|ui| ui.heading("BeamCraft"));
        ui.add_space(12.0);
        ui.label("Choose the environment and enter the author of your changes.");
        ui.add_space(8.0);
        egui::Grid::new("start-launcher")
            .num_columns(2)
            .spacing([12.0, 8.0])
            .show(ui, |ui| {
                ui.label("Environment");
                let selected = self
                    .environments
                    .iter()
                    .find(|e| e.usable_id() == self.environment.as_deref() && e.id.is_some())
                    .map_or_else(|| "choose…".to_owned(), EnvironmentView::label);
                egui::ComboBox::from_id_salt("start-environment")
                    .width(280.0)
                    .selected_text(selected)
                    .show_ui(ui, |ui| {
                        for env in &self.environments {
                            if let Some(id) = env.usable_id() {
                                ui.selectable_value(
                                    &mut self.environment,
                                    Some(id.to_owned()),
                                    env.label(),
                                );
                            }
                        }
                    });
                ui.end_row();
                ui.label("Name");
                ui.add(
                    egui::TextEdit::singleline(&mut self.name)
                        .desired_width(280.0)
                        .char_limit(128),
                );
                ui.end_row();
                ui.label("E-mail");
                ui.add(
                    egui::TextEdit::singleline(&mut self.email)
                        .desired_width(280.0)
                        .char_limit(254),
                );
                ui.end_row();
            });
        for env in &self.environments {
            if let Some(reason) = &env.refused {
                ui.small(format!("An environment was not loaded ({reason})."));
            }
        }
        if self.environments.iter().all(|e| e.usable_id().is_none()) {
            ui.small(
                "No environment was found; reinstall BeamCraft or add one to the settings folder.",
            );
        }
        ui.small("The name and e-mail mark the parts you create or change; they are no login.");
        ui.checkbox(&mut self.skip, "Don't show this window at start");
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if ui.add_enabled(!busy, egui::Button::new("OK")).clicked() {
                self.save_launcher();
            }
            if ui.add_enabled(!busy, egui::Button::new("Cancel")).clicked() {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            }
            if busy {
                ui.spinner();
            }
        });
    }

    fn home_ui(&mut self, ui: &mut egui::Ui, busy: bool) {
        ui.horizontal(|ui| {
            ui.heading("BeamCraft");
            ui.add_space(24.0);
            let author = self
                .settings
                .profile
                .as_ref()
                .map_or_else(String::new, |p| format!("{} <{}>", p.name, p.email));
            ui.label(format!(
                "Environment: {}   Author: {author}",
                self.settings.environment.as_deref().unwrap_or("none")
            ));
            if ui
                .add_enabled(!busy, egui::Button::new("Change…"))
                .clicked()
            {
                self.launcher = true;
            }
        });
        ui.separator();
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.tab, Tab::Recent, "Recent");
            ui.selectable_value(&mut self.tab, Tab::All, "All models");
            ui.selectable_value(&mut self.tab, Tab::Shared, "Shared models");
            ui.selectable_value(&mut self.tab, Tab::Create, "Create");
        });
        ui.separator();
        match self.tab {
            Tab::Recent => {
                let projects = self.recent.clone();
                if projects.is_empty() {
                    ui.label(
                        "No project was opened yet. Create one or list a folder under All models.",
                    );
                }
                self.project_table(ui, "start-recent", &projects, busy);
            }
            Tab::All => {
                ui.horizontal(|ui| {
                    ui.label("List the models in");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.folder)
                            .desired_width(420.0)
                            .char_limit(MAX_FIELD),
                    );
                    if ui
                        .add_enabled(!busy, egui::Button::new("Refresh"))
                        .clicked()
                    {
                        self.list_folder();
                    }
                });
                let projects = self.all.clone();
                self.project_table(ui, "start-all", &projects, busy);
            }
            Tab::Shared => {
                ui.label("Shared models in the BeamCraft cloud are coming soon.");
            }
            Tab::Create => self.create_ui(ui, busy),
        }
    }

    fn project_table(&mut self, ui: &mut egui::Ui, id: &str, projects: &[ProjectView], busy: bool) {
        let mut open = None;
        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::Grid::new(id)
                .num_columns(4)
                .striped(true)
                .spacing([16.0, 6.0])
                .show(ui, |ui| {
                    ui.strong("Name");
                    ui.strong("Created (UTC)");
                    ui.strong("Modified (UTC)");
                    ui.strong("Folder");
                    ui.end_row();
                    for project in projects {
                        let time = |ms: Option<u64>| ms.map(format_time).unwrap_or_default();
                        let button = egui::Button::new(egui::RichText::new(&project.name).strong())
                            .frame(false);
                        let clicked = ui
                            .add_enabled(project.ready() && !busy, button)
                            .on_disabled_hover_text(format!("cannot be opened: {}", project.state))
                            .clicked();
                        if clicked {
                            open = Some(project.path.clone());
                        }
                        ui.label(time(project.created_ms));
                        ui.label(time(project.modified_ms));
                        ui.small(&project.path);
                        ui.end_row();
                    }
                });
        });
        if let Some(path) = open {
            self.open_project(path);
        }
    }

    fn create_ui(&mut self, ui: &mut egui::Ui, busy: bool) {
        egui::Grid::new("start-create")
            .num_columns(2)
            .spacing([12.0, 8.0])
            .show(ui, |ui| {
                ui.label("Name");
                ui.add(
                    egui::TextEdit::singleline(&mut self.new_name)
                        .desired_width(420.0)
                        .char_limit(100),
                );
                ui.end_row();
                ui.label("Place in");
                ui.add(
                    egui::TextEdit::singleline(&mut self.new_parent)
                        .desired_width(420.0)
                        .char_limit(MAX_FIELD),
                );
                ui.end_row();
                ui.label("Mode");
                ui.horizontal(|ui| {
                    let _ = ui.radio(true, "Single-user");
                    ui.add_enabled(false, egui::RadioButton::new(false, "Multi-user (later)"));
                });
                ui.end_row();
                ui.label("Template");
                ui.horizontal(|ui| {
                    let _ = ui.radio(true, "Blank");
                });
                ui.end_row();
            });
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            let ready = !self.new_name.trim().is_empty() && !self.new_parent.trim().is_empty();
            if ui
                .add_enabled(ready && !busy, egui::Button::new("Create"))
                .clicked()
            {
                self.create_project();
            }
            if busy {
                ui.spinner();
            }
        });
        ui.small("A new folder of that name is made in the chosen place; it must not exist yet.");
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

    use super::*;

    #[test]
    fn reads_settings_replies() {
        let out = br#"{"config_dir":"/c","outcome":"settings","settings":{"environment":"blank","models_dir":null,"profile":{"actor_id":"0199e7a0-0000-7000-8000-000000000001","email":"a@b.cl","name":"Ana"},"recent":[],"skip_launcher":true,"version":1}}"#;
        let s = parse_settings(Some(0), out).unwrap();
        assert_eq!(s.environment.as_deref(), Some("blank"));
        assert_eq!(s.profile.unwrap().name, "Ana");
        assert!(s.skip_launcher && s.models_dir.is_none());
        let saved = String::from_utf8(out.to_vec())
            .unwrap()
            .replace("\"settings\",", "\"settings_saved\",");
        assert!(parse_settings(Some(0), saved.as_bytes()).is_ok());
        let refused = br#"{"code":"INVALID_ARGUMENT","diagnostics":[],"message":"bad e-mail","observed_context":null,"outcome":"rejected"}"#;
        let e = parse_settings(Some(2), refused).unwrap_err();
        assert_eq!(
            (e.code.as_str(), e.message.as_str()),
            ("INVALID_ARGUMENT", "bad e-mail")
        );
    }

    #[test]
    fn reads_environments_and_projects() {
        let out = br#"{"environments":[{"id":"blank","origin":"builtin","path":"/e","refused":null,"shadowed":false,"title":"Blank"},{"id":null,"origin":"user","path":"/u","refused":{"code":"INVALID_ARGUMENT","message":"no id"},"shadowed":false,"title":null}],"outcome":"environments","user_dir":null}"#;
        let envs = parse_environments(Some(0), out).unwrap();
        assert_eq!(envs[0].label(), "Blank (blank)");
        assert_eq!(envs[0].usable_id(), Some("blank"));
        assert_eq!(envs[1].usable_id(), None);
        assert!(envs[1].refused.as_deref().unwrap().contains("no id"));
        let out = br#"{"dir":"/m","outcome":"projects","projects":[{"created_ms":1792799652530,"modified_ms":1792799652605,"name":"A","path":"/m/A","project_id":"x","revision":null,"state":"ready"},{"created_ms":null,"modified_ms":null,"name":"B","path":"/m/B","project_id":null,"revision":null,"state":{"unreadable":"INTEGRITY_ERROR"}}],"truncated":false}"#;
        let projects = parse_projects(Some(0), out).unwrap();
        assert!(projects[0].ready());
        assert_eq!(projects[1].state, "INTEGRITY_ERROR");
        assert!(!projects[1].ready());
    }

    #[test]
    fn builds_arguments() {
        let args = project_create_args(Some(Path::new("/c")), "/m", "Nave 1", Some("id"));
        assert_eq!(
            args,
            [
                "project-create",
                "--parent",
                "/m",
                "--name",
                "Nave 1",
                "--actor-id",
                "id",
                "--config-dir",
                "/c"
            ]
        );
        let args = settings_set_args(None, "Ana", "a@b.cl", "blank", false);
        assert_eq!(args.last().map(String::as_str), Some("false"));
    }

    #[test]
    fn formats_utc_times() {
        assert_eq!(format_time(0), "1970-01-01 00:00");
        assert_eq!(format_time(951_782_400_000), "2000-02-29 00:00");
        assert_eq!(format_time(1_792_799_652_530), "2026-10-23 23:54");
    }
}
