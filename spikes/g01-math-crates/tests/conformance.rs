// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! Independent scalar oracle and native-crate comparisons. Raw to_bits, including signed zero.
// Integration-test oracles may fail loudly; production adapter retains deny lints.
#![allow(clippy::expect_used)]
use beamcraft_g01_math_crates_spike as adapter;
use glam::{DMat3, DQuat, DVec3};
use nalgebra::{Matrix3, Quaternion, UnitQuaternion, Vector3};
mod support;

fn rd([a, b, c]: [f64; 3], [x, y, z]: [f64; 3]) -> f64 {
    (a * x + b * y) + c * z
}
fn rc([a, b, c]: [f64; 3], [x, y, z]: [f64; 3]) -> [f64; 3] {
    [b * z - c * y, c * x - a * z, a * y - b * x]
}
fn rn(v: [f64; 3]) -> f64 {
    rd(v, v).sqrt()
}
fn rq([x, y, z, w]: [f64; 4]) -> [f64; 4] {
    let r = (((x * x + y * y) + z * z) + w * w).sqrt();
    [x / r, y / r, z / r, w / r]
}
fn rm([x, y, z, w]: [f64; 4]) -> [[f64; 3]; 3] {
    [
        [
            1.0 - 2.0 * (y * y + z * z),
            2.0 * (x * y - z * w),
            2.0 * (x * z + y * w),
        ],
        [
            2.0 * (x * y + z * w),
            1.0 - 2.0 * (x * x + z * z),
            2.0 * (y * z - x * w),
        ],
        [
            2.0 * (x * z - y * w),
            2.0 * (y * z + x * w),
            1.0 - 2.0 * (x * x + y * y),
        ],
    ]
}
fn mv([a, b, c]: [[f64; 3]; 3], v: [f64; 3]) -> [f64; 3] {
    [rd(a, v), rd(b, v), rd(c, v)]
}
fn na([x, y, z]: [f64; 3]) -> Vector3<f64> {
    Vector3::new(x, y, z)
}
fn arr(v: Vector3<f64>) -> [f64; 3] {
    [v.x, v.y, v.z]
}
fn nm([[a, b, c], [d, e, f], [g, h, i]]: [[f64; 3]; 3]) -> Matrix3<f64> {
    Matrix3::new(a, b, c, d, e, f, g, h, i)
}
fn gm([[a, b, c], [d, e, f], [g, h, i]]: [[f64; 3]; 3]) -> DMat3 {
    DMat3::from_cols(
        DVec3::new(a, d, g),
        DVec3::new(b, e, h),
        DVec3::new(c, f, i),
    )
}
fn nm_rows(m: Matrix3<f64>) -> [f64; 9] {
    let a = m.column(0);
    let b = m.column(1);
    let c = m.column(2);
    [a.x, b.x, c.x, a.y, b.y, c.y, a.z, b.z, c.z]
}
fn gm_rows(m: DMat3) -> [f64; 9] {
    let [a, d, g, b, e, h, c, f, i] = m.to_cols_array();
    [a, b, c, d, e, f, g, h, i]
}
fn flatten([[a, b, c], [d, e, f], [g, h, i]]: [[f64; 3]; 3]) -> [f64; 9] {
    [a, b, c, d, e, f, g, h, i]
}

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    fn scalar(&mut self) -> f64 {
        ((self.next() >> 11) as f64 * (1.0 / 9007199254740992.0)) * 2.0 - 1.0
    }
    fn vector(&mut self) -> [f64; 3] {
        // Exactly representable power-of-two scale, 2^-30 through 2^20.
        let scale = f64::from_bits((1023 - 30 + self.next() % 51) << 52);
        [
            self.scalar() * scale,
            self.scalar() * scale,
            self.scalar() * scale,
        ]
    }
    fn quat(&mut self) -> [f64; 4] {
        rq([self.scalar(), self.scalar(), self.scalar(), self.scalar()])
    }
}

