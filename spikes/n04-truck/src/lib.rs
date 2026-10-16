// SPDX-License-Identifier: MPL-2.0
//! Original bounded synthetic experiments through public truck APIs only.
//! Isolated research harness, not a BeamCraft kernel adapter.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::result::Result;
use truck_meshalgo::prelude::*;
use truck_modeling::{builder, *};

/// Fixed original case list, including rectangular G03 references.
pub const CASES: &[&str] = &[
    "plate",
    "g03-unit",
    "g03-unequal",
    "g03-dyadic",
    "g03-decimal",
    "thin-slab",
    "minimum-cube",
    "scaled-minimum",
    "scaled-below",
    "large-scale",
    "large-offset",
    "large-offset-thin",
    "hole",
    "pattern",
    "touching-hole",
    "near-hole",
    "near-hole-outside-band",
    "crossing-hole",
    "thin-wall",
    "contour",
    "plate-cut-0",
    "plate-cut-30",
    "plate-cut-45",
    "plate-cut-60",
    "plate-cut-vertex",
    "plate-cut-coincident",
    "box-cut-0",
    "box-cut-30",
    "box-cut-45",
    "box-cut-60",
    "box-cut-vertex",
    "box-cut-coincident",
    "hole-tol-small",
    "hole-tol-large",
    "hole-tol-scaled",
    "hole-fine-mesh",
    "tess-tol-below",
    "tess-tol-equal",
    "nonunit-axis",
    "plate-cut-consistency",
    "hole-consistency",
    "hole-coarse",
    "hole-medium-mesh",
    "hole-robust-mesh",
];

/// Experimental parameters; none changes BeamCraft policy.
#[derive(Clone, Debug)]
pub struct Case {
    /// Stable case name.
    pub name: &'static str,
    /// Cartesian dimensions, mm.
    pub dimensions: [f64; 3],
    /// Translation on each axis, mm.
    pub offset: f64,
    /// Analytic expected volume, mm3.
    pub expected_volume: f64,
    /// Expected manifold boundary Euler characteristic.
    pub expected_euler: i64,
    /// Exact B-rep V/E/F counts where construction fixes them.
    pub expected_counts: Option<[usize; 3]>,
    /// Boolean approximation parameter.
    pub boolean_tolerance: f64,
    /// Tessellation approximation parameter.
    pub mesh_tolerance: f64,
}

impl Case {
    /// F06 section 4 local-size floor; excludes translation and thickness.
    pub fn eps_geom(&self) -> f64 {
        let [a, b, _] = self.dimensions;
        0.001_f64.max(((a * a + b * b) + 0.0).sqrt() / 4_194_304.0)
    }

