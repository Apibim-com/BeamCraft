// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! Math projections of F06 cases, not mesh/quantity/engine acceptance tests.
//! Inputs and reference values: geometry-contract-cases-v0.1.json, frozen 0.1.0.

use beamcraft_geom::tolerance::{ROLL_CAP_RAD, Tolerance};
use beamcraft_geom::{
    Frame3, FrameCertificate, MathError, MemberFrame, MemberFrameRule, Plane3, Point3,
    RigidTransform, Translation3, Vec3, canonical_f64_bytes,
};

fn assert_frame_close(frame: Frame3, expected: [[f64; 3]; 3]) {
    for (actual, reference) in [frame.x(), frame.y(), frame.z()].into_iter().zip(expected) {
        for (a, e) in actual.components().into_iter().zip(reference) {
            // CR-TOL-001 dimensionless comparison only; never a production decision.
            let bound = if e == 0.0 { 1e-6 } else { 1e-6 * e.abs() };
            assert!((a - e).abs() <= bound, "{a} versus {e}");
        }
    }
    assert!(frame.certificate().is_valid());
}

fn assert_bits(a: [f64; 3], b: [f64; 3]) {
    assert_eq!(a.map(f64::to_bits), b.map(f64::to_bits));
}

#[test]
fn f06_pos_04_member_math() {
    let member = MemberFrame::new(
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(4000.0, 0.0, 0.0),
        200.0,
        300.0,
        None,
    )
    .unwrap();
    let frame = member.frame();
    assert!(frame.certificate().is_valid());
    assert_frame_close(frame, [[0.0, 1.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]]);
    assert_bits(frame.x().components(), [0.0, 1.0, 0.0]);
    assert_bits(frame.y().components(), [0.0, 0.0, 1.0]);
    assert_bits(frame.z().components(), [1.0, 0.0, 0.0]);
    assert!((member.scale().axis_length_mm() - 4000.0).abs() <= 1e-6);
}

#[test]
fn f06_pos_05_member_math() {
    let member = MemberFrame::new(
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(0.0, 0.0, 3000.0),
        200.0,
        300.0,
        None,
    )
    .unwrap();
    let frame = member.frame();
    assert!(frame.certificate().is_valid());
    assert_frame_close(frame, [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]);
    assert_bits(frame.x().components(), [1.0, 0.0, 0.0]);
    assert_bits(frame.y().components(), [0.0, 1.0, 0.0]);
    assert_bits(frame.z().components(), [0.0, 0.0, 1.0]);
    assert!((member.scale().axis_length_mm() - 3000.0).abs() <= 1e-6);
}

#[test]
fn f06_pos_06_member_math() {
    let member = MemberFrame::new(
        Point3::new(0.0, 0.0, 3000.0),
        Point3::new(0.0, 0.0, 0.0),
        200.0,
        300.0,
        None,
    )
    .unwrap();
    let frame = member.frame();
    assert!(frame.certificate().is_valid());
    assert_frame_close(frame, [[-1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, -1.0]]);
    assert_bits(frame.x().components(), [-1.0, 0.0, 0.0]);
    assert_bits(frame.y().components(), [0.0, 1.0, 0.0]);
    assert_bits(frame.z().components(), [0.0, 0.0, -1.0]);
    assert!((member.scale().axis_length_mm() - 3000.0).abs() <= 1e-6);
}

#[test]
fn f06_pos_07_member_math() {
    let member = MemberFrame::new(
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(0.0, 3000.0, 4000.0),
        200.0,
        300.0,
        None,
    )
    .unwrap();
    let frame = member.frame();
    assert!(frame.certificate().is_valid());
    assert_frame_close(frame, [[-1.0, 0.0, 0.0], [0.0, -0.8, 0.6], [0.0, 0.6, 0.8]]);
    assert!((member.scale().axis_length_mm() - 5000.0).abs() <= 1e-6);
}

#[test]
fn f06_pos_08_member_math() {
    let member = MemberFrame::new(
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(3000.0, 4000.0, 0.0),
        100.0,
        300.0,
        None,
    )
    .unwrap();
    let frame = member.frame();
    assert!(frame.certificate().is_valid());
    assert_frame_close(frame, [[-0.8, 0.6, 0.0], [0.0, 0.0, 1.0], [0.6, 0.8, 0.0]]);
    assert!((member.scale().axis_length_mm() - 5000.0).abs() <= 1e-6);
}

