// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use a5::coordinate_systems::Cartesian;
use a5::utils::vector::slerp;

const TOLERANCE: f64 = 1e-6;

fn close_to(a: f64, b: f64, tolerance: f64) -> bool {
    (a - b).abs() < tolerance
}

fn close_to_array(a: Cartesian, b: [f64; 3], tolerance: f64) -> bool {
    close_to(a.x(), b[0], tolerance)
        && close_to(a.y(), b[1], tolerance)
        && close_to(a.z(), b[2], tolerance)
}

#[test]
fn test_slerp_interpolation() {
    let a = Cartesian::new(1.0, 0.0, 0.0);
    let b = Cartesian::new(0.0, 1.0, 0.0);

    let result = slerp(a, b, 0.5);

    let expected = [1.0 / 2.0_f64.sqrt(), 1.0 / 2.0_f64.sqrt(), 0.0];
    assert!(close_to_array(result, expected, TOLERANCE));
}

#[test]
fn test_slerp_at_t_zero() {
    let a = Cartesian::new(1.0, 0.0, 0.0);
    let b = Cartesian::new(0.0, 1.0, 0.0);

    let result = slerp(a, b, 0.0);

    assert!(close_to_array(result, [1.0, 0.0, 0.0], TOLERANCE));
}

#[test]
fn test_slerp_at_t_one() {
    let a = Cartesian::new(1.0, 0.0, 0.0);
    let b = Cartesian::new(0.0, 1.0, 0.0);

    let result = slerp(a, b, 1.0);

    assert!(close_to_array(result, [0.0, 1.0, 0.0], TOLERANCE));
}

#[test]
fn test_slerp_identical_vectors() {
    let a = Cartesian::new(1.0, 0.0, 0.0);
    let b = Cartesian::new(1.0, 0.0, 0.0);

    let result = slerp(a, b, 0.5);

    // For identical vectors, slerp should return the same vector
    assert!(close_to_array(result, [1.0, 0.0, 0.0], TOLERANCE));
}

#[test]
fn test_slerp_different_t_values() {
    let a = Cartesian::new(1.0, 0.0, 0.0);
    let b = Cartesian::new(0.0, 1.0, 0.0);

    let result1 = slerp(a, b, 0.25);
    let result2 = slerp(a, b, 0.75);

    // At 0.25, should be closer to A (larger x component)
    assert!(result1.x() > result1.y());

    // At 0.75, should be closer to B (larger y component)
    assert!(result2.y() > result2.x());
}
