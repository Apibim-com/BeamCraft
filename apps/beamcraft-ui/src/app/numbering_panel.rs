// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! The Numbering bench (#846): preview part and assembly numbering (`preview-numbering`),
//! inspect stored approval status (`inspect-numbering`), and commit approved marks to the
//! model (`approve-numbering`).

use super::App;
use crate::command;
use crate::tools::{ToolError, decode, rejection, text};
use beamcraft_json::Json;
use std::path::Path;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, TryRecvError, channel};

/// One position group returned by `preview-numbering`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GroupRow {
    /// Position mark (`P1`, `B1`, `C1`, `A1`, ...).
    pub(crate) position: String,
    /// Kind (`plate`, `beam`, `column`, `assembly`).
    pub(crate) kind: String,
    /// Number of equivalent elements in the group.
    pub(crate) count: u64,
    /// Group status (`ok` or `overlapping`).
    pub(crate) status: String,
    /// Stored mark comparison (`same`, `different`, `unmarked`, `mixed`).
    pub(crate) stored: String,
    /// Human-readable description of the group.
    pub(crate) description: String,
}

/// Parsed result of `preview-numbering`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PreviewReport {
    /// SHA-256 preview token required by `approve-numbering --preview-token`.
    pub(crate) preview_token: String,
    /// Approval state (`current`, `stale`, `unapproved`, ...).
    pub(crate) state: String,
    /// Elements with stored marks.
    pub(crate) marked: u64,
    /// Elements without stored marks.
    pub(crate) unmarked: u64,
    /// Elements whose stored mark differs from the proposal.
    pub(crate) differing: u64,
    /// Proposed position groups.
    pub(crate) groups: Vec<GroupRow>,
}

/// State of the Numbering bench between frames.
pub(crate) struct Panel {
    /// `--plate-series` (`PREFIX:START`).
    pub(crate) plate_series: String,
    /// `--beam-series` (`PREFIX:START`).
    pub(crate) beam_series: String,
    /// `--column-series` (`PREFIX:START`).
    pub(crate) column_series: String,
    /// `--assembly-series` (`PREFIX:START`).
    pub(crate) assembly_series: String,
    /// `--tolerance` in mm.
    pub(crate) tolerance: String,
    /// `--compare-name`.
    pub(crate) compare_name: bool,
    /// Latest preview report.
    pub(crate) report: Option<PreviewReport>,
    pub(super) preview_job: Option<Receiver<Result<PreviewReport, ToolError>>>,
    pub(super) approve_job: Option<Receiver<Result<String, ToolError>>>,
    /// Last outcome or refusal message.
    pub(crate) note: Option<Result<String, String>>,
}

impl Panel {
    pub(super) fn empty() -> Self {
        Self {
            plate_series: "P:1".into(),
            beam_series: "B:1".into(),
            column_series: "C:1".into(),
            assembly_series: "A:1".into(),
            tolerance: "1".into(),
            compare_name: false,
            report: None,
            preview_job: None,
            approve_job: None,
            note: None,
        }
    }

    pub(crate) fn busy(&self) -> bool {
        self.preview_job.is_some() || self.approve_job.is_some()
    }
}

fn push_series_options(
    args: &mut Vec<String>,
    plate: &str,
    beam: &str,
    column: &str,
    assembly: &str,
    tolerance: &str,
    compare_name: bool,
) -> Result<(), String> {
    for (label, val, flag) in [
        ("Plate series", plate, "--plate-series"),
        ("Beam series", beam, "--beam-series"),
        ("Column series", column, "--column-series"),
        ("Assembly series", assembly, "--assembly-series"),
    ] {
        let val = val.trim();
        if !val.is_empty() {
            if !val.contains(':') {
                return Err(format!("{label}: enter PREFIX:START (for example P:1)."));
            }
            args.extend([flag.to_owned(), val.to_owned()]);
        }
    }
    let tol = tolerance.trim();
    if !tol.is_empty() {
        let parsed = tol
            .parse::<f64>()
            .map_err(|_| format!("Tolerance: \"{tol}\" is not a valid number."))?;
        if !parsed.is_finite() || parsed < 0.0 {
            return Err("Tolerance: enter a non-negative number of millimetres.".into());
        }
        args.extend(["--tolerance".into(), tol.to_owned()]);
    }
    args.extend([
        "--compare-name".into(),
        if compare_name { "true" } else { "false" }.into(),
    ]);
    Ok(())
}

