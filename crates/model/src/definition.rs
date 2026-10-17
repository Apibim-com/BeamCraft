// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! Logical content of the immutable profile and material definitions (F04 section 4, F06
//! section 2, P02).
//!
//! A definition is pinned by the full content digest of its stored USD layer, which the layer
//! engine computes; this module only admits the logical values with the F04 section 3 codes and
//! caps. There is no catalogue, no commercial designation and no inferred default: a material
//! without a density keeps an unknown density and never selects a guessed one.

use crate::command::CommandError;
use crate::error::ErrorCode;
use crate::limits::{DIMENSION_CAP_MM, MAX_DENSITY_KG_M3};
use beamcraft_geom::tolerance::ABSOLUTE_FLOOR_MM;

/// A physical (engineering) material. Display appearance is a separate reference and signature
/// (F04 section 4) and is not part of this value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MaterialSpec {
    /// Density in kg/m3. `None` is unknown, never a default.
    pub density_kg_m3: Option<f64>,
}

impl MaterialSpec {
    /// Admits the density: finite (`NON_FINITE_NUMBER`), at most 1,000,000 kg/m3
    /// (`LIMIT_EXCEEDED`) and greater than zero (`INVALID_ARGUMENT`).
    pub fn validate(&self) -> Result<(), CommandError> {
        let Some(density) = self.density_kg_m3 else {
            return Ok(());
        };
        if !density.is_finite() {
            return Err(CommandError::new(
                ErrorCode::NonFiniteNumber,
                "density_kg_m3",
                "value is not finite",
            ));
        }
        if density.abs() > MAX_DENSITY_KG_M3 {
            return Err(CommandError::new(
                ErrorCode::LimitExceeded,
                "density_kg_m3",
                "density exceeds the density cap",
            ));
        }
        if density <= 0.0 {
            return Err(CommandError::new(
                ErrorCode::InvalidArgument,
                "density_kg_m3",
                "density must be greater than zero",
            ));
        }
        Ok(())
    }
}

/// The first-scope parametric profile: a centred rectangle section (F06 section 2).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RectangleProfileSpec {
    /// Section width in millimetres (along the rolled local X).
    pub width_mm: f64,
    /// Section height in millimetres (along the rolled local Y).
    pub height_mm: f64,
}

impl RectangleProfileSpec {
    /// Admits both dimensions: finite (`NON_FINITE_NUMBER`), at most 1,000,000 mm
    /// (`LIMIT_EXCEEDED`) and at least the absolute geometric floor of 0.001 mm
    /// (`INVALID_ARGUMENT`).
    pub fn validate(&self) -> Result<(), CommandError> {
        let dimensions = [self.width_mm, self.height_mm];
        if dimensions.iter().any(|v| !v.is_finite()) {
            return Err(CommandError::new(
                ErrorCode::NonFiniteNumber,
                "profile",
                "value is not finite",
            ));
        }
        if dimensions.iter().any(|v| v.abs() > DIMENSION_CAP_MM) {
            return Err(CommandError::new(
                ErrorCode::LimitExceeded,
                "profile",
                "section dimension exceeds the dimension cap",
            ));
        }
        if dimensions.iter().any(|v| *v < ABSOLUTE_FLOOR_MM) {
            return Err(CommandError::new(
                ErrorCode::InvalidArgument,
                "profile",
                "section dimension is below the geometric floor",
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn density(value: Option<f64>) -> Result<(), ErrorCode> {
        MaterialSpec {
            density_kg_m3: value,
        }
        .validate()
        .map_err(|e| e.code)
    }

    fn rectangle(width_mm: f64, height_mm: f64) -> Result<(), ErrorCode> {
        RectangleProfileSpec {
            width_mm,
            height_mm,
        }
        .validate()
        .map_err(|e| e.code)
    }

    #[test]
    fn density_is_optional_and_unknown_is_not_replaced() {
        assert_eq!(density(None), Ok(()));
        assert_eq!(density(Some(7850.0)), Ok(()));
        assert_eq!(density(Some(MAX_DENSITY_KG_M3)), Ok(()));
        assert_eq!(density(Some(1e-9)), Ok(()));
    }

    #[test]
    fn density_rejections_have_their_codes() {
        assert_eq!(density(Some(f64::NAN)), Err(ErrorCode::NonFiniteNumber));
        assert_eq!(
            density(Some(f64::INFINITY)),
            Err(ErrorCode::NonFiniteNumber)
        );
        assert_eq!(density(Some(0.0)), Err(ErrorCode::InvalidArgument));
        assert_eq!(density(Some(-1.0)), Err(ErrorCode::InvalidArgument));
        assert_eq!(density(Some(1_000_000.5)), Err(ErrorCode::LimitExceeded));
    }

    #[test]
    fn rectangle_dimensions_follow_the_f04_caps_and_floor() {
        assert_eq!(rectangle(200.0, 300.0), Ok(()));
        assert_eq!(rectangle(0.001, 1_000_000.0), Ok(()));
        assert_eq!(rectangle(0.0009, 300.0), Err(ErrorCode::InvalidArgument));
        assert_eq!(rectangle(200.0, 0.0), Err(ErrorCode::InvalidArgument));
        assert_eq!(rectangle(-200.0, 300.0), Err(ErrorCode::InvalidArgument));
        assert_eq!(rectangle(200.0, 1_000_001.0), Err(ErrorCode::LimitExceeded));
        assert_eq!(rectangle(f64::NAN, 300.0), Err(ErrorCode::NonFiniteNumber));
        assert_eq!(
            rectangle(200.0, f64::NEG_INFINITY),
            Err(ErrorCode::NonFiniteNumber)
        );
    }
}
