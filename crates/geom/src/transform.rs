// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! Body-to-element rigid maps and translation-only element placement.

use crate::tolerance::check_point_cap;
use crate::{Frame3, MathError, Point3, Vec3};

/// Translation-only element placement, as required by the owner's rotation scope.
///
/// Plates stay as drawn; member roll is part of the member body frame. No
/// quaternion, matrix, shear, scale or general placement orientation is accepted.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Translation3 {
    offset: Vec3,
}

impl Translation3 {
    /// The identity placement.
    pub const IDENTITY: Self = Self { offset: Vec3::ZERO };
    /// Validates finite translation components and the inclusive 1e9 mm cap.
    pub fn new(x: f64, y: f64, z: f64) -> Result<Self, MathError> {
        let point = Point3::try_new(x, y, z)?;
        check_point_cap(point)?;
        Ok(Self {
            offset: Vec3::new(point.x, point.y, point.z)?,
        })
    }
    /// Returns the translation vector in mm.
    pub const fn offset(self) -> Vec3 {
        self.offset
    }
    /// Maps an element-local point to world coordinates, checking finite output.
    pub fn apply_point(self, point: Point3) -> Result<Point3, MathError> {
        point.translated(self.offset)
    }
}

/// A certified body-to-element map, rotate by the body basis, then translate.
///
/// This map expresses derived body axes (including a member's rolled frame).
/// It is not a general authored orientation mechanism. F06's approximate frame
/// certificate is its numeric gate; no exact-distance or accuracy bound is claimed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RigidTransform {
    frame: Frame3,
    origin: Point3,
}

impl RigidTransform {
    /// Exact identity body map.
    pub const IDENTITY: Self = Self {
        frame: Frame3::IDENTITY,
        origin: Point3::ORIGIN,
    };
    /// Uses an already certified body frame and validates its finite origin.
    ///
    /// Derived body origins may exceed authored-coordinate caps; only finiteness
    /// is checked here. Authored axis/contour caps are checked before derivation.
    pub fn from_body_frame(frame: Frame3, origin: Point3) -> Result<Self, MathError> {
        Ok(Self {
            frame,
            origin: origin.validated()?,
        })
    }
    /// Returns the body basis.
    pub const fn frame(self) -> Frame3 {
        self.frame
    }
    /// Returns the body origin in the enclosing coordinate system, in mm.
    pub const fn origin(self) -> Point3 {
        self.origin
    }
    /// Evaluates each coordinate as (((R0*x + R1*y) + R2*z) + t).
    pub fn apply_point(self, point: Point3) -> Result<Point3, MathError> {
        let p = point.validated()?;
        let rotated = self.frame.map_vector(Vec3::new(p.x, p.y, p.z)?)?;
        self.origin.translated(rotated)
    }
    /// Maps a vector through the body basis; translation has no effect.
    pub fn apply_vector(self, vector: Vec3) -> Result<Vec3, MathError> {
        self.frame.map_vector(vector)
    }
}
