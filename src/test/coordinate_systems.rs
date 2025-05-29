use crate::coordinate_systems::{Polar, Radians, Spherical};
use approx::assert_relative_eq;
use std::f64::consts::PI;

#[test]
fn test_coord_primitives() {
    assert_eq!(Radians::new_unchecked(1.23).0, 1.23)
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
