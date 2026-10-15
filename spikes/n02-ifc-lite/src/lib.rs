//! Original, isolated numeric probes for N02. No upstream implementation is copied.
#![forbid(unsafe_code)]
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unreachable,
    clippy::indexing_slicing
)]

use ifc_lite_geometry::{
    ClippingProcessor, Matrix4, Mesh, Plane, Point2, Point3, Profile2D, Vector3, extrude_profile,
};
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::hint::black_box;
use std::time::Instant;

type ProbeResult<T> = Result<T, Box<dyn Error>>;
type Position = (u32, u32, u32);

fn rectangle(w: f64, h: f64) -> Profile2D {
    Profile2D::new(vec![
        Point2::new(0.0, 0.0),
        Point2::new(w, 0.0),
        Point2::new(w, h),
        Point2::new(0.0, h),
    ])
}

fn plate(w: f64, h: f64, d: f64) -> ProbeResult<Mesh> {
    Ok(extrude_profile(&rectangle(w, h), d, None)?)
}

fn member() -> ProbeResult<Mesh> {
    let profile = Profile2D::new(vec![
        Point2::new(-100.0, -150.0),
        Point2::new(100.0, -150.0),
        Point2::new(100.0, 150.0),
        Point2::new(-100.0, 150.0),
    ]);
    // F06-POS-04: section X -> +Y, section Y -> +Z, extrusion Z -> +X.
    let transform = Matrix4::new(
        0.0, 0.0, 1.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    );
    Ok(extrude_profile(&profile, 4000.0, Some(transform))?)
}

fn position(mesh: &Mesh, index: u32) -> ProbeResult<[f64; 3]> {
    let offset = usize::try_from(index)?
        .checked_mul(3)
        .ok_or("offset overflow")?;
    let end = offset.checked_add(3).ok_or("offset overflow")?;
    let slice = mesh.positions.get(offset..end).ok_or("bad mesh index")?;
    let mut values = slice.iter().copied().map(f64::from);
    Ok([
        values.next().ok_or("missing x")?,
        values.next().ok_or("missing y")?,
        values.next().ok_or("missing z")?,
    ])
}

fn sub([a, b, c]: [f64; 3], [d, e, f]: [f64; 3]) -> [f64; 3] {
    [a - d, b - e, c - f]
}
fn cross([a, b, c]: [f64; 3], [d, e, f]: [f64; 3]) -> [f64; 3] {
    [b * f - c * e, c * d - a * f, a * e - b * d]
}
fn dot([a, b, c]: [f64; 3], [d, e, f]: [f64; 3]) -> f64 {
    (a * d + b * e) + c * f
}
fn key([a, b, c]: [f64; 3]) -> Position {
    let k = |v: f64| if v == 0.0 { 0 } else { (v as f32).to_bits() };
    (k(a), k(b), k(c))
}

#[derive(Debug)]
struct Metrics {
    vertices: usize,
    triangles: usize,
    unique_positions: usize,
    raw_edges: usize,
    raw_closed: bool,
    raw_euler: i64,
    geometric_edges: usize,
    geometric_closed: bool,
    geometric_euler: i64,
    geometric_components: usize,
    nonzero_triangles: usize,
    outward_triangles: usize,
    finite: bool,
    signed_volume: f64,
    area: f64,
    crate_volume: f64,
    bounds: ([f32; 3], [f32; 3]),
}

