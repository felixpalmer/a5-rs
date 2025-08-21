// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use crate::coordinate_systems::{
    Barycentric, Cartesian, Degrees, Face, FaceTriangle, LonLat, Polar, Radians, Spherical, IJ,
};
use crate::core::pentagon::{basis, basis_inverse};
use crate::projections::authalic::AuthalicProjection;

/// Convert degrees to radians
pub fn deg_to_rad(deg: Degrees) -> Radians {
    Radians::new_unchecked(deg.get() * (std::f64::consts::PI / 180.0))
}

/// Convert radians to degrees  
pub fn rad_to_deg(rad: Radians) -> Degrees {
    Degrees::new_unchecked(rad.get() * (180.0 / std::f64::consts::PI))
}

/// Convert face coordinates to polar coordinates
pub fn to_polar(face: Face) -> Polar {
    let x = face.x();
    let y = face.y();
    let rho = (x * x + y * y).sqrt(); // Radial distance from face center
    let gamma = Radians::new_unchecked(y.atan2(x)); // Azimuthal angle
    Polar::new(rho, gamma)
}

/// Convert polar coordinates to face coordinates
pub fn to_face(polar: Polar) -> Face {
    let rho = polar.rho();
    let gamma = polar.gamma().get();
    let x = rho * gamma.cos();
    let y = rho * gamma.sin();
    Face::new(x, y)
}

/// Convert face coordinates to barycentric coordinates
pub fn face_to_barycentric(p: Face, triangle: FaceTriangle) -> Barycentric {
    let p1 = triangle.a;
    let p2 = triangle.b;
    let p3 = triangle.c;

    let d31 = [p1.x() - p3.x(), p1.y() - p3.y()];
    let d23 = [p3.x() - p2.x(), p3.y() - p2.y()];
    let d3p = [p.x() - p3.x(), p.y() - p3.y()];

    let det = d23[0] * d31[1] - d23[1] * d31[0];
    let b0 = (d23[0] * d3p[1] - d23[1] * d3p[0]) / det;
    let b1 = (d31[0] * d3p[1] - d31[1] * d3p[0]) / det;
    let b2 = 1.0 - (b0 + b1);

    Barycentric::new(b0, b1, b2)
}

/// Convert barycentric coordinates to face coordinates
pub fn barycentric_to_face(bary: Barycentric, triangle: FaceTriangle) -> Face {
    let p1 = triangle.a;
    let p2 = triangle.b;
    let p3 = triangle.c;

    let x = bary.u * p1.x() + bary.v * p2.x() + bary.w * p3.x();
    let y = bary.u * p1.y() + bary.v * p2.y() + bary.w * p3.y();

    Face::new(x, y)
}

/// Convert cartesian coordinates to spherical coordinates
pub fn to_spherical(cart: Cartesian) -> Spherical {
    let x = cart.x();
    let y = cart.y();
    let z = cart.z();

    let theta = Radians::new_unchecked(y.atan2(x));
    let r = (x * x + y * y + z * z).sqrt();
    let phi = Radians::new_unchecked((z / r).acos());

    Spherical::new(theta, phi)
}

/// Convert spherical coordinates to cartesian coordinates
pub fn to_cartesian(spherical: Spherical) -> Cartesian {
    let theta = spherical.theta().get();
    let phi = spherical.phi().get();

    let sin_phi = phi.sin();
    let x = sin_phi * theta.cos();
    let y = sin_phi * theta.sin();
    let z = phi.cos();

    Cartesian::new(x, y, z)
}

/// Longitude offset for the spherical coordinate system
/// This is the angle between the Greenwich meridian and vector between the centers
/// of the first two origins (dodecahedron face centers)
const LONGITUDE_OFFSET: f64 = 93.0;

/// Contour type alias for a sequence of longitude/latitude points
pub type Contour = Vec<LonLat>;

