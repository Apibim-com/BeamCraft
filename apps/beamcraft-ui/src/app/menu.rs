// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! The original BeamCraft workspace drawer. Its command entries are presentation-only
//! placeholders: disabled widgets dispatch no commands and change no model data.

use crate::theme::{self, Palette};
use egui::{Align2, Context, Id, RichText, Stroke, Ui, vec2};

const GROUPS: &[(&str, &[&str])] = &[
    (
        "MODEL FILES",
        &["New model", "Open model", "Duplicate model", "Model folder"],
    ),
    (
        "EXCHANGE & OUTPUT",
        &["Import reference", "Export model", "Print drawings"],
    ),
    (
        "PROJECT",
        &["Project information", "Libraries", "Preferences"],
    ),
    (
        "SUPPORT",
        &["Activity log", "Check model", "Help", "About BeamCraft"],
    ),
    ("SESSION", &["Close model", "Exit BeamCraft"]),
];

fn entry(ui: &mut Ui, label: &str) -> egui::Response {
    ui.add_enabled(
        false,
        egui::Button::new(label)
            .frame(false)
            .min_size(vec2(ui.available_width(), 36.0)),
    )
    .on_disabled_hover_text("Not available yet")
}

/// Shows a left-aligned overlay without changing the viewport's size or camera.
pub(super) fn show(ctx: &Context, open: &mut bool) {
    if !*open {
        return;
    }
    let bounds = ctx.content_rect();
    let palette = Palette::of(ctx.theme());
    let margin = 16.0;
    let width = (bounds.width() - margin * 2.0).clamp(0.0, 304.0);
    let height = (bounds.height() - margin * 2.0).max(0.0);
    let area = egui::Modal::default_area(Id::unique("workspace-menu"))
        .anchor(Align2::LEFT_TOP, vec2(margin, margin));
    let response = egui::Modal::new(Id::unique("workspace-menu"))
        .area(area)
        .frame(
            egui::Frame::new()
                .fill(palette.surface)
                .stroke(Stroke::new(2.0, palette.ink))
                .corner_radius(12)
                .inner_margin(16),
        )
        .show(ctx, |ui| {
            ui.set_width((width - 32.0).max(0.0));
            ui.set_height((height - 32.0).max(0.0));
            ui.horizontal(|ui| {
                ui.label(theme::bold("Workspace", 22.0));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add(egui::Button::new("×").min_size(vec2(44.0, 44.0)))
                        .on_hover_text("Close menu (Esc)")
                        .clicked()
                    {
                        ui.close();
                    }
                });
            });
            ui.label(RichText::new("Model, project and application").color(palette.muted));
            ui.add_space(8.0);
            ui.separator();
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 2.0;
                    for (heading, entries) in GROUPS {
                        ui.add_space(12.0);
                        ui.label(theme::bold(*heading, 11.0).color(palette.muted));
                        for label in *entries {
                            entry(ui, label);
                        }
                    }
                });
        });
    if response.should_close() {
        *open = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_dismisses_the_drawer_and_entries_are_disabled() {
        let ctx = Context::default();
        crate::app::inter(&ctx);
        theme::install(&ctx);
        let mut open = true;
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                vec2(900.0, 700.0),
            )),
            ..Default::default()
        };
        let _ = ctx.run_ui(input.clone(), |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                for (_, entries) in GROUPS {
                    for label in *entries {
                        let response = entry(ui, label);
                        assert!(!response.enabled());
                        assert!(!response.clicked());
                    }
                }
            });
            show(ui.ctx(), &mut open);
        });
        assert!(open);
        let mut input = input;
        input.events.push(egui::Event::Key {
            key: egui::Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        });
        let _ = ctx.run_ui(input, |ui| show(ui.ctx(), &mut open));
        assert!(!open);
    }
}