fn metrics(mesh: &Mesh) -> ProbeResult<Metrics> {
    if mesh.positions.len() > 3 * 4096 || mesh.indices.len() > 3 * 8192 {
        return Err("probe output cap".into());
    }
    let mut raw = BTreeMap::<(u32, u32), (usize, i64)>::new();
    let mut geometric = BTreeMap::<(Position, Position), (usize, i64)>::new();
    let mut graph = BTreeMap::<Position, BTreeSet<Position>>::new();
    let mut unique = BTreeSet::new();
    let mut volume = 0.0;
    let mut area = 0.0;
    let mut nonzero = 0;
    let mut outward = 0;
    let (lo, hi) = mesh.bounds();
    let center = [
        (f64::from(lo.x) + f64::from(hi.x)) / 2.0,
        (f64::from(lo.y) + f64::from(hi.y)) / 2.0,
        (f64::from(lo.z) + f64::from(hi.z)) / 2.0,
    ];
    for xyz in mesh.positions.chunks_exact(3) {
        let mut it = xyz.iter().copied().map(f64::from);
        unique.insert(key([
            it.next().ok_or("x")?,
            it.next().ok_or("y")?,
            it.next().ok_or("z")?,
        ]));
    }
    for indices in mesh.indices.chunks_exact(3) {
        let mut it = indices.iter().copied();
        let a = it.next().ok_or("a")?;
        let b = it.next().ok_or("b")?;
        let c = it.next().ok_or("c")?;
        let p = position(mesh, a)?;
        let q = position(mesh, b)?;
        let r = position(mesh, c)?;
        let normal = cross(sub(q, p), sub(r, p));
        let magnitude = dot(normal, normal).sqrt();
        if magnitude > 0.0 {
            nonzero += 1;
        }
        if dot(normal, sub(p, center)) > 0.0 {
            outward += 1;
        }
        area += magnitude / 2.0;
        volume += dot(p, cross(q, r)) / 6.0;
        for (u, v) in [(a, b), (b, c), (c, a)] {
            let (edge, sign) = if u < v { ((u, v), 1) } else { ((v, u), -1) };
            let entry = raw.entry(edge).or_default();
            entry.0 += 1;
            entry.1 += sign;
        }
        let kp = key(p);
        let kq = key(q);
        let kr = key(r);
        for (u, v) in [(kp, kq), (kq, kr), (kr, kp)] {
            let (edge, sign) = if u < v { ((u, v), 1) } else { ((v, u), -1) };
            let entry = geometric.entry(edge).or_default();
            entry.0 += 1;
            entry.1 += sign;
            graph.entry(u).or_default().insert(v);
            graph.entry(v).or_default().insert(u);
        }
    }
    let mut seen = BTreeSet::new();
    let mut components = 0;
    for &start in graph.keys() {
        if seen.contains(&start) {
            continue;
        }
        components += 1;
        let mut pending = vec![start];
        while let Some(v) = pending.pop() {
            if !seen.insert(v) {
                continue;
            }
            if let Some(neighbors) = graph.get(&v) {
                pending.extend(neighbors.iter().copied().filter(|n| !seen.contains(n)));
            }
        }
    }
    let vertices = mesh.vertex_count();
    let triangles = mesh.triangle_count();
    let euler = |v: usize, e: usize| -> ProbeResult<i64> {
        Ok(i64::try_from(v)? - i64::try_from(e)? + i64::try_from(triangles)?)
    };
    Ok(Metrics {
        vertices,
        triangles,
        unique_positions: unique.len(),
        raw_edges: raw.len(),
        raw_closed: !raw.is_empty() && raw.values().all(|&(n, s)| n == 2 && s == 0),
        raw_euler: euler(vertices, raw.len())?,
        geometric_edges: geometric.len(),
        geometric_closed: !geometric.is_empty()
            && geometric.values().all(|&(n, s)| n == 2 && s == 0),
        geometric_euler: euler(unique.len(), geometric.len())?,
        geometric_components: components,
        nonzero_triangles: nonzero,
        outward_triangles: outward,
        finite: mesh.positions.iter().all(|v| v.is_finite()),
        signed_volume: volume,
        area,
        crate_volume: ifc_lite_geometry::kernel::mesh_volume::mesh_volume(mesh),
        bounds: ([lo.x, lo.y, lo.z], [hi.x, hi.y, hi.z]),
    })
}

fn bytes(mesh: &Mesh) -> Vec<u8> {
    let mut result = Vec::new();
    for v in mesh.positions.iter().chain(mesh.normals.iter()) {
        result.extend(v.to_le_bytes());
    }
    for v in &mesh.indices {
        result.extend(v.to_le_bytes());
    }
    // Debug includes the remaining public metadata; this is a same-build comparison only.
    result.extend(format!("{mesh:?}").bytes());
    result
}

