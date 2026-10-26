// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! Regular-grid insertion and display, using the existing grid commands and engine endpoints.

use crate::camera::View;
use crate::props::RevisionStamp;
use crate::scene::SceneSummary;
use crate::stats::Stats;
use crate::tools::{FormError, ToolError, decode, push_actor, rejection, text};
use crate::{command, theme};
use beamcraft_json::Json;
use beamcraft_model::grid::{
    GridLabelError, MAX_GRID_LINES_PER_FAMILY, MAX_GRIDS, parse_grid_expression, parse_grid_labels,
};
use beamcraft_model::id::ElementId;
use std::path::Path;

/// Inputs of one regular grid family. Empty labels ask the CLI for its default label scheme.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FamilyForm {
    /// First position followed by spacings, in millimetres.
    pub expression: String,
    /// Explicit labels, or empty for automatic labels.
    pub labels: String,
    /// Explicit segment start/end, or both empty for the engine default.
    pub span: [String; 2],
}

/// Editable regular-grid request, in millimetres and display degrees.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GridForm {
    /// Name stored on the grid.
    pub name: String,
    /// World XY origin.
    pub origin: [String; 2],
    /// Rotation about world Z in degrees.
    pub rotation: String,
    /// U lines, parallel to local Y.
    pub u: FamilyForm,
    /// V lines, parallel to local X.
    pub v: FamilyForm,
}

impl Default for GridForm {
    fn default() -> Self {
        Self {
            name: "Grid".into(),
            origin: ["0".into(), "0".into()],
            rotation: "0".into(),
            u: FamilyForm {
                expression: "0 2*4000".into(),
                labels: String::new(),
                span: [String::new(), String::new()],
            },
            v: FamilyForm {
                expression: "0 2*5000".into(),
                labels: String::new(),
                span: [String::new(), String::new()],
            },
        }
    }
}

fn form_error(field: &'static str, message: &str) -> FormError {
    FormError {
        field,
        message: message.to_owned(),
    }
}

fn decimal(field: &'static str, value: &str) -> Result<String, FormError> {
    let value = value.trim();
    if value.len() > 64 || !value.parse::<f64>().is_ok_and(f64::is_finite) {
        return Err(form_error(
            field,
            "Enter a finite number using a decimal point.",
        ));
    }
    Ok(value.to_owned())
}

impl GridForm {
    /// Builds the real `create-grid` arguments. No model is read or changed here.
    pub fn prepare(&self, store: &Path, actor: Option<&str>) -> Result<Vec<String>, FormError> {
        let name = self.name.trim();
        if name.is_empty() || name.len() > 256 {
            return Err(form_error(
                "Name",
                "Enter a grid name of at most 256 UTF-8 bytes.",
            ));
        }
        let store = store
            .to_str()
            .ok_or_else(|| form_error("Model", "The model path must be UTF-8."))?;
        let origin = [
            decimal("Origin X", &self.origin[0])?,
            decimal("Origin Y", &self.origin[1])?,
        ]
        .join(",");
        let mut args = vec![
            "create-grid".into(),
            "--store".into(),
            store.into(),
            "--name".into(),
            name.into(),
            "--origin".into(),
            origin,
            "--rotation".into(),
            decimal("Rotation", &self.rotation)?,
        ];
        for (prefix, family, field, labels_field) in [
            ("u", &self.u, "U coordinates", "U labels"),
            ("v", &self.v, "V coordinates", "V labels"),
        ] {
            let expression = family.expression.trim();
            if expression.is_empty() {
                if !family.labels.trim().is_empty()
                    || family.span.iter().any(|s| !s.trim().is_empty())
                {
                    return Err(form_error(
                        field,
                        "Coordinates are required when labels or line limits are supplied.",
                    ));
                }
                continue;
            }
            let (normalized, positions) = parse_grid_expression(expression).map_err(|_| form_error(field, "Enter a start coordinate followed by positive spacings, for example 0 2*4000. At most 200 lines are allowed."))?;
            args.extend([format!("--{prefix}-expression"), normalized]);
            if !family.labels.trim().is_empty() {
                let labels = parse_grid_labels(&family.labels, positions.len()).map_err(|error| {
                    let message = match error {
                        GridLabelError::Count => {
                            let count = family.labels
                                .split(|c: char| c.is_ascii_whitespace() || c == ',')
                                .filter(|word| !word.is_empty())
                                .count();
                            format!(
                                "Coordinates define {} axes, but only {count} labels were entered. Add the missing labels, or leave Labels blank for automatic labels.",
                                positions.len()
                            )
                        }
                        GridLabelError::Duplicate => "Each axis needs a unique label.".into(),
                        GridLabelError::Invalid | GridLabelError::Limit =>
                            "Use labels of up to 16 letters or digits, with -, _, ., + or /. Separate labels with spaces or commas.".into(),
                    };
                    form_error(labels_field, &message)
                })?;
                args.extend([format!("--{prefix}-labels"), labels.join(" ")]);
            }
            let [start, end] = &family.span;
            if !start.trim().is_empty() || !end.trim().is_empty() {
                args.extend([
                    format!("--{prefix}-span"),
                    [decimal("Span start", start)?, decimal("Span end", end)?].join(","),
                ]);
            }
        }
        if self.u.expression.trim().is_empty() && self.v.expression.trim().is_empty() {
            return Err(form_error("Coordinates", "Enter at least one grid family."));
        }
        push_actor(&mut args, actor);
        Ok(args)
    }
}

