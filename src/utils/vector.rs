// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use crate::coordinate_systems::Cartesian;

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

/// Cached `gamma` and `sin(gamma)` for a fixed (A, B) pair, so loops that
/// slerp many times along the same arc don't re-run `angle` and `sin`.
/// Build with `precompute_slerp(a, b)` and pass to `slerp_ctx` as the optional context.
#[derive(Debug, Clone, Copy)]
pub struct SlerpContext {
    pub gamma: f64,
    pub sin_gamma: f64,
}

/// Precompute the angle and its sine for a pair of vectors so that subsequent
/// slerp calls along the same arc avoid recomputing them.
pub fn precompute_slerp(a: Cartesian, b: Cartesian) -> SlerpContext {
    let gamma = angle(a, b);
    SlerpContext {
        gamma,
        sin_gamma: gamma.sin(),
    }
}

/// Spherical linear interpolation between two vectors.
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
    slerp_ctx(a, b, t, None)
}

/// Spherical linear interpolation between two vectors, with an optional
/// precomputed `{gamma, sin_gamma}` context. Supply when slerping many `t`
/// values along the same arc to avoid recomputing them.
pub fn slerp_ctx(a: Cartesian, b: Cartesian, t: f64, ctx: Option<SlerpContext>) -> Cartesian {
    let gamma = ctx.map(|c| c.gamma).unwrap_or_else(|| angle(a, b));
    if gamma < 1e-12 {
        return lerp(a, b, t);
    }
    let sin_gamma = ctx.map(|c| c.sin_gamma).unwrap_or_else(|| gamma.sin());
    let weight_a = ((1.0 - t) * gamma).sin() / sin_gamma;
    let weight_b = (t * gamma).sin() / sin_gamma;
    Cartesian::new(
        weight_a * a.x() + weight_b * b.x(),
        weight_a * a.y() + weight_b * b.y(),
        weight_a * a.z() + weight_b * b.z(),
    )
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

/// Compute angle between two vectors
fn angle(a: Cartesian, b: Cartesian) -> f64 {
    let dot_product = dot(a, b);
    let len_a = length(a);
    let len_b = length(b);
    let cos_angle = dot_product / (len_a * len_b);
    // Clamp to avoid numerical errors
    cos_angle.clamp(-1.0, 1.0).acos()
}
