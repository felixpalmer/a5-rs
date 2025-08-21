// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use a5_rs::coordinate_systems::Face;
use a5_rs::core::pentagon::{
    a, b, c, d, e, pentagon, u, v, w, v_angle, triangle, basis, basis_inverse, A, B, C, D, E,
    Mat2,
};

const TOLERANCE: f64 = 1e-10;

fn close_to(a: f64, b: f64, tolerance: f64) -> bool {
    (a - b).abs() < tolerance
}

#[test]
fn test_pentagon_angles() {
    assert_eq!(A.get(), 72.0);
    assert_eq!(B.get(), 127.94543761193603);
    assert_eq!(C.get(), 108.0);
    assert_eq!(D.get(), 82.29202980963508);
    assert_eq!(E.get(), 149.7625318412527);
}

#[test]
fn test_pentagon_vertices() {
    // Test vertex a
    assert!(close_to(a().x(), 0.0, TOLERANCE));
    assert!(close_to(a().y(), 0.0, TOLERANCE));

    // Test vertex b
    assert!(close_to(b().x(), 0.1993818474311588, TOLERANCE));
    assert!(close_to(b().y(), 0.3754138223914238, TOLERANCE));

    // Test vertex c
    assert!(close_to(c().x(), 0.6180339887498949, TOLERANCE));
    assert!(close_to(c().y(), 0.4490279765795854, TOLERANCE));

    // Test vertex d
    assert!(close_to(d().x(), 0.8174158361810537, TOLERANCE));
    assert!(close_to(d().y(), 0.0736141541881617, TOLERANCE));

    // Test vertex e
    assert!(close_to(e().x(), 0.418652141318736, TOLERANCE));
    assert!(close_to(e().y(), -0.07361415418816161, TOLERANCE));
}

#[test]
fn test_pentagon_shape() {
    let expected = [
        [0.0, 0.0],
        [0.1993818474311588, 0.3754138223914238],
        [0.6180339887498949, 0.4490279765795854],
        [0.8174158361810537, 0.0736141541881617],
        [0.418652141318736, -0.07361415418816161],
    ];

    let vertices = pentagon().get_vertices();
    for (i, expected_vertex) in expected.iter().enumerate() {
        assert!(
            close_to(vertices[i].x(), expected_vertex[0], TOLERANCE),
            "Pentagon vertex {}: expected x={}, got x={}",
            i,
            expected_vertex[0],
            vertices[i].x()
        );
        assert!(
            close_to(vertices[i].y(), expected_vertex[1], TOLERANCE),
            "Pentagon vertex {}: expected y={}, got y={}",
            i,
            expected_vertex[1],
            vertices[i].y()
        );
    }
}

#[test]
fn test_triangle_vertices() {
    // Test vertex u
    assert!(close_to(u().x(), 0.0, TOLERANCE));
    assert!(close_to(u().y(), 0.0, TOLERANCE));

    // Test vertex v
    assert!(close_to(v().x(), 0.6180339887498949, TOLERANCE));
    assert!(close_to(v().y(), 0.4490279765795854, TOLERANCE));

    // Test vertex w
    assert!(close_to(w().x(), 0.6180339887498949, TOLERANCE));
    assert!(close_to(w().y(), -0.4490279765795854, TOLERANCE));

    // Test angle V
    assert!(close_to(v_angle().get(), 0.6283185307179586, TOLERANCE));
}

#[test]
fn test_triangle_shape() {
    let expected = [
        [0.0, 0.0],
        [0.6180339887498949, 0.4490279765795854],
        [0.6180339887498949, -0.4490279765795854],
    ];

    let vertices = triangle().get_vertices();
    for (i, expected_vertex) in expected.iter().enumerate().take(3) {
        assert!(
            close_to(vertices[i].x(), expected_vertex[0], TOLERANCE),
            "Triangle vertex {}: expected x={}, got x={}",
            i,
            expected_vertex[0],
            vertices[i].x()
        );
        assert!(
            close_to(vertices[i].y(), expected_vertex[1], TOLERANCE),
            "Triangle vertex {}: expected y={}, got y={}",
            i,
            expected_vertex[1],
            vertices[i].y()
        );
    }
}

