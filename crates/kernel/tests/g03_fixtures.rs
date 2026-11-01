// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! G03 independent, fixture-driven checks of the public G02 primitive API.
//! Oracle arithmetic and topology use only std scalars and collections.

#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

use beamcraft_kernel::{PlateResult, RectangularPlate, rectangular_plate};
use beamcraft_model::digest::Sha256;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
struct Fixture {
    id: String,
    support: String,
    plan: String,
    mode: String,
    size: [f64; 3],
    origin: [f64; 3],
    expected: Vec<String>,
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/fixtures/g03")
}

fn read(path: &Path) -> String {
    assert!(std::fs::metadata(path).unwrap().len() <= 32_768);
    let text = std::fs::read_to_string(path).unwrap();
    assert!(!text.starts_with('\u{feff}') && !text.contains('\r'));
    text
}

fn rows(text: &str) -> Vec<Vec<String>> {
    text.lines()
        .filter(|s| !s.starts_with('#') && !s.is_empty())
        .map(|s| s.split('\t').map(str::to_owned).collect())
        .collect()
}

fn fixtures() -> Vec<Fixture> {
    let expectations = rows(&read(&root().join("expectations.tsv")));
    let mut expected_by_id = BTreeMap::new();
    for row in expectations {
        assert_eq!(row.len(), 8);
        assert!(row[7].starts_with("F06:"), "{}: missing sources", row[0]);
        assert!(expected_by_id.insert(row[0].clone(), row).is_none());
    }
    let mut paths = BTreeSet::new();
    let mut result = Vec::new();
    for entry in rows(&read(&root().join("manifest.tsv"))) {
        assert_eq!(entry.len(), 8);
        let id = &entry[0];
        assert!(id.starts_with("g03.plate."));
        let name = id.strip_prefix("g03.plate.").unwrap();
        assert!(name.bytes().all(|b| b.is_ascii_lowercase() || b == b'-'));
        assert_eq!(entry[1], format!("parameters/{name}.tsv"), "{id}: path");
        assert!(paths.insert(entry[1].clone()), "{id}: duplicate path");
        let input = read(&root().join(&entry[1]));
        assert_eq!(input.len().to_string(), entry[2], "{id}: bytes");
        assert_eq!(
            Sha256::digest(input.as_bytes()).unwrap().to_string(),
            entry[3],
            "{id}: SHA-256"
        );
        assert_eq!(entry[6], "BeamCraft contributors", "{id}: author");
        assert_eq!(entry[7], "MPL-2.0", "{id}: licence");
        let parameters = rows(&input);
        assert_eq!(parameters.len(), 1, "{id}: one input record");
        let p = &parameters[0];
        assert_eq!(p.len(), 8, "{id}: parameter schema");
        assert_eq!(&p[0], id, "{id}: input ID");
        result.push(Fixture {
            id: id.clone(),
            support: entry[4].clone(),
            plan: entry[5].clone(),
            mode: p[1].clone(),
            size: std::array::from_fn(|i| p[i + 2].parse().unwrap()),
            origin: std::array::from_fn(|i| p[i + 5].parse().unwrap()),
            expected: expected_by_id.remove(id).unwrap(),
        });
    }
    assert!(expected_by_id.is_empty(), "unlisted expectations");
    let disk_paths: BTreeSet<_> = std::fs::read_dir(root().join("parameters"))
        .unwrap()
        .map(|e| format!("parameters/{}", e.unwrap().file_name().to_str().unwrap()))
        .collect();
    assert_eq!(paths, disk_paths, "unlisted parameter files");
    assert_eq!(result.len(), 56, "fixture set version 1");
    result
}

fn input(f: &Fixture) -> RectangularPlate {
    let [width_mm, length_mm, thickness_mm] = f.size;
    RectangularPlate {
        width_mm,
        length_mm,
        thickness_mm,
    }
}

fn evaluate(f: &Fixture) -> PlateResult {
    rectangular_plate(input(f)).unwrap_or_else(|e| panic!("{}: {e:?}", f.id))
}

