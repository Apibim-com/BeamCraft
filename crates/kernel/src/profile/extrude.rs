// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! Straight extrusion of an admitted profile into a sealed, closed, canonical mesh.
//!
//! The solid is the profile extruded from `z = 0` to `z = thickness` (F04 P-1: the contour is
//! the bottom face). Vertex order: the bottom copy of every contour point (outer first, then
//! each cavity in input order), then the top copy. Face order: bottom cap, top cap, one wall
//! per outer edge, then the walls of each cavity. A polygonal cavity has one planar wall per
//! edge; a curve-approximation cavity has one wall face for the whole cavity, so a later hole
//! feature finds all its walls under one face id. Triangles are grouped by face in face order.
//!
//! # Local feature ids (provisional)
//!
//! Local ids use the F06 section 5 character set and length bound but the kind `extrusion`,
//! because F06 0.1.0 defines `fid1` roles only for the plate and member boxes and forbids
//! inventing box roles for other solids. A contract amendment must adopt this grammar (or a
//! successor) before an engine may qualify and persist these ids:
//!
//! * faces `f.z0` (bottom), `f.z1` (top), `f.o<k>` (outer edge k), `f.c<j>.s<k>` (edge k of
//!   polygonal cavity j) or `f.c<j>` (all walls of curve-approximation cavity j);
//! * edges `e.z0.<t>` / `e.z1.<t>` (bottom/top boundary edge from point k to k+1 of contour `t`)
//!   and `e.v.<t>` (vertical edge at point k);
//! * vertices `v.z0.<t>` / `v.z1.<t>`;
//!
//! where the contour token `<t>` is `o<k>` for the outer contour or `c<j>.<k>` for cavity j.
//! Indices follow the authored contour order, never mesh order, so ids survive edits that keep
//! the topology; a cyclic start change moves the roles (as for plate roles, F06 section 5
//! rule 9).

use super::diagnostic::{ProfileCode, ProfileFailure, ProfileField, Stage};
use super::exact::orient2d;
use super::{Cavity, Point2, Profile, admit, shoelace};
use beamcraft_geom::tolerance::{DIMENSION_CAP_MM, Tolerance};
use beamcraft_geom::{Point3, Vec3};
use beamcraft_model::digest::{ContentDigest, Sha256};
use core::cmp::Ordering;
use core::ops::Range;

const MAX_VERTICES: usize = 4096;
const MAX_TRIANGLES: usize = 8192;
const MAX_FACES: usize = 1024;
const MAX_EDGES: usize = 12_288;
const MAX_FEATURES: usize = 17_408;
const MAX_BLOB_BYTES: usize = 213_024;
/// Relative bound of the numeric volume certificate (a build certificate, not CR-TOL-001).
const VOLUME_CERTIFICATE: f64 = 1e-9;

/// Whether the analytic volume is that of the solid exactly as submitted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VolumeStatus {
    /// Every contour is an authored polygon: the volume is that of the submitted solid.
    Exact,
    /// A cavity approximates a curve: the volume is that of the polygonal solid, and the true
    /// curved solid differs by at most [`Extrusion::volume_error_bound_mm3`].
    Approximated,
}

/// Semantic role of a face of the extruded mesh.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FaceRole {
    /// The bottom cap at `z = 0`, normal `-Z`.
    BottomCap,
    /// The top cap at `z = thickness`, normal `+Z`.
    TopCap,
    /// The wall over one outer contour edge.
    OuterWall {
        /// Outer edge index, from point `edge` to `edge + 1`.
        edge: u32,
    },
    /// A wall of a cavity.
    CavityWall {
        /// Cavity index in input order.
        cavity: u32,
        /// Edge index for a polygonal cavity's planar wall; `None` for a curved wall that
        /// spans every edge of a curve-approximation cavity.
        edge: Option<u32>,
    },
}

/// Sealed extruded mesh; every index is snapshot-local, never a feature address.
#[derive(Debug, Clone, PartialEq)]
pub struct ExtrusionMesh {
    vertices: Vec<Point3>,
    triangles: Vec<[u32; 3]>,
    triangle_faces: Vec<u16>,
    roles: Vec<FaceRole>,
    face_ids: Vec<String>,
    normals: Vec<Option<Vec3>>,
    edge_ids: Vec<String>,
    edge_vertices: Vec<[u32; 2]>,
    vertex_ids: Vec<String>,
    cavity_faces: Vec<Range<usize>>,
    blob: Vec<u8>,
    digest: ContentDigest,
}

