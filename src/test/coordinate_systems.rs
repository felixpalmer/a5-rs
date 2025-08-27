use crate::coordinate_systems::{
    Barycentric, Cartesian, Degrees, Face, FaceTriangle, LonLat, Polar, Radians, Spherical,
    SphericalTriangle, IJ, KJ,
};
use approx::assert_relative_eq;
use std::f64::consts::PI;

#[test]
fn test_coord_primitives() {
    assert_eq!(Radians::new_unchecked(1.23).get(), 1.23)
}

#[test]
fn test_round_trip() {
    let polar = Polar::new(0.3, Radians::new_unchecked(0.4));
    let spherical = polar.project_gnomonic();
    let result = spherical.unproject_gnomonic();

    assert_relative_eq!(result.rho, polar.rho, epsilon = 1e-4);
    assert_relative_eq!(result.gamma.get(), polar.gamma.get(), epsilon = 1e-4);
}

#[test]
fn test_unproject_gnomonic() {
    let test_values = [
        (
            Polar::new(0.001, Radians::new_unchecked(0.0)),
            Spherical::new(Radians::new_unchecked(0.0), Radians::new_unchecked(0.001)),
        ),
        (
            Polar::new(0.001, Radians::new_unchecked(0.321)),
            Spherical::new(Radians::new_unchecked(0.321), Radians::new_unchecked(0.001)),
        ),
        (
            Polar::new(1.0, Radians::new_unchecked(PI)),
            Spherical::new(Radians::new_unchecked(PI), Radians::new_unchecked(PI / 4.0)),
        ),
        (
            Polar::new(0.5, Radians::new_unchecked(0.777)),
            Spherical::new(
                Radians::new_unchecked(0.777),
                Radians::new_unchecked(0.5f64.atan()),
            ),
        ),
    ];

    for (input_coords, expected) in test_values {
        let result = expected.unproject_gnomonic();
        assert_relative_eq!(result.rho, input_coords.rho, epsilon = 1e-4);
        assert_relative_eq!(result.gamma.get(), input_coords.gamma.get(), epsilon = 1e-4);
    }
}

#[test]
fn test_degrees_no_normalization() {
    assert_eq!(Degrees::new(370.0).get(), 370.0);
    assert_eq!(Degrees::new(-190.0).get(), -190.0);
    assert_eq!(Degrees::new(180.0).get(), 180.0);
    assert_eq!(Degrees::new(-180.0).get(), -180.0);
}

#[test]
fn test_lonlat_creation() {
    let lonlat = LonLat::new(120.5, 35.7);
    assert_eq!(lonlat.longitude(), 120.5);
    assert_eq!(lonlat.latitude(), 35.7);

    let from_tuple: LonLat = (120.5, 35.7).into();
    assert_eq!(from_tuple, lonlat);

    let to_tuple: (f64, f64) = lonlat.into();
    assert_eq!(to_tuple, (120.5, 35.7));
}

#[test]
fn test_degrees_radians_conversion() {
    let degrees = Degrees::new(180.0);
    let radians = degrees.to_radians();
    assert_relative_eq!(radians.get(), PI, epsilon = 1e-10);

    let back_to_degrees = radians.to_degrees();
    assert_relative_eq!(back_to_degrees.get(), 180.0, epsilon = 1e-10);
}

#[test]
fn test_face_coordinate_operations() {
    let face = Face::new(3.5, 4.2);
    assert_eq!(face.x(), 3.5);
    assert_eq!(face.y(), 4.2);

    // Test array conversions
    let arr: [f64; 2] = face.into();
    assert_eq!(arr, [3.5, 4.2]);

    let face_from_arr = Face::from([3.5, 4.2]);
    assert_eq!(face.x(), face_from_arr.x());
    assert_eq!(face.y(), face_from_arr.y());
}

#[test]
fn test_cartesian_coordinate_operations() {
    let cart = Cartesian::new(1.1, 2.2, 3.3);
    assert_eq!(cart.x(), 1.1);
    assert_eq!(cart.y(), 2.2);
    assert_eq!(cart.z(), 3.3);

    // Test array conversions
    let arr: [f64; 3] = cart.into();
    assert_eq!(arr, [1.1, 2.2, 3.3]);

    let cart_from_arr = Cartesian::from([1.1, 2.2, 3.3]);
    assert_eq!(cart.x(), cart_from_arr.x());
    assert_eq!(cart.y(), cart_from_arr.y());
    assert_eq!(cart.z(), cart_from_arr.z());
}

#[test]
fn test_ij_kj_coordinates() {
    let ij = IJ::new(5.0, 6.0);
    assert_eq!(ij.x(), 5.0);
    assert_eq!(ij.y(), 6.0);

    let kj = KJ::new(7.0, 8.0);
    assert_eq!(kj.x(), 7.0);
    assert_eq!(kj.y(), 8.0);
}

#[test]
fn test_barycentric_coordinates() {
    let bary = Barycentric::new(0.3, 0.4, 0.3);
    assert_eq!(bary.u, 0.3);
    assert_eq!(bary.v, 0.4);
    assert_eq!(bary.w, 0.3);

    assert!(bary.is_valid());
    assert!(bary.is_inside_triangle());

    // Test invalid coordinates (sum != 1)
    let invalid = Barycentric::new(0.3, 0.4, 0.4);
    assert!(!invalid.is_valid());

    // Test outside triangle (negative coordinate)
    let outside = Barycentric::new(-0.1, 0.6, 0.5);
    assert!(!outside.is_inside_triangle());
}

#[test]
fn test_triangle_types() {
    let face_triangle = FaceTriangle::new(
        Face::new(0.0, 0.0),
        Face::new(1.0, 0.0),
        Face::new(0.0, 1.0),
    );
    assert_eq!(face_triangle.a.x(), 0.0);
    assert_eq!(face_triangle.b.x(), 1.0);
    assert_eq!(face_triangle.c.y(), 1.0);

    let spherical_triangle = SphericalTriangle::new(
        Cartesian::new(1.0, 0.0, 0.0),
        Cartesian::new(0.0, 1.0, 0.0),
        Cartesian::new(0.0, 0.0, 1.0),
    );
    assert_eq!(spherical_triangle.a.x(), 1.0);
    assert_eq!(spherical_triangle.b.y(), 1.0);
    assert_eq!(spherical_triangle.c.z(), 1.0);
}
