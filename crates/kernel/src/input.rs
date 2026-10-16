// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! Dimension input for the G02 rectangular primitive (millimetres).

/// Raw dimensions of an origin plate spanning +X, +Y and +Z.
///
/// The builder validates all three dimensions on every call. This convenience
/// input does not admit an authored contour or replace F04 contour validation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RectangularPlate {
    /// Width a, along +X, in millimetres.
    pub width_mm: f64,
    /// Length b, along +Y, in millimetres.
    pub length_mm: f64,
    /// Thickness c, bottom at Z=0, in millimetres.
    pub thickness_mm: f64,
}