fn rational(s: &str) -> f64 {
    let (n, d) = s.split_once('/').unwrap();
    n.parse::<f64>().unwrap() / d.parse::<f64>().unwrap()
}

fn vector(s: &str) -> [f64; 3] {
    let components: Vec<_> = s.split(',').map(rational).collect();
    components.try_into().unwrap()
}

// CR-TOL-001 quantity comparison is not an admission epsilon. F06 section 4.7.
fn quantity_equal(actual: f64, expected: f64) -> bool {
    actual.is_finite() && (actual - expected).abs() <= 1e-6 * expected.abs()
}

// F06 sections 2/4: original dimension-API projection, no geom/kernel helpers.
fn scale([a, b, _]: [f64; 3]) -> (f64, f64) {
    let a = if a > 0.0 { a } else { 0.0 };
    let b = if b > 0.0 { b } else { 0.0 };
    let diagonal = ((a * a + b * b) + 0.0).sqrt();
    let floor = (diagonal * (1.0 / 4_194_304.0)).max(0.001);
    (diagonal, floor)
}

fn admission(size: [f64; 3]) -> Vec<String> {
    let fields = ["width_mm", "length_mm", "thickness_mm"];
    for (stage, checks) in [
        (
            "Finiteness",
            size.map(|x| (!x.is_finite()).then_some("GEO_NON_FINITE")),
        ),
        (
            "Caps",
            size.map(|x| (x > 1_000_000.0).then_some("GEO_ABOVE_LIMIT")),
        ),
    ] {
        let failures: Vec<_> = fields
            .iter()
            .zip(checks)
            .filter_map(|(field, code)| code.map(|code| format!("{stage}:{field}:{code}")))
            .collect();
        if !failures.is_empty() {
            return failures;
        }
    }
    let (_, floor) = scale(size);
    fields
        .iter()
        .zip(size)
        .filter_map(|(field, x)| {
            let code = if x <= 0.0 {
                "GEO_NOT_POSITIVE"
            } else if x < floor {
                "GEO_BELOW_MINIMUM"
            } else {
                return None;
            };
            Some(format!("Ranges:{field}:{code}"))
        })
        .collect()
}

#[derive(Clone)]
struct Observed {
    points: Vec<[f64; 3]>,
    triangles: Vec<[u32; 3]>,
    faces: Vec<u16>,
    volume: f64,
}

fn observe(result: &PlateResult) -> Observed {
    Observed {
        points: result
            .mesh()
            .vertices()
            .iter()
            .map(|p| [p.x, p.y, p.z])
            .collect(),
        triangles: result.mesh().triangles().to_vec(),
        faces: result.mesh().triangle_faces().to_vec(),
        volume: result.volume_mm3(),
    }
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|i| a[i] - b[i])
}

// Expand a determinant directly; no production vector cross/dot/volume routine.
fn determinant(a: [f64; 3], b: [f64; 3], c: [f64; 3]) -> f64 {
    ((a[0] * b[1] * c[2] + a[1] * b[2] * c[0]) + a[2] * b[0] * c[1])
        - ((a[2] * b[1] * c[0] + a[1] * b[0] * c[2]) + a[0] * b[2] * c[1])
}