fn show(name: &str, mesh: &Mesh) -> ProbeResult<()> {
    let m = metrics(mesh)?;
    println!("{name}: {m:?}");
    // Read every field to keep warning checks meaningful for the standalone probe.
    black_box((
        m.vertices,
        m.triangles,
        m.unique_positions,
        m.raw_edges,
        m.raw_closed,
        m.raw_euler,
        m.geometric_edges,
        m.geometric_closed,
        m.geometric_euler,
        m.geometric_components,
        m.nonzero_triangles,
        m.outward_triangles,
        m.finite,
        m.signed_volume,
        m.area,
        m.crate_volume,
        m.bounds,
    ));
    Ok(())
}

fn hostile(name: &str) -> ProbeResult<Mesh> {
    match name {
        "zero" => plate(1000.0, 500.0, 0.0),
        "negative" => plate(1000.0, 500.0, -10.0),
        "zero-width" => plate(0.0, 500.0, 10.0),
        "negative-width" => plate(-1000.0, 500.0, 10.0),
        "huge" => plate(1e300, 1e300, 1e300),
        "nan" => plate(1000.0, 500.0, f64::NAN),
        "infinity" => plate(1000.0, 500.0, f64::INFINITY),
        "nan-contour" => plate(f64::NAN, 500.0, 10.0),
        "infinite-contour" => plate(f64::INFINITY, 500.0, 10.0),
        "collinear" => Ok(extrude_profile(
            &Profile2D::new(vec![
                Point2::new(0.0, 0.0),
                Point2::new(1.0, 0.0),
                Point2::new(2.0, 0.0),
                Point2::new(3.0, 0.0),
            ]),
            10.0,
            None,
        )?),
        "self-intersecting" => Ok(extrude_profile(
            &Profile2D::new(vec![
                Point2::new(0.0, 0.0),
                Point2::new(10.0, 10.0),
                Point2::new(0.0, 10.0),
                Point2::new(10.0, 0.0),
            ]),
            10.0,
            None,
        )?),
        "empty" => Ok(extrude_profile(&Profile2D::new(Vec::new()), 10.0, None)?),
        _ => Err("unknown hostile case".into()),
    }
}

