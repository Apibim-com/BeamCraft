// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! T01.6 pure grid forms, bounded CLI listing decoding and session-only display state.
use beamcraft_json::Json;
use beamcraft_model::{
    grid::{DrawnGridLine, Grid, GridFamily, RegularGridFamily, parse_grid_expression},
    id::ElementId,
};
use std::{collections::BTreeSet, path::Path};
/// Available grid commands; all mutations use the same CLI as headless clients.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GridAction {
    /// Create a regular or initially drawn grid.
    #[default]
    Create,
    /// Edit regular families and frame.
    Update,
    /// Add a drawn line.
    Draw,
    /// Replace endpoints of a drawn line.
    Reshape,
    /// Remove one line.
    Remove,
    /// Relabel one line or a complete family.
    Relabel,
}
impl GridAction {
    /// User-visible action choices.
    pub const ALL: [Self; 6] = [
        Self::Create,
        Self::Update,
        Self::Draw,
        Self::Reshape,
        Self::Remove,
        Self::Relabel,
    ];
    /// Display label.
    pub const fn title(self) -> &'static str {
        match self {
            Self::Create => "New grid",
            Self::Update => "Edit grid",
            Self::Draw => "Draw line",
            Self::Reshape => "Reshape line",
            Self::Remove => "Remove line",
            Self::Relabel => "Relabel",
        }
    }
    fn command(self) -> &'static str {
        match self {
            Self::Create => "create-grid",
            Self::Update => "update-grid",
            Self::Draw => "draw-grid-line",
            Self::Reshape => "reshape-grid-line",
            Self::Remove => "remove-grid-line",
            Self::Relabel => "relabel-grid-line",
        }
    }
}
/// Text fields forwarded to engine-backed CLI commands.
#[derive(Debug, Clone, PartialEq)]
pub struct GridForm {
    baseline: Option<Box<GridForm>>,
    /// Selected action.
    pub action: GridAction,
    /// Existing GridId.
    pub target: String,
    /// Grid label.
    pub name: String,
    /// XY origin.
    pub origin: String,
    /// Degrees at the CLI edge.
    pub rotation: String,
    /// U expression; empty at creation chooses a drawn family.
    pub u_expression: String,
    /// Full U labels, or empty for creation's numeric helper.
    pub u_labels: String,
    /// Optional U span.
    pub u_span: String,
    /// V expression; empty at creation leaves V drawn.
    pub v_expression: String,
    /// Full V labels, or empty for creation's alpha helper.
    pub v_labels: String,
    /// Optional V span.
    pub v_span: String,
    /// Drawn/relabel family selector.
    pub family: String,
    /// Existing line identity for reshape/remove/relabel.
    pub axis_id: String,
    /// Drawn line label or replacement label.
    pub label: String,
    /// World XY start.
    pub from: String,
    /// World XY end.
    pub to: String,
}
impl Default for GridForm {
    fn default() -> Self {
        Self {
            baseline: None,
            action: GridAction::Create,
            target: String::new(),
            name: "Grid".into(),
            origin: "0,0".into(),
            rotation: "0".into(),
            u_expression: "0 5*6000".into(),
            u_labels: String::new(),
            u_span: String::new(),
            v_expression: "0 3*6000".into(),
            v_labels: String::new(),
            v_span: String::new(),
            family: "u".into(),
            axis_id: String::new(),
            label: "D1".into(),
            from: "0,0".into(),
            to: "0,6000".into(),
        }
    }
}
impl GridForm {
    /// Build the exact command to preview and commit. Geometry admission stays in the engine.
    pub fn prepare(&self, store: &Path) -> Result<Vec<String>, String> {
        let root = store.to_str().ok_or("store path is not UTF-8")?;
        let mut a = vec![self.action.command().into(), "--store".into(), root.into()];
        let mut push = |key: &str, value: &str, required: bool| -> Result<(), String> {
            if value.len() > 16384 || (required && value.trim().is_empty()) {
                return Err(format!("{key}: enter a bounded value"));
            }
            if !value.is_empty() {
                a.extend([key.to_owned(), value.to_owned()]);
            }
            Ok(())
        };
        match self.action {
            GridAction::Create | GridAction::Update => {
                if self.action == GridAction::Update {
                    push("--id", &self.target, true)?;
                }
                for (key, value, previous) in [
                    (
                        "--name",
                        &self.name,
                        self.baseline.as_ref().map(|b| &b.name),
                    ),
                    (
                        "--origin",
                        &self.origin,
                        self.baseline.as_ref().map(|b| &b.origin),
                    ),
                    (
                        "--rotation",
                        &self.rotation,
                        self.baseline.as_ref().map(|b| &b.rotation),
                    ),
                ] {
                    if self.action == GridAction::Create || previous != Some(value) {
                        push(key, value, true)?;
                    }
                }
                for (u, expression, labels, span, ue, ul, us) in [
                    (
                        true,
                        &self.u_expression,
                        &self.u_labels,
                        &self.u_span,
                        "--u-expression",
                        "--u-labels",
                        "--u-span",
                    ),
                    (
                        false,
                        &self.v_expression,
                        &self.v_labels,
                        &self.v_span,
                        "--v-expression",
                        "--v-labels",
                        "--v-span",
                    ),
                ] {
                    let old = self
                        .baseline
                        .as_ref()
                        .filter(|_| self.action == GridAction::Update)
                        .map(|b| {
                            if u {
                                (&b.u_expression, &b.u_labels, &b.u_span)
                            } else {
                                (&b.v_expression, &b.v_labels, &b.v_span)
                            }
                        });
                    if self.action == GridAction::Update
                        && old.is_some_and(|(e, _, _)| e.is_empty() != expression.is_empty())
                    {
                        return Err(
                            "Family modes cannot change; use the line tool for drawn families"
                                .into(),
                        );
                    }
                    if !expression.is_empty() {
                        if old.is_none_or(|(e, l, _)| e != expression || l != labels) {
                            push(ue, expression, true)?;
                            push(ul, labels, self.action == GridAction::Update)?;
                        }
                        if old.is_none_or(|(_, _, s)| s != span) {
                            push(us, span, false)?;
                        }
                    }
                }
                if self.action == GridAction::Create && self.u_expression.is_empty() {
                    push("--u-from", &self.from, true)?;
                    push("--u-to", &self.to, true)?;
                    push("--u-label", &self.label, true)?;
                }
            }
            GridAction::Draw | GridAction::Reshape => {
                push("--id", &self.target, true)?;
                if self.action == GridAction::Draw {
                    push("--family", &self.family, true)?;
                    push("--label", &self.label, true)?;
                } else {
                    push("--axis-id", &self.axis_id, true)?;
                }
                push("--from", &self.from, true)?;
                push("--to", &self.to, true)?;
            }
            GridAction::Remove => {
                push("--id", &self.target, true)?;
                push("--axis-id", &self.axis_id, true)?;
            }
            GridAction::Relabel => {
                push("--id", &self.target, true)?;
                if self.axis_id.is_empty() {
                    push("--family", &self.family, true)?;
                    push(
                        "--labels",
                        if self.family == "u" {
                            &self.u_labels
                        } else {
                            &self.v_labels
                        },
                        true,
                    )?;
                } else {
                    push("--axis-id", &self.axis_id, true)?;
                    push("--label", &self.label, true)?;
                }
            }
        }
        if self.action == GridAction::Update && a.len() == 5 {
            return Err("No grid changes to preview".into());
        }
        Ok(a)
    }
    /// Fill an existing grid's editable fields without inventing a mode conversion.
    pub fn select(&mut self, g: &Grid) {
        self.target = g.id.to_string();
        self.name = g.name.clone();
        let [x, y] = g.translation_mm;
        self.origin = format!("{x},{y}");
        let [z, w] = g.orientation_zw;
        self.rotation = (2. * z.atan2(w)).to_degrees().to_string();
        for (family, expression, labels, span) in [
            (
                &g.u,
                &mut self.u_expression,
                &mut self.u_labels,
                &mut self.u_span,
            ),
            (
                &g.v,
                &mut self.v_expression,
                &mut self.v_labels,
                &mut self.v_span,
            ),
        ] {
            match family {
                GridFamily::Regular(f) => {
                    *expression = f.expression.clone();
                    *labels = f.labels.join(" ");
                    let [a, b] = f.span_mm;
                    *span = format!("{a},{b}");
                }
                GridFamily::Drawn(lines) => {
                    expression.clear();
                    *labels = lines
                        .iter()
                        .map(|l| l.label.as_str())
                        .collect::<Vec<_>>()
                        .join(" ");
                    span.clear();
                }
            }
        }
        self.axis_id.clear();
        self.baseline = None;
        self.baseline = Some(Box::new(self.clone()));
    }
}
/// Session display state and staged command; changing visibility never creates a command.
#[derive(Debug, Clone)]
pub struct GridTools {
    /// Current form.
    pub form: GridForm,
    /// Pinned model records.
    pub grids: Vec<Grid>,
    /// Manifest pin of the listing, checked against the engineering snapshot before use.
    pub manifest: Option<String>,
    /// Grids hidden only in this window.
    pub hidden: BTreeSet<ElementId>,
    /// Native UI labels enabled.
    pub labels: bool,
    /// Engine snaps enabled for drawing and placement.
    pub snap: bool,
    /// World snap radius in millimetres.
    pub radius_mm: f64,
    /// Two-click drawn-line input is armed.
    pub drawing: bool,
    /// First uncommitted click.
    pub first: Option<[f64; 3]>,
    /// Previewed CLI command, never executed until Commit.
    pub staged: Option<Vec<String>>,
    /// Last command/query result.
    pub note: Option<Result<String, String>>,
}
impl Default for GridTools {
    fn default() -> Self {
        Self {
            form: GridForm::default(),
            grids: Vec::new(),
            manifest: None,
            hidden: BTreeSet::new(),
            labels: true,
            snap: true,
            radius_mm: 100.,
            drawing: false,
            first: None,
            staged: None,
            note: None,
        }
    }
}
impl GridTools {
    /// Cancel staged edits and point capture, leaving the model and visibility alone.
    pub fn cancel(&mut self) {
        self.staged = None;
        self.first = None;
        self.drawing = false;
    }
    /// Apply two clicks to the form only; no revision or command is published.
    pub fn click(&mut self, [x, y, z]: [f64; 3]) {
        match self.first.take() {
            None => self.first = Some([x, y, z]),
            Some([a, b, _]) => {
                self.form.from = format!("{a},{b}");
                self.form.to = format!("{x},{y}");
                self.drawing = false;
                self.staged = None;
            }
        }
    }
    /// Resolve a click with the engine against visible grids at the exact displayed manifest.
    pub fn snap_point(
        &self,
        snapshot: &beamcraft_engine::ViewSnapshot,
        point: [f64; 3],
    ) -> Result<[f64; 3], beamcraft_engine::ViewError> {
        if !self.snap
            || self.manifest.as_deref() != Some(snapshot.pin().revision().to_string().as_str())
        {
            return Ok(point);
        }
        use beamcraft_engine::query::pick::{SnapKind, SnapQuery, snap_with_grids};
        let visible = self
            .grids
            .iter()
            .filter(|g| !self.hidden.contains(&g.id))
            .map(|g| g.id)
            .collect();
        let query = SnapQuery {
            point_mm: point,
            radius_mm: self.radius_mm,
            kinds: vec![
                SnapKind::Vertex,
                SnapKind::AxisEnd,
                SnapKind::Axis,
                SnapKind::GridIntersection,
                SnapKind::GridEnd,
                SnapKind::GridLine,
            ],
            max_hits: 1,
        };
        Ok(
            snap_with_grids(snapshot, &query, &self.grids, Some(&visible))?
                .candidates
                .first()
                .map_or(point, |c| c.point_world_mm),
        )
    }
    /// Read a bounded exact grid listing from the common CLI protocol.
    pub fn apply_listing(&mut self, j: &Json) -> Result<(), String> {
        let values = j
            .get("grids")
            .and_then(Json::array)
            .map_err(|_| "grid listing has no grids")?;
        if values.len() > 64 {
            return Err("too many grids".into());
        }
        let grids = values
            .iter()
            .map(decode_grid)
            .collect::<Result<Vec<_>, _>>()?;
        beamcraft_engine::grid_query::validate(&grids).map_err(|e| e.to_string())?;
        self.manifest = j
            .get("model_manifest")
            .and_then(Json::string)
            .ok()
            .map(str::to_owned);
        self.grids = grids;
        self.hidden
            .retain(|id| self.grids.iter().any(|g| g.id == *id));
        Ok(())
    }
}
fn text<'a>(j: &'a Json, k: &str) -> Result<&'a str, String> {
    j.get(k)
        .and_then(Json::string)
        .ok()
        .filter(|s| s.len() <= 16384)
        .ok_or_else(|| format!("invalid grid field {k}"))
}
fn pair(j: &Json) -> Result<[f64; 2], String> {
    match j.array() {
        Ok([a, b]) => Ok([
            a.float()
                .ok()
                .filter(|v| v.is_finite())
                .ok_or("invalid coordinate")?,
            b.float()
                .ok()
                .filter(|v| v.is_finite())
                .ok_or("invalid coordinate")?,
        ]),
        _ => Err("invalid coordinate pair".into()),
    }
}
fn decode_grid(j: &Json) -> Result<Grid, String> {
    let id = |s: &str| ElementId::parse(s).map_err(|e| e.to_string());
    let family = |key| -> Result<GridFamily, String> {
        let f = j.get(key).map_err(|_| "missing family")?;
        let lines = f
            .get("lines")
            .and_then(Json::array)
            .ok()
            .filter(|a| a.len() <= 200)
            .ok_or("invalid lines")?;
        let lines = lines
            .iter()
            .map(|l| {
                Ok(DrawnGridLine {
                    axis_id: id(text(l, "axis_id")?)?,
                    label: text(l, "label")?.into(),
                    start_mm: pair(
                        l.get("local_start_mm")
                            .map_err(|_| "missing local endpoint")?,
                    )?,
                    end_mm: pair(
                        l.get("local_end_mm")
                            .map_err(|_| "missing local endpoint")?,
                    )?,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        match text(f, "mode")? {
            "drawn" => Ok(GridFamily::Drawn(lines)),
            "regular" => {
                let (expression, positions) = parse_grid_expression(text(f, "expression")?)
                    .map_err(|_| "invalid expression")?;
                Ok(GridFamily::Regular(RegularGridFamily {
                    expression,
                    positions_mm: positions,
                    labels: lines.iter().map(|l| l.label.clone()).collect(),
                    axis_ids: lines.iter().map(|l| l.axis_id).collect(),
                    span_mm: pair(f.get("span_mm").map_err(|_| "missing span")?)?,
                }))
            }
            _ => Err("unknown grid family mode".into()),
        }
    };
    let grid = Grid {
        id: id(text(j, "id")?)?,
        name: text(j, "name")?.into(),
        translation_mm: pair(j.get("origin_mm").map_err(|_| "missing origin")?)?,
        orientation_zw: pair(j.get("orientation_zw").map_err(|_| "missing orientation")?)?,
        u: family("u")?,
        v: family("v")?,
    };
    grid.validate().map_err(|e| e.to_string())?;
    Ok(grid)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    #[test]
    fn cancel_discards_preview_and_clicks_without_changing_visibility() {
        let mut g = GridTools::default();
        g.staged = Some(g.form.prepare(Path::new("store")).unwrap());
        g.drawing = true;
        g.click([1., 2., 0.]);
        g.labels = false;
        g.snap = false;
        g.cancel();
        assert!(g.staged.is_none());
        assert!(g.first.is_none());
        assert!(!g.drawing);
        assert!(!g.labels);
        assert!(!g.snap);
    }
    #[test]
    fn drawn_input_stages_cli_arguments_only() {
        let mut g = GridTools::default();
        g.form.action = GridAction::Draw;
        g.form.target = "01990000-0000-7000-8000-000000000001".into();
        g.click([1., 2., 0.]);
        g.click([3., 4., 0.]);
        let a = g.form.prepare(Path::new("store")).unwrap();
        assert_eq!(a.first().map(String::as_str), Some("draw-grid-line"));
        assert!(a.windows(2).any(|w| w == ["--from", "1,2"]));
        assert!(a.windows(2).any(|w| w == ["--to", "3,4"]));
        assert!(g.staged.is_none());
    }
}
