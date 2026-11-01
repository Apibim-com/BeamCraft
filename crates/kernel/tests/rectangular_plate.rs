// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! Independent readers and invariants for the G02 projections of F06 cases.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use beamcraft_geom::Point3;
use beamcraft_kernel::diagnostic::{Code, Field, Stage};
use beamcraft_kernel::feature_id::{PLATE_EDGE_VERTICES, PLATE_EDGES, PLATE_VERTICES};
use beamcraft_kernel::{RectangularPlate, rectangular_plate, rectangular_plate_with_cancel};
use beamcraft_model::error::ErrorCode;
use std::collections::{BTreeMap, BTreeSet};

fn input(a: f64, b: f64, c: f64) -> RectangularPlate {
    RectangularPlate {
        width_mm: a,
        length_mm: b,
        thickness_mm: c,
    }
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    (a[0] * b[0] + a[1] * b[1]) + a[2] * b[2]
}
fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn coords(p: Point3) -> [f64; 3] {
    [p.x, p.y, p.z]
}

fn independently_check(plate: RectangularPlate) {
    let result = rectangular_plate(plate).unwrap();
    let mesh = result.mesh();
    assert_eq!(mesh.vertices().len(), 8);
    assert_eq!(mesh.triangles().len(), 12);
    assert_eq!(mesh.face_table().len(), 6);
    assert_eq!(mesh.subdivision_scheme(), "none");
    assert_eq!(mesh.orientation(), "rightHanded");
    let mut edges: BTreeMap<(u32, u32), Vec<(u32, u32)>> = BTreeMap::new();
    let mut sum = 0.0;
    let mut area = 0.0;
    let center = [
        plate.width_mm / 2.0,
        plate.length_mm / 2.0,
        plate.thickness_mm / 2.0,
    ];
    // Connectivity is checked independently of Euler and incidence.
    let mut adjacency = [
        BTreeSet::new(),
        BTreeSet::new(),
        BTreeSet::new(),
        BTreeSet::new(),
        BTreeSet::new(),
        BTreeSet::new(),
        BTreeSet::new(),
        BTreeSet::new(),
    ];
    for (triangle, face) in mesh.triangles().iter().zip(mesh.triangle_faces()) {
        let [i, j, k] = *triangle;
        assert!(i < 8 && j < 8 && k < 8 && i != j && j != k && i != k);
        let [a, b, c] = [i, j, k].map(|i| coords(mesh.vertices()[i as usize]));
        let n = cross(sub(b, a), sub(c, a));
        assert!(dot(n, sub(a, center)) > 0.0, "outward on every triangle");
        let supplied = mesh.face_normals()[usize::from(*face)].components();
        let norm = dot(n, n).sqrt();
        assert!(norm > 0.0 && norm.is_finite());
        for (actual, expected) in n.map(|x| x / norm).into_iter().zip(supplied) {
            assert!((actual - expected).abs() <= 1e-6);
        }
        area += norm / 2.0;
        sum += dot(a, cross(b, c));
        for (u, v) in [(i, j), (j, k), (k, i)] {
            edges.entry((u.min(v), u.max(v))).or_default().push((u, v));
            adjacency[u as usize].insert(v);
        }
    }
    assert_eq!(edges.len(), 18);
    for uses in edges.values() {
        assert_eq!(uses.len(), 2);
        assert_eq!(uses[0], (uses[1].1, uses[1].0));
    }
    let mut seen = BTreeSet::from([0u32]);
    for _ in 0..8 {
        let next: Vec<_> = seen
            .iter()
            .flat_map(|i| adjacency[*i as usize].iter().copied())
            .collect();
        seen.extend(next);
    }
    assert_eq!(seen.len(), 8, "one connected shell");
    assert_eq!(8 + 12, edges.len() + 2);
    assert_eq!(
        mesh.vertices().len() + mesh.face_table().len(),
        PLATE_EDGES.len() + 2,
        "topological Euler"
    );
    let expected_volume = (plate.width_mm * plate.length_mm) * plate.thickness_mm;
    assert_eq!(result.volume_mm3().to_bits(), expected_volume.to_bits());
    assert!((sum / 6.0 - expected_volume).abs() <= 1e-6 * expected_volume.abs());
    let expected_area = 2.0
        * ((plate.width_mm * plate.length_mm + plate.width_mm * plate.thickness_mm)
            + plate.length_mm * plate.thickness_mm);
    assert!((area - expected_area).abs() <= 1e-6 * expected_area.abs());
    let distinct: BTreeSet<_> = mesh
        .vertices()
        .iter()
        .map(|p| coords(*p).map(f64::to_bits))
        .collect();
    assert_eq!(distinct.len(), 8);
    for p in mesh.vertices() {
        for (v, extent) in
            coords(*p)
                .into_iter()
                .zip([plate.width_mm, plate.length_mm, plate.thickness_mm])
        {
            assert!(v == 0.0 || v == extent);
            assert!(v.is_finite());
            if v == 0.0 {
                assert_eq!(v.to_bits(), 0);
            }
        }
    }
    assert_eq!(mesh.blob(), rectangular_plate(plate).unwrap().mesh().blob());
}