#[derive(Default)]
struct Stats {
    samples: u64,
    mismatches: u64,
    components: u64,
    max_ulp: u64,
    zero_sign: u64,
    nonfinite: u64,
    max_abs: f64,
}
fn ordered(x: f64) -> u64 {
    let b = x.to_bits();
    if b >> 63 != 0 { !b } else { b | (1 << 63) }
}
impl Stats {
    fn compare<const N: usize>(&mut self, actual: [f64; N], expected: [f64; N]) {
        self.samples += 1;
        let mut mismatch = false;
        for (a, b) in actual.into_iter().zip(expected) {
            if a.to_bits() != b.to_bits() {
                mismatch = true;
                self.components += 1;
                if a == 0.0 && b == 0.0 {
                    self.zero_sign += 1;
                }
                if !a.is_finite() || !b.is_finite() {
                    self.nonfinite += 1;
                } else {
                    self.max_ulp = self.max_ulp.max(ordered(a).abs_diff(ordered(b)));
                    self.max_abs = self.max_abs.max((a - b).abs());
                }
            }
        }
        self.mismatches += u64::from(mismatch);
    }
    fn print(&self, label: &str) {
        println!(
            "{label},{},{},{},{},{},{},{:.17e}",
            self.samples,
            self.mismatches,
            self.components,
            self.max_ulp,
            self.zero_sign,
            self.nonfinite,
            self.max_abs
        );
    }
}
#[derive(Default)]
struct Suite {
    dot: Stats,
    cross: Stats,
    norm: Stats,
    normalize: Stats,
    matrix: Stats,
    rotate: Stats,
    mul: Stats,
    world: Stats,
}
impl Suite {
    fn print(&self, name: &str) {
        for (op, s) in [
            ("dot", &self.dot),
            ("cross", &self.cross),
            ("norm", &self.norm),
            ("quat_normalize", &self.normalize),
            ("quat_matrix", &self.matrix),
            ("quat_rotate", &self.rotate),
            ("matrix_vector", &self.mul),
            ("world", &self.world),
        ] {
            s.print(&format!("{name}/{op}"));
        }
    }
}
fn sample(a: [f64; 3], b: [f64; 3], q: [f64; 4], n: &mut Suite, g: &mut Suite, ad: &mut Suite) {
    let (an, bn) = (na(a), na(b));
    let (ag, bg) = (DVec3::from_array(a), DVec3::from_array(b));
    let [x, y, z, w] = q;
    // Shared F06-normalized input isolates conversion/rotation from crate normalization.
    let nq = UnitQuaternion::new_unchecked(Quaternion::new(w, x, y, z));
    let gq = DQuat::from_xyzw(x, y, z, w);
    let reference = rm(q);
    let rvec = mv(reference, a);
    let normalized = rq(q);
    let nnormalized = UnitQuaternion::new_normalize(Quaternion::new(w, x, y, z));
    let nn = nnormalized.quaternion();
    n.dot.compare([an.dot(&bn)], [rd(a, b)]);
    g.dot.compare([ag.dot(bg)], [rd(a, b)]);
    ad.dot.compare([adapter::dot(ag, bg)], [rd(a, b)]);
    n.cross.compare(arr(an.cross(&bn)), rc(a, b));
    g.cross.compare(ag.cross(bg).to_array(), rc(a, b));
    ad.cross
        .compare(adapter::cross(ag, bg).to_array(), rc(a, b));
    n.norm.compare([an.norm()], [rn(a)]);
    g.norm.compare([ag.length()], [rn(a)]);
    ad.norm.compare([adapter::norm(ag)], [rn(a)]);
    n.normalize.compare([nn.i, nn.j, nn.k, nn.w], normalized);
    g.normalize.compare(gq.normalize().to_array(), normalized);
    let aq = adapter::quaternion(q).expect("admitted generated quaternion");
    let opposite = adapter::quaternion(q.map(|v| -v)).expect("antipode");
    assert_eq!(
        aq.to_array().map(f64::to_bits),
        opposite.to_array().map(f64::to_bits)
    );
    // Canonical sign/zero selection is intentionally separate from normalization arithmetic.
    let mut aqv = aq.to_array();
    if w < 0.0
        || (w == 0.0
            && [x, y, z]
                .into_iter()
                .find(|v| *v != 0.0)
                .is_some_and(|v| v < 0.0))
    {
        aqv = aqv.map(|v| -v);
    }
    ad.normalize.compare(
        aqv.map(|v| if v == 0.0 { 0.0 } else { v }),
        normalized.map(|v| if v == 0.0 { 0.0 } else { v }),
    );
    n.matrix.compare(
        nm_rows(nq.to_rotation_matrix().into_inner()),
        flatten(reference),
    );
    g.matrix
        .compare(gm_rows(DMat3::from_quat(gq)), flatten(reference));
    ad.matrix.compare(
        flatten(adapter::rotation(gq).map(|v| v.to_array())),
        flatten(reference),
    );
    n.rotate.compare(arr(nq.transform_vector(&an)), rvec);
    g.rotate.compare(gq.mul_vec3(ag).to_array(), rvec);
    ad.rotate.compare(
        adapter::multiply(adapter::rotation(gq), ag).to_array(),
        rvec,
    );
    n.mul.compare(arr(nm(reference) * an), rvec);
    g.mul.compare((gm(reference) * ag).to_array(), rvec);
    ad.mul.compare(
        adapter::multiply(reference.map(DVec3::from_array), ag).to_array(),
        rvec,
    );
    let expected = std::array::from_fn::<_, 3, _>(|i| {
        rvec.get(i).copied().expect("fixed size") + b.get(i).copied().expect("fixed size")
    });
    n.world.compare(arr(nm(reference) * an + bn), expected);
    g.world
        .compare((gm(reference) * ag + bg).to_array(), expected);
    ad.world.compare(
        (adapter::multiply(reference.map(DVec3::from_array), ag) + bg).to_array(),
        expected,
    );
}

