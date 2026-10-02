// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use crate::coordinate_systems::Cartesian;

/// Computes the scalar triple product a · (b × c).
/// Written out fully (same operation order as cross followed by dot,
/// so results are bit-identical) to avoid an intermediate vector on hot paths.
pub fn triple_product(a: Cartesian, b: Cartesian, c: Cartesian) -> f64 {
    a.x() * (b.y() * c.z() - b.z() * c.y())
        + a.y() * (b.z() * c.x() - b.x() * c.z())
        + a.z() * (b.x() * c.y() - b.y() * c.x())
}

/// Angle between two UNIT vectors, computed as 2·atan2(‖a−b‖, ‖a+b‖).
///
/// Unlike acos(a·b), which loses half the significant digits carried when the
/// vectors are nearly parallel (and all of them below ~1e-8 rad), this formula
/// keeps full working precision over the whole range [0, π]: the subtraction
/// a−b is exact for nearby vectors, and atan2 has no sensitive endpoints
/// (Kahan, "How Futile are Mindless Assessments of Roundoff…", §12).
/// Inputs are assumed to be unit length.
pub fn angle(a: Cartesian, b: Cartesian) -> f64 {
    let dx = a.x() - b.x();
    let dy = a.y() - b.y();
    let dz = a.z() - b.z();
    let sx = a.x() + b.x();
    let sy = a.y() + b.y();
    let sz = a.z() + b.z();
    let diff = (dx * dx + dy * dy + dz * dz).sqrt();
    let sum = (sx * sx + sy * sy + sz * sz).sqrt();
    2.0 * diff.atan2(sum)
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