#[test]
fn f06_pos_01_closed_outward_euler_volume_and_normal() {
    independently_check(input(1000.0, 500.0, 10.0));
    let result = rectangular_plate(input(1000.0, 500.0, 10.0)).unwrap();
    assert_eq!(result.volume_mm3(), 5_000_000.0);
}

#[test]
fn f06_pos_02_independent_blob_reader_and_published_digest() {
    let result = rectangular_plate(input(1000.0, 500.0, 10.0)).unwrap();
    let mesh = result.mesh();
    let blob = mesh.blob();
    assert_eq!(&blob[..8], b"BCMESH01");
    let header: Vec<_> = blob[8..32]
        .as_chunks::<4>()
        .0
        .iter()
        .map(|v| u32::from_le_bytes(*v))
        .collect();
    assert_eq!(header, [1, 8, 12, 6, 0, 0]);
    let positions: Vec<_> = blob[32..224]
        .as_chunks::<8>()
        .0
        .iter()
        .map(|v| f64::from_le_bytes(*v))
        .collect();
    let expected = [
        [0.0, 0.0, 0.0],
        [1000.0, 0.0, 0.0],
        [0.0, 500.0, 0.0],
        [1000.0, 500.0, 0.0],
        [0.0, 0.0, 10.0],
        [1000.0, 0.0, 10.0],
        [0.0, 500.0, 10.0],
        [1000.0, 500.0, 10.0],
    ];
    assert_eq!(
        positions,
        expected.into_iter().flatten().collect::<Vec<_>>()
    );
    let triangles: Vec<_> = blob[224..368]
        .as_chunks::<4>()
        .0
        .iter()
        .map(|v| u32::from_le_bytes(*v))
        .collect();
    assert_eq!(
        triangles,
        mesh.triangles()
            .iter()
            .flatten()
            .copied()
            .collect::<Vec<_>>()
    );
    let faces: Vec<_> = blob[368..]
        .as_chunks::<2>()
        .0
        .iter()
        .map(|v| u16::from_le_bytes(*v))
        .collect();
    assert_eq!(faces, [0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5]);
    // Independent frozen F06-POS-02 vector detects wrong ordering/endianness/tables.
    assert_eq!(
        mesh.mesh_digest().to_string(),
        "dce6476e9f58de6bfd38bef48e169972e4ddf22264b806f12f933b557a30ed86"
    );
}

#[test]
fn f06_pos_03_volume_projection_only() {
    let result = rectangular_plate(input(1000.0, 500.0, 10.0)).unwrap();
    assert_eq!(result.volume_m3(), 0.005);
}

#[test]
fn f06_pos_14_plate_inclusive_floor() {
    independently_check(input(1000.0, 500.0, 0.001));
    independently_check(input(0.001, 0.001, 0.001));
    assert!(rectangular_plate(input(1000.0, 500.0, 0.001f64.next_down())).is_err());
}

