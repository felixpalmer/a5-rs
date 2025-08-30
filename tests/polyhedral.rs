// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use a5::coordinate_systems::{Cartesian, Face, FaceTriangle, SphericalTriangle};
use a5::projections::polyhedral::PolyhedralProjection;
use a5::utils::vector::{vec3_distance, vec3_length};
use approx::assert_relative_eq;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
struct TestCase {
    input: [f64; 3],
    expected: [f64; 2],
}

#[derive(Debug, Deserialize, Serialize)]
struct InverseTestCase {
    input: [f64; 2],
    expected: [f64; 3],
}

#[derive(Debug, Deserialize, Serialize)]
struct StaticData {
    #[serde(rename = "TEST_SPHERICAL_TRIANGLE")]
    test_spherical_triangle: [[f64; 3]; 3],
    #[serde(rename = "TEST_FACE_TRIANGLE")]
    test_face_triangle: [[f64; 2]; 3],
}

#[derive(Debug, Deserialize, Serialize)]
struct TestData {
    forward: Vec<TestCase>,
    inverse: Vec<InverseTestCase>,
    #[serde(rename = "static")]
    static_data: StaticData,
}

const TOLERANCE: f64 = 1e-10;

fn load_test_data() -> TestData {
    let test_data_json = include_str!("fixtures/polyhedral.json");
    serde_json::from_str(test_data_json).expect("Failed to parse test data")
}

fn array_to_cartesian(arr: [f64; 3]) -> Cartesian {
    Cartesian::new(arr[0], arr[1], arr[2])
}

fn array_to_face(arr: [f64; 2]) -> Face {
    Face::new(arr[0], arr[1])
}

fn arrays_to_spherical_triangle(arrays: [[f64; 3]; 3]) -> SphericalTriangle {
    SphericalTriangle::new(
        array_to_cartesian(arrays[0]),
        array_to_cartesian(arrays[1]),
        array_to_cartesian(arrays[2]),
    )
}

fn arrays_to_face_triangle(arrays: [[f64; 2]; 3]) -> FaceTriangle {
    FaceTriangle::new(
        array_to_face(arrays[0]),
        array_to_face(arrays[1]),
        array_to_face(arrays[2]),
    )
}

fn vec3_angle(a: &Cartesian, b: &Cartesian) -> f64 {
    let dot_product = a.x() * b.x() + a.y() * b.y() + a.z() * b.z();
    let len_a = vec3_length(a);
    let len_b = vec3_length(b);
    let cos_angle = dot_product / (len_a * len_b);
    // Clamp to avoid numerical errors
    cos_angle.clamp(-1.0, 1.0).acos()
}

#[test]
fn test_polyhedral_forward_projections() {
    let test_data = load_test_data();
    let polyhedral = PolyhedralProjection::new();

    let spherical_triangle =
        arrays_to_spherical_triangle(test_data.static_data.test_spherical_triangle);
    let face_triangle = arrays_to_face_triangle(test_data.static_data.test_face_triangle);

    for test_case in test_data.forward {
        let input = array_to_cartesian(test_case.input);
        let expected = array_to_face(test_case.expected);

        let result = polyhedral.forward(input, spherical_triangle, face_triangle);

        assert_relative_eq!(result.x(), expected.x(), epsilon = TOLERANCE);
        assert_relative_eq!(result.y(), expected.y(), epsilon = TOLERANCE);
    }
}

#[test]
fn test_polyhedral_round_trip_forward() {
    let test_data = load_test_data();
    let polyhedral = PolyhedralProjection::new();

    let spherical_triangle =
        arrays_to_spherical_triangle(test_data.static_data.test_spherical_triangle);
    let face_triangle = arrays_to_face_triangle(test_data.static_data.test_face_triangle);

    let mut largest_error: f64 = 0.0;

    for test_case in test_data.forward {
        let spherical = array_to_cartesian(test_case.input);
        let polar = polyhedral.forward(spherical, spherical_triangle, face_triangle);
        let result = polyhedral.inverse(polar, face_triangle, spherical_triangle);

        let error = vec3_distance(&result, &spherical);
        largest_error = largest_error.max(error);

        assert_relative_eq!(result.x(), spherical.x(), epsilon = TOLERANCE);
        assert_relative_eq!(result.y(), spherical.y(), epsilon = TOLERANCE);
        assert_relative_eq!(result.z(), spherical.z(), epsilon = TOLERANCE);
    }

    // Test accuracy to specified precision
    const AUTHALIC_RADIUS: f64 = 6371.0072; // km
    let max_angle = [
        vec3_angle(&spherical_triangle.a, &spherical_triangle.b),
        vec3_angle(&spherical_triangle.b, &spherical_triangle.c),
        vec3_angle(&spherical_triangle.c, &spherical_triangle.a),
    ]
    .iter()
    .fold(0.0_f64, |a, &b| a.max(b));

    let max_arc_length_mm = AUTHALIC_RADIUS * max_angle * 1e9;
    const DESIRED_MM_PRECISION: f64 = 0.01;

    assert!(largest_error * max_arc_length_mm < DESIRED_MM_PRECISION);
}

#[test]
fn test_polyhedral_inverse_projections() {
    let test_data = load_test_data();
    let polyhedral = PolyhedralProjection::new();

    let spherical_triangle =
        arrays_to_spherical_triangle(test_data.static_data.test_spherical_triangle);
    let face_triangle = arrays_to_face_triangle(test_data.static_data.test_face_triangle);

    for test_case in test_data.inverse {
        let input = array_to_face(test_case.input);
        let expected = array_to_cartesian(test_case.expected);

        let result = polyhedral.inverse(input, face_triangle, spherical_triangle);

        assert_relative_eq!(result.x(), expected.x(), epsilon = TOLERANCE);
        assert_relative_eq!(result.y(), expected.y(), epsilon = TOLERANCE);
        assert_relative_eq!(result.z(), expected.z(), epsilon = TOLERANCE);
    }
}

#[test]
fn test_polyhedral_round_trip_inverse() {
    let test_data = load_test_data();
    let polyhedral = PolyhedralProjection::new();

    let spherical_triangle =
        arrays_to_spherical_triangle(test_data.static_data.test_spherical_triangle);
    let face_triangle = arrays_to_face_triangle(test_data.static_data.test_face_triangle);

    for test_case in test_data.inverse {
        let face_point = array_to_face(test_case.input);
        let spherical = polyhedral.inverse(face_point, face_triangle, spherical_triangle);
        let result = polyhedral.forward(spherical, spherical_triangle, face_triangle);

        assert_relative_eq!(result.x(), face_point.x(), epsilon = TOLERANCE);
        assert_relative_eq!(result.y(), face_point.y(), epsilon = TOLERANCE);
    }
}
