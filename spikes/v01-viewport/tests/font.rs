// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim
//! Font validation must reject malformed files instead of letting epaint panic.
use beamcraft_v01_viewport_spike::check_font;

#[test]
fn malformed_fonts_are_rejected() {
    assert!(check_font(b"").is_err());
    assert!(check_font(b"not a font at all").is_err());
}

#[test]
fn truncated_inter_is_rejected_and_full_inter_accepted() {
    let path = std::path::Path::new("../../assets/fonts/inter/Inter-Regular.ttf");
    let Ok(bytes) = std::fs::read(path) else {
        return;
    };
    assert!(check_font(&bytes).is_ok());
    assert!(check_font(bytes.get(..bytes.len() / 2).unwrap_or(&[])).is_err());
}