impl ExtrusionMesh {
    /// Vertices: all bottom points in contour order, then all top points.
    pub fn vertices(&self) -> &[Point3] {
        &self.vertices
    }
    /// Counter-clockwise (seen from outside) triangles, grouped by face in face order.
    pub fn triangles(&self) -> &[[u32; 3]] {
        &self.triangles
    }
    /// Per-triangle index into [`Self::face_table`].
    pub fn triangle_faces(&self) -> &[u16] {
        &self.triangle_faces
    }
    /// Local face ids in canonical order.
    pub fn face_table(&self) -> &[String] {
        &self.face_ids
    }
    /// Role of each face, in face-table order.
    pub fn face_roles(&self) -> &[FaceRole] {
        &self.roles
    }
    /// Outward unit normal per face; `None` for a curved cavity wall that has no single normal.
    pub fn face_normals(&self) -> &[Option<Vec3>] {
        &self.normals
    }
    /// Local edge ids: bottom edges, then top edges, then vertical edges.
    pub fn edge_table(&self) -> &[String] {
        &self.edge_ids
    }
    /// Vertex index pair of each edge in [`Self::edge_table`] order.
    pub fn edge_vertices(&self) -> &[[u32; 2]] {
        &self.edge_vertices
    }
    /// Local vertex ids in mesh vertex order.
    pub fn vertex_table(&self) -> &[String] {
        &self.vertex_ids
    }
    /// Face-table index range of the walls of cavity `cavity`, for hole features.
    pub fn cavity_walls(&self, cavity: usize) -> Option<Range<usize>> {
        self.cavity_faces.get(cavity).cloned()
    }
    /// Explicit USD-compatible subdivision scheme; never rely on a default.
    pub const fn subdivision_scheme(&self) -> &'static str {
        "none"
    }
    /// Front faces have counter-clockwise winding seen from outside.
    pub const fn orientation(&self) -> &'static str {
        "rightHanded"
    }
    /// The BCMESH01 element-local blob (layout 1, little-endian).
    pub fn blob(&self) -> &[u8] {
        &self.blob
    }
    /// SHA-256 of the exact blob.
    pub const fn mesh_digest(&self) -> ContentDigest {
        self.digest
    }
}

/// Successful extrusion, available only after every self-check.
#[derive(Debug, Clone, PartialEq)]
pub struct Extrusion {
    mesh: ExtrusionMesh,
    thickness_mm: f64,
    net_area_mm2: f64,
    volume_mm3: f64,
    status: VolumeStatus,
    error_bound_mm3: f64,
    tolerance: Tolerance,
}

impl Extrusion {
    /// The sealed closed mesh.
    pub const fn mesh(&self) -> &ExtrusionMesh {
        &self.mesh
    }
    /// Extrusion thickness in millimetres.
    pub const fn thickness_mm(&self) -> f64 {
        self.thickness_mm
    }
    /// Net cap area (outer minus cavities) in mm2.
    pub const fn net_area_mm2(&self) -> f64 {
        self.net_area_mm2
    }
    /// Analytic volume `net_area * thickness` of the polygonal solid in mm3.
    pub const fn volume_mm3(&self) -> f64 {
        self.volume_mm3
    }
    /// Analytic volume in m3 (division by exactly 1e9).
    pub fn volume_m3(&self) -> f64 {
        self.volume_mm3 / 1_000_000_000.0
    }
    /// Whether the volume is exact for the submitted solid or approximates curved cavities.
    pub const fn volume_status(&self) -> VolumeStatus {
        self.status
    }
    /// Upper bound of the difference to the true curved solid; zero when [`VolumeStatus::Exact`].
    pub const fn volume_error_bound_mm3(&self) -> f64 {
        self.error_bound_mm3
    }
    /// Local-size policy used for admission.
    pub const fn tolerance(&self) -> Tolerance {
        self.tolerance
    }
}

/// Extrudes a profile along +Z by `thickness_mm`, validating every input.
pub fn extrude(
    outer: &[Point2],
    cavities: &[Cavity],
    thickness_mm: f64,
) -> Result<Extrusion, ProfileFailure> {
    extrude_with_cancel(outer, cavities, thickness_mm, |_| false)
}

