// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! Original synthetic scenes built with the real kernel, for the headless command and tests.
//!
//! P01 does not exist yet, so there are no authored elements to snapshot. These scenes feed the
//! contract with genuine kernel plate results. Two things in them are stand-ins, and every
//! consumer output must say so: every fifth element is labelled `support` (a separate-object kind,
//! so the kind filter has something to hide) while still carrying a plate body, and, only when
//! requested, every third element carries a distinct "simplified" plate (the kernel has no
//! operations yet, so there is nothing real to simplify).

use crate::element::{ElementId, ElementKind};
use crate::error::{ViewCode, ViewError};
use crate::limits::MAX_ELEMENTS;
use crate::scene::SceneElement;
use beamcraft_geom::Translation3;
use beamcraft_kernel::{PlateResult, RectangularPlate, rectangular_plate};

/// A scripted change to a synthetic scene, for exercising incremental deltas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyntheticEdit {
    /// Moves the first element by +1000 mm along X: a placement-only change.
    MoveFirst,
    /// Makes the first element's plate 2 mm thicker, keeping width and length: a shape change
    /// that can be repeated.
    ResizeFirst,
    /// Appends one new element.
    AddOne,
    /// Removes the last element.
    RemoveLast,
}

impl SyntheticEdit {
    /// Parses `move-first`, `resize-first`, `add-one` or `remove-last`.
    pub fn parse(token: &str) -> Option<Self> {
        match token {
            "move-first" => Some(Self::MoveFirst),
            "resize-first" => Some(Self::ResizeFirst),
            "add-one" => Some(Self::AddOne),
            "remove-last" => Some(Self::RemoveLast),
            _ => None,
        }
    }
}

fn unsupported() -> ViewError {
    ViewError::new(ViewCode::NumericUncertain)
}

fn plate(width: f64, length: f64, thickness: f64) -> Result<PlateResult, ViewError> {
    rectangular_plate(RectangularPlate {
        width_mm: width,
        length_mm: length,
        thickness_mm: thickness,
    })
    .map_err(|_| unsupported())
}

fn synthetic_id(ordinal: u64) -> Result<ElementId, ViewError> {
    Ok(ElementId::parse(&format!(
        "01990000-0000-7000-8000-{ordinal:012x}"
    ))?)
}

fn element(ordinal: u64, stand_in_simplified: bool) -> Result<SceneElement, ViewError> {
    let n = u32::try_from(ordinal % 1_000_000).map_err(|_| unsupported())?;
    let width = 65.0 + f64::from(n % 11) * 6.0;
    let length = 70.0 + f64::from(n % 17) * 3.0;
    let thickness = 6.0 + f64::from(n % 8) * 2.0;
    let column = f64::from(n % 100) * 160.0;
    let row = f64::from(n / 100 % 10_000) * 160.0;
    let kind = if n % 5 == 4 {
        ElementKind::Support
    } else {
        ElementKind::Plate
    };
    let placement = Translation3::new(column, row, 0.0).map_err(|_| unsupported())?;
    let element = SceneElement::new(
        synthetic_id(ordinal)?,
        kind,
        placement,
        plate(width, length, thickness)?,
    );
    if stand_in_simplified && n % 3 == 2 {
        Ok(element.with_simplified(plate(width, length, thickness / 2.0)?))
    } else {
        Ok(element)
    }
}

/// `count` varied kernel plates on a 100-column grid (pitch 160 mm), ids `…-000000000000` upward.
///
/// `count` must be 1 through 65,536 (`VIEW_ELEMENT_LIMIT`, `VIEW_ARGUMENT_INVALID`), checked
/// before any allocation.
pub fn plate_scene(count: u32, stand_in_simplified: bool) -> Result<Vec<SceneElement>, ViewError> {
    if count == 0 {
        return Err(ViewError::new(ViewCode::ArgumentInvalid));
    }
    let count = usize::try_from(count).map_err(|_| ViewError::new(ViewCode::ElementLimit))?;
    if count > MAX_ELEMENTS {
        return Err(ViewError::new(ViewCode::ElementLimit));
    }
    let mut scene = Vec::with_capacity(count);
    for ordinal in 0..count {
        let ordinal = u64::try_from(ordinal).map_err(|_| unsupported())?;
        scene.push(element(ordinal, stand_in_simplified)?);
    }
    Ok(scene)
}

/// Applies one scripted edit to a scene built by [`plate_scene`].
///
/// An edit that needs an element on an empty scene, or that would exceed the element cap, is
/// rejected; the scene is left unchanged on error.
pub fn apply_edit(
    scene: &mut Vec<SceneElement>,
    edit: SyntheticEdit,
    stand_in_simplified: bool,
) -> Result<(), ViewError> {
    let empty = || ViewError::new(ViewCode::ArgumentInvalid);
    match edit {
        SyntheticEdit::MoveFirst => {
            let first = scene.first_mut().ok_or_else(empty)?;
            let [x, y, z] = first.placement.offset().components();
            first.placement = Translation3::new(x + 1000.0, y, z).map_err(|_| unsupported())?;
        }
        SyntheticEdit::ResizeFirst => {
            let first = scene.first_mut().ok_or_else(empty)?;
            // Vertex bits are i = xb + 2*yb + 4*zb from the origin, so vertices 1, 2 and 4 give
            // the current width, length and thickness exactly as the kernel built them.
            let v = first.full.mesh().vertices();
            let (w, l, t) = match (v.first(), v.get(1), v.get(2), v.get(4)) {
                (Some(o), Some(x), Some(y), Some(z)) => (x.x - o.x, y.y - o.y, z.z - o.z),
                _ => return Err(unsupported()),
            };
            first.full = plate(w, l, t + 2.0)?;
        }
        SyntheticEdit::AddOne => {
            if scene.len() >= MAX_ELEMENTS {
                return Err(ViewError::new(ViewCode::ElementLimit));
            }
            let ordinal = u64::try_from(scene.len()).map_err(|_| unsupported())?;
            scene.push(element(ordinal, stand_in_simplified)?);
        }
        SyntheticEdit::RemoveLast => {
            scene.pop().ok_or_else(empty)?;
        }
    }
    Ok(())
}
