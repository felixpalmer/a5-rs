// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use a5_rs::coordinate_systems::{Radians, Spherical};
use a5_rs::core::constants::PI_OVER_5;
use a5_rs::core::coordinate_transforms::to_cartesian;
use a5_rs::core::origin::{
    find_nearest_origin, get_origins, haversine, is_nearest_origin, quintant_to_segment,
    segment_to_quintant,
};
use a5_rs::utils::vector::vec3_length;
use approx::assert_abs_diff_eq;
use serde_json::Value;

const TOLERANCE: f64 = 1e-10;

fn load_fixture() -> Value {
    let fixture_data = include_str!("fixtures/origins.json");
    serde_json::from_str(fixture_data).expect("Failed to parse origins fixtures")
}

fn quat_length(q: &[f64; 4]) -> f64 {
    (q[0] * q[0] + q[1] * q[1] + q[2] * q[2] + q[3] * q[3]).sqrt()
}

#[test]
fn test_origins_count() {
    let origins = get_origins();
    assert_eq!(origins.len(), 12);
}

#[test]
fn test_origins_match_fixture() {
    let origins = get_origins();
    let fixture = load_fixture();

    assert_eq!(origins.len(), fixture.as_array().unwrap().len());

    for (origin, expected) in origins.iter().zip(fixture.as_array().unwrap()) {
        // Check id
        assert_eq!(origin.id, expected["id"].as_u64().unwrap() as u8);

        // Check axis (spherical coordinates)
        let expected_axis = expected["axis"].as_array().unwrap();
        assert_abs_diff_eq!(
            origin.axis.theta().get(),
            expected_axis[0].as_f64().unwrap(),
            epsilon = TOLERANCE
        );
        assert_abs_diff_eq!(
            origin.axis.phi().get(),
            expected_axis[1].as_f64().unwrap(),
            epsilon = TOLERANCE
        );

        // Check quaternion
        let expected_quat = expected["quat"].as_array().unwrap();
        for i in 0..4 {
            assert_abs_diff_eq!(
                origin.quat[i],
                expected_quat[i].as_f64().unwrap(),
                epsilon = TOLERANCE
            );
        }

        // Check angle
        assert_abs_diff_eq!(
            origin.angle.get(),
            expected["angle"].as_f64().unwrap(),
            epsilon = TOLERANCE
        );

        // Check orientation array
        let expected_orientation = expected["orientation"].as_array().unwrap();
        assert_eq!(origin.orientation.len(), expected_orientation.len());

        // Check firstQuintant
        assert_eq!(
            origin.first_quintant,
            expected["firstQuintant"].as_u64().unwrap() as usize
        );
    }
}

#[test]
fn test_origin_properties() {
    let origins = get_origins();

    for origin in origins {
        // Check axis is unit vector when converted to cartesian
        let cartesian = to_cartesian(origin.axis);
        let length = vec3_length(&cartesian);
        assert_abs_diff_eq!(length, 1.0, epsilon = TOLERANCE);

        // Check quaternion is normalized
        let q_length = quat_length(&origin.quat);
        assert_abs_diff_eq!(q_length, 1.0, epsilon = TOLERANCE);
    }
}

#[test]
fn test_find_nearest_origin_for_face_centers() {
    let origins = get_origins();

    for origin in origins {
        let point = origin.axis;
        let nearest = find_nearest_origin(point);
        assert_eq!(nearest.id, origin.id);
    }
}

#[test]
fn test_find_nearest_origin_for_boundary_points() {
    // Test points halfway between adjacent origins
    let boundary_points = [
        // Between north pole and equatorial faces
        (
            Spherical::new(
                Radians::new_unchecked(0.0),
                Radians::new_unchecked(PI_OVER_5.get() / 2.0),
            ),
            vec![0, 1],
        ),
        // Between equatorial faces
        (
            Spherical::new(
                Radians::new_unchecked(2.0 * PI_OVER_5.get()),
                Radians::new_unchecked(PI_OVER_5.get()),
            ),
            vec![3, 4],
        ),
        // Between equatorial and south pole
        (
            Spherical::new(
                Radians::new_unchecked(0.0),
                Radians::new_unchecked(std::f64::consts::PI - PI_OVER_5.get() / 2.0),
            ),
            vec![9, 10],
        ),
    ];

    for (point, expected_origins) in &boundary_points {
        let nearest = find_nearest_origin(*point);
        assert!(expected_origins.contains(&(nearest.id as i32)));
    }
}

#[test]
fn test_find_nearest_origin_for_antipodal_points() {
    let origins = get_origins();

    // Test points opposite to face centers
    for origin in origins {
        let theta = origin.axis.theta().get();
        let phi = origin.axis.phi().get();
        // Add π to theta and π-phi to get antipodal point
        let antipodal = Spherical::new(
            Radians::new_unchecked(theta + std::f64::consts::PI),
            Radians::new_unchecked(std::f64::consts::PI - phi),
        );

        let nearest = find_nearest_origin(antipodal);
        // Should find one of the faces near the antipodal point
        assert_ne!(nearest.id, origin.id);
    }
}

