// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use a5_rs::coordinate_systems::{Polar, Radians, Spherical};
use a5_rs::projections::GnomonicProjection;
use serde_json::Value;

const TOLERANCE: f64 = 1e-10;

fn close_to(a: f64, b: f64, tolerance: f64) -> bool {
    (a - b).abs() < tolerance
}

fn close_to_polar(a: Polar, b: [f64; 2], tolerance: f64) -> bool {
    close_to(a.rho(), b[0], tolerance) && close_to(a.gamma().get(), b[1], tolerance)
}

fn close_to_spherical(a: Spherical, b: [f64; 2], tolerance: f64) -> bool {
    close_to(a.theta().get(), b[0], tolerance) && close_to(a.phi().get(), b[1], tolerance)
}

#[test]
fn test_gnomonic_forward() {
    let gnomonic = GnomonicProjection;
    let test_data: Value = serde_json::from_str(include_str!("fixtures/gnomonic.json")).unwrap();

    for test_case in test_data["forward"].as_array().unwrap() {
        let input = test_case["input"].as_array().unwrap();
        let expected = test_case["expected"].as_array().unwrap();

        let spherical = Spherical::new(
            Radians::new_unchecked(input[0].as_f64().unwrap()),
            Radians::new_unchecked(input[1].as_f64().unwrap()),
        );

        let result = gnomonic.forward(spherical);
        let expected_polar = [expected[0].as_f64().unwrap(), expected[1].as_f64().unwrap()];

        assert!(
            close_to_polar(result, expected_polar, TOLERANCE),
            "Forward projection failed for input [{}, {}]. Expected {:?}, got [{}, {}]",
            spherical.theta().get(),
            spherical.phi().get(),
            expected_polar,
            result.rho(),
            result.gamma().get()
        );
    }
}

#[test]
fn test_gnomonic_inverse() {
    let gnomonic = GnomonicProjection;
    let test_data: Value = serde_json::from_str(include_str!("fixtures/gnomonic.json")).unwrap();

    for test_case in test_data["inverse"].as_array().unwrap() {
        let input = test_case["input"].as_array().unwrap();
        let expected = test_case["expected"].as_array().unwrap();

        let polar = Polar::new(
            input[0].as_f64().unwrap(),
            Radians::new_unchecked(input[1].as_f64().unwrap()),
        );

        let result = gnomonic.inverse(polar);
        let expected_spherical = [expected[0].as_f64().unwrap(), expected[1].as_f64().unwrap()];

        assert!(
            close_to_spherical(result, expected_spherical, TOLERANCE),
            "Inverse projection failed for input [{}, {}]. Expected {:?}, got [{}, {}]",
            polar.rho(),
            polar.gamma().get(),
            expected_spherical,
            result.theta().get(),
            result.phi().get()
        );
    }
}

#[test]
fn test_gnomonic_forward_round_trip() {
    let gnomonic = GnomonicProjection;
    let test_data: Value = serde_json::from_str(include_str!("fixtures/gnomonic.json")).unwrap();

    for test_case in test_data["forward"].as_array().unwrap() {
        let input = test_case["input"].as_array().unwrap();

        let spherical = Spherical::new(
            Radians::new_unchecked(input[0].as_f64().unwrap()),
            Radians::new_unchecked(input[1].as_f64().unwrap()),
        );

        let polar = gnomonic.forward(spherical);
        let result = gnomonic.inverse(polar);

        assert!(
            close_to_spherical(
                result,
                [spherical.theta().get(), spherical.phi().get()],
                TOLERANCE
            ),
            "Forward round trip failed for input [{}, {}]. Expected [{}, {}], got [{}, {}]",
            spherical.theta().get(),
            spherical.phi().get(),
            spherical.theta().get(),
            spherical.phi().get(),
            result.theta().get(),
            result.phi().get()
        );
    }
}

#[test]
fn test_gnomonic_inverse_round_trip() {
    let gnomonic = GnomonicProjection;
    let test_data: Value = serde_json::from_str(include_str!("fixtures/gnomonic.json")).unwrap();

    for test_case in test_data["inverse"].as_array().unwrap() {
        let input = test_case["input"].as_array().unwrap();

        let polar = Polar::new(
            input[0].as_f64().unwrap(),
            Radians::new_unchecked(input[1].as_f64().unwrap()),
        );

        let spherical = gnomonic.inverse(polar);
        let result = gnomonic.forward(spherical);

        assert!(
            close_to_polar(result, [polar.rho(), polar.gamma().get()], TOLERANCE),
            "Inverse round trip failed for input [{}, {}]. Expected [{}, {}], got [{}, {}]",
            polar.rho(),
            polar.gamma().get(),
            polar.rho(),
            polar.gamma().get(),
            result.rho(),
            result.gamma().get()
        );
    }
}