#[test]
fn f06_pos_09_member_math() {
    let member = MemberFrame::new(
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(0.0005, 0.0, 1000.0),
        200.0,
        300.0,
        None,
    )
    .unwrap();
    let frame = member.frame();
    assert!(frame.certificate().is_valid());
    assert_frame_close_component(frame.x(), [1.0, 0.0, -5e-07]);
    assert_frame_close_component(frame.y(), [0.0, 1.0, 0.0]);
    assert_frame_close_component(frame.z(), [5e-07, 0.0, 1.0]);
    assert_eq!(member.rule(), MemberFrameRule::NearVertical);
    // F06 erratum: re-orthogonalization yields one ulp above 1, despite the case prose.
    // Independent scalar evaluation of steps a-c gives 0x3ff0000000000001.
    assert_bits(
        frame.y().components(),
        [0.0, f64::from_bits(0x3ff0_0000_0000_0001), 0.0],
    );
}

#[test]
fn f06_pos_10_member_math() {
    let member = MemberFrame::new(
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(0.002, 0.0, 1000.0),
        200.0,
        300.0,
        None,
    )
    .unwrap();
    let frame = member.frame();
    assert!(frame.certificate().is_valid());
    assert_frame_close_component(frame.x(), [0.0, 1.0, 0.0]);
    assert_frame_close_component(frame.y(), [-0.999999999998, 0.0, 1.999999999996e-06]);
    assert_eq!(member.rule(), MemberFrameRule::Horizontal);
}

#[test]
fn f06_pos_15_member_math() {
    let member = MemberFrame::new(
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(0.0, 0.0, 1000.0),
        1000000.0,
        1000000.0,
        None,
    )
    .unwrap();
    let frame = member.frame();
    assert!(frame.certificate().is_valid());
}

#[test]
fn f06_pos_41_member_math() {
    let member = MemberFrame::new(
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(0.0, 0.08000002, 160000.08),
        200.0,
        300.0,
        None,
    )
    .unwrap();
    let frame = member.frame();
    assert!(frame.certificate().is_valid());
    assert_frame_close(
        frame,
        [
            [1.0, 0.0, 0.0],
            [0.0, 0.999999999999875, -4.99999875e-07],
            [0.0, 4.99999875e-07, 0.999999999999875],
        ],
    );
    assert_eq!(member.rule(), MemberFrameRule::NearVertical);
    assert!((member.scale().axis_length_mm() - 160000.08000002).abs() <= 1e-6);
}

#[test]
fn f06_pos_42_member_math() {
    let member = MemberFrame::new(
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(0.0, 0.002, 1000.0),
        200.0,
        300.0,
        None,
    )
    .unwrap();
    let frame = member.frame();
    assert!(frame.certificate().is_valid());
    assert_frame_close_component(frame.x(), [-1.0, 0.0, 0.0]);
    assert_frame_close_component(frame.y(), [0.0, -0.999999999998, 1.999999999996e-06]);
    assert_frame_close_component(frame.z(), [0.0, 1.999999999996e-06, 0.999999999998]);
    assert_eq!(member.rule(), MemberFrameRule::Horizontal);
}

#[test]
fn f06_pos_46_member_math() {
    let member = MemberFrame::new(
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(0.5, 0.0, 500000.0),
        200.0,
        300.0,
        None,
    )
    .unwrap();
    let frame = member.frame();
    assert!(frame.certificate().is_valid());
    assert_frame_close_component(frame.y(), [0.0, 1.0, 0.0]);
    assert_frame_close_component(frame.z(), [9.99999999999e-07, 0.0, 0.999999999999]);
    assert_eq!(member.rule(), MemberFrameRule::NearVertical);
    assert_bits(frame.y().components(), [0.0, 1.0, 0.0]);
}