#[test]
fn deterministic_conformance() {
    let (mut n, mut g, mut ad) = (Suite::default(), Suite::default(), Suite::default());
    let mut rng = Rng(0x6a09_e667_f3bc_c909);
    for _ in 0..100_000 {
        let a = rng.vector();
        let b = rng.vector();
        let q = rng.quat();
        sample(a, b, q, &mut n, &mut g, &mut ad);
    }
    for &v in support::VECTORS {
        sample(
            v,
            [1.0, -2.0, 3.0],
            [0.0, 0.0, 0.0, 1.0],
            &mut n,
            &mut g,
            &mut ad,
        );
    }
    for &q in support::QUATERNIONS {
        sample(
            [3000.0, 0.0, 10.0],
            [100.0, 200.0, 300.0],
            q,
            &mut n,
            &mut g,
            &mut ad,
        );
    }
    // Cancellation, large origins, gradual underflow, signed zero and half-turns.
    for a in [
        [1e9, 1e9, 0.001],
        [1.0, 1.0, 1.0],
        [-0.0, 0.0, -0.0],
        [f64::from_bits(1), 0.001, 0.0],
        [100.0, 0.0, 5.0],
    ] {
        for q in [
            [0.5, 0.5, 0.5, 0.5],
            [1.0, 0.0, 0.0, 0.0],
            [f64::from_bits(1), 0.0, 0.0, 1.0],
        ] {
            sample(a, [1.0, -1.0, 1e-16], q, &mut n, &mut g, &mut ad);
        }
    }
    println!(
        "crate/operation,samples,mismatch_samples,mismatch_components,max_ulp,signed_zero_components,nonfinite_components,max_absolute_error"
    );
    n.print("nalgebra");
    g.print("glam");
    ad.print("adapter");
    for s in [
        &ad.dot,
        &ad.cross,
        &ad.norm,
        &ad.normalize,
        &ad.matrix,
        &ad.rotate,
        &ad.mul,
        &ad.world,
    ] {
        assert_eq!(s.mismatches, 0);
    }
    for s in [&g.dot, &g.cross, &g.norm, &g.mul, &g.world] {
        assert_eq!(s.mismatches, 0);
    }
    assert!(n.rotate.mismatches > 0 && g.rotate.mismatches > 0);
}