fn audit(f: &Fixture, mesh: &Observed) -> Result<(), String> {
    // A bounded test checker of boxes, not a runtime imported-mesh certifier.
    macro_rules! require {
        ($condition:expr, $message:literal) => {
            match $condition {
                true => {}
                false => return Err(format!("{}: {}", f.id, $message)),
            }
        };
    }
    let expected_counts: Vec<usize> = f.expected[6]
        .split(',')
        .map(|x| x.parse().unwrap())
        .collect();
    require!(expected_counts.len() == 7, "count schema");
    require!(mesh.points.len() == expected_counts[0], "vertex count");
    require!(mesh.triangles.len() == expected_counts[3], "triangle count");
    require!(
        mesh.faces.len() == mesh.triangles.len(),
        "triangle face count"
    );
    let low = vector(&f.expected[4]);
    let high = vector(&f.expected[5]);
    let mut corners = BTreeSet::new();
    let mut min = [f64::INFINITY; 3];
    let mut max = [f64::NEG_INFINITY; 3];
    for p in &mesh.points {
        require!(
            p.iter()
                .all(|x| x.is_finite() && (*x != 0.0 || x.to_bits() == 0)),
            "finite canonical coordinates"
        );
        // Origin primitive bounds/corners are exactly the input scalars.
        require!(
            p.iter()
                .enumerate()
                .all(|(i, x)| *x == low[i] || *x == high[i]),
            "vertex is an analytic corner"
        );
        corners.insert(p.map(f64::to_bits));
        for axis in 0..3 {
            if p[axis] < min[axis] {
                min[axis] = p[axis];
            }
            if p[axis] > max[axis] {
                max[axis] = p[axis];
            }
        }
    }
    require!(corners.len() == 8, "eight distinct corners");
    require!(min == low && max == high, "analytic bounds");
    // Recover faces geometrically, and box edges by adjacent distinct face roles.
    // This never imports the kernel's face/edge/triangle constant tables.
    let mut edges = BTreeMap::<(u32, u32), Vec<(usize, bool)>>::new();
    let mut face_counts = [0usize; 6];
    let mut used = BTreeSet::<u32>::new();
    let mut signed_volume = 0.0;
    let mut area = 0.0;
    let anchor = f.size.map(|x| x / 2.0);
    for (triangle, face) in mesh.triangles.iter().zip(&mesh.faces) {
        let [ia, ib, ic] = *triangle;
        require!(
            ia != ib && ib != ic && ia != ic,
            "distinct triangle indices"
        );
        let points: Option<Vec<_>> = triangle
            .iter()
            .map(|i| {
                usize::try_from(*i)
                    .ok()
                    .and_then(|i| mesh.points.get(i).copied())
            })
            .collect();
        let points = points.ok_or_else(|| format!("{}: index range", f.id))?;
        let (a, b, c) = (points[0], points[1], points[2]);
        let (ab, ac) = (sub(b, a), sub(c, a));
        let normal = [
            ab[1] * ac[2] - ab[2] * ac[1],
            ab[2] * ac[0] - ab[0] * ac[2],
            ab[0] * ac[1] - ab[1] * ac[0],
        ];
        let plane: Vec<_> = (0..3).filter(|&i| a[i] == b[i] && b[i] == c[i]).collect();
        require!(plane.len() == 1, "one axis-aligned face plane");
        let axis = plane[0];
        let upper = a[axis] == high[axis];
        let inferred_face = 2 * axis + usize::from(upper);
        require!(
            usize::from(*face) == inferred_face,
            "face role matches geometry"
        );
        let sign = if upper { 1.0 } else { -1.0 };
        require!(normal[axis] * sign > 0.0, "outward nonzero triangle");
        face_counts[inferred_face] += 1;
        area +=
            ((normal[0] * normal[0] + normal[1] * normal[1]) + normal[2] * normal[2]).sqrt() / 2.0;
        signed_volume += determinant(sub(a, anchor), sub(b, anchor), sub(c, anchor)) / 6.0;
        used.extend(triangle);
        for (a, b) in [(ia, ib), (ib, ic), (ic, ia)] {
            edges
                .entry((a.min(b), a.max(b)))
                .or_default()
                .push((inferred_face, a < b));
        }
    }
    require!(
        face_counts.iter().filter(|&&n| n != 0).count() == expected_counts[2],
        "face count"
    );
    require!(face_counts == [2; 6], "two triangles per face");
    require!(used.len() == mesh.points.len(), "all vertices used");
    require!(edges.len() == expected_counts[4], "mesh edge count");
    let mut box_edges = 0usize;
    for occurrences in edges.values() {
        require!(occurrences.len() == 2, "closed edge incidence");
        require!(
            occurrences[0].1 != occurrences[1].1,
            "opposite edge directions"
        );
        box_edges += usize::from(occurrences[0].0 != occurrences[1].0);
    }
    require!(box_edges == expected_counts[1], "topological edge count");
    require!(
        mesh.points.len() + mesh.triangles.len() == edges.len() + expected_counts[6],
        "triangle Euler characteristic"
    );
    require!(
        mesh.points.len() + expected_counts[2] == box_edges + expected_counts[6],
        "topological Euler characteristic"
    );
    let mut reached = BTreeSet::from([0u32]);
    for _ in 0..mesh.points.len() {
        for &(a, b) in edges.keys() {
            if reached.contains(&a) || reached.contains(&b) {
                reached.extend([a, b]);
            }
        }
    }
    require!(
        reached.len() == mesh.points.len() && expected_counts[5] == 1,
        "one connected shell"
    );
    let expected_volume = rational(&f.expected[3]);
    require!(
        quantity_equal(mesh.volume, expected_volume),
        "analytic rational volume"
    );
    require!(
        signed_volume > 0.0 && quantity_equal(signed_volume, expected_volume),
        "signed shell volume"
    );
    let [a, b, c] = f.size;
    require!(
        quantity_equal(area, 2.0 * ((a * b + a * c) + b * c)),
        "shell surface area"
    );
    Ok(())
}

