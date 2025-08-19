// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use crate::coordinate_systems::{
    Barycentric, Cartesian, Face, FaceTriangle, LonLat, Polar, Spherical, 
    Degrees, Radians
};

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

/// Convert longitude/latitude to spherical coordinates
/// Note: This is a simplified version that doesn't include authalic projection
/// The full implementation would require the authalic projection module
pub fn from_lon_lat(lonlat: LonLat) -> Spherical {
    let longitude = lonlat.longitude();
    let latitude = lonlat.latitude();
    
    let theta = deg_to_rad(Degrees::new_unchecked(longitude + LONGITUDE_OFFSET));
    
    // Simplified conversion without authalic projection
    // In the full implementation, this would use authalic.forward(geodetic_lat)
    let geodetic_lat = deg_to_rad(Degrees::new_unchecked(latitude));
    let phi = Radians::new_unchecked(std::f64::consts::FRAC_PI_2 - geodetic_lat.get());
    
    Spherical::new(theta, phi)
}

/// Convert spherical coordinates to longitude/latitude
/// Note: This is a simplified version that doesn't include authalic projection
/// The full implementation would require the authalic projection module
pub fn to_lon_lat(spherical: Spherical) -> LonLat {
    let theta = spherical.theta();
    let phi = spherical.phi();
    
    let longitude = rad_to_deg(theta);
    let longitude = Degrees::new_unchecked(longitude.get() - LONGITUDE_OFFSET);
    
    // Simplified conversion without authalic projection
    // In the full implementation, this would use authalic.inverse(authalic_lat)
    let authalic_lat = std::f64::consts::FRAC_PI_2 - phi.get();
    let latitude = rad_to_deg(Radians::new_unchecked(authalic_lat));
    
    LonLat::new(longitude.get(), latitude.get())
}