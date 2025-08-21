// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use crate::coordinate_systems::Cartesian;

/// Returns a difference measure between two vectors, a - b
/// D = sqrt(1 - dot(a,b)) / sqrt(2)
/// D = 1: a and b are perpendicular
/// D = 0: a and b are the same
/// D = NaN: a and b are opposite (shouldn't happen in IVEA as we're using normalized vectors in the same hemisphere)
///
/// D is a measure of the angle between the two vectors. sqrt(2) can be ignored when comparing ratios.
///
/// # Arguments
///
/// * `a` - The first vector
/// * `b` - The second vector
///
/// # Returns
///
/// The difference between the two vectors
pub fn vector_difference(a: Cartesian, b: Cartesian) -> f64 {
    // Original implementation is unstable for small angles as dot(A, B) approaches 1
    // return (1.0 - dot(a, b)).sqrt();

    // dot(A, B) = cos(x) as A and B are normalized
    // Using double angle formula for cos(2x) = 1 - 2sin(x)^2, can rewrite as:
    // 1 - cos(x) = 2 * sin(x/2)^2)
    //            = 2 * sin(x/2)^2
    // ⇒ sqrt(1 - cos(x)) = sqrt(2) * sin(x/2)
    // Angle x/2 can be obtained as the angle between A and the normalized midpoint of A and B
    // ⇒ sin(x/2) = |cross(A, midpointAB)|

    let midpoint_ab = lerp(a, b, 0.5);
    let midpoint_ab = normalize(midpoint_ab);
    let cross_result = cross(a, midpoint_ab);
    let d = length(cross_result);

    // Math.sin(x) = x for x < 1e-8
    if d < 1e-8 {
        // When A and B are close or equal sin(x/2) ≈ x/2, just take the half-distance between A and B
        let ab = subtract(a, b);
        let half_distance = 0.5 * length(ab);
        return half_distance;
    }
    d
}

/// Computes the triple product of three vectors
///
/// # Arguments
///
/// * `a` - The first vector
/// * `b` - The second vector
/// * `c` - The third vector
///
/// # Returns
///
/// The scalar result
pub fn triple_product(a: Cartesian, b: Cartesian, c: Cartesian) -> f64 {
    let cross_bc = cross(b, c);
    dot(a, cross_bc)
}

/// Computes the quadruple product of four vectors
///
/// # Arguments
///
/// * `a` - The first vector
/// * `b` - The second vector
/// * `c` - The third vector
/// * `d` - The fourth vector
///
/// # Returns
///
/// The result vector
pub fn quadruple_product(a: Cartesian, b: Cartesian, c: Cartesian, d: Cartesian) -> Cartesian {
    let cross_cd = cross(c, d);
    let triple_product_acd = dot(a, cross_cd);
    let triple_product_bcd = dot(b, cross_cd);
    let scaled_a = scale(a, triple_product_bcd);
    let scaled_b = scale(b, triple_product_acd);
    subtract(scaled_b, scaled_a)
}

/// Spherical linear interpolation between two vectors
///
/// # Arguments
///
/// * `a` - The first vector
/// * `b` - The second vector
/// * `t` - The interpolation parameter (0 to 1)
///
/// # Returns
///
/// The interpolated vector
pub fn slerp(a: Cartesian, b: Cartesian, t: f64) -> Cartesian {
    let gamma = angle(a, b);
    if gamma < 1e-12 {
        return lerp(a, b, t);
    }
    let weight_a = ((1.0 - t) * gamma).sin() / gamma.sin();
    let weight_b = (t * gamma).sin() / gamma.sin();
    let scaled_a = scale(a, weight_a);
    let scaled_b = scale(b, weight_b);
    add(scaled_a, scaled_b)
}

// Helper functions for 3D vector operations

/// Compute dot product of two vectors
fn dot(a: Cartesian, b: Cartesian) -> f64 {
    a.x() * b.x() + a.y() * b.y() + a.z() * b.z()
}

/// Compute cross product of two vectors
fn cross(a: Cartesian, b: Cartesian) -> Cartesian {
    Cartesian::new(
        a.y() * b.z() - a.z() * b.y(),
        a.z() * b.x() - a.x() * b.z(),
        a.x() * b.y() - a.y() * b.x(),
    )
}

/// Compute length of a vector
pub fn length(v: Cartesian) -> f64 {
    (v.x() * v.x() + v.y() * v.y() + v.z() * v.z()).sqrt()
}

/// Helper alias for the public length function
pub fn vec3_length(v: &Cartesian) -> f64 {
    length(*v)
}

/// Normalize a vector
fn normalize(v: Cartesian) -> Cartesian {
    let len = length(v);
    if len == 0.0 {
        return v;
    }
    Cartesian::new(v.x() / len, v.y() / len, v.z() / len)
}

/// Linear interpolation between two vectors
fn lerp(a: Cartesian, b: Cartesian, t: f64) -> Cartesian {
    Cartesian::new(
        a.x() + t * (b.x() - a.x()),
        a.y() + t * (b.y() - a.y()),
        a.z() + t * (b.z() - a.z()),
    )
}

/// Subtract two vectors
fn subtract(a: Cartesian, b: Cartesian) -> Cartesian {
    Cartesian::new(a.x() - b.x(), a.y() - b.y(), a.z() - b.z())
}

/// Distance between two 3D vectors
pub fn vec3_distance(a: &Cartesian, b: &Cartesian) -> f64 {
    length(subtract(*a, *b))
}

/// Add two vectors
fn add(a: Cartesian, b: Cartesian) -> Cartesian {
    Cartesian::new(a.x() + b.x(), a.y() + b.y(), a.z() + b.z())
}

/// Scale a vector by a scalar
fn scale(v: Cartesian, s: f64) -> Cartesian {
    Cartesian::new(v.x() * s, v.y() * s, v.z() * s)
}

/// Compute angle between two vectors
fn angle(a: Cartesian, b: Cartesian) -> f64 {
    let dot_product = dot(a, b);
    let len_a = length(a);
    let len_b = length(b);
    let cos_angle = dot_product / (len_a * len_b);
    // Clamp to avoid numerical errors
    cos_angle.clamp(-1.0, 1.0).acos()
}
