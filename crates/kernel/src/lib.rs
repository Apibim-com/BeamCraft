// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! Bounded, headless analytic geometry. G02 builds origin rectangular plates;
//! the G05 [`profile`] module extrudes planar profiles with cavities.
//!
//! The G02 builder is dimension-based, not the complete F06 contour/placement
//! request adapter. No rotation, I/O or dependency keys are exposed.

pub mod build;
pub mod diagnostic;
pub mod feature_id;
pub mod input;
pub mod mesh;
pub mod profile;
pub mod result;

pub use build::{rectangular_plate, rectangular_plate_with_cancel};
pub use input::RectangularPlate;
pub use result::PlateResult;