#[test]
fn f06_pos_50_member_math() {
    let member = MemberFrame::new(
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(4000.0, 0.0, 0.0),
        200.0,
        300.0,
        Some(0.0),
    )
    .unwrap();
    let frame = member.frame();
    assert!(frame.certificate().is_valid());
    assert_frame_close(frame, [[0.0, 1.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]]);
    assert_bits(frame.x().components(), [0.0, 1.0, 0.0]);
    assert_bits(frame.y().components(), [0.0, 0.0, 1.0]);
    assert_bits(frame.z().components(), [1.0, 0.0, 0.0]);
    assert_eq!(member.rule(), MemberFrameRule::Horizontal);
}

#[test]
fn f06_pos_51_member_math() {
    let member = MemberFrame::new(
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(4000.0, 0.0, 0.0),
        200.0,
        300.0,
        Some(core::f64::consts::FRAC_PI_2),
    )
    .unwrap();
    let frame = member.frame();
    assert!(frame.certificate().is_valid());
    assert_frame_close(frame, [[0.0, 0.0, 1.0], [0.0, -1.0, 0.0], [1.0, 0.0, 0.0]]);
    assert_eq!(member.rule(), MemberFrameRule::Horizontal);
}

#[test]
fn f06_pos_52_member_math() {
    let member = MemberFrame::new(
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(0.0, 0.08000002, 160000.08),
        200.0,
        300.0,
        Some(core::f64::consts::FRAC_PI_2),
    )
    .unwrap();
    let frame = member.frame();
    assert!(frame.certificate().is_valid());
    assert_frame_close(
        frame,
        [
            [0.0, 0.999999999999875, -4.99999875e-07],
            [-1.0, 0.0, 0.0],
            [0.0, 4.99999875e-07, 0.999999999999875],
        ],
    );
    assert_eq!(member.rule(), MemberFrameRule::NearVertical);
}

#[test]
fn f06_pos_57_member_math() {
    let member = MemberFrame::new(
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(0.0791015625, 0.0, 0.0),
        393216.0,
        524288.0,
        None,
    )
    .unwrap();
    let frame = member.frame();
    assert!(frame.certificate().is_valid());
    assert!((member.scale().section_half_diagonal_mm() - 327680.0).abs() <= 1e-6);
    assert_eq!(
        member.scale().tolerance().local_size_mm().to_bits(),
        327680.0_f64.to_bits()
    );
    assert_eq!(
        member.scale().tolerance().eps_geom_mm().to_bits(),
        0.078125_f64.to_bits()
    );
}

#[test]
fn f06_pos_58_member_math() {
    let member = MemberFrame::new(
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(524288.0, 0.0, 0.0),
        0.1259765625,
        300.0,
        None,
    )
    .unwrap();
    let frame = member.frame();
    assert!(frame.certificate().is_valid());
    assert_eq!(
        member.scale().tolerance().local_size_mm().to_bits(),
        524288.0_f64.to_bits()
    );
    assert_eq!(
        member.scale().tolerance().eps_geom_mm().to_bits(),
        0.125_f64.to_bits()
    );
}

fn assert_frame_close_component(actual: Vec3, expected: [f64; 3]) {
    for (a, e) in actual.components().into_iter().zip(expected) {
        let bound = if e == 0.0 { 1e-6 } else { 1e-6 * e.abs() };
        assert!((a - e).abs() <= bound, "{a} versus {e}");
    }
}

#[test]
fn f06_neg_04_member_math() {
    assert_eq!(
        MemberFrame::new(
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(0.0009, 0.0, 0.0),
            200.0,
            300.0,
            None
        ),
        Err(MathError::AxisTooShort)
    );
}

#[test]
fn f06_neg_05_member_math() {
    assert_eq!(
        MemberFrame::new(
            Point3::new(10.0, 20.0, 30.0),
            Point3::new(10.0, 20.0, 30.0),
            200.0,
            300.0,
            None
        ),
        Err(MathError::AxisTooShort)
    );
}

#[test]
fn f06_neg_06_member_math() {
    assert_eq!(
        MemberFrame::new(
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(0.0, 0.0, 1000.0),
            1000000.001,
            300.0,
            None
        ),
        Err(MathError::AboveLimit)
    );
}

#[test]
fn f06_neg_07_member_math() {
    assert_eq!(
        MemberFrame::new(
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(0.0, 0.0, 1000.0),
            200.0,
            0.0,
            None
        ),
        Err(MathError::NotPositive)
    );
}

