use a5_rs::coordinate_systems::{Face, Radians, Spherical};
use a5_rs::projections::DodecahedronProjection;
use a5_rs::core::utils::OriginId;
use approx::assert_relative_eq;
use serde_json::Value;

const TOLERANCE: f64 = 1e-10;

fn load_test_data() -> Value {
    let test_data = include_str!("../tests/fixtures/dodecahedron-test-data.json");
    serde_json::from_str(test_data).expect("Failed to parse test data")
}

#[test]
fn test_dodecahedron_forward_projections() {
    let test_data = load_test_data();
    let origin_id: OriginId = test_data["static"]["ORIGIN_ID"].as_u64().expect("Origin ID should be a number") as u8;
    let mut dodecahedron = DodecahedronProjection::new().expect("Failed to create DodecahedronProjection");

    let forward_tests = test_data["forward"].as_array().expect("Forward tests should be an array");

    for test_case in forward_tests {
        let input = test_case["input"].as_array().expect("Input should be an array");
        let expected = test_case["expected"].as_array().expect("Expected should be an array");

        let spherical = Spherical::new(
            Radians::new_unchecked(input[0].as_f64().expect("Theta should be a number")),
            Radians::new_unchecked(input[1].as_f64().expect("Phi should be a number")),
        );

        let result = dodecahedron.forward(spherical, origin_id).expect("Forward projection should succeed");

        assert_relative_eq!(
            result.x(),
            expected[0].as_f64().expect("Expected X should be a number"),
            epsilon = TOLERANCE
        );
        assert_relative_eq!(
            result.y(),
            expected[1].as_f64().expect("Expected Y should be a number"),
            epsilon = TOLERANCE
        );
    }
}

#[test]
fn test_dodecahedron_inverse_projections() {
    let test_data = load_test_data();
    let origin_id: OriginId = test_data["static"]["ORIGIN_ID"].as_u64().expect("Origin ID should be a number") as u8;
    let mut dodecahedron = DodecahedronProjection::new().expect("Failed to create DodecahedronProjection");

    let inverse_tests = test_data["inverse"].as_array().expect("Inverse tests should be an array");

    for test_case in inverse_tests {
        let input = test_case["input"].as_array().expect("Input should be an array");
        let expected = test_case["expected"].as_array().expect("Expected should be an array");

        let face = Face::new(
            input[0].as_f64().expect("X should be a number"),
            input[1].as_f64().expect("Y should be a number"),
        );

        let result = dodecahedron.inverse(face, origin_id).expect("Inverse projection should succeed");

        assert_relative_eq!(
            result.theta().get(),
            expected[0].as_f64().expect("Expected theta should be a number"),
            epsilon = TOLERANCE
        );
        assert_relative_eq!(
            result.phi().get(),
            expected[1].as_f64().expect("Expected phi should be a number"),
            epsilon = TOLERANCE
        );
    }
}

#[test]
fn test_dodecahedron_forward_round_trip() {
    let test_data = load_test_data();
    let origin_id: OriginId = test_data["static"]["ORIGIN_ID"].as_u64().expect("Origin ID should be a number") as u8;
    let mut dodecahedron = DodecahedronProjection::new().expect("Failed to create DodecahedronProjection");

    let forward_tests = test_data["forward"].as_array().expect("Forward tests should be an array");

    for test_case in forward_tests.iter().take(20) { // Test first 20 for performance
        let input = test_case["input"].as_array().expect("Input should be an array");

        let spherical = Spherical::new(
            Radians::new_unchecked(input[0].as_f64().expect("Theta should be a number")),
            Radians::new_unchecked(input[1].as_f64().expect("Phi should be a number")),
        );

        let face = dodecahedron.forward(spherical, origin_id).expect("Forward projection should succeed");
        let result = dodecahedron.inverse(face, origin_id).expect("Inverse projection should succeed");

        assert_relative_eq!(
            result.theta().get(),
            spherical.theta().get(),
            epsilon = TOLERANCE
        );
        assert_relative_eq!(
            result.phi().get(),
            spherical.phi().get(),
            epsilon = TOLERANCE
        );
    }
}

#[test]
fn test_dodecahedron_inverse_round_trip() {
    let test_data = load_test_data();
    let origin_id: OriginId = test_data["static"]["ORIGIN_ID"].as_u64().expect("Origin ID should be a number") as u8;
    let mut dodecahedron = DodecahedronProjection::new().expect("Failed to create DodecahedronProjection");

    let inverse_tests = test_data["inverse"].as_array().expect("Inverse tests should be an array");

    for test_case in inverse_tests.iter().take(20) { // Test first 20 for performance
        let input = test_case["input"].as_array().expect("Input should be an array");

        let face = Face::new(
            input[0].as_f64().expect("X should be a number"),
            input[1].as_f64().expect("Y should be a number"),
        );

        let spherical = dodecahedron.inverse(face, origin_id).expect("Inverse projection should succeed");
        let result = dodecahedron.forward(spherical, origin_id).expect("Forward projection should succeed");

        assert_relative_eq!(
            result.x(),
            face.x(),
            epsilon = TOLERANCE
        );
        assert_relative_eq!(
            result.y(),
            face.y(),
            epsilon = TOLERANCE
        );
    }
}

#[test]
fn test_dodecahedron_error_handling() {
    let mut dodecahedron = DodecahedronProjection::new().expect("Failed to create DodecahedronProjection");
    
    // Test with invalid origin ID
    let spherical = Spherical::new(
        Radians::new_unchecked(0.0),
        Radians::new_unchecked(0.0),
    );
    
    let result = dodecahedron.forward(spherical, 255); // Invalid origin ID
    assert!(result.is_err());

    let face = Face::new(0.0, 0.0);
    let result = dodecahedron.inverse(face, 255); // Invalid origin ID
    assert!(result.is_err());
}