// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! G06 mapping to the original G03 fixtures: the plate meshes the real G02 builder
//! seals are re-read as BCMESH01 blobs and certified and measured by the generic
//! G06 checks, then compared with the fixtures' exact rational references.
//!
//! Fixtures whose expectation is not `ok` have no mesh, and the four placeholder
//! fixtures (through-hole, cavity, thin-wall, nonrectangular) need G05 contours
//! and G09 cuts: they are listed and deliberately not run or faked.

#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

use beamcraft_kernel::measure::cross_check_plate;
use beamcraft_kernel::topology::{Limits, analyze, analyze_with_tolerance};
use beamcraft_kernel::trimesh::TriMesh;
use beamcraft_kernel::verdict::ShellStatus;
use beamcraft_kernel::{steel::RectangularPlate, steel::rectangular_plate};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/fixtures/g03")
}

fn rows(name: &str) -> Vec<Vec<String>> {
    std::fs::read_to_string(root().join(name))
        .unwrap()
        .lines()
        .filter(|s| !s.starts_with('#') && !s.is_empty())
        .map(|s| s.split('\t').map(str::to_owned).collect())
        .collect()
}

fn rational(s: &str) -> f64 {
    let (n, d) = s.split_once('/').unwrap();
    n.parse::<f64>().unwrap() / d.parse::<f64>().unwrap()
}

fn vector(s: &str) -> [f64; 3] {
    let v: Vec<f64> = s.split(',').map(rational).collect();
    v.try_into().unwrap()
}

#[test]
fn supported_g03_plates_certify_and_measure_to_their_exact_references() {
    let expectations: BTreeMap<String, Vec<String>> = rows("expectations.tsv")
        .into_iter()
        .map(|r| (r[0].clone(), r))
        .collect();
    let mut exercised = Vec::new();
    let mut not_run = Vec::new();
    for entry in rows("manifest.tsv") {
        let id = &entry[0];
        let expected = &expectations[id];
        if expected[1] != "ok" {
            not_run.push(format!("{id} ({})", expected[1]));
            continue;
        }
        let p = &rows(&entry[1])[0];
        let size: [f64; 3] = std::array::from_fn(|i| p[i + 2].parse().unwrap());
        let plate = rectangular_plate(RectangularPlate {
            width_mm: size[0],
            length_mm: size[1],
            thickness_mm: size[2],
        })
        .unwrap_or_else(|e| panic!("{id}: {e:?}"));
        let mesh = TriMesh::from_blob(plate.mesh().blob()).unwrap();
        // The body's own F06 scale: the plate rule excludes thickness from D.
        let a =
            analyze_with_tolerance(&mesh, plate.tolerance(), Limits::DEFAULT, |_| false).unwrap();
        assert_eq!(a.status(), ShellStatus::Certified, "{id}");
        if id == "g03.plate.scaled-minimum" {
            // Documented limitation: with no body inputs the generic rule measures D
            // from vertex 0 in 3D, which is slightly larger than the plate's contour D,
            // so a thickness exactly at the plate's inclusive minimum is below its floor.
            assert_eq!(analyze(&mesh).unwrap().status(), ShellStatus::Degenerate);
        }
        // Counts: V,E,F,T,meshE,shells,euler.
        let counts: Vec<i64> = expected[6].split(',').map(|x| x.parse().unwrap()).collect();
        assert_eq!(a.vertex_count() as i64, counts[0], "{id}: V");
        assert_eq!(a.triangle_count() as i64, counts[3], "{id}: T");
        assert_eq!(a.edge_count() as i64, counts[4], "{id}: mesh edges");
        assert_eq!(a.shells().len() as i64, counts[5], "{id}: shells");
        assert_eq!(a.euler_characteristic(), counts[6], "{id}: Euler");
        assert_eq!(a.shells()[0].genus, Some(0), "{id}");
        // Bounds are exact dyadic rationals; the vertices are the input scalars.
        assert_eq!(a.bounds().min, vector(&expected[4]), "{id}: min");
        assert_eq!(a.bounds().max, vector(&expected[5]), "{id}: max");
        // Exact rational volume versus the exact-sum mesh enclosure. Parsing the
        // rational to binary64 costs at most a few ulps, so widen by 4e-16 relative.
        let want = rational(&expected[3]);
        let volume = beamcraft_kernel::measure::measure(&mesh, &a)
            .unwrap()
            .volume_mm3()
            .unwrap();
        assert!(
            volume.lo * (1.0 - 4e-16) <= want && want <= volume.hi * (1.0 + 4e-16),
            "{id}: {volume:?} vs {want}"
        );
        let check = cross_check_plate(&plate).unwrap();
        assert!(check.agrees, "{id}: {check:?}");
        exercised.push(id.clone());
    }
    assert_eq!(exercised.len(), 22, "{exercised:?}");
    assert_eq!(not_run.len(), 34, "{not_run:?}");
    for placeholder in ["through-hole", "cavity", "thin-wall", "nonrectangular"] {
        assert!(
            not_run
                .iter()
                .any(|n| n.starts_with(&format!("g03.plate.{placeholder} "))),
            "{placeholder} must stay not run"
        );
    }
}