#[test]
fn f06_neg_43_member_math() {
    assert_eq!(
        MemberFrame::new(
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(4000.0, 0.0, 0.0),
            200.0,
            300.0,
            Some(f64::NAN)
        ),
        Err(MathError::NonFinite)
    );
}

#[test]
fn f06_neg_44_member_math() {
    assert_eq!(
        MemberFrame::new(
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(4000.0, 0.0, 0.0),
            200.0,
            300.0,
            Some(7.0)
        ),
        Err(MathError::AboveLimit)
    );
}

#[test]
fn f06_neg_47_member_math() {
    assert_eq!(
        MemberFrame::new(
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(0.0771484375, 0.0, 0.0),
            393216.0,
            524288.0,
            None
        ),
        Err(MathError::AxisTooShort)
    );
}

#[test]
fn f06_neg_48_member_math() {
    assert_eq!(
        MemberFrame::new(
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(524288.0, 0.0, 0.0),
            0.1240234375,
            300.0,
            None
        ),
        Err(MathError::BelowMinimum)
    );
}

#[test]
fn f06_neg_51_member_math() {
    assert_eq!(
        MemberFrame::new(
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1000000.5, 0.0, 0.0),
            200.0,
            300.0,
            None
        ),
        Err(MathError::AboveLimit)
    );
}

#[test]
fn f06_neg_52_member_math() {
    assert_eq!(
        MemberFrame::new(
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1000.0, 0.0, 0.0),
            -3000000.0,
            300.0,
            None
        ),
        Err(MathError::NotPositive)
    );
}

#[test]
fn f06_neg_53_member_math() {
    assert_eq!(
        MemberFrame::new(
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1000.0, 0.0, 0.0),
            -1e308,
            300.0,
            None
        ),
        Err(MathError::NotPositive)
    );
}

#[test]
fn f06_neg_01_plate_scale_math() {
    let points = [
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(f64::NAN, 0.0, 0.0),
        Point3::new(f64::NAN, 500.0, 0.0),
        Point3::new(0.0, 500.0, 0.0),
    ];
    assert_eq!(Tolerance::for_plate(&points), Err(MathError::NonFinite));
}

#[test]
fn f06_neg_03_plate_scale_math() {
    let points = [
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(1000.0, 0.0, 0.0),
        Point3::new(1000.0, 500.0, 0.0),
        Point3::new(0.0, 500.0, 0.0),
    ];
    let tolerance = Tolerance::for_plate(&points).unwrap();
    assert_eq!(
        tolerance.check_dimension(0.0009),
        Err(MathError::BelowMinimum)
    );
}

#[test]
fn f06_pos_53_plate_scale_math() {
    let points = [
        Point3::new(999000000.0, 0.0, 0.0),
        Point3::new(999393216.0, 0.0, 0.0),
        Point3::new(999393216.0, 524288.0, 0.0),
        Point3::new(999000000.0, 524288.0, 0.0),
    ];
    let tolerance = Tolerance::for_plate(&points).unwrap();
    assert_eq!(tolerance.local_size_mm().to_bits(), 655360.0_f64.to_bits());
    assert_eq!(tolerance.eps_geom_mm().to_bits(), 0.15625_f64.to_bits());
    tolerance.check_dimension(0.1572265625).unwrap();
}

#[test]
fn f06_pos_54_plate_scale_math() {
    let points = [
        Point3::new(999000000.0, 0.0, 0.0),
        Point3::new(999393216.0, 0.0, 0.0),
        Point3::new(999393216.0, 524288.0, 0.0),
        Point3::new(999000000.0, 524288.0, 0.0),
    ];
    let tolerance = Tolerance::for_plate(&points).unwrap();
    assert_eq!(tolerance.local_size_mm().to_bits(), 655360.0_f64.to_bits());
    assert_eq!(tolerance.eps_geom_mm().to_bits(), 0.15625_f64.to_bits());
    tolerance.check_dimension(0.15625).unwrap();
}