/// Builds the CLI arguments for `preview-numbering`.
pub(crate) fn preview_args(
    store: &Path,
    plate: &str,
    beam: &str,
    column: &str,
    assembly: &str,
    tolerance: &str,
    compare_name: bool,
) -> Result<Vec<String>, String> {
    let mut args = vec![
        "preview-numbering".into(),
        "--store".into(),
        store.to_string_lossy().into_owned(),
    ];
    push_series_options(
        &mut args,
        plate,
        beam,
        column,
        assembly,
        tolerance,
        compare_name,
    )?;
    Ok(args)
}

/// Builds the CLI arguments for `approve-numbering`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn approve_args(
    store: &Path,
    actor: Option<&str>,
    preview_token: &str,
    plate: &str,
    beam: &str,
    column: &str,
    assembly: &str,
    tolerance: &str,
    compare_name: bool,
) -> Result<Vec<String>, String> {
    let token = preview_token.trim();
    if token.is_empty() {
        return Err("Run Preview numbering first to obtain a preview token.".into());
    }
    let mut args = vec![
        "approve-numbering".into(),
        "--store".into(),
        store.to_string_lossy().into_owned(),
        "--preview-token".into(),
        token.to_owned(),
    ];
    push_series_options(
        &mut args,
        plate,
        beam,
        column,
        assembly,
        tolerance,
        compare_name,
    )?;
    if let Some(actor) = actor {
        args.extend(["--actor-id".into(), actor.to_owned()]);
    }
    Ok(args)
}

/// Parses the JSON output of `preview-numbering`.
pub(crate) fn parse_preview(
    exit_code: Option<i32>,
    stdout: &[u8],
) -> Result<PreviewReport, ToolError> {
    let reply = decode(stdout)?;
    if let Some(error) = rejection(&reply, exit_code) {
        return Err(error);
    }
    let preview_token = text(&reply, "preview_token")
        .ok_or_else(|| ToolError::local("UNREADABLE_REPLY", "missing preview_token"))?;
    let approval = reply
        .get("approval")
        .map_err(|_| ToolError::local("UNREADABLE_REPLY", "missing approval"))?;
    let state = text(approval, "state").unwrap_or_else(|| "unknown".into());
    let marked = approval.get("marked").and_then(Json::unsigned).unwrap_or(0);
    let unmarked = approval
        .get("unmarked")
        .and_then(Json::unsigned)
        .unwrap_or(0);
    let differing = approval
        .get("differing")
        .and_then(Json::unsigned)
        .unwrap_or(0);
    let raw_groups = reply
        .get("groups")
        .and_then(Json::array)
        .map_err(|_| ToolError::local("UNREADABLE_REPLY", "missing groups array"))?;
    let mut groups = Vec::with_capacity(raw_groups.len());
    for g in raw_groups {
        groups.push(GroupRow {
            position: text(g, "position").unwrap_or_default(),
            kind: text(g, "kind").unwrap_or_default(),
            count: g.get("count").and_then(Json::unsigned).unwrap_or(0),
            status: text(g, "status").unwrap_or_default(),
            stored: text(g, "stored").unwrap_or_default(),
            description: text(g, "description").unwrap_or_default(),
        });
    }
    Ok(PreviewReport {
        preview_token,
        state,
        marked,
        unmarked,
        differing,
        groups,
    })
}

impl App {
    /// Starts a `preview-numbering` run on a background worker.
    pub(crate) fn numbering_preview(&mut self) {
        if self.source.read_only() || self.numbering.busy() {
            return;
        }
        let args = match preview_args(
            &self.source.store,
            &self.numbering.plate_series,
            &self.numbering.beam_series,
            &self.numbering.column_series,
            &self.numbering.assembly_series,
            &self.numbering.tolerance,
            self.numbering.compare_name,
        ) {
            Ok(args) => args,
            Err(msg) => {
                self.numbering.note = Some(Err(msg));
                return;
            }
        };
        let cli = self.source.cli.clone();
        let stats = Arc::clone(&self.stats);
        let (tx, rx) = channel();
        let spawned = std::thread::Builder::new()
            .name("beamcraft-numbering-preview".into())
            .spawn(move || {
                let res = command::run(&cli, &args, command::DEADLINE, &stats)
                    .and_then(|out| parse_preview(out.code, &out.stdout));
                let _ = tx.send(res);
            });
        match spawned {
            Ok(_) => {
                self.numbering.preview_job = Some(rx);
                self.numbering.note = None;
            }
            Err(_) => {
                self.numbering.note =
                    Some(Err("The numbering preview could not be started.".into()));
            }
        }
    }