/// Extrudes with the F06 section 10 cancellation points P0 through P3.
///
/// The callback sees Validation, Build, Checks, Quantities in that order and returns true to
/// discard the candidate. There is no poll after sealing. The caller owns callback safety.
pub fn extrude_with_cancel(
    outer: &[Point2],
    cavities: &[Cavity],
    thickness_mm: f64,
    mut cancelled: impl FnMut(Stage) -> bool,
) -> Result<Extrusion, ProfileFailure> {
    poll(&mut cancelled, Stage::Validation)?;
    let profile = admit::admit(outer, cavities, Some(thickness_mm))?;
    run(&profile, thickness_mm, &mut cancelled)
}

/// Extrudes an already admitted profile; the thickness is validated against its tolerance.
pub fn extrude_profile(
    profile: &Profile,
    thickness_mm: f64,
    mut cancelled: impl FnMut(Stage) -> bool,
) -> Result<Extrusion, ProfileFailure> {
    poll(&mut cancelled, Stage::Validation)?;
    let thickness_failure =
        |code, stage| ProfileFailure::global(code, ProfileField::Thickness, stage);
    if !thickness_mm.is_finite() {
        return Err(thickness_failure(ProfileCode::NonFinite, Stage::Finiteness));
    }
    if thickness_mm > DIMENSION_CAP_MM {
        return Err(thickness_failure(ProfileCode::AboveLimit, Stage::Caps));
    }
    if thickness_mm <= 0.0 {
        return Err(thickness_failure(ProfileCode::NotPositive, Stage::Ranges));
    }
    if thickness_mm < profile.tolerance().eps_geom_mm() {
        return Err(thickness_failure(ProfileCode::BelowMinimum, Stage::Ranges));
    }
    run(profile, thickness_mm, &mut cancelled)
}

fn poll(cancelled: &mut impl FnMut(Stage) -> bool, stage: Stage) -> Result<(), ProfileFailure> {
    if cancelled(stage) {
        Err(ProfileFailure::global(
            ProfileCode::Cancelled,
            ProfileField::Cancellation,
            stage,
        ))
    } else {
        Ok(())
    }
}

fn mesh_failure(code: ProfileCode, stage: Stage) -> ProfileFailure {
    ProfileFailure::global(code, ProfileField::Mesh, stage)
}

fn numeric(stage: Stage) -> ProfileFailure {
    mesh_failure(ProfileCode::NumericUncertain, stage)
}

// The analytic volume is trusted only if the independently summed mesh volume agrees within a
// relative bound of the magnitude of the terms being cancelled (a build certificate; the
// CR-TOL-001 test comparison rule is never read by the kernel, F06 section 4 rule 7).
fn volume_certified(certificate: &Certificate, volume: f64, thickness: f64) -> bool {
    let scale = certificate.mesh_magnitude + certificate.analytic_magnitude * thickness;
    volume.is_finite()
        && volume > 0.0
        && (certificate.mesh_volume - volume).abs() <= VOLUME_CERTIFICATE * scale
}

fn run(
    profile: &Profile,
    thickness: f64,
    cancelled: &mut impl FnMut(Stage) -> bool,
) -> Result<Extrusion, ProfileFailure> {
    poll(cancelled, Stage::Build)?;
    check_limits(profile)?;
    let triangles = profile.triangulate()?;
    let built = build(profile, thickness, &triangles)?;
    poll(cancelled, Stage::Checks)?;
    let certificate = self_check(&built, profile)?;
    poll(cancelled, Stage::Quantities)?;
    let net_area = profile.net_area_mm2();
    let volume = net_area * thickness;
    if !volume_certified(&certificate, volume, thickness) {
        return Err(ProfileFailure::global(
            ProfileCode::NumericUncertain,
            ProfileField::Volume,
            Stage::Quantities,
        ));
    }
    let mut error_bound = 0.0;
    let mut approximated = false;
    for cavity in profile.cavities() {
        if let Some(a) = cavity.approximation() {
            approximated = true;
            let perimeter: f64 = cavity
                .points()
                .iter()
                .zip(cavity.points().iter().cycle().skip(1))
                .map(|(p, q)| ((q.x - p.x) * (q.x - p.x) + (q.y - p.y) * (q.y - p.y)).sqrt())
                .sum();
            error_bound += perimeter * a.sagitta_bound_mm * thickness;
        }
    }
    Ok(Extrusion {
        mesh: built.into_mesh()?,
        thickness_mm: thickness,
        net_area_mm2: net_area,
        volume_mm3: volume,
        status: if approximated {
            VolumeStatus::Approximated
        } else {
            VolumeStatus::Exact
        },
        error_bound_mm3: error_bound,
        tolerance: profile.tolerance(),
    })
}

