use crate::coordinate_systems::{Polar, Radians, Spherical};
use approx::assert_relative_eq;

#[test]
fn test_round_trip() {
    let polar = Polar::new(0.3, Radians::new_unchecked(0.4));
    let spherical = polar.project_gnomonic();
    let result = spherical.unproject_gnomonic();

    assert_relative_eq!(result.rho, polar.rho, epsilon = 1e-4);
    assert_relative_eq!(result.gamma.get(), polar.gamma.get(), epsilon = 1e-4);
}
