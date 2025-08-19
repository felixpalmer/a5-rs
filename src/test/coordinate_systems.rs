use crate::coordinate_systems::{Degrees, LonLat, Polar, Radians, Spherical};
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
fn test_degrees_normalization() {
    assert_eq!(Degrees::new(370.0).get(), 10.0);
    assert_eq!(Degrees::new(-190.0).get(), 170.0);
    assert_eq!(Degrees::new(180.0).get(), 180.0);
    assert_eq!(Degrees::new(-180.0).get(), 180.0);
}

#[test]
fn test_degrees_latitude_clamping() {
    assert_eq!(Degrees::new_latitude(100.0).get(), 90.0);
    assert_eq!(Degrees::new_latitude(-100.0).get(), -90.0);
    assert_eq!(Degrees::new_latitude(45.0).get(), 45.0);
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