fn face_count(profile: &Profile) -> usize {
    let walls: usize = profile
        .cavities()
        .iter()
        .map(|c| {
            if c.approximation().is_some() {
                1
            } else {
                c.points().len()
            }
        })
        .sum();
    2 + profile.outer().len() + walls
}

// Derived counts are compared with the F06 section 6 hard caps before any mesh allocation.
fn check_limits(profile: &Profile) -> Result<(), ProfileFailure> {
    let overflow = || mesh_failure(ProfileCode::BufferLimit, Stage::Build);
    let n = profile.point_count();
    let h = profile.cavities().len();
    let vertices = n.checked_mul(2).ok_or_else(overflow)?;
    let triangles = n
        .checked_mul(4)
        .and_then(|v| v.checked_add(h.checked_mul(4)?))
        .and_then(|v| v.checked_sub(4))
        .ok_or_else(overflow)?;
    let pad8 = |v: usize| v.div_ceil(8) * 8;
    let blob = 32 + 24 * vertices + pad8(12 * triangles) + pad8(2 * triangles);
    if vertices > MAX_VERTICES || triangles > MAX_TRIANGLES || blob > MAX_BLOB_BYTES {
        return Err(overflow());
    }
    let faces = face_count(profile);
    let edges = n.checked_mul(3).ok_or_else(overflow)?;
    if faces > MAX_FACES || edges > MAX_EDGES || vertices + edges + faces > MAX_FEATURES {
        return Err(mesh_failure(ProfileCode::TopologyLimit, Stage::Build));
    }
    Ok(())
}

struct Built {
    vertices: Vec<Point3>,
    triangles: Vec<[u32; 3]>,
    triangle_faces: Vec<u16>,
    roles: Vec<FaceRole>,
    face_ids: Vec<String>,
    normals: Vec<Option<Vec3>>,
    edge_ids: Vec<String>,
    edge_vertices: Vec<[u32; 2]>,
    vertex_ids: Vec<String>,
    cavity_faces: Vec<Range<usize>>,
    point_count: usize,
}

fn contour_token(cavity: Option<usize>, k: usize) -> String {
    match cavity {
        None => format!("o{k}"),
        Some(j) => format!("c{j}.{k}"),
    }
}

