// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! The levels form: it prepares `create-level`, `update-level` and `delete-element` as the same
//! commands the CLI runs. One gesture is one command, hence one revision and one undo unit.

use crate::camera::View;
use crate::stats::Stats;
use crate::tools::{FormError, LevelChoice, ToolError, decode, push_actor, rejection};
use crate::{command, theme};
use beamcraft_model::id::ElementId;
use std::path::Path;

/// The longest name a section caption shows, in characters.
const MAX_CAPTION_CHARS: usize = 32;
/// The screen height of one caption, in points.
const CAPTION_HEIGHT: f32 = 20.0;
/// The distance of a level's mark from the pane's right edge, in points.
const MARK_INSET: f32 = 16.0;

/// The text of the level form.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LevelForm {
    /// The level's label.
    pub name: String,
    /// The elevation in millimetres, as typed.
    pub elevation: String,
}

fn form_error(field: &'static str, message: &str) -> FormError {
    FormError {
        field,
        message: message.to_owned(),
    }
}

fn store_text(store: &Path) -> Result<&str, FormError> {
    store
        .to_str()
        .ok_or_else(|| form_error("Model", "The model path must be UTF-8."))
}

impl LevelForm {
    /// The form showing an existing level.
    pub fn of(level: &LevelChoice) -> Self {
        Self {
            name: level.name.clone(),
            elevation: level.elevation_mm.to_string(),
        }
    }

    fn name(&self) -> Result<&str, FormError> {
        let name = self.name.trim();
        if name.is_empty() || name.len() > 256 {
            return Err(form_error(
                "Name",
                "Enter a level name of at most 256 UTF-8 bytes.",
            ));
        }
        Ok(name)
    }

    fn elevation(&self) -> Result<f64, FormError> {
        self.elevation
            .trim()
            .parse::<f64>()
            .ok()
            .filter(|value| value.is_finite())
            .ok_or_else(|| form_error("Elevation", "Enter a finite number using a decimal point."))
    }

    /// The arguments of `create-level`.
    pub fn prepare_create(
        &self,
        store: &Path,
        actor: Option<&str>,
    ) -> Result<Vec<String>, FormError> {
        let mut args = vec![
            "create-level".to_owned(),
            "--store".into(),
            store_text(store)?.into(),
            "--name".into(),
            self.name()?.into(),
            "--elevation".into(),
            self.elevation()?.to_string(),
        ];
        push_actor(&mut args, actor);
        Ok(args)
    }

    /// Whether the form differs from `level`, as the values it would send.
    pub fn differs(&self, level: &LevelChoice) -> bool {
        self.name.trim() != level.name
            || !self
                .elevation()
                .is_ok_and(|value| value == level.elevation_mm)
    }

    /// The arguments of `update-level`, naming only the values that changed.
    pub fn prepare_update(
        &self,
        level: &LevelChoice,
        store: &Path,
        actor: Option<&str>,
    ) -> Result<Vec<String>, FormError> {
        let name = self.name()?;
        let elevation = self.elevation()?;
        ElementId::parse(&level.id).map_err(|_| form_error("Level", "Select a level."))?;
        let mut args = vec![
            "update-level".to_owned(),
            "--store".into(),
            store_text(store)?.into(),
            "--id".into(),
            level.id.clone(),
        ];
        if name != level.name {
            args.extend(["--name".into(), name.into()]);
        }
        if elevation != level.elevation_mm {
            args.extend(["--elevation".into(), elevation.to_string()]);
        }
        push_actor(&mut args, actor);
        Ok(args)
    }
}

/// The arguments of `delete-element` for a level.
pub fn prepare_delete(
    level: &LevelChoice,
    store: &Path,
    actor: Option<&str>,
) -> Result<Vec<String>, FormError> {
    ElementId::parse(&level.id).map_err(|_| form_error("Level", "Select a level."))?;
    let mut args = vec![
        "delete-element".to_owned(),
        "--store".into(),
        store_text(store)?.into(),
        "--id".into(),
        level.id.clone(),
    ];
    push_actor(&mut args, actor);
    Ok(args)
}

/// Reads a command's reply; a level still in use gets a message that says what to do.
pub fn parse_reply(exit: Option<i32>, bytes: &[u8]) -> Result<(), ToolError> {
    let reply = decode(bytes)?;
    match rejection(&reply, exit) {
        Some(mut error) => {
            if error.code == "DEPENDENCY_CONFLICT" {
                "Elements still use this level. Assign them to another level first."
                    .clone_into(&mut error.message);
            }
            Err(error)
        }
        None => Ok(()),
    }
}