#[test]
fn f06_pos_55_plate_scale_math() {
    let points = [
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(3.0, 0.0, 0.0),
        Point3::new(3.0, 4.0, 0.0),
        Point3::new(0.0, 4.0, 0.0),
    ];
    let tolerance = Tolerance::for_plate(&points).unwrap();
    assert_eq!(tolerance.local_size_mm().to_bits(), 5.0_f64.to_bits());
    assert_eq!(tolerance.eps_geom_mm().to_bits(), 0.001_f64.to_bits());
    tolerance.check_dimension(0.001).unwrap();
}

#[test]
fn f06_pos_56_plate_scale_math() {
    let points = [
        Point3::new(999000000.0, 0.0, 0.0),
        Point3::new(999393216.0, 0.0, 0.75),
        Point3::new(999393216.0, 524288.0, 0.0),
        Point3::new(999000000.0, 524288.0, 0.75),
    ];
    let tolerance = Tolerance::for_plate(&points).unwrap();
    assert_eq!(tolerance.local_size_mm().to_bits(), 655360.0_f64.to_bits());
    assert_eq!(tolerance.eps_geom_mm().to_bits(), 0.15625_f64.to_bits());
    tolerance.check_dimension(10.0).unwrap();
    let plane = Plane3::new(points[0], Vec3::Z).unwrap();
    assert_eq!(plane.signed_distance(points[1]).unwrap(), 0.75);
    assert_eq!(plane.within_planarity_band(points[1], tolerance), Ok(true));
}

#[test]
fn f06_neg_45_plate_scale_math() {
    let points = [
        Point3::new(999000000.0, 0.0, 0.0),
        Point3::new(999393216.0, 0.0, 0.0),
        Point3::new(999393216.0, 524288.0, 0.0),
        Point3::new(999000000.0, 524288.0, 0.0),
    ];
    let tolerance = Tolerance::for_plate(&points).unwrap();
    assert_eq!(tolerance.local_size_mm().to_bits(), 655360.0_f64.to_bits());
    assert_eq!(tolerance.eps_geom_mm().to_bits(), 0.15625_f64.to_bits());
    assert_eq!(
        tolerance.check_dimension(0.1552734375),
        Err(MathError::BelowMinimum)
    );
}

#[test]
fn f06_neg_46_plate_scale_math() {
    let points = [
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(750000.0, 0.0, 0.0),
        Point3::new(1500000.0, 0.0, 0.0),
        Point3::new(2250000.0, 0.0, 0.0),
        Point3::new(2250000.0, 1000.0, 0.0),
        Point3::new(1500000.0, 1000.0, 0.0),
        Point3::new(750000.0, 1000.0, 0.0),
        Point3::new(0.0, 1000.0, 0.0),
    ];
    assert_eq!(Tolerance::for_plate(&points), Err(MathError::AboveLimit));
}

#[test]
fn f06_neg_49_plate_scale_math() {
    let points = [
        Point3::new(999000000.0, 0.0, 0.0),
        Point3::new(999393216.0, 0.0, 0.875),
        Point3::new(999393216.0, 524288.0, 0.0),
        Point3::new(999000000.0, 524288.0, 0.875),
    ];
    let tolerance = Tolerance::for_plate(&points).unwrap();
    assert_eq!(tolerance.local_size_mm().to_bits(), 655360.0_f64.to_bits());
    assert_eq!(tolerance.eps_geom_mm().to_bits(), 0.15625_f64.to_bits());
    tolerance.check_dimension(10.0).unwrap();
    let plane = Plane3::new(points[0], Vec3::Z).unwrap();
    assert_eq!(plane.signed_distance(points[1]).unwrap(), 0.875);
    assert_eq!(plane.within_planarity_band(points[1], tolerance), Ok(false));
}

#[test]
fn f06_pos_14_inclusive_absolute_minimum() {
    let points = [
        Point3::ORIGIN,
        Point3::new(1000.0, 0.0, 0.0),
        Point3::new(1000.0, 500.0, 0.0),
        Point3::new(0.0, 500.0, 0.0),
    ];
    let t = Tolerance::for_plate(&points).unwrap();
    t.check_dimension(0.001).unwrap();
    let m = MemberFrame::new(
        Point3::ORIGIN,
        Point3::new(0.001, 0.0, 0.0),
        200.0,
        300.0,
        None,
    )
    .unwrap();
    assert_eq!(m.scale().axis_length_mm().to_bits(), 0.001_f64.to_bits());
    assert!(
        !t.coincident(Point3::ORIGIN, Point3::new(0.001, 0.0, 0.0))
            .unwrap()
    );
    assert!(
        t.coincident(Point3::ORIGIN, Point3::new(0.001_f64.next_down(), 0.0, 0.0))
            .unwrap()
    );
}