fn build(
    profile: &Profile,
    thickness: f64,
    cap_triangles: &[[u32; 3]],
) -> Result<Built, ProfileFailure> {
    let fail = || numeric(Stage::Build);
    let id = |i: usize| u32::try_from(i).map_err(|_| fail());
    let n = profile.point_count();
    let mut flat: Vec<Point2> = Vec::with_capacity(n);
    // (cavity index, start, length, one curved wall)
    let mut spans: Vec<(Option<usize>, usize, usize, bool)> = Vec::new();
    spans.push((None, 0, profile.outer().len(), false));
    flat.extend_from_slice(profile.outer());
    for (j, cavity) in profile.cavities().iter().enumerate() {
        spans.push((
            Some(j),
            flat.len(),
            cavity.points().len(),
            cavity.approximation().is_some(),
        ));
        flat.extend_from_slice(cavity.points());
    }
    let mut vertices: Vec<Point3> = Vec::with_capacity(2 * n);
    // + 0.0 turns -0.0 into +0.0: every public position is canonical, as in the blob (F06 section 2).
    vertices.extend(flat.iter().map(|p| Point3::new(p.x + 0.0, p.y + 0.0, 0.0)));
    vertices.extend(
        flat.iter()
            .map(|p| Point3::new(p.x + 0.0, p.y + 0.0, thickness)),
    );

    let mut triangles: Vec<[u32; 3]> = Vec::with_capacity(cap_triangles.len() * 2 + 2 * n);
    let mut triangle_faces: Vec<u16> = Vec::with_capacity(triangles.capacity());
    let top = id(n)?;
    for [a, b, c] in cap_triangles {
        triangles.push([*a, *c, *b]);
        triangle_faces.push(0);
    }
    for [a, b, c] in cap_triangles {
        triangles.push([top + a, top + b, top + c]);
        triangle_faces.push(1);
    }

    let mut roles = vec![FaceRole::BottomCap, FaceRole::TopCap];
    let mut face_ids = vec!["extrusion:f.z0".to_owned(), "extrusion:f.z1".to_owned()];
    let mut normals: Vec<Option<Vec3>> = vec![
        Some(Vec3::new(0.0, 0.0, -1.0).map_err(|_| fail())?),
        Some(Vec3::new(0.0, 0.0, 1.0).map_err(|_| fail())?),
    ];
    let mut cavity_faces: Vec<Range<usize>> = Vec::with_capacity(profile.cavities().len());
    for &(cavity, start, len, curved) in &spans {
        let first_face = roles.len();
        for k in 0..len {
            let (i, j) = (start + k, start + (k + 1) % len);
            let (p, q) = (
                flat.get(i).copied().ok_or_else(fail)?,
                flat.get(j).copied().ok_or_else(fail)?,
            );
            let new_face = !(curved && k > 0);
            if new_face {
                let (role, face_id) = match cavity {
                    None => (
                        FaceRole::OuterWall { edge: id(k)? },
                        format!("extrusion:f.o{k}"),
                    ),
                    Some(c) if curved => (
                        FaceRole::CavityWall {
                            cavity: id(c)?,
                            edge: None,
                        },
                        format!("extrusion:f.c{c}"),
                    ),
                    Some(c) => (
                        FaceRole::CavityWall {
                            cavity: id(c)?,
                            edge: Some(id(k)?),
                        },
                        format!("extrusion:f.c{c}.s{k}"),
                    ),
                };
                roles.push(role);
                face_ids.push(face_id);
                let normal = if curved {
                    None
                } else {
                    let (dx, dy) = (q.x - p.x, q.y - p.y);
                    let length = (dx * dx + dy * dy).sqrt();
                    Some(Vec3::new(dy / length, -dx / length, 0.0).map_err(|_| fail())?)
                };
                normals.push(normal);
            }
            let face = u16::try_from(roles.len() - 1).map_err(|_| fail())?;
            let (bi, bj) = (id(i)?, id(j)?);
            let (ti, tj) = (top + bi, top + bj);
            triangles.push([bi, bj, tj]);
            triangles.push([bi, tj, ti]);
            triangle_faces.push(face);
            triangle_faces.push(face);
        }
        if cavity.is_some() {
            cavity_faces.push(first_face..roles.len());
        }
    }

    let mut edge_ids: Vec<String> = Vec::with_capacity(3 * n);
    let mut edge_vertices: Vec<[u32; 2]> = Vec::with_capacity(3 * n);
    for (level, label) in [(0usize, "z0"), (n, "z1")] {
        for &(cavity, start, len, _) in &spans {
            for k in 0..len {
                edge_ids.push(format!("extrusion:e.{label}.{}", contour_token(cavity, k)));
                edge_vertices.push([id(level + start + k)?, id(level + start + (k + 1) % len)?]);
            }
        }
    }
    for &(cavity, start, len, _) in &spans {
        for k in 0..len {
            edge_ids.push(format!("extrusion:e.v.{}", contour_token(cavity, k)));
            edge_vertices.push([id(start + k)?, id(n + start + k)?]);
        }
    }
    let mut vertex_ids: Vec<String> = Vec::with_capacity(2 * n);
    for label in ["z0", "z1"] {
        for &(cavity, _, len, _) in &spans {
            for k in 0..len {
                vertex_ids.push(format!("extrusion:v.{label}.{}", contour_token(cavity, k)));
            }
        }
    }
    Ok(Built {
        vertices,
        triangles,
        triangle_faces,
        roles,
        face_ids,
        normals,
        edge_ids,
        edge_vertices,
        vertex_ids,
        cavity_faces,
        point_count: n,
    })
}