#[test]
fn api_fit_and_policy() {
    let q = [
        0.0,
        0.0,
        std::f64::consts::FRAC_1_SQRT_2,
        std::f64::consts::FRAC_1_SQRT_2,
    ];
    let pose = adapter::Pose::new(q, DVec3::new(100.0, 200.0, 300.0)).expect("valid");
    let world = pose.world(DVec3::new(3000.0, 0.0, 0.0)).expect("finite");
    approx::assert_abs_diff_eq!(world.x, 100.0, epsilon = 1e-9);
    approx::assert_abs_diff_eq!(world.y, 3200.0, epsilon = 1e-9);
    let antipode =
        adapter::Pose::new(q.map(|v| -v), DVec3::new(100.0, 200.0, 300.0)).expect("valid");
    assert_eq!(
        world.to_array().map(f64::to_bits),
        antipode
            .world(DVec3::new(3000.0, 0.0, 0.0))
            .expect("valid")
            .to_array()
            .map(f64::to_bits)
    );
    for &q in support::QUATERNIONS {
        let a = adapter::quaternion(q).expect("positive case");
        let b = adapter::quaternion(q.map(|v| -v)).expect("antipode");
        assert_eq!(
            a.to_array().map(f64::to_bits),
            b.to_array().map(f64::to_bits)
        );
    }
    assert_eq!(
        adapter::quaternion([0.0; 4]),
        Err(adapter::Error::Quaternion)
    );
    assert_eq!(
        adapter::quaternion([0.0, 0.0, 0.0, 1.0 - 1e-10]),
        Err(adapter::Error::Quaternion)
    );
    assert_eq!(
        adapter::quaternion([f64::NAN, 0.0, 0.0, 1.0]),
        Err(adapter::Error::NonFinite)
    );
    assert!(adapter::quaternion([0.0, 0.0, 0.0, 1.0 - 1e-13]).is_ok());
    assert!(adapter::Pose::new(q, DVec3::splat(1e9 + 1.0)).is_err());
    assert!(pose.world(DVec3::new(1e9 + 500_000.0, 0.0, 0.0)).is_ok());
    approx::assert_abs_diff_eq!(
        rn([100.0, 0.0, 5.0]),
        100.124_921_972_503_93,
        epsilon = 1e-13
    );
    // Hamilton i*j=k, active +Z rotation in both crate APIs.
    let ni = UnitQuaternion::new_unchecked(Quaternion::new(0.0, 1.0, 0.0, 0.0));
    let nj = UnitQuaternion::new_unchecked(Quaternion::new(0.0, 0.0, 1.0, 0.0));
    assert_eq!((ni * nj).quaternion().k, 1.0);
    assert_eq!(
        (DQuat::from_xyzw(1.0, 0.0, 0.0, 0.0) * DQuat::from_xyzw(0.0, 1.0, 0.0, 0.0)).z,
        1.0
    );
    let [qx, qy, qz, qw] = q;
    let nq = UnitQuaternion::new_normalize(Quaternion::new(qw, qx, qy, qz));
    let iso = nalgebra::Isometry3::from_parts(nalgebra::Translation3::new(100.0, 200.0, 300.0), nq);
    let np = iso.transform_point(&nalgebra::Point3::new(3000.0, 0.0, 0.0));
    let affine = glam::DAffine3::from_rotation_translation(
        DQuat::from_array(q),
        DVec3::new(100.0, 200.0, 300.0),
    );
    let gp = affine.transform_point3(DVec3::new(3000.0, 0.0, 0.0));
    approx::assert_abs_diff_eq!(np.y, 3200.0, epsilon = 1e-9);
    approx::assert_abs_diff_eq!(gp.y, 3200.0, epsilon = 1e-9);
}