#[test]
fn original_supported_fixtures_match_independent_references() {
    let mut count = 0;
    for f in fixtures().into_iter().filter(|f| f.support == "supported") {
        count += 1;
        assert_eq!(f.mode, "rectangle", "{}", f.id);
        assert_eq!(f.origin, [0.0; 3], "{}", f.id);
        assert_eq!(f.expected[1], "ok", "{}", f.id);
        assert_eq!(f.expected[2], "-", "{}", f.id);
        assert!(
            admission(f.size).is_empty(),
            "{}: independent admission",
            f.id
        );
        let result = evaluate(&f);
        audit(&f, &observe(&result)).unwrap();
        let [a, b, c] = f.size;
        assert_eq!(
            result.volume_mm3().to_bits(),
            ((a * b) * c).to_bits(),
            "{}: F06 order",
            f.id
        );
        assert_eq!(
            result.volume_m3().to_bits(),
            (((a * b) * c) / 1e9).to_bits(),
            "{}: units",
            f.id
        );
        let (diagonal, floor) = scale(f.size);
        assert_eq!(
            result.tolerance().local_size_mm().to_bits(),
            diagonal.to_bits(),
            "{}: L",
            f.id
        );
        assert_eq!(
            result.tolerance().eps_geom_mm().to_bits(),
            floor.to_bits(),
            "{}: floor",
            f.id
        );
        assert_eq!(
            result.mesh().face_table().len(),
            6,
            "{}: semantic faces",
            f.id
        );
        for (face, name) in result.mesh().face_table().iter().enumerate() {
            let axis = ["x", "y", "z"][face / 2];
            assert_eq!(
                *name,
                format!("plate:f.{axis}{}", face % 2),
                "{}: role",
                f.id
            );
        }
        assert_eq!(result.mesh().orientation(), "rightHanded", "{}", f.id);
        assert_eq!(result.mesh().subdivision_scheme(), "none", "{}", f.id);
        let repeated = evaluate(&f);
        assert_eq!(
            result.mesh().blob(),
            repeated.mesh().blob(),
            "{}: repeat bytes",
            f.id
        );
    }
    assert_eq!(count, 22);
}

#[test]
fn original_negative_fixtures_report_the_first_stage_and_all_its_codes() {
    let mut count = 0;
    for f in fixtures().into_iter().filter(|f| f.support == "invalid") {
        count += 1;
        let expected: Vec<_> = f.expected[2].split(',').map(str::to_owned).collect();
        assert_eq!(
            admission(f.size),
            expected,
            "{}: independently declared errors",
            f.id
        );
        let failure = match rectangular_plate(input(&f)) {
            Ok(_) => panic!("{}: expected rejection, got geometry", f.id),
            Err(failure) => failure,
        };
        let actual: Vec<_> = failure
            .diagnostics()
            .map(|d| {
                let f04 = match d.code.as_str() {
                    "GEO_NON_FINITE" => "NON_FINITE_NUMBER",
                    "GEO_ABOVE_LIMIT" => "LIMIT_EXCEEDED",
                    "GEO_NOT_POSITIVE" | "GEO_BELOW_MINIMUM" => "INVALID_ARGUMENT",
                    _ => "unexpected diagnostic",
                };
                assert_eq!(d.code.f04_code().as_str(), f04, "{}: F04 mapping", f.id);
                format!("{:?}:{}:{}", d.stage, d.field.as_str(), d.code.as_str())
            })
            .collect();
        assert_eq!(actual, expected, "{}: diagnostics", f.id);
        assert_eq!(failure.status(), f.expected[1], "{}: status", f.id);
        assert!(
            f.expected[3..7].iter().all(|s| s == "-"),
            "{}: no partial values",
            f.id
        );
    }
    assert_eq!(count, 26);
}

