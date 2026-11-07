// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! The shared widgets of the side panels: one disclosure chevron, the collapsible group bar of
//! a properties grid, the lighter fold of a sub-block, the two-column property row and the
//! spacing tokens they share. Every panel draws its groups and rows through these, so the
//! project browser, the properties of a view, a part, a bolt or a grid all look alike.

use crate::theme::Palette;
use egui::collapsing_header::CollapsingState;
use egui::{Align2, FontId, Rect, Response, Sense, Stroke, Ui, pos2, vec2};

/// The smallest gap: between an icon and its text, under a group bar.
pub const SPACE_XS: f32 = 4.0;
/// The gap between the rows of a block and the inner margin of a panel.
pub const SPACE_S: f32 = 8.0;
/// The gap between blocks.
pub const SPACE_M: f32 = 12.0;
/// The gap between sections of a panel.
pub const SPACE_L: f32 = 16.0;
/// The height of one row of a properties grid, a group bar or a tree row.
pub const ROW_HEIGHT: f32 = 24.0;
/// The width of the label column of a properties grid.
pub const LABEL_WIDTH: f32 = 104.0;
/// The square a disclosure chevron is drawn in.
pub const CHEVRON: f32 = 16.0;
/// Slack left at the end of a row that fills the panel: a row sized to the exact available
/// width can come out a pixel wider, and a resizable panel would follow it wider every frame.
pub const FIT: f32 = 6.0;

/// Paints the disclosure chevron centred on `center`: pointing right when closed (`openness`
/// 0), turning down as the block opens (1). Two strokes of one weight, the way the icon set is
/// drawn, rather than a filled triangle.
pub fn chevron(painter: &egui::Painter, center: egui::Pos2, openness: f32, color: egui::Color32) {
    let angle = openness.clamp(0.0, 1.0) * std::f32::consts::FRAC_PI_2;
    let (sin, cos) = angle.sin_cos();
    // The right-pointing chevron, then rotated a quarter turn clockwise as it opens.
    let turn = |x: f32, y: f32| center + vec2(x * cos - y * sin, x * sin + y * cos);
    painter.line(
        vec![turn(-1.75, -3.5), turn(1.75, 0.0), turn(-1.75, 3.5)],
        Stroke::new(1.5, color),
    );
}

/// The chevron as an egui toggle-button painter, for a [`CollapsingState`] header.
pub fn paint_chevron(ui: &mut Ui, openness: f32, response: &Response) {
    let palette = Palette::current(ui);
    let color = if response.hovered() {
        palette.text
    } else {
        palette.muted
    };
    chevron(ui.painter(), response.rect.center(), openness, color);
}

/// Draws one clickable header row of `width` with a chevron at its left and `title` beside it,
/// toggling `state` when clicked. `fill` paints the row first (the group bar's band).
fn header_row(
    ui: &mut Ui,
    state: &mut CollapsingState,
    title: &str,
    strong: bool,
    fill: Option<egui::Color32>,
) -> Response {
    let palette = Palette::current(ui);
    let width = (ui.available_width() - FIT).max(40.0);
    let (rect, response) = ui.allocate_exact_size(vec2(width, ROW_HEIGHT), Sense::click());
    if response.clicked() {
        state.toggle(ui);
    }
    let openness = state.openness(ui.ctx());
    let painter = ui.painter();
    let hovered = response.hovered() || response.has_focus();
    match (fill, hovered) {
        (Some(fill), false) => {
            painter.rect_filled(rect, 3, fill);
        }
        (_, true) => {
            painter.rect_filled(rect, 3, palette.select);
        }
        (None, false) => {}
    }
    chevron(
        painter,
        pos2(rect.left() + SPACE_S + CHEVRON / 2.0 - 4.0, rect.center().y),
        openness,
        if hovered { palette.text } else { palette.muted },
    );
    let font = FontId::proportional(12.0);
    let galley = painter.layout_no_wrap(title.to_owned(), font, palette.text);
    let text_at = pos2(
        rect.left() + SPACE_S + CHEVRON,
        rect.center().y - galley.size().y / 2.0,
    );
    let clip = Rect::from_min_max(pos2(text_at.x, rect.top()), rect.max);
    if strong {
        // A faux bold: the same text a hair to the right, so the bar title reads as a heading
        // without loading a second font.
        painter
            .with_clip_rect(clip)
            .galley(text_at + vec2(0.4, 0.0), galley.clone(), palette.text);
    }
    painter
        .with_clip_rect(clip)
        .galley(text_at, galley, palette.text);
    let open = state.is_open();
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::Role::DisclosureTriangle, true, open, title)
    });
    response
}

