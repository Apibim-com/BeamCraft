// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! Headless geometry mathematics for BeamCraft, in millimetres and radians.
//!
//! Coordinates are right-handed, Z up. Checked operations follow F06 0.1.0's
//! scalar binary64 order, reject non-finite results and canonicalize signed zero.
//! No quaternion or general element orientation is provided (owner decision).

pub mod error;
pub mod frame;
pub mod plane;
pub mod tolerance;
pub mod transform;
pub mod vec;

pub use error::MathError;
pub use frame::{Frame3, FrameCertificate, MemberFrame, MemberFrameRule};
pub use plane::Plane3;
pub use transform::{RigidTransform, Translation3};
pub use vec::{Point3, Vec3, canonical_f64_bytes};