#[test]
fn test_basis_matrices() {
    let expected_basis = [
        0.6180339887498949,
        0.4490279765795854,
        0.6180339887498949,
        -0.4490279765795854,
    ];

    let expected_inverse = [
        0.8090169943749475,
        0.8090169943749475,
        1.1135163644116068,
        -1.1135163644116068,
    ];

    let basis_mat = basis();
    let basis_inverse_mat = basis_inverse();

    // Test basis matrix values
    assert!(close_to(basis_mat.m00, expected_basis[0], TOLERANCE));
    assert!(close_to(basis_mat.m10, expected_basis[1], TOLERANCE));
    assert!(close_to(basis_mat.m01, expected_basis[2], TOLERANCE));
    assert!(close_to(basis_mat.m11, expected_basis[3], TOLERANCE));

    // Test inverse basis matrix values
    assert!(close_to(basis_inverse_mat.m00, expected_inverse[0], TOLERANCE));
    assert!(close_to(basis_inverse_mat.m10, expected_inverse[1], TOLERANCE));
    assert!(close_to(basis_inverse_mat.m01, expected_inverse[2], TOLERANCE));
    assert!(close_to(basis_inverse_mat.m11, expected_inverse[3], TOLERANCE));
}

#[test]
fn test_basis_matrix_multiplication_identity() {
    let basis_mat = basis();
    let basis_inverse_mat = basis_inverse();

    // Test that BASIS * BASIS_INVERSE = Identity
    // Manual matrix multiplication: result = basis * basis_inverse
    let r00 = basis_mat.m00 * basis_inverse_mat.m00 + basis_mat.m01 * basis_inverse_mat.m10;
    let r01 = basis_mat.m00 * basis_inverse_mat.m01 + basis_mat.m01 * basis_inverse_mat.m11;
    let r10 = basis_mat.m10 * basis_inverse_mat.m00 + basis_mat.m11 * basis_inverse_mat.m10;
    let r11 = basis_mat.m10 * basis_inverse_mat.m01 + basis_mat.m11 * basis_inverse_mat.m11;

    // Should be identity matrix [[1, 0], [0, 1]]
    assert!(
        close_to(r00, 1.0, TOLERANCE),
        "Expected 1.0, got {}",
        r00
    );
    assert!(
        close_to(r01, 0.0, TOLERANCE),
        "Expected 0.0, got {}",
        r01
    );
    assert!(
        close_to(r10, 0.0, TOLERANCE),
        "Expected 0.0, got {}",
        r10
    );
    assert!(
        close_to(r11, 1.0, TOLERANCE),
        "Expected 1.0, got {}",
        r11
    );
}

#[test]
fn test_mat2_operations() {
    let mat = Mat2::new(2.0, 1.0, 3.0, 4.0);

    // Test determinant
    let det = mat.determinant();
    assert!(close_to(det, 5.0, TOLERANCE)); // 2*4 - 1*3 = 5

    // Test inverse
    let inv = mat.inverse().unwrap();
    assert!(close_to(inv.m00, 0.8, TOLERANCE)); // 4/5
    assert!(close_to(inv.m01, -0.2, TOLERANCE)); // -1/5
    assert!(close_to(inv.m10, -0.6, TOLERANCE)); // -3/5
    assert!(close_to(inv.m11, 0.4, TOLERANCE)); // 2/5

    // Test transform
    let point = Face::new(1.0, 2.0);
    let transformed = mat.transform(point);
    assert!(close_to(transformed.x(), 4.0, TOLERANCE)); // 2*1 + 1*2
    assert!(close_to(transformed.y(), 11.0, TOLERANCE)); // 3*1 + 4*2
}

#[test]
fn test_singleton_behavior() {
    // Test that accessing pentagon vertices multiple times returns the same values
    let a1 = a();
    let a2 = a();
    assert_eq!(a1, a2);

    let pentagon1 = pentagon();
    let pentagon2 = pentagon();
    // Compare some properties since PentagonShape doesn't implement PartialEq
    assert_eq!(pentagon1.get_vertices(), pentagon2.get_vertices());
}