#[test]
fn f06_pos_16_17_18_local_feature_projections() {
    let a = rectangular_plate(input(1000.0, 500.0, 10.0)).unwrap();
    let b = rectangular_plate(input(2000.0, 500.0, 10.0)).unwrap();
    assert_eq!(a.mesh().face_table(), b.mesh().face_table());
    let ids: Vec<_> = a
        .mesh()
        .face_table()
        .iter()
        .chain(PLATE_EDGES.iter())
        .chain(PLATE_VERTICES.iter())
        .copied()
        .collect();
    assert_eq!(ids.len(), 26);
    assert_eq!(ids.iter().collect::<BTreeSet<_>>().len(), 26);
    for id in &ids {
        assert!(
            id.bytes()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || b":.-".contains(&c))
        );
        assert!(id.len() + 42 <= 64); // fid1: + canonical UUID + ':' prefix.
    }
    // Engine qualification is deliberately outside this slice; local names are shared.
    for uuid in [
        "00000000-0000-7000-8000-000000000001",
        "00000000-0000-7000-8000-000000000002",
    ] {
        assert_eq!(
            ids.iter()
                .map(|id| format!("fid1:{uuid}:{id}"))
                .collect::<BTreeSet<_>>()
                .len(),
            26
        );
    }
    for (i, id) in PLATE_VERTICES.iter().enumerate() {
        assert_eq!(
            *id,
            format!("plate:v.x{}.y{}.z{}", i & 1, (i >> 1) & 1, i >> 2)
        );
    }
    let mut edge_set = BTreeSet::new();
    for (id, [u, v]) in PLATE_EDGES.iter().zip(PLATE_EDGE_VERTICES) {
        let p = coords(a.mesh().vertices()[u as usize]);
        let q = coords(a.mesh().vertices()[v as usize]);
        assert_eq!(p.into_iter().zip(q).filter(|(x, y)| x != y).count(), 1);
        for role in id.strip_prefix("plate:e.").unwrap().split('.') {
            let axis = match role.as_bytes()[0] {
                b'x' => 0,
                b'y' => 1,
                _ => 2,
            };
            let value = if role.ends_with('0') {
                0.0
            } else {
                [1000.0, 500.0, 10.0][axis]
            };
            assert_eq!(p[axis], value);
            assert_eq!(q[axis], value);
        }
        assert!(edge_set.insert((u.min(v), u.max(v))));
    }
    assert_eq!(edge_set.len(), 12);
}

#[test]
fn f06_pos_53_54_neg_45_scaled_floor_projections() {
    for c in [0.1572265625, 0.15625] {
        let result = rectangular_plate(input(393216.0, 524288.0, c)).unwrap();
        assert_eq!(result.tolerance().local_size_mm(), 655360.0);
        assert_eq!(result.tolerance().eps_geom_mm(), 0.15625);
        independently_check(input(393216.0, 524288.0, c));
    }
    for c in [0.1552734375, 0.15625f64.next_down()] {
        let failure = rectangular_plate(input(393216.0, 524288.0, c)).unwrap_err();
        assert_eq!(failure.status(), "invalid_input");
        let d = failure.diagnostics().next().unwrap();
        assert_eq!(
            (d.code, d.field, d.stage),
            (Code::BelowMinimum, Field::Thickness, Stage::Ranges)
        );
    }
}

#[test]
fn f06_neg_01_nonfinite_in_every_dimension_and_stage_precedence() {
    for v in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        for dims in [[v, 1.0, 1.0], [1.0, v, 1.0], [1.0, 1.0, v]] {
            let [a, b, c] = dims;
            let failure = rectangular_plate(input(a, b, c)).unwrap_err();
            assert_eq!(failure.status(), "invalid_input");
            for d in failure.diagnostics() {
                assert_eq!(d.code, Code::NonFinite);
                assert_eq!(d.stage, Stage::Finiteness);
                assert_eq!(d.code.f04_code(), ErrorCode::NonFiniteNumber);
            }
        }
    }
    let f = rectangular_plate(input(f64::NAN, 1e9, -1.0)).unwrap_err();
    assert_eq!(
        f.diagnostics().map(|d| d.field).collect::<Vec<_>>(),
        [Field::Width]
    );
}

#[test]
fn f06_neg_03_below_minimum_each_dimension() {
    for [a, b, c] in [[0.0009, 1.0, 1.0], [1.0, 0.0009, 1.0], [1.0, 1.0, 0.0009]] {
        let f = rectangular_plate(input(a, b, c)).unwrap_err();
        assert_eq!(f.diagnostics().next().unwrap().code, Code::BelowMinimum);
    }
}