    /// Select a member of the fixed case list.
    pub fn named(name: &str) -> Option<Self> {
        let &name = CASES.iter().find(|&&item| item == name)?;
        let mut case = Self {
            name,
            dimensions: [100.0, 80.0, 10.0],
            offset: 0.0,
            expected_volume: 80_000.0,
            expected_euler: 2,
            expected_counts: Some([8, 12, 6]),
            boolean_tolerance: 0.001,
            mesh_tolerance: 0.01,
        };
        match name {
            "g03-unit" => case.dimensions = [1.0; 3],
            "g03-unequal" => case.dimensions = [23.0, 47.0, 3.0],
            "g03-dyadic" => case.dimensions = [0.125, 0.25, 0.0625],
            "g03-decimal" => case.dimensions = [0.1, 0.2, 0.3],
            "thin-slab" => case.dimensions = [100.0, 80.0, 0.001],
            "minimum-cube" => case.dimensions = [0.001; 3],
            "scaled-minimum" => case.dimensions = [3072.0, 4096.0, 5.0 / 4096.0],
            "scaled-below" => case.dimensions = [3072.0, 4096.0, 0.001],
            "large-scale" => case.dimensions = [300_000.0, 400_000.0, 500_000.0 / 4_194_304.0],
            "large-offset" => case.offset = 1e6,
            "large-offset-thin" => {
                case.offset = 1e6;
                case.dimensions = [100.0, 80.0, 0.001];
            }
            _ => {}
        }
        let [a, b, c] = case.dimensions;
        case.expected_volume = (a * b) * c;
        if name.contains("hole") || name == "pattern" || name == "nonunit-axis" {
            case.expected_counts = None;
            let count = if name == "pattern" { 4.0 } else { 1.0 };
            case.expected_volume -= count * std::f64::consts::PI * 25.0 * c;
            case.expected_euler = if name == "pattern" { -6 } else { 0 };
            if name == "crossing-hole" {
                case.expected_volume = a * b * c - std::f64::consts::PI * 25.0 * c / 2.0;
                case.expected_euler = 2;
            }
            if name == "hole-tol-small" {
                case.boolean_tolerance = 0.00001;
            }
            if name == "hole-tol-large" {
                case.boolean_tolerance = 0.01;
            }
            if name == "hole-consistency" {
                case.boolean_tolerance = 0.01;
            }
            if name == "hole-coarse" {
                case.boolean_tolerance = 0.1;
            }
            if name == "hole-tol-scaled" {
                case.boolean_tolerance = case.eps_geom();
            }
            if name == "hole-fine-mesh" {
                case.mesh_tolerance = 0.0001;
            }
            if name == "hole-medium-mesh" || name == "hole-robust-mesh" {
                case.mesh_tolerance = 0.001;
            }
        }
        if name == "thin-wall" {
            case.expected_volume = (100.0 * 80.0 - 99.998 * 79.998) * 10.0;
            case.expected_euler = 0;
            case.expected_counts = None;
        }
        if name == "contour" {
            case.expected_volume = (100.0 * 30.0 + 30.0 * 50.0) * 10.0;
            case.expected_counts = Some([12, 18, 8]);
        }
        if name.contains("cut") {
            case.expected_counts = None;
            if name.starts_with("box") {
                case.dimensions = [10.0; 3];
            }
            let [a, b, c] = case.dimensions;
            case.expected_volume = if name.ends_with("vertex") {
                1000.0 / 6.0
            } else if name.ends_with("coincident") {
                (a * b) * c
            } else {
                (a * b) * c / 2.0
            };
        }
        if name == "tess-tol-below" {
            case.mesh_tolerance = TOLERANCE / 2.0;
        }
        if name == "tess-tol-equal" {
            case.mesh_tolerance = TOLERANCE;
        }
        Some(case)
    }
}

fn rectangle(dim: [f64; 3], origin: Point3) -> Solid {
    let [a, b, c] = dim;
    let start = builder::vertex(origin);
    let edge = builder::tsweep(&start, Vector3::new(a, 0.0, 0.0));
    let face = builder::tsweep(&edge, Vector3::new(0.0, b, 0.0));
    builder::tsweep(&face, Vector3::new(0.0, 0.0, c))
}

fn polygon(points: &[[f64; 2]]) -> Result<Face, String> {
    if !(3..=16).contains(&points.len()) {
        return Err("contour cap".into());
    }
    let vertices: Vec<_> = points
        .iter()
        .map(|&[x, y]| builder::vertex(Point3::new(x, y, 0.0)))
        .collect();
    let edges = vertices
        .iter()
        .zip(vertices.iter().cycle().skip(1))
        .take(vertices.len())
        .map(|(a, b)| builder::line(a, b))
        .collect::<Vec<_>>();
    builder::try_attach_plane(&[Wire::from(edges)]).map_err(|e| format!("plane: {e}"))
}

fn cylinder(cx: f64, cy: f64, nonunit: bool) -> Result<Solid, String> {
    let origin = Point3::new(cx, cy, -10.0);
    let vertex = builder::vertex(origin + Vector3::new(5.0, 0.0, 0.0));
    let axis = Vector3::new(0.0, 0.0, if nonunit { 2.0 } else { 1.0 });
    let wire: Wire = builder::rsweep(&vertex, origin, axis, Rad(std::f64::consts::TAU));
    let face = builder::try_attach_plane(&[wire]).map_err(|e| format!("disk: {e}"))?;
    Ok(builder::tsweep(&face, Vector3::new(0.0, 0.0, 30.0)))
}