impl Built {
    fn into_mesh(self) -> Result<ExtrusionMesh, ProfileFailure> {
        let fail = || mesh_failure(ProfileCode::SelfCheckFailed, Stage::Quantities);
        let blob = encode(
            &self.vertices,
            &self.triangles,
            &self.triangle_faces,
            self.roles.len(),
        )
        .ok_or_else(fail)?;
        let digest = Sha256::digest(&blob).map_err(|_| fail())?;
        Ok(ExtrusionMesh {
            vertices: self.vertices,
            triangles: self.triangles,
            triangle_faces: self.triangle_faces,
            roles: self.roles,
            face_ids: self.face_ids,
            normals: self.normals,
            edge_ids: self.edge_ids,
            edge_vertices: self.edge_vertices,
            vertex_ids: self.vertex_ids,
            cavity_faces: self.cavity_faces,
            blob,
            digest,
        })
    }
}

fn pad8(n: usize) -> usize {
    n.div_ceil(8) * 8
}

/// Encodes the F06 section 6 BCMESH01 layout 1 blob (element-local space), little-endian.
pub(crate) fn encode(
    vertices: &[Point3],
    triangles: &[[u32; 3]],
    faces: &[u16],
    face_count: usize,
) -> Option<Vec<u8>> {
    let (v, t) = (vertices.len(), triangles.len());
    let total = 32usize
        .checked_add(v.checked_mul(24)?)?
        .checked_add(pad8(t.checked_mul(12)?))?
        .checked_add(pad8(t.checked_mul(2)?))?;
    let mut blob: Vec<u8> = Vec::new();
    blob.try_reserve_exact(total).ok()?;
    blob.extend_from_slice(b"BCMESH01");
    for word in [
        1u32,
        u32::try_from(v).ok()?,
        u32::try_from(t).ok()?,
        u32::try_from(face_count).ok()?,
        0,
        0,
    ] {
        blob.extend_from_slice(&word.to_le_bytes());
    }
    for p in vertices {
        for value in [p.x, p.y, p.z] {
            let value = if value == 0.0 { 0.0 } else { value };
            blob.extend_from_slice(&value.to_le_bytes());
        }
    }
    for index in triangles.iter().flatten() {
        blob.extend_from_slice(&index.to_le_bytes());
    }
    blob.resize(32 + 24 * v + pad8(12 * t), 0);
    for face in faces {
        blob.extend_from_slice(&face.to_le_bytes());
    }
    blob.resize(total, 0);
    (blob.len() == total).then_some(blob)
}

struct Certificate {
    mesh_volume: f64,
    mesh_magnitude: f64,
    analytic_magnitude: f64,
}