/// A collapsible group of a properties grid: a filled bar with a chevron and the group's
/// title, and its rows under it. `title` also keys the open state, so it must be unique in its
/// panel; `salt` tells two panels' groups of one title apart.
pub fn section<R>(
    ui: &mut Ui,
    salt: &str,
    title: &str,
    default_open: bool,
    add: impl FnOnce(&mut Ui) -> R,
) -> Option<R> {
    let palette = Palette::current(ui);
    let id = ui.make_persistent_id(("kit-section", salt, title));
    let mut state = CollapsingState::load_with_default_open(ui.ctx(), id, default_open);
    header_row(ui, &mut state, title, true, Some(palette.head));
    let out = state.show_body_unindented(ui, |ui| {
        ui.add_space(SPACE_XS);
        let out = add(ui);
        ui.add_space(SPACE_S);
        out
    });
    out.map(|r| r.inner)
}

/// A lighter disclosure inside a block (precise placement, line limits, the work plane): no
/// band, the chevron and the title only, and its content indented under the title.
pub fn fold<R>(
    ui: &mut Ui,
    salt: &str,
    title: &str,
    default_open: bool,
    add: impl FnOnce(&mut Ui) -> R,
) -> Option<R> {
    let id = ui.make_persistent_id(("kit-fold", salt, title));
    let mut state = CollapsingState::load_with_default_open(ui.ctx(), id, default_open);
    header_row(ui, &mut state, title, false, None);
    let out = state.show_body_unindented(ui, |ui| {
        ui.horizontal(|ui| {
            ui.add_space(SPACE_S + CHEVRON - ui.spacing().item_spacing.x);
            ui.vertical(add).inner
        })
        .inner
    });
    out.map(|r| r.inner)
}

/// A tree node: a chevron toggle and a header drawn beside it on one row
/// ([`TreeNode::header`]), then its body indented under it while open ([`TreeNode::body`]).
/// The two halves are separate calls, so the header and the body may borrow the same state.
pub struct TreeNode {
    state: CollapsingState,
}

impl TreeNode {
    /// Draws the header row of the node keyed by `id`: the chevron, then `header`. Returns
    /// the node, the row's response and what the header returned.
    pub fn header<H>(
        ui: &mut Ui,
        id: egui::Id,
        default_open: bool,
        header: impl FnOnce(&mut Ui) -> H,
    ) -> (Self, Response, H) {
        let mut state = CollapsingState::load_with_default_open(ui.ctx(), id, default_open);
        let row = ui.horizontal(|ui| {
            ui.set_min_height(ROW_HEIGHT);
            ui.spacing_mut().item_spacing.x = SPACE_XS;
            let (_, toggle) = ui.allocate_exact_size(vec2(CHEVRON, ROW_HEIGHT), Sense::click());
            if toggle.clicked() {
                state.toggle(ui);
            }
            let openness = state.openness(ui.ctx());
            paint_chevron(ui, openness, &toggle);
            header(ui)
        });
        (Self { state }, row.response, row.inner)
    }

    /// Draws the body under the header while the node is open, indented to the header's icon
    /// with a guide line down from the chevron, and stores the node's state.
    pub fn body<B>(mut self, ui: &mut Ui, body: impl FnOnce(&mut Ui) -> B) -> Option<B> {
        let opened = self.state.show_body_unindented(ui, |ui| {
            ui.horizontal(|ui| {
                let palette = Palette::current(ui);
                let guide_x = ui.cursor().left() + CHEVRON / 2.0;
                ui.add_space(CHEVRON);
                let inner = ui.vertical(body);
                let r = inner.response.rect;
                ui.painter().vline(
                    guide_x,
                    r.top()..=r.bottom(),
                    Stroke::new(1.0, palette.rule.gamma_multiply(0.7)),
                );
                inner.inner
            })
            .inner
        });
        opened.map(|r| r.inner)
    }
}

/// Toggles the tree node keyed by `id`, as a click on its name does.
pub fn toggle_node(ui: &Ui, id: egui::Id, default_open: bool) {
    let mut state = CollapsingState::load_with_default_open(ui.ctx(), id, default_open);
    state.toggle(ui);
    state.store(ui.ctx());
}

/// A search field with its magnifier inside the frame, filling the row up to `reserve`
/// points kept free at its end. Returns the text edit's response.
pub fn search_field(ui: &mut Ui, text: &mut String, hint: &str, reserve: f32) -> Response {
    let palette = Palette::current(ui);
    let width = (ui.available_width() - reserve - FIT).max(60.0);
    let (rect, _) = ui.allocate_exact_size(vec2(width, ROW_HEIGHT + 2.0), Sense::hover());
    let edit = ui.put(
        rect.shrink2(vec2(0.0, 1.0)),
        egui::TextEdit::singleline(text)
            .hint_text(hint)
            .char_limit(128)
            .margin(egui::Margin {
                left: 26,
                right: 6,
                top: 4,
                bottom: 4,
            }),
    );
    crate::theme::paint_icon(
        ui.painter(),
        Rect::from_center_size(pos2(rect.left() + 13.0, rect.center().y), vec2(14.0, 14.0)),
        crate::theme::Icon::Search,
        palette.muted,
    );
    edit
}