fn halfspace(case: &Case) -> Result<Solid, String> {
    let [a, b, c] = case.dimensions;
    let (point, normal) = if case.name.ends_with("vertex") {
        (
            Point3::new(10.0, 0.0, 0.0),
            Vector3::new(1.0, 1.0, 1.0).normalize(),
        )
    } else if case.name.ends_with("coincident") {
        (Point3::new(a, b / 2.0, c / 2.0), Vector3::unit_x())
    } else {
        let degrees: f64 = if case.name.ends_with("consistency") {
            30.0
        } else {
            case.name
                .rsplit('-')
                .next()
                .ok_or("angle")?
                .parse()
                .map_err(|_| "angle")?
        };
        let angle = degrees.to_radians();
        (
            Point3::new(a / 2.0, b / 2.0, c / 2.0),
            Vector3::new(angle.cos(), angle.sin(), 0.0),
        )
    };
    let u = normal.cross(Vector3::unit_z()).normalize();
    let v = normal.cross(u);
    // Other five cutter faces are >=1000 mm from the plane centre.
    // Specimen corners are <150 mm away; only the intended plane clips.
    let r = 1000.0;
    let corners = [
        point - u * r - v * r,
        point + u * r - v * r,
        point + u * r + v * r,
        point - u * r + v * r,
    ];
    let vertices: Vec<_> = corners.iter().map(|p| builder::vertex(*p)).collect();
    let edges: Vec<_> = vertices
        .iter()
        .zip(vertices.iter().cycle().skip(1))
        .take(4)
        .map(|(x, y)| builder::line(x, y))
        .collect();
    let mut face =
        builder::try_attach_plane(&[Wire::from(edges)]).map_err(|e| format!("cutter: {e}"))?;
    face.invert();
    Ok(builder::tsweep(&face, -normal * (2.0 * r)))
}

/// Successful measurements and raw byte evidence.
#[derive(Debug)]
pub struct Evaluation {
    /// TSV metrics, not production admission.
    pub row: String,
    /// Little-endian f64 positions followed by triangle position indices (u64).
    pub mesh_bytes: Vec<u8>,
    /// Triangle coordinates in emitted order (nine f64 per triangle).
    pub triangle_bytes: Vec<u8>,
}

/// Evaluate one bounded case; isolate the call for research robustness probes.
pub fn evaluate(case: &Case) -> Result<Evaluation, String> {
    eprintln!("stage=build");
    let mut solid = rectangle(
        case.dimensions,
        Point3::new(case.offset, case.offset, case.offset),
    );
    let inputs: HashSet<_> = solid.face_iter().map(|f| f.id()).collect();
    if case.name == "contour" {
        let face = polygon(&[
            [0.0, 0.0],
            [100.0, 0.0],
            [100.0, 30.0],
            [30.0, 30.0],
            [30.0, 80.0],
            [0.0, 80.0],
        ])?;
        solid = builder::tsweep(&face, Vector3::new(0.0, 0.0, 10.0));
    } else if case.name == "thin-wall" {
        let mut tool = rectangle([99.998, 79.998, 30.0], Point3::new(0.001, 0.001, -10.0));
        tool.not();
        solid = truck_shapeops::and(&solid, &tool, case.boolean_tolerance).ok_or("boolean None")?;
    } else if case.name.contains("hole") || case.name == "pattern" || case.name == "nonunit-axis" {
        let centres = if case.name == "pattern" {
            vec![(25.0, 20.0), (75.0, 20.0), (25.0, 60.0), (75.0, 60.0)]
        } else {
            vec![(
                match case.name {
                    "touching-hole" => 5.0,
                    "near-hole" => 5.0005,
                    "near-hole-outside-band" => 5.002,
                    "crossing-hole" => 0.0,
                    _ => 50.0,
                },
                40.0,
            )]
        };
        for (x, y) in centres {
            let mut tool = cylinder(x, y, case.name == "nonunit-axis")?;
            tool.not();
            eprintln!("stage=boolean");
            solid =
                truck_shapeops::and(&solid, &tool, case.boolean_tolerance).ok_or("boolean None")?;
        }
    } else if case.name.contains("cut") {
        eprintln!("stage=boolean");
        solid = truck_shapeops::and(&solid, &halfspace(case)?, case.boolean_tolerance)
            .ok_or("boolean None")?;
    }
    measure(case, &solid, &inputs)
}

