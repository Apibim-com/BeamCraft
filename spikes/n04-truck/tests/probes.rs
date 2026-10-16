// SPDX-License-Identifier: MPL-2.0
//! Independent references and actual public API calls for the research harness.
#![allow(clippy::expect_used)] // Test-only assertions on fixed case IDs and schema.

use beamcraft_n04_truck_spike::{CASES, Case, HEADER, evaluate};

fn metric(row: &str, key: &str) -> String {
    HEADER
        .split('\t')
        .zip(row.split('\t'))
        .find(|(name, _)| *name == key)
        .expect("known column")
        .1
        .to_owned()
}

#[test]
fn rectangular_g03_references_and_index_closure_gap() {
    // G03 README: V=abc, V/E/F=8/12/6, chi=2. Original fixture parameters.
    for (name, expected) in [
        ("g03-unit", 1.0),
        ("g03-unequal", 3243.0),
        ("g03-dyadic", 1.0 / 512.0),
        ("g03-decimal", 0.006),
        ("thin-slab", 8.0),
        ("minimum-cube", 1e-9),
        ("scaled-minimum", 15360.0),
    ] {
        let result = evaluate(&Case::named(name).expect("fixed case")).expect("rectangle builds");
        let volume: f64 = metric(&result.row, "mesh_volume").parse().expect("volume");
        assert!(
            (volume - expected).abs() <= 1e-6 * expected,
            "{name}: {volume}"
        );
        for (key, value) in [
            ("brep_v", "8"),
            ("brep_e", "12"),
            ("brep_f", "6"),
            ("brep_chi", "2"),
            ("brep_closed", "true"),
            ("raw_closed", "false"),
        ] {
            assert_eq!(metric(&result.row, key), value, "{name}: {key}");
        }
    }
}

#[test]
fn concave_contour_volume_is_sum_of_two_disjoint_rectangles() {
    let result = evaluate(&Case::named("contour").expect("case")).expect("contour extrusion");
    let volume: f64 = metric(&result.row, "mesh_volume").parse().expect("volume");
    assert!((volume - 45000.0).abs() <= 0.045);
    assert_eq!(metric(&result.row, "brep_chi"), "2");
}

#[test]
fn translated_thin_plate_uses_local_size_and_retains_thickness() {
    let local = Case::named("thin-slab").expect("case");
    let placed = Case::named("large-offset-thin").expect("case");
    assert_eq!(local.eps_geom(), placed.eps_geom());
    let result = evaluate(&placed).expect("translated extrusion");
    let volume: f64 = metric(&result.row, "mesh_volume").parse().expect("volume");
    assert!((volume - 8.0).abs() <= 8e-6);
}

#[test]
fn geometric_floor_is_admission_not_test_tolerance_or_truck_policy() {
    let a = Case::named("scaled-minimum").expect("case");
    let b = Case::named("scaled-below").expect("case");
    assert_eq!(a.eps_geom(), 5.0 / 4096.0);
    assert!(b.dimensions[2] < b.eps_geom());
    // Actual truck accepts a body that a future BeamCraft guard must reject.
    assert!(evaluate(&b).is_ok());
}

#[test]
fn all_expected_values_are_finite_and_case_ids_unique() {
    let mut seen = std::collections::BTreeSet::new();
    for name in CASES {
        assert!(seen.insert(name));
        let c = Case::named(name).expect("case");
        assert!(c.expected_volume.is_finite() && c.expected_volume > 0.0);
    }
    assert!(Case::named("arbitrary-input").is_none());
}

#[test]
fn below_documented_tessellation_floor_is_caught_without_reusing_state() {
    let result =
        std::panic::catch_unwind(|| evaluate(&Case::named("tess-tol-below").expect("case")));
    assert!(result.is_err());
    assert!(evaluate(&Case::named("g03-unit").expect("case")).is_ok());
}