#[test]
fn f06_neg_02_nonfinite_translation() {
    assert_eq!(
        Translation3::new(f64::NEG_INFINITY, 0.0, 0.0),
        Err(MathError::NonFinite)
    );
}

#[test]
fn f06_neg_09_scale_shear_reflection_cannot_form_a_body_map() {
    for axes in [
        [Vec3::new(2.0, 0.0, 0.0).unwrap(), Vec3::Y, Vec3::Z],
        [Vec3::X, Vec3::new(1.0, 1.0, 0.0).unwrap(), Vec3::Z],
        [Vec3::X, Vec3::Y, Vec3::new(0.0, 0.0, -1.0).unwrap()],
    ] {
        let [x, y, z] = axes;
        assert_eq!(Frame3::try_new(x, y, z), Err(MathError::InvalidFrame));
    }
}

#[test]
fn f06_neg_31_positive_dot_injection() {
    let y = Vec3::new(2e-6, 1.0, 0.0).unwrap();
    assert_eq!(
        Frame3::try_new(Vec3::X, y, Vec3::Z),
        Err(MathError::InvalidFrame)
    );
    let c = FrameCertificate::measure(Vec3::X, y, Vec3::Z).unwrap();
    assert_eq!(c.absolute_dots[0], 2e-6);
    assert!(!c.is_valid());
}

#[test]
fn f06_neg_35_negative_dot_injection() {
    let y = Vec3::new(-2e-6, 1.0, 0.0).unwrap();
    assert_eq!(
        Frame3::try_new(Vec3::X, y, Vec3::Z),
        Err(MathError::InvalidFrame)
    );
    let c = FrameCertificate::measure(Vec3::X, y, Vec3::Z).unwrap();
    assert_eq!(c.absolute_dots[0], 2e-6);
    assert!(!c.is_valid());
}

#[test]
fn f06_pos_50_zero_roll_bypasses_trig_bit_exactly() {
    for roll in [Some(0.0), Some(-0.0)] {
        let a = MemberFrame::new(
            Point3::ORIGIN,
            Point3::new(4000.0, 0.0, 0.0),
            200.0,
            300.0,
            None,
        )
        .unwrap();
        let b = MemberFrame::new(
            Point3::ORIGIN,
            Point3::new(4000.0, 0.0, 0.0),
            200.0,
            300.0,
            roll,
        )
        .unwrap();
        assert_bits(a.frame().x().components(), b.frame().x().components());
        assert_bits(a.frame().y().components(), b.frame().y().components());
        assert_bits(a.frame().z().components(), b.frame().z().components());
    }
}

#[test]
fn finite_vector_point_plane_and_numeric_failures() {
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(Vec3::new(value, 0.0, 0.0), Err(MathError::NonFinite));
        assert_eq!(Point3::try_new(0.0, value, 0.0), Err(MathError::NonFinite));
        assert_eq!(
            Point3::new(value, 0.0, 0.0).checked_distance_to(Point3::ORIGIN),
            Err(MathError::NonFinite)
        );
        assert_eq!(
            Plane3::new(Point3::new(0.0, 0.0, value), Vec3::Z),
            Err(MathError::NonFinite)
        );
        assert_eq!(
            RigidTransform::IDENTITY.apply_point(Point3::new(0.0, value, 0.0)),
            Err(MathError::NonFinite)
        );
    }
    assert_eq!(Vec3::ZERO.normalized(), Err(MathError::ZeroAxis));
    assert_eq!(
        Plane3::new(Point3::ORIGIN, Vec3::ZERO),
        Err(MathError::ZeroAxis)
    );
    assert_eq!(
        Frame3::try_new(Vec3::ZERO, Vec3::Y, Vec3::Z),
        Err(MathError::InvalidFrame)
    );
    let huge = Vec3::new(f64::MAX, 0.0, 0.0).unwrap();
    assert_eq!(huge.norm(), Err(MathError::NumericUncertain));
    assert_eq!(huge.scaled(2.0), Err(MathError::NumericUncertain));
    assert_eq!(
        Point3::new(-f64::MAX, 0.0, 0.0).vector_to(Point3::new(f64::MAX, 0.0, 0.0)),
        Err(MathError::NumericUncertain)
    );
    let tiny = Vec3::new(f64::from_bits(1), 0.0, 0.0).unwrap();
    assert_eq!(tiny.normalized(), Err(MathError::NumericUncertain));
}