#[test]
fn test_haversine_identical_points() {
    let point = Spherical::new(Radians::new_unchecked(0.0), Radians::new_unchecked(0.0));
    assert_eq!(haversine(point, point), 0.0);

    let point2 = Spherical::new(
        Radians::new_unchecked(std::f64::consts::PI / 4.0),
        Radians::new_unchecked(std::f64::consts::PI / 3.0),
    );
    assert_eq!(haversine(point2, point2), 0.0);
}

#[test]
fn test_haversine_symmetry() {
    let p1 = Spherical::new(
        Radians::new_unchecked(0.0),
        Radians::new_unchecked(std::f64::consts::PI / 4.0),
    );
    let p2 = Spherical::new(
        Radians::new_unchecked(std::f64::consts::PI / 2.0),
        Radians::new_unchecked(std::f64::consts::PI / 3.0),
    );

    let d1 = haversine(p1, p2);
    let d2 = haversine(p2, p1);

    assert_abs_diff_eq!(d1, d2, epsilon = TOLERANCE);
}

#[test]
fn test_haversine_increases_with_angular_separation() {
    let origin = Spherical::new(Radians::new_unchecked(0.0), Radians::new_unchecked(0.0));
    let distances = [
        Spherical::new(
            Radians::new_unchecked(0.0),
            Radians::new_unchecked(std::f64::consts::PI / 6.0),
        ), // 30°
        Spherical::new(
            Radians::new_unchecked(0.0),
            Radians::new_unchecked(std::f64::consts::PI / 4.0),
        ), // 45°
        Spherical::new(
            Radians::new_unchecked(0.0),
            Radians::new_unchecked(std::f64::consts::PI / 3.0),
        ), // 60°
        Spherical::new(
            Radians::new_unchecked(0.0),
            Radians::new_unchecked(std::f64::consts::PI / 2.0),
        ), // 90°
    ];

    let mut last_distance = 0.0;
    for point in &distances {
        let distance = haversine(origin, *point);
        assert!(distance > last_distance);
        last_distance = distance;
    }
}

#[test]
fn test_haversine_longitude_separation() {
    let lat = std::f64::consts::PI / 4.0; // Fixed latitude
    let p1 = Spherical::new(Radians::new_unchecked(0.0), Radians::new_unchecked(lat));
    let p2 = Spherical::new(
        Radians::new_unchecked(std::f64::consts::PI),
        Radians::new_unchecked(lat),
    );
    let p3 = Spherical::new(
        Radians::new_unchecked(std::f64::consts::PI / 2.0),
        Radians::new_unchecked(lat),
    );

    let d1 = haversine(p1, p2); // 180° separation
    let d2 = haversine(p1, p3); // 90° separation

    assert!(d1 > d2);
}

#[test]
fn test_haversine_known_values() {
    // Test against some pre-calculated values
    let cases = [
        (
            Spherical::new(Radians::new_unchecked(0.0), Radians::new_unchecked(0.0)),
            Spherical::new(
                Radians::new_unchecked(0.0),
                Radians::new_unchecked(std::f64::consts::PI / 2.0),
            ),
            0.5, // sin²(π/4) = 0.5
        ),
        (
            Spherical::new(
                Radians::new_unchecked(0.0),
                Radians::new_unchecked(std::f64::consts::PI / 4.0),
            ),
            Spherical::new(
                Radians::new_unchecked(std::f64::consts::PI / 2.0),
                Radians::new_unchecked(std::f64::consts::PI / 4.0),
            ),
            0.25, // For points at same latitude
        ),
    ];

    for (p1, p2, expected) in &cases {
        assert_abs_diff_eq!(haversine(*p1, *p2), *expected, epsilon = 1e-4);
    }
}

#[test]
fn test_quintant_conversion() {
    let origins = get_origins();
    let origin = &origins[0];

    for quintant in 0..5 {
        let (segment, _orientation) = quintant_to_segment(quintant, origin);
        let (round_trip_quintant, _) = segment_to_quintant(segment, origin);
        assert_eq!(round_trip_quintant, quintant);
    }
}

#[test]
fn test_is_nearest_origin_for_face_centers() {
    let origins = get_origins();

    for origin in origins {
        let nearest = find_nearest_origin(origin.axis);
        assert_eq!(nearest.id, origin.id);
    }
}

#[test]
fn test_is_nearest_origin_for_boundary_points() {
    let origins = get_origins();

    // Test points halfway between adjacent origins
    let boundary_points = [
        // Between north pole and equatorial faces
        (
            Spherical::new(
                Radians::new_unchecked(0.0),
                Radians::new_unchecked(PI_OVER_5.get() / 2.0),
            ),
            &origins[0],
        ),
        // Between equatorial faces
        (
            Spherical::new(
                Radians::new_unchecked(2.0 * PI_OVER_5.get()),
                Radians::new_unchecked(PI_OVER_5.get()),
            ),
            &origins[3],
        ),
        // Between equatorial and south pole
        (
            Spherical::new(
                Radians::new_unchecked(0.0),
                Radians::new_unchecked(std::f64::consts::PI - PI_OVER_5.get() / 2.0),
            ),
            &origins[9],
        ),
    ];

    for (point, origin) in &boundary_points {
        assert!(!is_nearest_origin(*point, origin));
    }
}