#[test]
fn dimensions_zero_negative_huge_negative_caps_and_aggregation() {
    for v in [0.0, -0.0, -1.0, -f64::MAX] {
        let f = rectangular_plate(input(v, v, v)).unwrap_err();
        assert_eq!(
            f.diagnostics()
                .map(|d| (d.code, d.field, d.stage))
                .collect::<Vec<_>>(),
            [
                (Code::NotPositive, Field::Width, Stage::Ranges),
                (Code::NotPositive, Field::Length, Stage::Ranges),
                (Code::NotPositive, Field::Thickness, Stage::Ranges),
            ]
        );
    }
    let f = rectangular_plate(input(-1.0, 1_000_001.0, 1_000_001.0)).unwrap_err();
    assert_eq!(f.status(), "limit_exceeded");
    assert_eq!(
        f.diagnostics()
            .map(|d| (d.code, d.field, d.stage))
            .collect::<Vec<_>>(),
        [
            (Code::AboveLimit, Field::Length, Stage::Caps),
            (Code::AboveLimit, Field::Thickness, Stage::Caps),
        ]
    );
    for [a, b, c] in [
        [1e6f64.next_up(), 1.0, 1.0],
        [1.0, 1e6f64.next_up(), 1.0],
        [1.0, 1.0, 1e6f64.next_up()],
    ] {
        assert_eq!(
            rectangular_plate(input(a, b, c))
                .unwrap_err()
                .diagnostics()
                .next()
                .unwrap()
                .code,
            Code::AboveLimit
        );
    }
    independently_check(input(1e6, 1e6, 1e6));
    independently_check(input(1e6, 1.0, 1.0));
    independently_check(input(1.0, 1e6, 1.0));
    independently_check(input(1.0, 1.0, 1e6));
}

#[test]
fn f06_neg_16_17_and_pos_37_cancellation() {
    let stages = [
        Stage::Validation,
        Stage::Build,
        Stage::Checks,
        Stage::Quantities,
    ];
    for (i, stage) in stages.into_iter().enumerate() {
        let mut polls = Vec::new();
        let f = rectangular_plate_with_cancel(input(1000.0, 500.0, 10.0), |s| {
            polls.push(s);
            s == stage
        })
        .unwrap_err();
        assert_eq!(polls, &stages[..=i]);
        assert_eq!(f.status(), "cancelled");
        assert_eq!(f.diagnostics().next().unwrap().stage, stage);
        assert_eq!(f.diagnostics().next().unwrap().code, Code::Cancelled);
    }
    let mut polls = 0;
    let result = rectangular_plate_with_cancel(input(1000.0, 500.0, 10.0), |_| {
        polls += 1;
        polls > 4
    })
    .unwrap();
    assert_eq!(polls, 4);
    assert_eq!(result.volume_mm3(), 5e6);
    // Invalid input never reaches P1; P0 takes precedence even over invalid input.
    assert_eq!(
        rectangular_plate_with_cancel(input(f64::NAN, 0.0, 0.0), |_| true)
            .unwrap_err()
            .status(),
        "cancelled"
    );
}

#[test]
fn deterministic_admitted_grid_has_independent_invariants() {
    for a in [0.001, 0.125, 1.25, 123.456789, 4096.0, 1e6] {
        for b in [0.001, 0.5, 3.75, 517.234567, 1e6] {
            for c in [0.001, 0.375, 10.125, 1e6] {
                let p = input(a, b, c);
                let floor = 0.001f64.max((a * a + b * b).sqrt() / 4194304.0);
                if a >= floor && b >= floor && c >= floor {
                    independently_check(p);
                }
            }
        }
    }
}

#[test]
fn diagnostics_are_fixed_ascii_and_use_existing_f04_codes() {
    for (code, wire, f04) in [
        (
            Code::NonFinite,
            "GEO_NON_FINITE",
            ErrorCode::NonFiniteNumber,
        ),
        (
            Code::NotPositive,
            "GEO_NOT_POSITIVE",
            ErrorCode::InvalidArgument,
        ),
        (
            Code::BelowMinimum,
            "GEO_BELOW_MINIMUM",
            ErrorCode::InvalidArgument,
        ),
        (
            Code::AxisTooShort,
            "GEO_AXIS_TOO_SHORT",
            ErrorCode::InvalidArgument,
        ),
        (
            Code::AboveLimit,
            "GEO_ABOVE_LIMIT",
            ErrorCode::LimitExceeded,
        ),
        (
            Code::NumericUncertain,
            "GEO_NUMERIC_UNCERTAIN",
            ErrorCode::UnsupportedGeometry,
        ),
        (
            Code::SelfCheckFailed,
            "GEO_SELF_CHECK_FAILED",
            ErrorCode::UnsupportedGeometry,
        ),
        (Code::Cancelled, "GEO_CANCELLED", ErrorCode::Cancelled),
    ] {
        assert_eq!(code.as_str(), wire);
        assert_eq!(code.f04_code(), f04);
    }
    for field in [
        Field::Width,
        Field::Length,
        Field::Thickness,
        Field::LocalSize,
        Field::Mesh,
        Field::Volume,
        Field::Cancellation,
    ] {
        assert!(field.as_str().is_ascii() && field.as_str().len() <= 64);
    }
}