/// Convert face coordinates to IJ coordinates using BASIS_INVERSE matrix
pub fn face_to_ij(face: Face) -> IJ {
    let basis_inverse_mat = basis_inverse();
    let x = face.x();
    let y = face.y();

    let u = basis_inverse_mat.m00 * x + basis_inverse_mat.m01 * y;
    let v = basis_inverse_mat.m10 * x + basis_inverse_mat.m11 * y;

    IJ::new(u, v)
}

/// Convert IJ coordinates to face coordinates using BASIS matrix
pub fn ij_to_face(ij: IJ) -> Face {
    let basis_mat = basis();
    let u = ij.x();
    let v = ij.y();

    let x = basis_mat.m00 * u + basis_mat.m01 * v;
    let y = basis_mat.m10 * u + basis_mat.m11 * v;

    Face::new(x, y)
}

/// Convert longitude/latitude to spherical coordinates
pub fn from_lon_lat(lonlat: LonLat) -> Spherical {
    let longitude = lonlat.longitude();
    let latitude = lonlat.latitude();

    let theta = deg_to_rad(Degrees::new_unchecked(longitude + LONGITUDE_OFFSET));

    let geodetic_lat = deg_to_rad(Degrees::new_unchecked(latitude));
    let authalic = AuthalicProjection;
    let authalic_lat = authalic.forward(geodetic_lat);
    let phi = Radians::new_unchecked(std::f64::consts::FRAC_PI_2 - authalic_lat.get());

    Spherical::new(theta, phi)
}

/// Convert spherical coordinates to longitude/latitude
pub fn to_lon_lat(spherical: Spherical) -> LonLat {
    let theta = spherical.theta();
    let phi = spherical.phi();

    let longitude = rad_to_deg(theta);
    let longitude = Degrees::new_unchecked(longitude.get() - LONGITUDE_OFFSET);

    let authalic_lat = Radians::new_unchecked(std::f64::consts::FRAC_PI_2 - phi.get());
    let authalic = AuthalicProjection;
    let geodetic_lat = authalic.inverse(authalic_lat);
    let latitude = rad_to_deg(geodetic_lat);

    LonLat::new(longitude.get(), latitude.get())
}

/// Normalizes longitude values in a contour to handle antimeridian crossing
pub fn normalize_longitudes(contour: Contour) -> Contour {
    if contour.is_empty() {
        return contour;
    }

    // Calculate center in Cartesian space to avoid poles & antimeridian crossing issues
    let points: Vec<Cartesian> = contour
        .iter()
        .map(|&lonlat| to_cartesian(from_lon_lat(lonlat)))
        .collect();

    let mut center = Cartesian::new(0.0, 0.0, 0.0);
    for point in &points {
        center = Cartesian::new(
            center.x() + point.x(),
            center.y() + point.y(),
            center.z() + point.z(),
        );
    }

    // Normalize center
    let length = (center.x().powi(2) + center.y().powi(2) + center.z().powi(2)).sqrt();
    if length > 0.0 {
        center = Cartesian::new(
            center.x() / length,
            center.y() / length,
            center.z() / length,
        );
    }

    let center_spherical = to_spherical(center);
    let center_lonlat = to_lon_lat(center_spherical);
    let mut center_lon = center_lonlat.longitude();
    let center_lat = center_lonlat.latitude();

    // Near poles, use first point's longitude
    if !(-89.99..=89.99).contains(&center_lat) {
        center_lon = contour[0].longitude();
    }

    // Normalize center longitude to be in the range -180 to 180
    center_lon = ((center_lon + 180.0) % 360.0 + 360.0) % 360.0 - 180.0;

    // Normalize each point relative to center
    contour
        .into_iter()
        .map(|lonlat| {
            let mut longitude = lonlat.longitude();
            let latitude = lonlat.latitude();

            // Adjust longitude to be closer to center
            while longitude - center_lon > 180.0 {
                longitude -= 360.0;
            }
            while longitude - center_lon < -180.0 {
                longitude += 360.0;
            }

            LonLat::new(longitude, latitude)
        })
        .collect()
}