/// Execute one bounded-size probe. The Python supervisor bounds process time and memory.
pub fn run(mode: &str) -> ProbeResult<()> {
    match mode {
        "suite" => {
            let p = plate(1000.0, 500.0, 10.0)?;
            show("plate-mm", &p)?;
            let m = member()?;
            show("member-mm", &m)?;
            show("plate-metres", &plate(1.0, 0.5, 0.01)?)?;
            println!(
                "repeat-plate={} repeat-member={}",
                bytes(&p) == bytes(&plate(1000.0, 500.0, 10.0)?),
                bytes(&m) == bytes(&member()?)
            );
            let profile = rectangle(1000.0, 500.0);
            println!("rectangle-triangulation={:?}", profile.triangulate()?);
            let mut hole = rectangle(1000.0, 500.0);
            hole.add_hole(vec![
                Point2::new(100.0, 100.0),
                Point2::new(100.0, 200.0),
                Point2::new(300.0, 200.0),
                Point2::new(300.0, 100.0),
            ]);
            let h = extrude_profile(&hole, 10.0, None)?;
            show("extruded-hole", &h)?;
            println!(
                "repeat-hole={}",
                bytes(&h) == bytes(&extrude_profile(&hole, 10.0, None)?)
            );
            for (name, w, h, d) in [
                ("minimum-mm", 0.001, 0.001, 0.001),
                ("thin-mm", 1000.0, 500.0, 0.001),
                ("maximum-mm", 1e6, 1e6, 1e6),
                ("fractional-mm", 1000.123456789, 500.123456789, 10.123456789),
            ] {
                match plate(w, h, d) {
                    Ok(m) => show(name, &m)?,
                    Err(e) => println!("{name}: error={e}"),
                }
            }
            let translated = extrude_profile(
                &rectangle(1000.0, 500.0),
                0.001,
                Some(Matrix4::new(
                    1.0, 0.0, 0.0, 1e9, 0.0, 1.0, 0.0, 1e9, 0.0, 0.0, 1.0, 1e9, 0.0, 0.0, 0.0, 1.0,
                )),
            )?;
            show("thin-at-large-origin", &translated)?;
            println!("clip-default-epsilon={}", ClippingProcessor::new().epsilon);
        }
        "clip" => {
            let p = plate(1000.0, 500.0, 10.0)?;
            let c = ClippingProcessor::new();
            let plane = Plane::new(Point3::new(500.0, 0.0, 0.0), Vector3::new(1.0, 0.0, 0.0));
            let result = c.clip_mesh(&p, &plane)?;
            show("plane-cut", &result)?;
            println!(
                "repeat-clip={} failures={:?}",
                bytes(&result) == bytes(&c.clip_mesh(&p, &plane)?),
                c.take_failures()
            );
        }
        "boolean" => {
            let p = plate(1000.0, 500.0, 10.0)?;
            let cutter = extrude_profile(
                &rectangle(200.0, 100.0),
                30.0,
                Some(Matrix4::new(
                    1.0, 0.0, 0.0, 100.0, 0.0, 1.0, 0.0, 100.0, 0.0, 0.0, 1.0, -10.0, 0.0, 0.0,
                    0.0, 1.0,
                )),
            )?;
            let c = ClippingProcessor::new();
            let result = c.subtract_mesh(&p, &cutter);
            println!(
                "high-level-group={}",
                match &result {
                    ifc_lite_geometry::GroupCut::Cut(_) => "Cut",
                    ifc_lite_geometry::GroupCut::Retessellated(_) => "Retessellated",
                    ifc_lite_geometry::GroupCut::Rejected(_) => "Rejected",
                }
            );
            if let Some(m) = result.into_mesh() {
                show("high-level-hole", &m)?;
            }
            println!("high-level-failures={:?}", c.take_failures());
            let m = ifc_lite_geometry::kernel::mesh_bridge::subtract(&p, &cutter);
            show("kernel-hole", &m)?;
            println!(
                "repeat-kernel-hole={}",
                bytes(&m)
                    == bytes(&ifc_lite_geometry::kernel::mesh_bridge::subtract(
                        &p, &cutter
                    ))
            );
        }
        "dump" => {
            use std::io::Write;
            let p = plate(1000.0, 500.0, 10.0)?;
            let m = member()?;
            let mut stream = std::io::stdout().lock();
            for mesh in [p, m] {
                let b = bytes(&mesh);
                stream.write_all(&u64::try_from(b.len())?.to_le_bytes())?;
                stream.write_all(&b)?;
            }
        }
        "bench" => {
            for name in ["plates", "members"] {
                let begin = Instant::now();
                let mut vertices = 0;
                let mut triangles = 0;
                for _ in 0..1000 {
                    let m = if name == "plates" {
                        plate(1000.0, 500.0, 10.0)?
                    } else {
                        member()?
                    };
                    vertices += m.vertex_count();
                    triangles += m.triangle_count();
                    black_box(m);
                }
                println!(
                    "bench-{name}: count=1000 elapsed-ns={} vertices={vertices} triangles={triangles}",
                    begin.elapsed().as_nanos()
                );
            }
        }
        name => match hostile(name) {
            Ok(m) => show(name, &m)?,
            Err(e) => println!("{name}: error={e:?}; display={e}"),
        },
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_plate_and_member_analytic_crosscheck() -> ProbeResult<()> {
        for (mesh, expected) in [
            (plate(1000.0, 500.0, 10.0)?, 5_000_000.0),
            (member()?, 240_000_000.0),
        ] {
            let m = metrics(&mesh)?;
            assert_eq!(m.triangles, 12);
            assert_eq!(m.unique_positions, 8);
            assert_eq!(m.geometric_euler, 2);
            assert!(m.geometric_closed);
            assert_eq!(m.geometric_components, 1);
            assert_eq!(m.nonzero_triangles, 12);
            assert_eq!(m.outward_triangles, 12);
            assert_eq!(m.signed_volume, expected);
        }
        Ok(())
    }
    #[test]
    fn repeat_buffers_in_one_build() -> ProbeResult<()> {
        assert_eq!(
            bytes(&plate(1000.0, 500.0, 10.0)?),
            bytes(&plate(1000.0, 500.0, 10.0)?)
        );
        assert_eq!(bytes(&member()?), bytes(&member()?));
        Ok(())
    }
}