/// An accepted grid creation, returned by the real CLI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreatedGrid {
    /// Identity of the new grid.
    pub id: String,
    /// Revision containing it.
    pub revision: RevisionStamp,
}

/// Reads a creation reply while preserving engine refusals.
pub fn parse_created(exit: Option<i32>, bytes: &[u8]) -> Result<CreatedGrid, ToolError> {
    let json = decode(bytes)?;
    if let Some(mut error) = rejection(&json, exit) {
        let message = match error.message.as_str() {
            "GRID_LABEL_DUPLICATE" => Some(
                "Axis labels must be unique across both grid families. Choose different U and V labels.",
            ),
            "GRID_LABEL_COUNT" => Some("Enter exactly one label for each grid line."),
            "GRID_LABEL_INVALID" => {
                Some("Use axis labels of up to 16 letters or digits, with -, _, ., + or /.")
            }
            "GRID_SPACING_INVALID" | "GRID_SPACING_MM" => {
                Some("Each spacing must be at least 1 mm.")
            }
            "GRID_EXPRESSION_INVALID" => {
                Some("Enter a start coordinate followed by positive spacings.")
            }
            "GRID_LINES_PER_FAMILY" => Some("A grid family can contain at most 200 lines."),
            "GRID_COORDINATE_CAP_MM" => {
                Some("Grid coordinates must stay within 1,000,000 mm of the origin.")
            }
            _ => None,
        };
        if let Some(message) = message {
            error.message = message.to_owned();
        }
        return Err(error);
    }
    let bad = || {
        ToolError::local(
            "UNREADABLE_REPLY",
            "The grid command returned an incomplete reply.",
        )
    };
    if !matches!(
        text(&json, "outcome").as_deref(),
        Some("grid_created" | "grid_created_replayed")
    ) {
        return Err(bad());
    }
    let id = text(&json, "element_id")
        .filter(|id| ElementId::parse(id).is_ok())
        .ok_or_else(bad)?;
    let revision = json
        .get("revision")
        .ok()
        .and_then(RevisionStamp::from_json)
        .ok_or_else(bad)?;
    Ok(CreatedGrid { id, revision })
}

/// Executes one prepared grid request; call on a worker thread.
pub fn commit(cli: &Path, args: &[String], stats: &Stats) -> Result<CreatedGrid, ToolError> {
    let out = command::run(cli, args, command::DEADLINE, stats)?;
    parse_created(out.code, &out.stdout)
}

/// One label and its engine-computed world segment.
#[derive(Debug, Clone, PartialEq)]
pub struct GridLine {
    /// Axis label.
    pub label: String,
    /// World endpoints in millimetres.
    pub points: [[f64; 3]; 2],
}

/// Read-only display data from the same pinned revision as the meshes.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct GridScene {
    /// World segments, bounded by the grid model limits.
    pub lines: Vec<GridLine>,
}

