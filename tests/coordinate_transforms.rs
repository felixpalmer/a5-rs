// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use a5_rs::coordinate_systems::{
    Barycentric, Cartesian, Degrees, Face, FaceTriangle, LonLat, Polar, Radians, Spherical,
};
use a5_rs::core::coordinate_transforms::{
    barycentric_to_face, deg_to_rad, face_to_barycentric, from_lon_lat, rad_to_deg, to_cartesian,
    to_face, to_lon_lat, to_polar, to_spherical,
};
use approx::assert_relative_eq;

const TOLERANCE: f64 = 1e-10;

#[test]
fn test_angle_conversions() {
    // Test degrees to radians
    assert_relative_eq!(
        deg_to_rad(Degrees::new_unchecked(180.0)).get(),
        std::f64::consts::PI,
        epsilon = TOLERANCE
    );
    assert_relative_eq!(
        deg_to_rad(Degrees::new_unchecked(90.0)).get(),
        std::f64::consts::PI / 2.0,
        epsilon = TOLERANCE
    );
    assert_relative_eq!(
        deg_to_rad(Degrees::new_unchecked(0.0)).get(),
        0.0,
        epsilon = TOLERANCE
    );

    // Test radians to degrees
    assert_relative_eq!(
        rad_to_deg(Radians::new_unchecked(std::f64::consts::PI)).get(),
        180.0,
        epsilon = TOLERANCE
    );
    assert_relative_eq!(
        rad_to_deg(Radians::new_unchecked(std::f64::consts::PI / 2.0)).get(),
        90.0,
        epsilon = TOLERANCE
    );
    assert_relative_eq!(
        rad_to_deg(Radians::new_unchecked(0.0)).get(),
        0.0,
        epsilon = TOLERANCE
    );
}

#[test]
fn test_polar_face_conversions() {
    let test_cases = [
        (
            Face::new(0.0, 0.0),
            Polar::new(0.0, Radians::new_unchecked(0.0)),
        ),
        (
            Face::new(1.0, 0.0),
            Polar::new(1.0, Radians::new_unchecked(0.0)),
        ),
        (
            Face::new(0.0, 1.0),
            Polar::new(1.0, Radians::new_unchecked(std::f64::consts::PI / 2.0)),
        ),
        (
            Face::new(-1.0, 0.0),
            Polar::new(1.0, Radians::new_unchecked(std::f64::consts::PI)),
        ),
    ];

    for (face, polar) in test_cases {
        // Test face to polar
        let result_polar = to_polar(face);
        assert_relative_eq!(result_polar.rho(), polar.rho(), epsilon = TOLERANCE);
        assert_relative_eq!(
            result_polar.gamma().get(),
            polar.gamma().get(),
            epsilon = TOLERANCE
        );

        // Test polar to face
        let result_face = to_face(polar);
        assert_relative_eq!(result_face.x(), face.x(), epsilon = TOLERANCE);
        assert_relative_eq!(result_face.y(), face.y(), epsilon = TOLERANCE);
    }
}

#[test]
fn test_barycentric_conversions() {
    let triangle = FaceTriangle::new(
        Face::new(0.0, 0.0),
        Face::new(1.0, 0.0),
        Face::new(0.0, 1.0),
    );

    // Test triangle vertices
    let vertices = [triangle.a, triangle.b, triangle.c];
    let expected_bary = [
        Barycentric::new(1.0, 0.0, 0.0),
        Barycentric::new(0.0, 1.0, 0.0),
        Barycentric::new(0.0, 0.0, 1.0),
    ];

    for (i, &vertex) in vertices.iter().enumerate() {
        let bary = face_to_barycentric(vertex, triangle);

        assert_relative_eq!(bary.u, expected_bary[i].u, epsilon = TOLERANCE);
        assert_relative_eq!(bary.v, expected_bary[i].v, epsilon = TOLERANCE);
        assert_relative_eq!(bary.w, expected_bary[i].w, epsilon = TOLERANCE);

        // Round-trip test
        let result_face = barycentric_to_face(bary, triangle);
        assert_relative_eq!(result_face.x(), vertex.x(), epsilon = TOLERANCE);
        assert_relative_eq!(result_face.y(), vertex.y(), epsilon = TOLERANCE);
    }

    // Test edge midpoints
    let edge_midpoints = [
        Face::new(0.5, 0.0), // Midpoint of a-b edge
        Face::new(0.0, 0.5), // Midpoint of a-c edge
        Face::new(0.5, 0.5), // Midpoint of b-c edge
    ];

    let expected_edge_bary = [
        Barycentric::new(0.5, 0.5, 0.0),
        Barycentric::new(0.5, 0.0, 0.5),
        Barycentric::new(0.0, 0.5, 0.5),
    ];

    for (i, &midpoint) in edge_midpoints.iter().enumerate() {
        let bary = face_to_barycentric(midpoint, triangle);

        assert_relative_eq!(bary.u, expected_edge_bary[i].u, epsilon = TOLERANCE);
        assert_relative_eq!(bary.v, expected_edge_bary[i].v, epsilon = TOLERANCE);
        assert_relative_eq!(bary.w, expected_edge_bary[i].w, epsilon = TOLERANCE);

        // Check that barycentric coordinates sum to 1
        assert_relative_eq!(bary.u + bary.v + bary.w, 1.0, epsilon = TOLERANCE);

        // Round-trip test
        let result_face = barycentric_to_face(bary, triangle);
        assert_relative_eq!(result_face.x(), midpoint.x(), epsilon = TOLERANCE);
        assert_relative_eq!(result_face.y(), midpoint.y(), epsilon = TOLERANCE);
    }
}