#[test]
fn frames_and_exploratory_roll() {
    for &[a, b] in support::AXES {
        assert!(
            adapter::Frame::new(DVec3::from_array(a), DVec3::from_array(b))
                .expect("admitted axis")
                .certified()
        );
    }
    for (d, near) in [
        (DVec3::new(0.0005, 0.0, 1000.0), true),
        (DVec3::new(0.002, 0.0, 1000.0), false),
        (DVec3::new(0.0, 0.08000002, 160000.08), true),
        (DVec3::new(0.0, 0.002, 1000.0), false),
        (DVec3::new(0.5, 0.0, 500000.0), true),
    ] {
        assert_eq!(
            adapter::Frame::new(DVec3::ZERO, d)
                .expect("F06")
                .near_vertical,
            near
        );
    }
    for error in [1e-9, -1e-9] {
        assert!(
            !adapter::Frame {
                x: DVec3::X,
                y: DVec3::new(error, 1.0, 0.0),
                z: DVec3::Z,
                near_vertical: true
            }
            .certified()
        );
    }
    assert!(adapter::Frame::new(DVec3::ZERO, DVec3::ZERO).is_err());
    assert!(adapter::Frame::new(DVec3::ZERO, DVec3::new(0.0009, 0.0, 0.0)).is_err());
    let f = adapter::Frame::new(DVec3::ZERO, DVec3::Z * 1000.0).expect("vertical");
    let roll_rad = std::f64::consts::FRAC_PI_2;
    // Exploratory API only: trig is forbidden in F06 kernel arithmetic. Not an admitted field.
    let g = DQuat::from_axis_angle(f.z, roll_rad);
    let n = UnitQuaternion::from_axis_angle(
        &nalgebra::Unit::new_normalize(na(f.z.to_array())),
        roll_rad,
    );
    let gx = g * f.x;
    let nx = n * na(f.x.to_array());
    approx::assert_abs_diff_eq!(gx.y, 1.0, epsilon = 1e-15);
    approx::assert_abs_diff_eq!(nx.y, 1.0, epsilon = 1e-15);
    assert!((g * f.z - f.z).length() < 1e-15);
    let rolled = adapter::Frame {
        x: g * f.x,
        y: g * f.y,
        z: f.z,
        near_vertical: true,
    };
    assert!(rolled.certified());
}

#[test]
fn native_frame_primitives_and_random_certificates() {
    // The branch, projection and certificate remain BeamCraft policy, regardless of storage crate.
    let mut rng = Rng(0xbb67_ae85_84ca_a73b);
    for _ in 0..100_000 {
        let v = rng.vector();
        let d = DVec3::from_array(v) * 1024.0;
        if let Ok(f) = adapter::Frame::new(DVec3::ZERO, d) {
            assert!(f.certified());
            let nx = na(f.y.to_array()).cross(&na(f.z.to_array()));
            let gx = f.y.cross(f.z);
            assert_eq!(arr(nx).map(f64::to_bits), gx.to_array().map(f64::to_bits));
            assert!((na(f.x.to_array()).dot(&na(f.y.to_array()))).abs() <= 1e-12);
            assert!((f.x.dot(f.y)).abs() <= 1e-12);
        }
    }
    for &[a, b] in support::AXES {
        let d = na(b) - na(a);
        let hh = d.x * d.x + d.y * d.y;
        let ll = hh + d.z * d.z;
        let l = ll.sqrt();
        let z = Vector3::new(d.x / l, d.y / l, d.z / l);
        let y = if hh < 1e-12 * ll {
            let p = Vector3::new(-z.x * z.y, 1.0 - z.y * z.y, -z.z * z.y);
            let n = p.norm();
            p / n
        } else {
            let denominator = hh.sqrt() * l;
            Vector3::new(
                (-d.x * d.z) / denominator,
                (-d.y * d.z) / denominator,
                hh / denominator,
            )
        };
        let x0 = y.cross(&z);
        let x = x0 / x0.norm();
        let y = z.cross(&x);
        let f = adapter::Frame::new(DVec3::from_array(a), DVec3::from_array(b)).expect("F06 axis");
        // nalgebra scalar division is component-wise for this type; certificates are policy.
        assert!(
            adapter::Frame {
                x: DVec3::from_array(arr(x)),
                y: DVec3::from_array(arr(y)),
                z: DVec3::from_array(arr(z)),
                near_vertical: f.near_vertical
            }
            .certified()
        );
        for (native, reference) in [(x, f.x), (y, f.y), (z, f.z)] {
            for (u, v) in arr(native).into_iter().zip(reference.to_array()) {
                assert!((u - v).abs() <= 1e-15);
            }
        }
    }
}