fn measure(case: &Case, solid: &Solid, inputs: &HashSet<FaceID>) -> Result<Evaluation, String> {
    eprintln!("stage=brep-checks");
    let nv = solid
        .vertex_iter()
        .map(|v| v.id())
        .collect::<HashSet<_>>()
        .len();
    let ne = solid
        .edge_iter()
        .map(|e| e.id())
        .collect::<HashSet<_>>()
        .len();
    let nf = solid.face_iter().count();
    let mut face_chi = 0_i64;
    let mut incidence = HashMap::<EdgeID, (usize, i64)>::new();
    for face in solid.face_iter() {
        // A trimmed face with b boundary loops has chi=2-b, not always 1.
        face_chi += 2 - i64::try_from(face.boundaries().len()).map_err(|_| "face cap")?;
        for wire in face.boundaries() {
            for edge in wire.edge_iter() {
                let value = incidence.entry(edge.id()).or_default();
                value.0 += 1;
                value.1 += if edge.orientation() { 1 } else { -1 };
            }
        }
    }
    let brep_euler = i64::try_from(nv).map_err(|_| "vertex cap")?
        - i64::try_from(ne).map_err(|_| "edge cap")?
        + face_chi;
    let brep_closed = !incidence.is_empty()
        && incidence.values().all(|&(n, b)| n == 2 && b == 0)
        && solid
            .boundaries()
            .iter()
            .all(|s| s.shell_condition() == ShellCondition::Closed && s.is_connected());
    // The convenience consistency API is an additional independent probe: it can
    // panic for an IntersectionCurve even when boolean construction returned.
    let geometric_consistent =
        if case.name.ends_with("consistency") || case.expected_counts.is_some() {
            solid.is_geometric_consistent().to_string()
        } else {
            "not_run".to_owned()
        };
    let retained_faces = solid
        .face_iter()
        .filter(|f| inputs.contains(&f.id()))
        .count();
    eprintln!("stage=tessellation");
    let meshed = if case.name == "hole-robust-mesh" {
        solid.robust_triangulation(case.mesh_tolerance)
    } else {
        solid.triangulation(case.mesh_tolerance)
    };
    let missing_surfaces = meshed.face_iter().filter(|f| f.surface().is_none()).count();
    let mesh = meshed.to_polygon();
    eprintln!("stage=mesh-audit");
    if mesh.positions().len() > 200_000 || mesh.tri_faces().len() > 400_000 {
        return Err("audit mesh cap".into());
    }
    if !mesh.quad_faces().is_empty() || !mesh.other_faces().is_empty() {
        return Err("nontriangular tessellation".into());
    }
    let positions = mesh.positions();
    let anchor = positions.first().copied().ok_or("empty tessellation")?;
    let mut low = [f64::INFINITY; 3];
    let mut high = [f64::NEG_INFINITY; 3];
    let mut mesh_bytes = Vec::new();
    let mut aliases = BTreeMap::new();
    let mut geometric_ids = Vec::with_capacity(positions.len());
    for p in positions {
        for ((min, max), x) in low.iter_mut().zip(high.iter_mut()).zip([p.x, p.y, p.z]) {
            if !x.is_finite() {
                return Err("nonfinite tessellation".into());
            }
            *min = min.min(x);
            *max = max.max(x);
            mesh_bytes.extend_from_slice(&x.to_le_bytes());
        }
        let key = [p.x, p.y, p.z].map(|x| if x == 0.0 { 0 } else { x.to_bits() });
        let next = aliases.len();
        let id = *aliases.entry(key).or_insert(next);
        geometric_ids.push(id);
    }
    let mut raw_edges = BTreeMap::new();
    let mut geom_edges = BTreeMap::new();
    let mut volume = 0.0;
    let mut correction = 0.0;
    let mut degenerate = 0_usize;
    let mut triangle_bytes = Vec::new();
    for triangle in mesh.tri_faces() {
        let [v0, v1, v2] = triangle;
        let ids = [v0.pos, v1.pos, v2.pos];
        let mut pts = Vec::with_capacity(3);
        let mut gids = Vec::with_capacity(3);
        for id in ids {
            let p = positions.as_slice().get(id).ok_or("bad position index")?;
            pts.push(*p - anchor);
            gids.push(*geometric_ids.as_slice().get(id).ok_or("bad alias index")?);
            mesh_bytes
                .extend_from_slice(&u64::try_from(id).map_err(|_| "index width")?.to_le_bytes());
            for x in [p.x, p.y, p.z] {
                triangle_bytes.extend_from_slice(&x.to_le_bytes());
            }
        }
        let [a, b, c] = pts.as_slice() else {
            return Err("triangle arity".into());
        };
        if (*b - *a).cross(*c - *a).magnitude2() == 0.0 {
            degenerate += 1;
        }
        // Divergence theorem: signed tetrahedra at a local anchor, compensated sum.
        let value = a.dot(b.cross(*c)) / 6.0;
        let y = value - correction;
        let t = volume + y;
        correction = (t - volume) - y;
        volume = t;
        add_edges(&mut raw_edges, &ids);
        let [ga, gb, gc] = gids.as_slice() else {
            return Err("triangle aliases".into());
        };
        add_edges(&mut geom_edges, &[*ga, *gb, *gc]);
    }
    let raw_closed = closed(&raw_edges);
    let geom_closed = closed(&geom_edges);
    let triangles = mesh.tri_faces().len();
    let geom_euler = i64::try_from(aliases.len()).map_err(|_| "count")?
        - i64::try_from(geom_edges.len()).map_err(|_| "count")?
        + i64::try_from(triangles).map_err(|_| "count")?;
    let rel_error = (volume - case.expected_volume).abs() / case.expected_volume.abs();
    let exact_counts = case.expected_counts.is_none_or(|v| v == [nv, ne, nf]);
    let geometry_ok = rel_error <= 1e-6
        && brep_closed
        && brep_euler == case.expected_euler
        && exact_counts
        && missing_surfaces == 0
        && degenerate == 0;
    let caps_ok = positions.len() <= 4096
        && triangles <= 8192
        && nf <= 1024
        && ne <= 12_288
        && solid.boundaries().len() <= 16;
    let [a, b, c] = case.dimensions;
    let expected_high = if case.name.ends_with("vertex") {
        [10.0; 3]
    } else if case.name.contains("cut") && !case.name.ends_with("coincident") {
        let degrees: f64 = if case.name.ends_with("consistency") {
            30.0
        } else {
            case.name
                .rsplit('-')
                .next()
                .ok_or("angle")?
                .parse()
                .map_err(|_| "angle")?
        };
        let t = degrees.to_radians();
        // n.x*x+n.y*y <= n.x*a/2+n.y*b/2. Positive coefficients.
        // Axis extrema occur at y=0 and x=0 respectively.
        let d = t.cos() * a / 2.0 + t.sin() * b / 2.0;
        [
            (d / t.cos()).min(a),
            if degrees == 0.0 {
                b
            } else {
                (d / t.sin()).min(b)
            },
            c,
        ]
    } else {
        [case.offset + a, case.offset + b, case.offset + c]
    };
    let bounds_ok = low
        .iter()
        .zip([case.offset; 3])
        .chain(high.iter().zip(expected_high))
        .all(|(actual, expected)| (*actual - expected).abs() <= 0.001 + 1e-6 * expected.abs());
    let status = if geometry_ok && bounds_ok {
        "PASS"
    } else {
        "WRONG"
    };
    let row = format!(
        "{}\t{}\t{:.17e}\t{:.17e}\t{:.9e}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{:?}\t{:?}\t{:.9e}\t{:.9e}\t{:.9e}\t{}\t{}\t{}\t{}",
        case.name,
        status,
        case.expected_volume,
        volume,
        rel_error,
        nv,
        ne,
        nf,
        brep_euler,
        brep_closed,
        geometric_consistent,
        solid.boundaries().len(),
        positions.len(),
        triangles,
        raw_closed,
        aliases.len(),
        geom_euler,
        geom_closed,
        retained_faces,
        low,
        high,
        case.eps_geom(),
        case.boolean_tolerance,
        case.mesh_tolerance,
        missing_surfaces,
        degenerate,
        caps_ok,
        bounds_ok
    );
    Ok(Evaluation {
        row,
        mesh_bytes,
        triangle_bytes,
    })
}

fn add_edges(map: &mut BTreeMap<(usize, usize), (usize, i64)>, ids: &[usize; 3]) {
    let [a, b, c] = *ids;
    for (x, y) in [(a, b), (b, c), (c, a)] {
        let item = map.entry((x.min(y), x.max(y))).or_default();
        item.0 += 1;
        item.1 += if x < y { 1 } else { -1 };
    }
}

fn closed(map: &BTreeMap<(usize, usize), (usize, i64)>) -> bool {
    !map.is_empty()
        && map
            .iter()
            .all(|((a, b), (n, balance))| a != b && *n == 2 && *balance == 0)
}

/// Columns for successful calls. Error/panic records are shorter.
pub const HEADER: &str = "case\tstatus\texpected_volume\tmesh_volume\trel_error\tbrep_v\tbrep_e\tbrep_f\tbrep_chi\tbrep_closed\tgeometric_consistent\tshells\tmesh_v\ttriangles\traw_closed\tunique_positions\tposition_chi\tposition_closed\tretained_input_face_ids\tmin\tmax\teps_geom\tbool_tol\tmesh_tol\tmissing_surfaces\tdegenerate\tf06_caps\tbounds_ok";