#[test]
fn unsupported_and_unconfirmed_descriptions_do_not_claim_kernel_execution() {
    let mut counts = BTreeMap::new();
    for f in fixtures() {
        *counts.entry(f.support.clone()).or_insert(0) += 1;
        match f.support.as_str() {
            "supported" | "invalid" => {}
            "unsupported" => {
                assert_eq!(f.mode, "placeholder", "{}", f.id);
                assert_eq!(f.expected[1], "not_run", "{}", f.id);
                assert!(
                    f.expected[2..7].iter().all(|s| s == "-"),
                    "{}: no invented values",
                    f.id
                );
                assert!(
                    f.plan.contains("A02") || f.plan.contains("A04"),
                    "{}: plan",
                    f.id
                );
            }
            "unverified" => {
                assert_eq!(f.expected[1], "not_run", "{}", f.id);
                assert!(
                    f.origin.iter().any(|x| x.abs() == 1e9),
                    "{}: large coordinate",
                    f.id
                );
                assert!(f.plan.contains("G09"), "{}: conformance gate", f.id);
                assert!(
                    admission(f.size).is_empty(),
                    "{}: local admission only",
                    f.id
                );
                let [a, b, c] = f.size;
                assert!(
                    quantity_equal((a * b) * c, rational(&f.expected[3])),
                    "{}: ideal volume",
                    f.id
                );
                assert_eq!(vector(&f.expected[4]), f.origin, "{}: ideal minimum", f.id);
                assert_eq!(
                    vector(&f.expected[5]),
                    std::array::from_fn(|i| f.origin[i] + f.size[i]),
                    "{}: rounded ideal maximum",
                    f.id
                );
            }
            _ => panic!("{}: undeclared support status", f.id),
        }
    }
    assert_eq!(
        counts,
        BTreeMap::from([
            ("supported".into(), 22),
            ("invalid".into(), 26),
            ("unverified".into(), 4),
            ("unsupported".into(), 4)
        ])
    );
}

#[test]
fn independent_checker_detects_corrupt_output_for_every_supported_fixture() {
    for f in fixtures().into_iter().filter(|f| f.support == "supported") {
        let result = evaluate(&f);
        let original = observe(&result);
        audit(&f, &original).unwrap();
        for mutation in 0..9 {
            let mut bad = original.clone();
            match mutation {
                0 => {
                    bad.triangles.remove(0);
                    bad.faces.remove(0);
                }
                1 => bad.triangles[0].swap(1, 2),
                2 => bad.triangles[0] = bad.triangles[1],
                3 => bad.triangles[0][0] = 8,
                4 => bad.points[0] = bad.points[1],
                5 => bad.faces[0] = 5,
                6 => bad.volume *= 2.0,
                7 => bad.points[0][0] = f64::NAN,
                _ => bad.points[0][0] = -0.0,
            }
            let failure = audit(&f, &bad).err().unwrap_or_else(|| {
                panic!("{}: mutation {mutation} escaped independent checker", f.id)
            });
            assert!(
                failure.contains(&f.id),
                "{}: diagnostic names fixture",
                f.id
            );
        }
        let mut reordered = original;
        reordered.triangles.reverse();
        reordered.faces.reverse();
        audit(&f, &reordered).unwrap();
    }
}
