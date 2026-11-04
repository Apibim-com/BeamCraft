// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! `--self-test FILE`: a scripted run of the view workflow on the window's own state, without a
//! window or a store, written as a JSON report. Each step drives the same code a person's
//! actions run and measures the result from that state:
//!
//! * `move`: views open as tabs, a tab dragged onto a group's right edge splits the work area,
//!   and dragging the divider resizes the groups.
//! * `act`: a filter hides one kind of element and colours another in a view, the first matching
//!   override wins, a preset fills another view, and the view file keeps all of it.
//! * `hud`: the project browser lists the views by type with their folders inside.

use crate::colour_sets::Overlay;
use crate::dock::{Dock, Drop, Side};
use crate::view_overlay::{FilterMatches, overlay_of, rule_of};
use crate::views::{ViewKind, Views, new_folder_path};
use beamcraft_engine::ElementId;
use beamcraft_engine::filter::Operator;
use beamcraft_engine::views::{
    FilterCondition, FilterField, FilterOverride, ModelView, ViewBook, ViewFilter,
};
use beamcraft_engine::work_plane::WorkPlane;
use beamcraft_json::{Json, Limits};
use beamcraft_model::id::Uuid7;
use std::collections::BTreeSet;
use std::sync::Arc;

/// One step's outcome.
struct Step {
    step: &'static str,
    passed: bool,
    detail: String,
}

fn id(n: u64) -> Result<Uuid7, String> {
    Uuid7::parse(&format!("019a7bf0-0000-7000-8000-{n:012}")).map_err(|_| "bad id".to_owned())
}

fn element(n: u64) -> Result<ElementId, String> {
    ElementId::parse(&format!("019a7bf0-0000-7000-9000-{n:012}"))
        .map_err(|_| "bad element id".to_owned())
}

/// Tabs, a split by dragging a tab onto an edge, and a divider drag.
fn move_step() -> Result<String, String> {
    let area = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1200.0, 800.0));
    let mut dock = Dock::default();
    dock.sync(&[1, 2, 3], Some(3));
    if dock.groups().len() != 1 {
        return Err("three views did not open as tabs of one group".into());
    }
    let pos = egui::pos2(area.max.x - 20.0, area.center().y);
    let (drop, _) = dock.drop_at(area, pos).ok_or("nothing under the drop")?;
    if !matches!(drop, Drop::Beside(_, Side::Right)) {
        return Err(format!("a drop on the right edge gave {drop:?}"));
    }
    dock.insert(3, drop);
    let layout = dock.layout(area);
    let right = layout
        .iter()
        .find(|(g, _)| dock.group_of(3) == Some(*g))
        .map(|(_, r)| *r)
        .ok_or("the dragged tab has no group")?;
    let before = right.min.x;
    let divider = dock.dividers(area).into_iter().next().ok_or("no divider")?;
    dock.drag_divider(
        &divider.path,
        divider.room,
        divider.across,
        egui::pos2(800.0, 0.0),
    );
    let after = dock
        .layout(area)
        .iter()
        .find(|(g, _)| dock.group_of(3) == Some(*g))
        .map_or(0.0, |(_, r)| r.min.x);
    if layout.len() != 2 || (after - 800.0).abs() > 0.5 {
        return Err(format!("{} groups, divider at {after}", layout.len()));
    }
    Ok(format!(
        "3 tabs in 1 group; the dragged tab split the area (its group starts at x {before:.0}); \
         the divider moved it to x {after:.0}"
    ))
}

