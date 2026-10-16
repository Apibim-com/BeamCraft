// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! Bounded, headless analytic geometry. G02 builds origin rectangular plates.
//!
//! This is a dimension-based builder, not the complete F06 contour/placement
//! request adapter. No rotation, general contour, I/O or dependency keys are exposed.

pub mod build;
pub mod diagnostic;
pub mod feature_id;
pub mod input;
pub mod mesh;
pub mod result;

// G06: shell certification, exact predicates, measurements and a bounded box index.
mod bigint;
pub mod enclosure;
mod exact;
mod intersect;
pub mod measure;
pub mod spatial;
pub mod topology;
pub mod trimesh;
pub mod verdict;

pub use build::{rectangular_plate, rectangular_plate_with_cancel};
pub use input::RectangularPlate;
pub use result::PlateResult;