/// Runs one prepared level command.
pub fn commit(cli: &Path, args: &[String], stats: &Stats) -> Result<(), ToolError> {
    let out = command::run(cli, args, command::DEADLINE, stats)?;
    parse_reply(out.code, &out.stdout)
}

/// The levels from the highest down; equal elevations keep the order of their names.
pub fn sorted(levels: &[LevelChoice]) -> Vec<&LevelChoice> {
    let mut rows: Vec<&LevelChoice> = levels.iter().collect();
    rows.sort_by(|a, b| {
        b.elevation_mm
            .total_cmp(&a.elevation_mm)
            .then_with(|| a.name.cmp(&b.name))
    });
    rows
}

/// Where a level's line runs across `rect`, in screen points. Only a level seen edge-on is a line:
/// an orthographic view whose screen up is the model's Z. Any other view has none, and neither
/// has a level outside the pane.
pub fn section_row(view: &View, rect: egui::Rect, origin_z: f64, elevation_mm: f64) -> Option<f32> {
    let [.., right_z, _] = view.right;
    let [.., up_z, _] = view.up;
    let [.., back_z, _] = view.back;
    let [eye_x, eye_y, ..] = view.eye;
    let [ortho, ..] = view.mode;
    if ortho < 0.5 || up_z.abs() < 0.999 || right_z.abs() > 0.001 || back_z.abs() > 0.001 {
        return None;
    }
    // Right below the eye the point is at the middle of the screen, whatever the depth.
    let [_, y, _] = view.project([eye_x, eye_y, (elevation_mm - origin_z) as f32]);
    let row = rect.top() + (1.0 - y) * rect.height() / 2.0;
    (row.is_finite() && rect.y_range().contains(row)).then_some(row)
}

/// The caption of a level's line: its name, cut to fit, and its elevation.
fn caption(level: &LevelChoice) -> String {
    let name: String = level.name.chars().take(MAX_CAPTION_CHARS).collect();
    let cut = if name.len() < level.name.len() {
        "…"
    } else {
        ""
    };
    format!("{name}{cut}  {:+} mm", level.elevation_mm)
}