    /// Starts an `approve-numbering` run on a background worker.
    pub(crate) fn numbering_approve(&mut self) {
        if self.other_command_running() || self.numbering.busy() {
            self.numbering.note = Some(Err("Another command is running; wait for it.".into()));
            return;
        }
        let token = self
            .numbering
            .report
            .as_ref()
            .map_or("", |r| r.preview_token.as_str());
        let args = match approve_args(
            &self.source.store,
            self.source.actor.as_deref(),
            token,
            &self.numbering.plate_series,
            &self.numbering.beam_series,
            &self.numbering.column_series,
            &self.numbering.assembly_series,
            &self.numbering.tolerance,
            self.numbering.compare_name,
        ) {
            Ok(args) => args,
            Err(msg) => {
                self.numbering.note = Some(Err(msg));
                return;
            }
        };
        let cli = self.source.cli.clone();
        let stats = Arc::clone(&self.stats);
        let (tx, rx) = channel();
        let spawned = std::thread::Builder::new()
            .name("beamcraft-numbering-approve".into())
            .spawn(move || {
                let res = command::run(&cli, &args, command::DEADLINE, &stats).and_then(|out| {
                    let reply = decode(&out.stdout)?;
                    if let Some(err) = rejection(&reply, out.code) {
                        return Err(err);
                    }
                    let marks = reply.get("marks").and_then(Json::unsigned).unwrap_or(0);
                    let groups = reply.get("groups").and_then(Json::unsigned).unwrap_or(0);
                    Ok(format!(
                        "Numbering approved: {marks} mark(s) across {groups} group(s)."
                    ))
                });
                let _ = tx.send(res);
            });
        match spawned {
            Ok(_) => {
                self.numbering.approve_job = Some(rx);
                self.numbering.note = None;
            }
            Err(_) => {
                self.numbering.note =
                    Some(Err("The numbering approval could not be started.".into()));
            }
        }
    }

    /// Polls background preview and approval jobs for the Numbering bench.
    pub(crate) fn poll_numbering(&mut self) {
        if let Some(rx) = &self.numbering.preview_job {
            match rx.try_recv() {
                Ok(Ok(report)) => {
                    self.numbering.preview_job = None;
                    let summary = format!(
                        "Preview ready: {} group(s) · status: {}.",
                        report.groups.len(),
                        report.state
                    );
                    self.numbering.report = Some(report);
                    self.numbering.note = Some(Ok(summary));
                }
                Ok(Err(err)) => {
                    self.numbering.preview_job = None;
                    self.numbering.note = Some(Err(err.to_string()));
                }
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => {
                    self.numbering.preview_job = None;
                }
            }
        }
        let Some(rx) = &self.numbering.approve_job else {
            return;
        };
        let result = match rx.try_recv() {
            Ok(res) => res,
            Err(TryRecvError::Empty) => return,
            Err(TryRecvError::Disconnected) => Err(ToolError::local(
                "CLI_FAILED",
                "The numbering approval ended without a result.",
            )),
        };
        self.numbering.approve_job = None;
        match result {
            Ok(msg) => {
                self.numbering.note = Some(Ok(msg));
                self.numbering_preview();
                self.reload();
            }
            Err(error) => {
                if error.code == "OUTCOME_UNKNOWN" {
                    self.numbering_preview();
                    self.reload();
                }
                self.numbering.note = Some(Err(error.to_string()));
            }
        }
    }

