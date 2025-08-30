// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use a5::coordinate_systems::Radians;
use a5::projections::AuthalicProjection;
use serde_json::Value;

const TOLERANCE: f64 = 1e-10;
const ROUND_TRIP_TOLERANCE: f64 = 1e-15;

fn close_to(a: f64, b: f64, tolerance: f64) -> bool {
    (a - b).abs() < tolerance
}

#[test]
fn test_authalic_forward() {
    let authalic = AuthalicProjection;
    let test_data: Value = serde_json::from_str(include_str!("fixtures/authalic.json")).unwrap();

    for test_case in test_data["forward"].as_array().unwrap() {
        let input = test_case["input"].as_f64().unwrap();
        let expected = test_case["expected"].as_f64().unwrap();

        let phi = Radians::new_unchecked(input);
        let result = authalic.forward(phi);

        assert!(
            close_to(result.get(), expected, TOLERANCE),
            "Forward projection failed for input {}. Expected {}, got {}",
            input,
            expected,
            result.get()
        );
    }
}

#[test]
fn test_authalic_forward_round_trip() {
    let authalic = AuthalicProjection;
    let test_data: Value = serde_json::from_str(include_str!("fixtures/authalic.json")).unwrap();

    for test_case in test_data["forward"].as_array().unwrap() {
        let input = test_case["input"].as_f64().unwrap();

        let phi = Radians::new_unchecked(input);
        let authalic_lat = authalic.forward(phi);
        let result = authalic.inverse(authalic_lat);

        assert!(
            close_to(result.get(), input, ROUND_TRIP_TOLERANCE),
            "Forward round trip failed for input {}. Expected {}, got {}",
            input,
            input,
            result.get()
        );
    }
}

#[test]
fn test_authalic_inverse() {
    let authalic = AuthalicProjection;
    let test_data: Value = serde_json::from_str(include_str!("fixtures/authalic.json")).unwrap();

    for test_case in test_data["inverse"].as_array().unwrap() {
        let input = test_case["input"].as_f64().unwrap();
        let expected = test_case["expected"].as_f64().unwrap();

        let phi = Radians::new_unchecked(input);
        let result = authalic.inverse(phi);

        assert!(
            close_to(result.get(), expected, TOLERANCE),
            "Inverse projection failed for input {}. Expected {}, got {}",
            input,
            expected,
            result.get()
        );
    }
}

#[test]
fn test_authalic_inverse_round_trip() {
    let authalic = AuthalicProjection;
    let test_data: Value = serde_json::from_str(include_str!("fixtures/authalic.json")).unwrap();

    for test_case in test_data["inverse"].as_array().unwrap() {
        let input = test_case["input"].as_f64().unwrap();

        let phi = Radians::new_unchecked(input);
        let geodetic_lat = authalic.inverse(phi);
        let result = authalic.forward(geodetic_lat);

        assert!(
            close_to(result.get(), input, ROUND_TRIP_TOLERANCE),
            "Inverse round trip failed for input {}. Expected {}, got {}",
            input,
            input,
            result.get()
        );
    }
}
