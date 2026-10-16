// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim
//! Original synthetic plate parameters; all mesh construction belongs to G02.
use crate::Error;
use beamcraft_kernel::{PlateResult, RectangularPlate, rectangular_plate};
/// Maximum admitted scene size, before any allocation.
pub const MAX_OBJECTS: u32 = 100_000;
/// Session-owned synthetic object (id is a spike ordinal, not a production UUID).
#[derive(Debug, Clone, PartialEq)]
pub struct Object {
    /// Stable ordinal independent of selected scene size.
    pub id: u32,
    /// Display placement in mm.
    pub translation: [f32; 3],
    /// Validated G02 mesh and its analytic result.
    pub plate: PlateResult,
    /// Display scale of the shared kernel-generated unit box; never engineering input.
    pub scale: [f32; 3],
}
/// Immutable in-process session snapshot.
#[derive(Debug, PartialEq)]
pub struct Scene {
    /// Objects in ascending stable-id order.
    pub objects: Vec<Object>,
    /// Display camera target only, not an engineering bounds result.
    pub target: [f32; 3],
    /// Display framing extent in mm.
    pub extent: f32,
}
/// Build each varied plate with the real kernel, with fixed grid width for prefix stability.
pub fn build(count: u32) -> Result<Scene, Error> {
    if count == 0 || count > MAX_OBJECTS {
        return Err(Error::Input("count must be 1..=100000"));
    }
    let mut objects = Vec::new();
    objects
        .try_reserve_exact(count as usize)
        .map_err(|_| Error::Allocation)?;
    for id in 0..count {
        let width = 65.0 + f64::from(id % 11) * 6.0;
        let length = 70.0 + f64::from(id % 17) * 3.0;
        let thickness = 6.0 + f64::from(id % 8) * 2.0;
        let plate = rectangular_plate(RectangularPlate {
            width_mm: width,
            length_mm: length,
            thickness_mm: thickness,
        })
        .map_err(Error::Kernel)?;
        objects.push(Object {
            id,
            translation: [(id % 100) as f32 * 160.0, (id / 100) as f32 * 160.0, 0.0],
            plate,
            scale: [width as f32, length as f32, thickness as f32],
        });
    }
    let width = count.min(100) as f32 * 160.0;
    let height = count.div_ceil(100) as f32 * 160.0;
    Ok(Scene {
        objects,
        target: [width * 0.5 - 16.0, height * 0.5 - 16.0, 8.0],
        extent: width.max(height),
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn counts_ids_meshes_and_determinism() {
        for count in [1, 1_000, 10_000, 100_000] {
            let a = build(count).expect("valid synthetic scene");
            assert_eq!(a.objects.len(), count as usize);
            for (id, o) in a.objects.iter().enumerate() {
                assert_eq!(o.id as usize, id);
                assert_eq!(o.plate.mesh().triangles().len(), 12);
                assert_eq!(o.plate.mesh().blob().len(), 392);
                assert!(o.plate.volume_mm3() > 0.0);
            }
        }
        let a = build(1_000).expect("scene");
        let b = build(1_001).expect("scene");
        assert!(a.objects.iter().zip(&b.objects).all(|(a, b)| a == b));
        assert_eq!(a, build(1_000).expect("repeat"));
    }
    #[test]
    fn limits_before_allocating() {
        assert!(build(0).is_err());
        assert!(build(MAX_OBJECTS + 1).is_err());
    }
}
