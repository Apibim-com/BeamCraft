// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim
//! Original synthetic dyadic counterexamples; no external fixtures or oracle process.
use robust::{Coord, Coord3D, incircle, orient2d, orient3d};
fn c(x: f64, y: f64) -> Coord<f64> {
    Coord { x, y }
}
fn c3(x: f64, y: f64, z: f64) -> Coord3D<f64> {
    Coord3D { x, y, z }
}
fn naive2(a: Coord<f64>, b: Coord<f64>, c: Coord<f64>) -> f64 {
    (a.x - c.x) * (b.y - c.y) - (a.y - c.y) * (b.x - c.x)
}
fn naive3(a: Coord3D<f64>, b: Coord3D<f64>, c: Coord3D<f64>, d: Coord3D<f64>) -> f64 {
    let (ax, ay, az) = (a.x - d.x, a.y - d.y, a.z - d.z);
    let (bx, by, bz) = (b.x - d.x, b.y - d.y, b.z - d.z);
    let (cx, cy, cz) = (c.x - d.x, c.y - d.y, c.z - d.z);
    (az * (bx * cy - cx * by) + bz * (cx * ay - ax * cy)) + cz * (ax * by - bx * ay)
}
fn naive_circle(a: Coord<f64>, b: Coord<f64>, c: Coord<f64>, d: Coord<f64>) -> f64 {
    let (ax, ay, bx, by, cx, cy) = (
        a.x - d.x,
        a.y - d.y,
        b.x - d.x,
        b.y - d.y,
        c.x - d.x,
        c.y - d.y,
    );
    ((ax * ax + ay * ay) * (bx * cy - cx * by) + (bx * bx + by * by) * (cx * ay - ax * cy))
        + (cx * cx + cy * cy) * (ax * by - bx * ay)
}
#[test]
fn exact_signs_and_limits() {
    let n = 134_217_728.0;
    let (a, b, o) = (c(n, n - 1.0), c(n + 1.0, n), c(0.0, 0.0));
    let (naive, exact) = (naive2(a, b, o), orient2d(a, b, o));
    println!("orient2d naive={naive:?} robust={exact:?} exact_integer_determinant=1");
    assert_eq!(naive, 0.0);
    assert_eq!(exact, 1.0);
    let (a, b, o, d) = (
        c3(n, n - 1.0, 0.0),
        c3(n + 1.0, n, 0.0),
        c3(0.0, 0.0, 0.0),
        c3(0.0, 0.0, 1.0),
    );
    let (naive, exact) = (naive3(a, b, o, d), orient3d(a, b, o, d));
    println!("orient3d naive={naive:?} robust={exact:?} exact_integer_determinant=-1");
    assert_eq!(naive, 0.0);
    assert_eq!(exact, -1.0);
    assert_eq!(orient3d(a, b, o, c3(1.0, 0.0, 0.0)), 0.0);
    let (a, b, c0, d) = (c(n, 0.0), c(0.0, n), c(-n, 0.0), c(1.0, -n));
    let (naive, exact) = (naive_circle(a, b, c0, d), incircle(a, b, c0, d));
    println!(
        "incircle naive={naive:?} robust={exact:?} exact_integer_determinant={:?}",
        -2.0 * n * n
    );
    assert_eq!(naive, 0.0);
    assert!(exact < 0.0);
    let s = 1e-200;
    let answer = orient2d(c(0.0, 0.0), c(s, 0.0), c(0.0, s));
    println!("underflow orient2d={answer:?}, exact sign positive");
    assert_eq!(answer, 0.0);
    let d = glam::DVec3::new(0.5, 0.0, 500000.0);
    let hh = d.x * d.x + d.y * d.y;
    let ll = hh + d.z * d.z;
    assert!(hh < 1e-12 * ll);
    assert!(orient2d(c(0.0, 0.0), c(d.x, d.z), c(0.0, 1.0)) > 0.0);
}