/// Paints the levels seen edge-on: a dashed line across the pane and, at its right end, a mark
/// with the name and elevation. Levels too close to caption keep their lines.
pub fn paint_sections(
    ui: &egui::Ui,
    rect: egui::Rect,
    view: &View,
    origin_z: f64,
    levels: &[LevelChoice],
    selected: Option<&str>,
) {
    let painter = ui.painter_at(rect);
    let palette = theme::Palette::current(ui);
    let mut last_caption = f32::NEG_INFINITY;
    for level in sorted(levels) {
        let Some(row) = section_row(view, rect, origin_z, level.elevation_mm) else {
            continue;
        };
        let active = selected == Some(level.id.as_str());
        let color = if active {
            theme::BRAND
        } else {
            palette.link.gamma_multiply(0.8)
        };
        let stroke = egui::Stroke::new(if active { 2.0 } else { 1.0 }, color);
        painter.extend(egui::Shape::dashed_line(
            &[egui::pos2(rect.left(), row), egui::pos2(rect.right(), row)],
            stroke,
            10.0,
            5.0,
        ));
        let tip = egui::pos2(rect.right() - MARK_INSET, row);
        painter.add(egui::Shape::convex_polygon(
            vec![
                tip,
                tip + egui::vec2(5.0, -8.0),
                tip + egui::vec2(-5.0, -8.0),
            ],
            color,
            egui::Stroke::NONE,
        ));
        if row - last_caption < CAPTION_HEIGHT {
            continue;
        }
        last_caption = row;
        let galley = painter.layout_no_wrap(
            caption(level),
            egui::FontId::proportional(12.0),
            palette.text,
        );
        let size = galley.size();
        let corner = egui::pos2(
            (rect.right() - MARK_INSET - 10.0 - size.x).max(rect.left() + 2.0),
            row - size.y - 3.0,
        );
        painter.rect_filled(
            egui::Rect::from_min_size(corner, size).expand(2.0),
            2,
            palette.surface,
        );
        painter.galley(corner, galley, palette.text);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camera::Camera;
    use beamcraft_engine::work_plane::WorkPlane;

    #[test]
    fn a_level_is_a_line_only_when_seen_edge_on() {
        let rect = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(100.0, 100.0));
        let aligned = |name| {
            let plane = WorkPlane::parallel(name, 0.0).unwrap();
            Camera::aligned_to(plane, [0.0; 3], 1000.0, 1000.0)
                .unwrap()
                .view(1.0)
                .unwrap()
        };
        for name in ["xz", "yz"] {
            let view = aligned(name);
            let row = section_row(&view, rect, 0.0, 250.0).unwrap();
            assert!((row - 25.0).abs() < 0.01, "{name}: {row}");
            // The scene is drawn relative to its origin, and so is the level.
            let shifted = section_row(&view, rect, 250.0, 250.0).unwrap();
            assert!((shifted - 50.0).abs() < 0.01, "{name}: {shifted}");
            assert_eq!(section_row(&view, rect, 0.0, 900.0), None, "off the pane");
        }
        assert_eq!(section_row(&aligned("xy"), rect, 0.0, 0.0), None, "a plan");
        let orbit = Camera::new([0.0; 3], 1000.0, 1.0)
            .unwrap()
            .view(1.0)
            .unwrap();
        assert_eq!(section_row(&orbit, rect, 0.0, 0.0), None, "a 3D view");
    }

    #[test]
    fn a_long_name_is_cut_on_a_character_boundary() {
        let mut long = LevelChoice {
            id: ID.into(),
            name: "Я".repeat(100),
            elevation_mm: -250.5,
            elements: 0,
        };
        let text = caption(&long);
        assert!(text.starts_with(&"Я".repeat(MAX_CAPTION_CHARS)));
        assert!(text.ends_with("…  -250.5 mm"));
        long.name = "L1".into();
        long.elevation_mm = 3000.0;
        assert_eq!(caption(&long), "L1  +3000 mm");
    }

    const ID: &str = "019a7bf0-0000-7000-8000-000000300001";

    fn level() -> LevelChoice {
        LevelChoice {
            id: ID.into(),
            name: "L1".into(),
            elevation_mm: 3000.0,
            elements: 2,
        }
    }

    fn form(name: &str, elevation: &str) -> LevelForm {
        LevelForm {
            name: name.into(),
            elevation: elevation.into(),
        }
    }

    #[test]
    fn create_requires_a_name_and_a_finite_elevation() {
        let store = Path::new("model");
        let args = form(" Roof ", "-250.5")
            .prepare_create(store, None)
            .unwrap();
        assert_eq!(
            args,
            [
                "create-level",
                "--store",
                "model",
                "--name",
                "Roof",
                "--elevation",
                "-250.5"
            ]
        );
        assert_eq!(
            form("", "0").prepare_create(store, None).unwrap_err().field,
            "Name"
        );
        for bad in ["", "abc", "NaN", "inf", "1,5"] {
            assert_eq!(
                form("L", bad)
                    .prepare_create(store, None)
                    .unwrap_err()
                    .field,
                "Elevation",
                "{bad}"
            );
        }
    }

    #[test]
    fn update_names_only_what_changed() {
        let store = Path::new("model");
        let args = form("L1", "3300")
            .prepare_update(&level(), store, None)
            .unwrap();
        assert_eq!(
            args,
            [
                "update-level",
                "--store",
                "model",
                "--id",
                ID,
                "--elevation",
                "3300"
            ]
        );
        assert!(!form("L1", "3000.0").differs(&level()));
        assert!(form("L2", "3000").differs(&level()));
        assert!(form("L1", "x").differs(&level()));
    }

    #[test]
    fn a_used_level_is_explained_and_the_rows_run_from_the_top() {
        let reply = br#"{"outcome":"rejected","code":"DEPENDENCY_CONFLICT","message":"m"}"#;
        let error = parse_reply(Some(1), reply).unwrap_err();
        assert_eq!(error.code, "DEPENDENCY_CONFLICT");
        assert!(error.message.contains("another level"));
        let mut low = level();
        low.elevation_mm = -1.0;
        let levels = [low, level()];
        let rows = sorted(&levels);
        assert_eq!(
            rows.iter().map(|l| l.elevation_mm).collect::<Vec<_>>(),
            [3000.0, -1.0]
        );
    }
}