/// Filters hide and colour by the first matching override; a preset fills a view; the view
/// file round-trips.
fn act_step() -> Result<String, String> {
    let mut book = ViewBook::new();
    let filter = |name: &str, kind: &str| {
        ViewFilter::new(
            name,
            vec![FilterCondition::new(FilterField::Type, Operator::Eq, kind)],
        )
    };
    let err = |e: beamcraft_model::command::CommandError| e.to_string();
    book.save_filter(filter("Columns", "column"), None)
        .map_err(err)?;
    book.save_filter(filter("Beams", "beam"), None)
        .map_err(err)?;
    let mut plan = ModelView::new(id(1)?, "Level 1", WorkPlane::default(), 12_000.0);
    plan.overrides = vec![
        FilterOverride::parse_spec("Columns|hide").map_err(err)?,
        FilterOverride::parse_spec("Beams|show|0.25,0.55,0.85|0").map_err(err)?,
    ];
    book.insert(plan.clone()).map_err(err)?;
    // The model: elements 1 and 2 are columns, 3 and 4 beams.
    let columns: BTreeSet<ElementId> = [element(1)?, element(2)?].into_iter().collect();
    let beams: BTreeSet<ElementId> = [element(3)?, element(4)?].into_iter().collect();
    let mut matches = FilterMatches::default();
    matches.follow("self-test");
    for (name, ids) in [("Columns", &columns), ("Beams", &beams)] {
        let filter = book.filter(name).ok_or("a filter went missing")?;
        matches.insert(rule_of(filter).0, Ok(Arc::new(ids.clone())));
    }
    let (overlay, wanted, refused): (Overlay, _, _) = overlay_of(&plan, &book, &matches, None);
    let hidden = columns.iter().filter(|e| overlay.hides(**e)).count();
    let coloured = beams
        .iter()
        .filter(|e| overlay.tint_of(**e) == [0.25, 0.55, 0.85, 1.0])
        .count();
    if !wanted.is_empty() || !refused.is_empty() || hidden != 2 || coloured != 2 {
        return Err(format!(
            "{hidden} of 2 columns hidden, {coloured} of 2 beams coloured"
        ));
    }
    book.save_preset(plan.to_preset("Steel plan"))
        .map_err(err)?;
    let mut other = ModelView::new(id(2)?, "Level 2", WorkPlane::default(), 5_000.0);
    other.apply_preset(book.preset("Steel plan").ok_or("no preset")?);
    book.insert(other.clone()).map_err(err)?;
    let json = beamcraft_host::views::book_to_json(&book).map_err(err)?;
    let bytes = json
        .encode(&Limits::DEFAULT)
        .map_err(|_| "the view file does not encode")?;
    let read = beamcraft_json::decode(&bytes, &Limits::DEFAULT)
        .map_err(|_| "the view file does not decode")?;
    let back = beamcraft_host::views::book_from_json(&read).map_err(err)?;
    if back != book || other.overrides.len() != 2 {
        return Err("the view file or the preset lost something".into());
    }
    Ok(format!(
        "the plan hides {hidden} columns and colours {coloured} beams; the preset filled \
         \"Level 2\" with {} filters; the view file kept {} views, {} filters and {} preset",
        other.overrides.len(),
        back.len(),
        back.filters().len(),
        back.presets().len()
    ))
}

/// The browser tree lists the kinds and the folders inside them.
fn hud_step() -> Result<String, String> {
    let mut views = Views::default();
    let err = |e: beamcraft_model::command::CommandError| e.to_string();
    let mut plan = ModelView::new(id(1)?, "Level 1", WorkPlane::default(), 12_000.0);
    plan.folder = new_folder_path(ViewKind::Plan, "", "Levels");
    let mut section = ModelView::new(
        id(2)?,
        "Section A",
        WorkPlane::parallel("yz", 0.0).map_err(err)?,
        9_000.0,
    );
    section.depth = Some([300.0, 300.0]);
    let mut home = ModelView::new(id(3)?, "3D View", WorkPlane::default(), 20_000.0);
    home.is_default = true;
    for view in [plan, section, home] {
        views.book.insert(view).map_err(err)?;
    }
    let tree = views.browser_tree();
    let listed: Vec<String> = tree
        .iter()
        .filter(|(_, node)| !node.all_views().is_empty())
        .map(|(kind, node)| {
            let folders: Vec<&str> = node.folders.iter().map(|f| f.name.as_str()).collect();
            if folders.is_empty() {
                kind.label().to_owned()
            } else {
                format!("{} ({})", kind.label(), folders.join(", "))
            }
        })
        .collect();
    let plans = tree
        .iter()
        .find(|(k, _)| *k == ViewKind::Plan)
        .map(|(_, n)| {
            n.folders
                .iter()
                .any(|f| f.name == "Levels" && f.views.len() == 1)
        });
    if listed.len() != 3 || plans != Some(true) {
        return Err(format!("the browser lists {listed:?}"));
    }
    Ok(format!("the browser lists {}", listed.join(", ")))
}

/// Runs every step and returns the report.
pub fn run() -> Json {
    let steps = [
        ("move", move_step()),
        ("act", act_step()),
        ("hud", hud_step()),
    ]
    .into_iter()
    .map(|(step, result)| match result {
        Ok(detail) => Step {
            step,
            passed: true,
            detail,
        },
        Err(detail) => Step {
            step,
            passed: false,
            detail,
        },
    })
    .map(|s| {
        Json::object([
            ("step", Json::text(s.step)),
            ("passed", Json::Bool(s.passed)),
            ("detail", Json::text(s.detail)),
        ])
    })
    .collect();
    Json::object([("version", Json::uint(1)), ("steps", Json::Array(steps))])
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    #[test]
    fn every_step_of_the_self_test_passes() {
        let report = super::run();
        for step in report.get("steps").unwrap().array().unwrap() {
            assert_eq!(
                step.get("passed").unwrap(),
                &beamcraft_json::Json::Bool(true),
                "{step:?}"
            );
        }
    }
}
