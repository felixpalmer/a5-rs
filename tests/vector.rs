// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use a5_rs::coordinate_systems::Cartesian;
use a5_rs::utils::vector::{quadruple_product, slerp, vector_difference};

const TOLERANCE: f64 = 1e-6;

fn close_to(a: f64, b: f64, tolerance: f64) -> bool {
    (a - b).abs() < tolerance
}

fn close_to_array(a: Cartesian, b: [f64; 3], tolerance: f64) -> bool {
    close_to(a.x(), b[0], tolerance)
        && close_to(a.y(), b[1], tolerance)
        && close_to(a.z(), b[2], tolerance)
}

fn normalize_vector(v: Cartesian) -> Cartesian {
    let len = (v.x() * v.x() + v.y() * v.y() + v.z() * v.z()).sqrt();
    if len == 0.0 {
        return v;
    }
    Cartesian::new(v.x() / len, v.y() / len, v.z() / len)
}

#[test]
fn test_vector_difference_identical_vectors() {
    let a = Cartesian::new(1.0, 0.0, 0.0);
    let b = Cartesian::new(1.0, 0.0, 0.0);
    let result = vector_difference(a, b);
    assert!(close_to(result, 0.0, TOLERANCE));
}

#[test]
fn test_vector_difference_perpendicular_vectors() {
    let a = Cartesian::new(1.0, 0.0, 0.0);
    let b = Cartesian::new(0.0, 1.0, 0.0);
    let result = vector_difference(a, b);
    assert!(close_to(result, (0.5_f64).sqrt(), TOLERANCE));
}

#[test]
fn test_vector_difference_small_angles() {
    let a = Cartesian::new(1.0, 0.0, 0.0);
    let b = normalize_vector(Cartesian::new(0.999, 0.001, 0.0));
    let result = vector_difference(a, b);
    assert!(result > 0.0);
    assert!(result < 0.1);
}

#[test]
fn test_quadruple_product_basic() {
    let a = Cartesian::new(1.0, 0.0, 0.0);
    let b = Cartesian::new(0.0, 1.0, 0.0);
    let c = Cartesian::new(0.0, 0.0, 1.0);
    let d = normalize_vector(Cartesian::new(1.0, 1.0, 1.0));

    let result = quadruple_product(a, b, c, d);

    // Result should be a 3D vector
    assert!(result.x().is_finite());
    assert!(result.y().is_finite());
    assert!(result.z().is_finite());
}

#[test]
fn test_quadruple_product_orthogonal() {
    let a = Cartesian::new(1.0, 0.0, 0.0);
    let b = Cartesian::new(0.0, 1.0, 0.0);
    let c = Cartesian::new(0.0, 0.0, 1.0);
    let d = Cartesian::new(1.0, 0.0, 0.0);

    let result = quadruple_product(a, b, c, d);

    // The quadruple product of these vectors should be non-zero
    assert!(result.x() != 0.0 || result.y() != 0.0 || result.z() != 0.0);
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
