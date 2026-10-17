// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! Standard UsdGeom values derived from a mesh buffer: the authored `extent` and the flat
//! per-face `normals`. Authoring and validation share these functions, so an ordinary reader
//! sees values that agree with the points it reads.

/// Tolerance, in components of a unit normal, when a stored normal is compared with the one
/// computed from the points. Another tool may normalise in a different order.
pub(super) const NORMAL_TOLERANCE: f32 = 1e-5;

/// Axis-aligned bounds of the points as `[min, max]`; `None` for an empty buffer or a
/// non-finite coordinate.
pub(super) fn extent(points: &[[f32; 3]]) -> Option<[[f32; 3]; 2]> {
    let first = points.first()?;
    let mut min = *first;
    let mut max = *first;
    for p in points {
        for axis in 0..3 {
            let (v, lo, hi) = (p.get(axis)?, min.get_mut(axis)?, max.get_mut(axis)?);
            if !v.is_finite() {
                return None;
            }
            *lo = lo.min(*v);
            *hi = hi.max(*v);
        }
    }
    Some([min, max])
}

/// One unit normal per face by Newell's method, in the order of `counts`. A face whose area
/// vanishes gets the zero vector. `None` when the buffers disagree (index out of range, counts
/// not summing to the index count).
pub(super) fn face_normals(
    points: &[[f32; 3]],
    counts: &[i32],
    indices: &[i32],
) -> Option<Vec<[f32; 3]>> {
    let mut normals = Vec::with_capacity(counts.len());
    let mut cursor = 0usize;
    for count in counts {
        let count = usize::try_from(*count).ok()?;
        let end = cursor.checked_add(count)?;
        let face = indices.get(cursor..end)?;
        cursor = end;
        let mut sum = [0.0f64; 3];
        for (i, a) in face.iter().enumerate() {
            let b = face.get((i + 1) % face.len())?;
            let a = points.get(usize::try_from(*a).ok()?)?;
            let b = points.get(usize::try_from(*b).ok()?)?;
            let (ax, ay, az) = (f64::from(a[0]), f64::from(a[1]), f64::from(a[2]));
            let (bx, by, bz) = (f64::from(b[0]), f64::from(b[1]), f64::from(b[2]));
            sum[0] += (ay - by) * (az + bz);
            sum[1] += (az - bz) * (ax + bx);
            sum[2] += (ax - bx) * (ay + by);
        }
        let length = (sum[0] * sum[0] + sum[1] * sum[1] + sum[2] * sum[2]).sqrt();
        normals.push(if length > 0.0 && length.is_finite() {
            [
                (sum[0] / length) as f32,
                (sum[1] / length) as f32,
                (sum[2] / length) as f32,
            ]
        } else {
            [0.0; 3]
        });
    }
    (cursor == indices.len()).then_some(normals)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SQUARE: [[f32; 3]; 4] = [
        [0.0, 0.0, 0.0],
        [2.0, 0.0, 0.0],
        [2.0, 3.0, 0.0],
        [0.0, 3.0, 0.0],
    ];

    #[test]
    fn extent_is_min_and_max() {
        assert_eq!(extent(&SQUARE), Some([[0.0, 0.0, 0.0], [2.0, 3.0, 0.0]]));
        assert_eq!(extent(&[]), None);
        assert_eq!(extent(&[[f32::NAN, 0.0, 0.0]]), None);
    }

    #[test]
    fn counter_clockwise_face_points_up() {
        let normals = face_normals(&SQUARE, &[4], &[0, 1, 2, 3]).unwrap();
        assert_eq!(normals, vec![[0.0, 0.0, 1.0]]);
        let reversed = face_normals(&SQUARE, &[4], &[3, 2, 1, 0]).unwrap();
        assert_eq!(reversed, vec![[0.0, 0.0, -1.0]]);
    }

    #[test]
    fn degenerate_face_has_zero_normal_and_bad_buffers_are_refused() {
        assert_eq!(
            face_normals(&SQUARE, &[3], &[0, 0, 0]).unwrap(),
            vec![[0.0; 3]]
        );
        assert!(face_normals(&SQUARE, &[4], &[0, 1, 2, 4]).is_none());
        assert!(face_normals(&SQUARE, &[4], &[0, 1, 2]).is_none());
        assert!(face_normals(&SQUARE, &[3], &[0, 1, 2, 3]).is_none());
        assert!(face_normals(&SQUARE, &[-1], &[0, 1, 2]).is_none());
    }
}