#[test]
fn ordered_arithmetic_and_body_transform() {
    let a = Vec3::new(1e16, -1e16, 1.0).unwrap();
    assert_eq!(
        a.dot(Vec3::new(1.0, 1.0, 1.0).unwrap()).unwrap().to_bits(),
        1.0_f64.to_bits()
    );
    assert_eq!(
        Point3::ORIGIN.checked_distance_to(Point3::new(3.0, 4.0, 12.0)),
        Ok(13.0)
    );
    assert_eq!(Vec3::X.cross(Vec3::Y), Ok(Vec3::Z));
    let frame = Frame3::try_new(Vec3::Y, Vec3::Z, Vec3::X).unwrap();
    let map = RigidTransform::from_body_frame(frame, Point3::new(100.0, 200.0, 300.0)).unwrap();
    assert_eq!(
        map.apply_point(Point3::new(3.0, 4.0, 12.0)),
        Ok(Point3::new(112.0, 203.0, 304.0))
    );
    assert_eq!(map.apply_vector(Vec3::X), Ok(Vec3::Y));
    let plane = Plane3::new(
        Point3::new(0.0, 0.0, 3.0),
        Vec3::new(0.0, 0.0, 2.0).unwrap(),
    )
    .unwrap();
    assert_eq!(
        plane.signed_distance(Point3::new(100.0, 20.0, 8.0)),
        Ok(5.0)
    );
    assert_eq!(plane.signed_distance(Point3::ORIGIN), Ok(-3.0));
}

#[test]
fn certificate_inclusive_bound_and_all_residuals() {
    let c = FrameCertificate {
        unit_deviations: [1e-6; 3],
        absolute_dots: [1e-6; 3],
        handedness_deviation: 1e-6,
    };
    assert!(c.is_valid());
    for v in [1e-6_f64.next_up(), f64::NAN, f64::INFINITY, -1e-6] {
        assert!(
            !FrameCertificate {
                handedness_deviation: v,
                ..c
            }
            .is_valid()
        );
        for slot in 0..3 {
            let mut d = c;
            d.unit_deviations[slot] = v;
            assert!(!d.is_valid());
            let mut d = c;
            d.absolute_dots[slot] = v;
            assert!(!d.is_valid());
        }
    }
}

#[test]
fn canonical_encoding_and_checked_output_zeros() {
    assert_eq!(canonical_f64_bytes(-0.0), Ok([0; 8]));
    assert_eq!(canonical_f64_bytes(1.0), Ok([0x3f, 0xf0, 0, 0, 0, 0, 0, 0]));
    assert_eq!(canonical_f64_bytes(f64::NAN), Err(MathError::NonFinite));
    assert_eq!(
        canonical_f64_bytes(f64::INFINITY),
        Err(MathError::NonFinite)
    );
    assert_bits(Vec3::new(-0.0, -0.0, -0.0).unwrap().components(), [0.0; 3]);
    let p = Translation3::IDENTITY
        .apply_point(Point3::new(-0.0, -0.0, -0.0))
        .unwrap();
    assert_bits([p.x, p.y, p.z], [0.0; 3]);
}