    /// Renders the Numbering inspector panel.
    pub(crate) fn numbering_panel(&mut self, ui: &mut egui::Ui) {
        use crate::kit;
        use crate::theme::{self, Icon, Kind, Palette};
        let palette = Palette::current(ui);
        ui.set_max_width(ui.available_width());
        kit::type_header(ui, Icon::Numbering, "Numbering", |ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if theme::icon_button(
                    ui,
                    Icon::Refresh,
                    "Preview numbering",
                    !self.numbering.busy() && !self.other_command_running(),
                )
                .clicked()
                {
                    self.numbering_preview();
                }
            });
        });

        let busy = self.numbering.busy() || self.other_command_running();

        kit::section(ui, "num", "Series & settings", true, |ui| {
            super::field(ui, "Plate series", &mut self.numbering.plate_series, 90.0);
            super::field(ui, "Beam series", &mut self.numbering.beam_series, 90.0);
            super::field(ui, "Column series", &mut self.numbering.column_series, 90.0);
            super::field(
                ui,
                "Assembly series",
                &mut self.numbering.assembly_series,
                90.0,
            );
            super::field(ui, "Tolerance, mm", &mut self.numbering.tolerance, 90.0);
            kit::prop_row(ui, "Compare name", |ui| {
                ui.checkbox(&mut self.numbering.compare_name, "");
            });
            kit::prop_row(ui, "", |ui| {
                ui.horizontal(|ui| {
                    if theme::button(ui, Kind::Secondary, "Preview", !busy).clicked() {
                        self.numbering_preview();
                    }
                    let can_approve = !busy && self.numbering.report.is_some();
                    if theme::button(ui, Kind::Primary, "Approve", can_approve).clicked() {
                        self.numbering_approve();
                    }
                });
            });
        });

        if let Some(report) = &self.numbering.report {
            kit::section(ui, "num", "Approval status", true, |ui| {
                kit::prop_info(ui, "State", Some(&report.state), "—");
                let counts = format!(
                    "{} marked · {} unmarked · {} differing",
                    report.marked, report.unmarked, report.differing
                );
                kit::prop_info(ui, "Elements", Some(&counts), "—");
            });

            kit::section(ui, "num", "Positions preview", true, |ui| {
                if report.groups.is_empty() {
                    kit::prop_row(ui, "Positions", |ui| {
                        ui.label(
                            egui::RichText::new("No parts or assemblies in model.")
                                .size(11.0)
                                .color(palette.muted),
                        );
                    });
                } else {
                    for g in &report.groups {
                        let badge = format!("{} ({}× {})", g.position, g.count, g.kind);
                        let detail = if g.description.is_empty() {
                            g.stored.clone()
                        } else {
                            format!("{} · {}", g.description, g.stored)
                        };
                        kit::prop_info(ui, &badge, Some(&detail), "—");
                    }
                }
            });
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn numbering_command_builders_match_cli_contract() {
        let store = Path::new("model");
        let prev = preview_args(store, "P:1", "B:1", "C:1", "A:1", "1.0", true).unwrap();
        assert_eq!(
            prev,
            [
                "preview-numbering",
                "--store",
                "model",
                "--plate-series",
                "P:1",
                "--beam-series",
                "B:1",
                "--column-series",
                "C:1",
                "--assembly-series",
                "A:1",
                "--tolerance",
                "1.0",
                "--compare-name",
                "true",
            ]
        );
        let app = approve_args(
            store,
            Some("actor-1"),
            "tok123",
            "P:1",
            "B:1",
            "C:1",
            "A:1",
            "1",
            false,
        )
        .unwrap();
        assert_eq!(app[0], "approve-numbering");
        assert!(app.contains(&"--preview-token".to_owned()));
        assert!(app.contains(&"tok123".to_owned()));
    }

    #[test]
    fn parse_preview_reads_token_and_groups() {
        let json = br#"{"outcome":"numbering_preview","revision_id":"r1","preview_token":"abc123","approval":{"state":"unapproved","marked":0,"unmarked":2,"differing":0},"groups":[{"kind":"beam","position":"B1","number":1,"status":"ok","count":2,"description":"IPE200","stored":"unmarked"}]}"#;
        let parsed = parse_preview(Some(0), json).unwrap();
        assert_eq!(parsed.preview_token, "abc123");
        assert_eq!(parsed.state, "unapproved");
        assert_eq!(parsed.unmarked, 2);
        assert_eq!(parsed.groups.len(), 1);
        assert_eq!(parsed.groups[0].position, "B1");
        assert_eq!(parsed.groups[0].count, 2);
    }
}