// Bounded checks on the candidate: index validity, nonzero area, exact cap orientation,
// paired opposite edge incidence, connectivity, Euler characteristic for genus = cavities,
// and a positive signed volume. Work is linear in the triangle count plus one sort.
fn self_check(built: &Built, profile: &Profile) -> Result<Certificate, ProfileFailure> {
    let fail = || mesh_failure(ProfileCode::SelfCheckFailed, Stage::Checks);
    let v = built.vertices.len();
    let t = built.triangles.len();
    if v != built.point_count * 2 || t != built.triangle_faces.len() || t == 0 {
        return Err(fail());
    }
    let vertex = |i: u32| {
        usize::try_from(i)
            .ok()
            .and_then(|i| built.vertices.get(i))
            .copied()
    };
    let anchor = {
        let mut ax = f64::INFINITY;
        let mut ay = f64::INFINITY;
        for p in &built.vertices {
            ax = ax.min(p.x);
            ay = ay.min(p.y);
        }
        Point3::new(ax, ay, 0.0)
    };
    let mut directed: Vec<u64> = Vec::new();
    directed.try_reserve_exact(3 * t).map_err(|_| fail())?;
    let mut used = vec![false; v];
    let mut parent: Vec<usize> = (0..v).collect();
    let mut volume_sum = 0.0;
    let mut magnitude_sum = 0.0;
    for (tri, face) in built.triangles.iter().zip(&built.triangle_faces) {
        let [ia, ib, ic] = *tri;
        let (Some(a), Some(b), Some(c)) = (vertex(ia), vertex(ib), vertex(ic)) else {
            return Err(fail());
        };
        if ia == ib || ib == ic || ia == ic {
            return Err(fail());
        }
        let is_cap = a.z == b.z && b.z == c.z;
        if is_cap {
            let flat = |p: Point3| Point2::new(p.x, p.y);
            let sign = orient2d(flat(a), flat(b), flat(c)).map_err(|_| fail())?;
            let expected = if *face == 0 {
                Ordering::Less
            } else if *face == 1 {
                Ordering::Greater
            } else {
                return Err(fail());
            };
            if sign != expected {
                return Err(fail());
            }
        } else {
            let cross = (
                (b.y - a.y) * (c.z - a.z) - (b.z - a.z) * (c.y - a.y),
                (b.z - a.z) * (c.x - a.x) - (b.x - a.x) * (c.z - a.z),
                (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x),
            );
            if (cross.0 == 0.0 && cross.1 == 0.0 && cross.2 == 0.0) || *face < 2 {
                return Err(fail());
            }
        }
        for (x, y) in [(ia, ib), (ib, ic), (ic, ia)] {
            directed.push((u64::from(x) << 32) | u64::from(y));
        }
        for i in [ia, ib, ic] {
            *used
                .get_mut(usize::try_from(i).map_err(|_| fail())?)
                .ok_or_else(fail)? = true;
        }
        union(&mut parent, ia, ib).ok_or_else(fail)?;
        union(&mut parent, ib, ic).ok_or_else(fail)?;
        // Signed tetrahedron against a corner anchor: det(a-r, b-r, c-r), summed over 6.
        let (ra, rb, rc) = (rel(a, anchor), rel(b, anchor), rel(c, anchor));
        let det = ra.0 * (rb.1 * rc.2 - rb.2 * rc.1) - ra.1 * (rb.0 * rc.2 - rb.2 * rc.0)
            + ra.2 * (rb.0 * rc.1 - rb.1 * rc.0);
        volume_sum += det;
        magnitude_sum += det.abs();
    }
    if !used.iter().all(|u| *u) || !volume_sum.is_finite() || volume_sum <= 0.0 {
        return Err(fail());
    }
    directed.sort_unstable();
    if directed.windows(2).any(|w| matches!(w, [a, b] if a == b)) {
        return Err(fail());
    }
    let mut undirected = 0usize;
    for key in &directed {
        let (x, y) = (key >> 32, key & 0xffff_ffff);
        if directed.binary_search(&((y << 32) | x)).is_err() {
            return Err(fail());
        }
        undirected += usize::from(x < y);
    }
    let root = find(&mut parent, 0).ok_or_else(fail)?;
    for i in 1..v {
        if find(&mut parent, i).ok_or_else(fail)? != root {
            return Err(fail());
        }
    }
    let genus = profile.cavities().len();
    let lhs = i64::try_from(v + t).map_err(|_| fail())?;
    let rhs = i64::try_from(undirected + 2).map_err(|_| fail())?
        - 2 * i64::try_from(genus).map_err(|_| fail())?;
    if lhs != rhs {
        return Err(fail());
    }
    let mut analytic_magnitude = shoelace(profile.outer()).1;
    for cavity in profile.cavities() {
        analytic_magnitude += shoelace(cavity.points()).1;
    }
    Ok(Certificate {
        mesh_volume: volume_sum / 6.0,
        mesh_magnitude: magnitude_sum / 6.0,
        analytic_magnitude,
    })
}

fn rel(p: Point3, r: Point3) -> (f64, f64, f64) {
    (p.x - r.x, p.y - r.y, p.z - r.z)
}

fn find(parent: &mut [usize], i: usize) -> Option<usize> {
    let mut root = i;
    for _ in 0..=parent.len() {
        let p = *parent.get(root)?;
        if p == root {
            let mut cur = i;
            for _ in 0..=parent.len() {
                let next = *parent.get(cur)?;
                if next == root {
                    break;
                }
                *parent.get_mut(cur)? = root;
                cur = next;
            }
            return Some(root);
        }
        root = p;
    }
    None
}