/// One row of a properties grid: the label in the fixed label column, muted, and the value
/// widget beside it taking the rest of the row, with a thin rule under the row.
pub fn prop_row<R>(ui: &mut Ui, label: &str, add: impl FnOnce(&mut Ui) -> R) -> R {
    let palette = Palette::current(ui);
    let row = ui.horizontal(|ui| {
        ui.set_min_height(ROW_HEIGHT);
        ui.add_space(SPACE_S);
        ui.add_sized(
            [LABEL_WIDTH - SPACE_S, ROW_HEIGHT - 4.0],
            egui::Label::new(egui::RichText::new(label).size(12.0).color(palette.muted)).truncate(),
        )
        .on_hover_text(label);
        add(ui)
    });
    let rect = row.response.rect;
    ui.painter().hline(
        rect.x_range(),
        rect.max.y + 1.0,
        Stroke::new(1.0, palette.rule.gamma_multiply(0.6)),
    );
    row.inner
}

/// A read-only property row: the value right-aligned in the value column, with its unit muted
/// after it, so numbers line up down the grid.
pub fn prop_value(ui: &mut Ui, label: &str, value: &str, unit: Option<&str>) {
    prop_row(ui, label, |ui| {
        let palette = Palette::current(ui);
        let width = (ui.available_width() - FIT).max(40.0);
        let (rect, response) =
            ui.allocate_exact_size(vec2(width, ROW_HEIGHT - 4.0), Sense::hover());
        let painter = ui.painter_at(rect);
        let unit_width = unit.map_or(0.0, |unit| {
            let galley =
                painter.layout_no_wrap(unit.to_owned(), FontId::proportional(11.0), palette.muted);
            let w = galley.size().x;
            painter.galley(
                pos2(
                    rect.right() - w - SPACE_XS,
                    rect.center().y - galley.size().y / 2.0,
                ),
                galley,
                palette.muted,
            );
            w + SPACE_XS * 2.0
        });
        painter.text(
            pos2(rect.right() - unit_width - SPACE_XS, rect.center().y),
            Align2::RIGHT_CENTER,
            value,
            FontId::proportional(12.0),
            palette.text,
        );
        response.on_hover_text(match unit {
            Some(unit) => format!("{value} {unit}"),
            None => value.to_owned(),
        });
    });
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    fn frame(ctx: &egui::Context, events: Vec<egui::Event>, run: impl FnMut(&mut Ui)) {
        let input = egui::RawInput {
            events,
            screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, vec2(320.0, 400.0))),
            ..Default::default()
        };
        let mut run = run;
        ctx.run_ui(input, |ui| run(ui))
            .drop_without_applying_deltas();
    }

    #[test]
    fn a_section_opens_and_closes_from_its_whole_bar() {
        let ctx = egui::Context::default();
        let shown = std::cell::Cell::new(false);
        let body = |ui: &mut Ui| {
            shown.set(false);
            let _ = section(ui, "t", "Geometry", false, |ui| {
                shown.set(true);
                ui.label("row");
            });
        };
        frame(&ctx, Vec::new(), body);
        assert!(!shown.get(), "closed by default");
        // A click far right on the bar, not on the chevron, opens it.
        let at = pos2(250.0, ROW_HEIGHT / 2.0);
        let click = |pressed| egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        frame(&ctx, vec![egui::Event::PointerMoved(at), click(true)], body);
        frame(&ctx, vec![click(false)], body);
        ctx.memory_mut(|m| m.everything_is_visible());
        for _ in 0..30 {
            frame(&ctx, Vec::new(), body);
        }
        assert!(shown.get(), "the click opened it");
    }

    #[test]
    fn the_chevron_turns_a_quarter_as_it_opens() {
        let ctx = egui::Context::default();
        let mut closed = Vec::new();
        let mut open = Vec::new();
        ctx.run_ui(egui::RawInput::default(), |ui| {
            let painter = ui.painter();
            chevron(painter, egui::Pos2::ZERO, 0.0, egui::Color32::WHITE);
            chevron(painter, egui::Pos2::ZERO, 1.0, egui::Color32::WHITE);
        })
        .drop_without_applying_deltas();
        let output = ctx.run_ui(egui::RawInput::default(), |ui| {
            let painter = ui.painter();
            chevron(painter, egui::Pos2::ZERO, 0.0, egui::Color32::WHITE);
            chevron(painter, egui::Pos2::ZERO, 1.0, egui::Color32::WHITE);
        });
        let shapes = output.shapes.clone();
        output.drop_without_applying_deltas();
        for (n, shape) in shapes.iter().enumerate() {
            if let egui::Shape::Path(path) = &shape.shape {
                if n == 0 {
                    closed.clone_from(&path.points);
                } else {
                    open.clone_from(&path.points);
                }
            }
        }
        // Closed: the tip points right (+x); open: it points down (+y).
        assert!(closed[1].x > closed[0].x && (closed[1].y).abs() < 1e-3);
        assert!(open[1].y > open[0].y && (open[1].x).abs() < 1e-3);
    }
}