#[test]
fn stage_order_caps_counts_and_inclusive_limits() {
    assert_eq!(
        MemberFrame::new(
            Point3::ORIGIN,
            Point3::new(2e9, 0.0, 0.0),
            2e6,
            300.0,
            Some(f64::NAN)
        ),
        Err(MathError::NonFinite)
    );
    assert_eq!(
        Tolerance::for_plate(&[Point3::ORIGIN; 1025]),
        Err(MathError::AboveLimit)
    );
    assert_eq!(
        Tolerance::for_plate(&[Point3::ORIGIN; 2]),
        Err(MathError::InvalidPointCount)
    );
    assert_eq!(
        Translation3::new(1e9, 0.0, -1e9).unwrap().offset(),
        Vec3::new(1e9, 0.0, -1e9).unwrap()
    );
    assert_eq!(
        Translation3::new(1e9_f64.next_up(), 0.0, 0.0),
        Err(MathError::AboveLimit)
    );
    for roll in [-ROLL_CAP_RAD, ROLL_CAP_RAD] {
        assert!(
            MemberFrame::new(
                Point3::ORIGIN,
                Point3::new(1e6, 0.0, 0.0),
                1e6,
                1e6,
                Some(roll)
            )
            .is_ok()
        );
    }
    assert_eq!(
        MemberFrame::new(
            Point3::ORIGIN,
            Point3::new(1000.0, 0.0, 0.0),
            200.0,
            300.0,
            Some(ROLL_CAP_RAD.next_up())
        ),
        Err(MathError::AboveLimit)
    );
}

#[test]
fn equality_uses_horizontal_rule_and_adjacent_values_keep_written_comparison() {
    // hh == binary64(1e-12)*LL at this representable axis.
    let dz = 999.9999999995_f64;
    let dx = 0.001_f64;
    let hh = dx * dx;
    let ll = hh + dz * dz;
    assert_eq!(hh.to_bits(), (1e-12 * ll).to_bits());
    let at =
        MemberFrame::new(Point3::ORIGIN, Point3::new(dx, 0.0, dz), 200.0, 300.0, None).unwrap();
    assert_eq!(at.rule(), MemberFrameRule::Horizontal);
    for z in [dz.next_down(), dz.next_up()] {
        let ll = hh + z * z;
        let expected = if hh < 1e-12 * ll {
            MemberFrameRule::NearVertical
        } else {
            MemberFrameRule::Horizontal
        };
        let frame =
            MemberFrame::new(Point3::ORIGIN, Point3::new(dx, 0.0, z), 200.0, 300.0, None).unwrap();
        assert_eq!(frame.rule(), expected);
    }
}

#[test]
fn transform_order_has_independent_binary64_reference() {
    let frame = Frame3::try_new(
        Vec3::new(0.6, 0.8, 0.0).unwrap(),
        Vec3::new(-0.8, 0.6, 0.0).unwrap(),
        Vec3::Z,
    )
    .unwrap();
    let map = RigidTransform::from_body_frame(frame, Point3::new(10.0, 20.0, 30.0)).unwrap();
    let p = map.apply_point(Point3::new(1e16, -1e16, 1.0)).unwrap();
    // Independently evaluated scalar binary64 expressions, with the final translation last.
    assert_eq!(
        [p.x.to_bits(), p.y.to_bits(), p.z.to_bits()],
        [
            0x4348_de76_816d_8005,
            0x431c_6bf5_2634_0050,
            0x403f_0000_0000_0000
        ]
    );
}

#[test]
fn synthetic_axes_cover_both_signs_meridians_and_scales() {
    for dx in [-300000.0, -1000.0, -0.0005, 0.0, 0.0005, 1000.0, 300000.0] {
        for dy in [-300000.0, -1000.0, -0.0005, 0.0, 0.0005, 1000.0, 300000.0] {
            for dz in [-300000.0, -1000.0, 0.0, 1000.0, 300000.0] {
                let end = Point3::new(dx, dy, dz);
                if end == Point3::ORIGIN {
                    continue;
                }
                // Some tiny horizontal axes are deliberately below the absolute minimum.
                match MemberFrame::new(Point3::ORIGIN, end, 200.0, 300.0, None) {
                    Ok(m) => {
                        assert!(m.frame().certificate().is_valid());
                        assert!(m.frame().x().dot(m.frame().z()).unwrap().abs() <= 1e-6);
                        assert!(m.frame().y().dot(m.frame().z()).unwrap().abs() <= 1e-6);
                    }
                    Err(e) => {
                        assert_eq!(e, MathError::AxisTooShort);
                        assert!(Point3::ORIGIN.distance_to(end) < 0.001);
                    }
                }
            }
        }
    }
}