fn union(parent: &mut [usize], a: u32, b: u32) -> Option<()> {
    let ra = find(parent, usize::try_from(a).ok()?)?;
    let rb = find(parent, usize::try_from(b).ok()?)?;
    if ra != rb {
        *parent.get_mut(ra)? = rb;
    }
    Some(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate() -> (Profile, Built) {
        let outer = vec![
            Point2::new(0.0, 0.0),
            Point2::new(40.0, 0.0),
            Point2::new(40.0, 30.0),
            Point2::new(0.0, 30.0),
        ];
        let hole = Cavity::polygon(vec![
            Point2::new(10.0, 10.0),
            Point2::new(10.0, 20.0),
            Point2::new(20.0, 20.0),
            Point2::new(20.0, 10.0),
        ]);
        let profile = Profile::admit(&outer, &[hole]).unwrap();
        let tris = profile.triangulate().unwrap();
        let built = build(&profile, 3.0, &tris).unwrap();
        (profile, built)
    }

    #[test]
    fn pristine_candidate_passes_and_its_volume_is_certified() {
        let (profile, built) = candidate();
        let certificate = self_check(&built, &profile).unwrap();
        let volume = profile.net_area_mm2() * 3.0;
        assert_eq!(volume, (1200.0 - 100.0) * 3.0);
        assert!(volume_certified(&certificate, volume, 3.0));
        assert!(!volume_certified(&certificate, volume * 1.000_001, 3.0));
        assert!(!volume_certified(&certificate, f64::NAN, 3.0));
        assert!(!volume_certified(&certificate, 0.0, 3.0));
    }

    #[test]
    fn corrupted_candidates_never_seal() {
        let wall = |b: &Built| b.triangle_faces.iter().position(|f| *f >= 2).unwrap();
        for mutation in 0..9 {
            let (profile, mut b) = candidate();
            let w = wall(&b);
            match mutation {
                0 => b.triangles[w].swap(1, 2), // inward wall triangle
                1 => {
                    b.triangles.remove(w);
                    b.triangle_faces.remove(w);
                } // missing triangle: open shell
                2 => b.triangles[w][0] = 9999,
                3 => b.triangles[w][0] = b.triangles[w][1],
                4 => {
                    let t = b.triangles[w];
                    let f = b.triangle_faces[w];
                    b.triangles.push(t);
                    b.triangle_faces.push(f);
                } // directed edge used twice
                5 => b.triangles.iter_mut().for_each(|t| t.swap(1, 2)), // wholly inward
                6 => b.vertices[10].z = 2.0, // top vertex off its cap plane
                7 => b.triangle_faces[0] = 4, // a cap triangle claimed by a wall face
                _ => {
                    b.vertices.push(Point3::new(1.0, 1.0, 1.0)); // an extra unused vertex
                }
            }
            let failure = self_check(&b, &profile).err().unwrap();
            assert_eq!(
                failure.diagnostics()[0].code,
                ProfileCode::SelfCheckFailed,
                "{mutation}"
            );
            assert_eq!(failure.diagnostics()[0].stage, Stage::Checks);
        }
    }

    #[test]
    fn a_displaced_vertex_keeps_the_topology_but_fails_the_volume_certificate() {
        let (profile, mut b) = candidate();
        b.vertices[6].x += 5.0; // top copy of the corner (40, 30)
        let certificate = self_check(&b, &profile).unwrap();
        assert!(!volume_certified(
            &certificate,
            profile.net_area_mm2() * 3.0,
            3.0
        ));
    }

    #[test]
    fn wrong_genus_fails_the_euler_check() {
        // The same mesh checked against a profile with an extra cavity cannot match genus 2.
        let (_, b) = candidate();
        let outer = vec![
            Point2::new(0.0, 0.0),
            Point2::new(40.0, 0.0),
            Point2::new(40.0, 30.0),
            Point2::new(0.0, 30.0),
        ];
        let holes = [
            Cavity::polygon(vec![
                Point2::new(10.0, 10.0),
                Point2::new(10.0, 20.0),
                Point2::new(20.0, 20.0),
                Point2::new(20.0, 10.0),
            ]),
            Cavity::polygon(vec![
                Point2::new(25.0, 5.0),
                Point2::new(25.0, 10.0),
                Point2::new(30.0, 10.0),
                Point2::new(30.0, 5.0),
            ]),
        ];
        let other = Profile::admit(&outer, &holes).unwrap();
        assert!(self_check(&b, &other).is_err());
    }

    #[test]
    fn blob_encoding_refuses_inconsistent_sizes() {
        assert!(encode(&[], &[], &[], 0).is_some());
        let v = [Point3::new(0.0, 0.0, 0.0)];
        let blob = encode(&v, &[[0, 0, 0]], &[0], 1).unwrap();
        assert_eq!(blob.len(), 32 + 24 + 16 + 8);
    }
}