#[test]
fn test_spherical_cartesian_conversions() {
    // Test north pole
    let north_pole = to_cartesian(Spherical::new(
        Radians::new_unchecked(0.0),
        Radians::new_unchecked(0.0),
    ));
    assert_relative_eq!(north_pole.x(), 0.0, epsilon = TOLERANCE);
    assert_relative_eq!(north_pole.y(), 0.0, epsilon = TOLERANCE);
    assert_relative_eq!(north_pole.z(), 1.0, epsilon = TOLERANCE);

    // Test equator at 0 longitude
    let equator_0 = to_cartesian(Spherical::new(
        Radians::new_unchecked(0.0),
        Radians::new_unchecked(std::f64::consts::PI / 2.0),
    ));
    assert_relative_eq!(equator_0.x(), 1.0, epsilon = TOLERANCE);
    assert_relative_eq!(equator_0.y(), 0.0, epsilon = TOLERANCE);
    assert_relative_eq!(equator_0.z(), 0.0, epsilon = TOLERANCE);

    // Test equator at 90° longitude
    let equator_90 = to_cartesian(Spherical::new(
        Radians::new_unchecked(std::f64::consts::PI / 2.0),
        Radians::new_unchecked(std::f64::consts::PI / 2.0),
    ));
    assert_relative_eq!(equator_90.x(), 0.0, epsilon = TOLERANCE);
    assert_relative_eq!(equator_90.y(), 1.0, epsilon = TOLERANCE);
    assert_relative_eq!(equator_90.z(), 0.0, epsilon = TOLERANCE);

    // Test round trip conversion
    let original = Spherical::new(
        Radians::new_unchecked(std::f64::consts::PI / 4.0),
        Radians::new_unchecked(std::f64::consts::PI / 6.0),
    );
    let cartesian = to_cartesian(original);
    let spherical = to_spherical(cartesian);

    assert_relative_eq!(
        spherical.theta().get(),
        original.theta().get(),
        epsilon = TOLERANCE
    );
    assert_relative_eq!(
        spherical.phi().get(),
        original.phi().get(),
        epsilon = TOLERANCE
    );
}

#[test]
fn test_lonlat_spherical_conversions() {
    let test_points = [
        LonLat::new(0.0, 0.0),     // Equator
        LonLat::new(90.0, 0.0),    // Equator
        LonLat::new(180.0, 0.0),   // Equator
        LonLat::new(0.0, 45.0),    // Mid latitude
        LonLat::new(0.0, -45.0),   // Mid latitude
        LonLat::new(-90.0, -45.0), // West hemisphere mid-latitude
        LonLat::new(180.0, 45.0),  // Date line mid-latitude
        LonLat::new(90.0, 45.0),   // East hemisphere mid-latitude
        LonLat::new(0.0, 90.0),    // North pole
        LonLat::new(0.0, -90.0),   // South pole
        LonLat::new(123.0, 45.0),  // Arbitrary point
    ];

    // Test round trip conversion
    for lonlat in test_points {
        let spherical = from_lon_lat(lonlat);
        let result = to_lon_lat(spherical);

        assert_relative_eq!(
            result.longitude(),
            lonlat.longitude(),
            epsilon = 1e-6 // Slightly higher tolerance due to simplified conversion
        );
        assert_relative_eq!(
            result.latitude(),
            lonlat.latitude(),
            epsilon = 1e-6 // Slightly higher tolerance due to simplified conversion
        );
    }
}

#[test]
fn test_barycentric_properties() {
    let bary = Barycentric::new(0.3, 0.4, 0.3);

    // Test validity check
    assert!(bary.is_valid());

    // Test inside triangle check
    assert!(bary.is_inside_triangle());

    // Test invalid barycentric coordinates
    let invalid_bary = Barycentric::new(0.3, 0.4, 0.4); // Sum > 1
    assert!(!invalid_bary.is_valid());

    // Test outside triangle
    let outside_bary = Barycentric::new(-0.1, 0.6, 0.5);
    assert!(!outside_bary.is_inside_triangle());

    // Test array conversions
    let arr: [f64; 3] = bary.into();
    let bary_from_arr = Barycentric::from(arr);
    assert_eq!(bary.u, bary_from_arr.u);
    assert_eq!(bary.v, bary_from_arr.v);
    assert_eq!(bary.w, bary_from_arr.w);
}

#[test]
fn test_coordinate_type_conversions() {
    // Test Face conversions
    let face = Face::new(1.5, 2.5);
    let face_arr: [f64; 2] = face.into();
    assert_eq!(face_arr, [1.5, 2.5]);

    let face_from_arr = Face::from([1.5, 2.5]);
    assert_eq!(face.x(), face_from_arr.x());
    assert_eq!(face.y(), face_from_arr.y());

    // Test Cartesian conversions
    let cart = Cartesian::new(1.0, 2.0, 3.0);
    let cart_arr: [f64; 3] = cart.into();
    assert_eq!(cart_arr, [1.0, 2.0, 3.0]);

    let cart_from_arr = Cartesian::from([1.0, 2.0, 3.0]);
    assert_eq!(cart.x(), cart_from_arr.x());
    assert_eq!(cart.y(), cart_from_arr.y());
    assert_eq!(cart.z(), cart_from_arr.z());

    // Test FaceTriangle conversions
    let triangle_arr = [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]];
    let triangle = FaceTriangle::from(triangle_arr);

    assert_eq!(triangle.a.x(), 0.0);
    assert_eq!(triangle.a.y(), 0.0);
    assert_eq!(triangle.b.x(), 1.0);
    assert_eq!(triangle.b.y(), 0.0);
    assert_eq!(triangle.c.x(), 0.0);
    assert_eq!(triangle.c.y(), 1.0);
}