impl GridScene {
    /// Reads a bounded `list-grids`-shaped reply. An absent legacy frame means no grids.
    pub fn from_reply(bytes: &[u8]) -> Result<Self, ToolError> {
        if bytes.is_empty() {
            return Ok(Self::default());
        }
        let json = decode(bytes)?;
        if let Some(error) = rejection(&json, Some(0)) {
            return Err(error);
        }
        let bad = || {
            ToolError::local(
                "UNREADABLE_REPLY",
                "The model returned invalid grid display data.",
            )
        };
        if text(&json, "outcome").as_deref() != Some("grids") {
            return Err(bad());
        }
        let grids = json.get("grids").and_then(Json::array).map_err(|_| bad())?;
        if grids.len() > MAX_GRIDS {
            return Err(bad());
        }
        let mut lines = Vec::new();
        for grid in grids {
            let initial_count = lines.len();
            let mut labels = Vec::new();
            for family in ["u", "v"] {
                let axes = grid
                    .get(family)
                    .and_then(|f| f.get("lines"))
                    .and_then(Json::array)
                    .map_err(|_| bad())?;
                if axes.len() > MAX_GRID_LINES_PER_FAMILY {
                    return Err(bad());
                }
                for axis in axes {
                    let label = text(axis, "label")
                        .filter(|s| s.len() <= 64 && s.chars().count() <= 16 && !s.is_empty())
                        .ok_or_else(bad)?;
                    labels.push(label.clone());
                    let point = |key| -> Result<[f64; 3], ToolError> {
                        let coords = axis.get(key).and_then(Json::array).map_err(|_| bad())?;
                        let [x, y, z] = coords else {
                            return Err(bad());
                        };
                        let point = [
                            x.float().map_err(|_| bad())?,
                            y.float().map_err(|_| bad())?,
                            z.float().map_err(|_| bad())?,
                        ];
                        if point.iter().any(|n| !n.is_finite() || n.abs() > 1e7) || point[2] != 0.0
                        {
                            return Err(bad());
                        }
                        Ok(point)
                    };
                    lines.push(GridLine {
                        label,
                        points: [point("start_mm")?, point("end_mm")?],
                    });
                }
            }
            if lines.len() == initial_count
                || beamcraft_model::grid::validate_grid_labels(&labels).is_err()
            {
                return Err(bad());
            }
        }
        Ok(Self { lines })
    }

    /// Fits the camera to the union of engineering bounds and grid guides, in scene coordinates.
    pub fn framing(&self, scene: &SceneSummary) -> ([f32; 3], f32) {
        if self.lines.is_empty() {
            return scene.framing();
        }
        let mut low = if scene.elements == 0 {
            [f32::INFINITY; 3]
        } else {
            scene.min
        };
        let mut high = if scene.elements == 0 {
            [f32::NEG_INFINITY; 3]
        } else {
            scene.max
        };
        for line in &self.lines {
            for point in line.points {
                for (((lo, hi), value), origin) in low
                    .iter_mut()
                    .zip(&mut high)
                    .zip(point)
                    .zip(scene.origin_mm)
                {
                    let value = (value - origin) as f32;
                    *lo = lo.min(value);
                    *hi = hi.max(value);
                }
            }
        }
        let centre = std::array::from_fn(|i| {
            (low.get(i).copied().unwrap_or(0.0) + high.get(i).copied().unwrap_or(0.0)) / 2.0
        });
        let extent = low
            .iter()
            .zip(high)
            .map(|(a, b)| (b - a) * (b - a))
            .sum::<f32>()
            .sqrt()
            .max(1.0);
        (centre, extent)
    }

    /// Paints construction guides and labels using engine endpoints, without authoring geometry.
    pub fn paint(&self, ui: &egui::Ui, rect: egui::Rect, view: &View, origin: [f64; 3]) {
        let painter = ui.painter_at(rect);
        let palette = theme::Palette::current(ui);
        for line in &self.lines {
            let project = |point: [f64; 3]| {
                let local = std::array::from_fn(|i| {
                    (point.get(i).copied().unwrap_or(0.0) - origin.get(i).copied().unwrap_or(0.0))
                        as f32
                });
                let [x, y, depth] = view.project(local);
                (x.is_finite()
                    && y.is_finite()
                    && x.abs() < 1e5
                    && y.abs() < 1e5
                    && (0.0..=1.0).contains(&depth))
                .then(|| {
                    egui::pos2(
                        rect.left() + (x + 1.0) * rect.width() / 2.0,
                        rect.top() + (1.0 - y) * rect.height() / 2.0,
                    )
                })
            };
            let [Some(a), Some(b)] = line.points.map(project) else {
                continue;
            };
            painter.line_segment(
                [a, b],
                egui::Stroke::new(1.0, palette.link.gamma_multiply(0.6)),
            );
            for end in [a, b] {
                if rect.shrink(14.0).contains(end) {
                    let galley = painter.layout_no_wrap(
                        line.label.clone(),
                        egui::FontId::proportional(12.0),
                        palette.text,
                    );
                    let size = (galley.size() + egui::vec2(10.0, 6.0)).max(egui::vec2(24.0, 24.0));
                    let label_rect = egui::Rect::from_center_size(end, size);
                    painter.rect_filled(label_rect, 12, palette.surface);
                    painter.rect_stroke(
                        label_rect,
                        12,
                        egui::Stroke::new(1.0, palette.link),
                        egui::StrokeKind::Inside,
                    );
                    painter.galley(end - galley.size() / 2.0, galley, palette.text);
                }
            }
        }
    }
}
