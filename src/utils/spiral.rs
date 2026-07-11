// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use crate::coordinate_systems::{Cartesian, Spherical};
use crate::core::coordinate_transforms::to_cartesian;
use crate::core::utils::Quat;

/// Number of perturbed sample points the spiral can produce. Tuned on a
/// corpus of ~3500 spherical points × 8 resolutions, such that the spiral
/// hits a strictly-containing cell within these many iterations for all
/// but a handful of points right at the polar singularity at very high
/// resolutions.
pub const SPIRAL_SAMPLE_COUNT: usize = 24;

/// Azimuthal step between consecutive samples in the rotated tangent plane.
/// 1.4 rad (~80°) sits on a flat plateau of the parameter sweep.
const ANGLE_STEP_RAD: f64 = 1.4;

// Precomputed unit-direction spiral at the canonical pole's tangent plane
// (z=0). Each entry is the tangent direction of one sample. The pattern
// is independent of resolution; per spiral the directions are rotated to
// the input point's tangent plane via a single quaternion.
fn spiral_directions() -> &'static [[f64; 3]; SPIRAL_SAMPLE_COUNT] {
    use std::sync::OnceLock;
    static DIRS: OnceLock<[[f64; 3]; SPIRAL_SAMPLE_COUNT]> = OnceLock::new();
    DIRS.get_or_init(|| {
        let mut out = [[0.0; 3]; SPIRAL_SAMPLE_COUNT];
        for (i, slot) in out.iter_mut().enumerate() {
            let a = (i as f64 + 1.0) * ANGLE_STEP_RAD;
            *slot = [a.cos(), a.sin(), 0.0];
        }
        out
    })
}

/// Lazy spiral sampler around a center point on the unit sphere — used by
/// `spherical_to_cell` to discover nearby cells when the projection-based
/// estimate lands in the wrong one.
///
/// Construction precomputes the pole→center quaternion. `sample(i)`
/// rotates the i-th cached direction into the tangent plane at `center`,
/// scales by the appropriate radius, and returns a Cartesian point near
/// the unit sphere — the consumer of the spiral (the dodecahedron
/// projection) wants Cartesian anyway, so we skip the spherical
/// round-trip entirely. The point is slightly off the unit sphere by
/// O(R²); downstream callers either tolerate this or normalise.
pub struct Spiral {
    c0: Cartesian,
    q: Quat,
    scale_rad: f64,
}

impl Spiral {
    /// Initialise a spiral around `center` on the unit sphere. The
    /// tangent-plane radius of the outermost sample is `scale_rad`;
    /// intermediate samples scale linearly between 0 and that.
    /// The pole→center rotation handles the antipode case internally.
    pub fn new(center: Spherical, scale_rad: f64) -> Self {
        let c0 = to_cartesian(center);
        let q = quat_rotation_to_from_pole(c0);
        Self { c0, q, scale_rad }
    }

    /// Return the i-th spiral sample (0 ≤ i < SPIRAL_SAMPLE_COUNT).
    /// Sample i sits at tangent-plane offset of magnitude
    /// `(i+1)/(SPIRAL_SAMPLE_COUNT+1) · scale_rad` from `center`,
    /// rotated by azimuth `(i+1) · 1.4 rad` in `center`'s tangent frame.
    pub fn sample(&self, i: usize) -> Cartesian {
        let dir = spiral_directions()[i];
        let rotated = transform_quat(dir, self.q);
        let r = ((i as f64 + 1.0) / (SPIRAL_SAMPLE_COUNT as f64 + 1.0)) * self.scale_rad;
        Cartesian::new(
            self.c0.x() + rotated[0] * r,
            self.c0.y() + rotated[1] * r,
            self.c0.z() + rotated[2] * r,
        )
    }
}

/// Compute the shortest-rotation quaternion from the canonical pole
/// (0,0,1) to `target`. Mirrors gl-matrix `quat.rotationTo(out, POLE, target)`.
fn quat_rotation_to_from_pole(target: Cartesian) -> Quat {
    let bx = target.x();
    let by = target.y();
    let bz = target.z();
    // POLE = (0, 0, 1), so dot = bz, cross(POLE, target) = (-by, bx, 0)
    let dot = bz;

    if dot < -0.999999 {
        // Antipode: rotate PI around an axis perpendicular to POLE.
        // cross(xUnit (1,0,0), POLE (0,0,1)) = (0*1 - 0*0, 0*0 - 1*1, 1*0 - 0*0) = (0, -1, 0)
        // Length 1, so normalize is a no-op. Use this axis.
        let axis = [0.0, -1.0, 0.0];
        return set_axis_angle(axis, std::f64::consts::PI);
    } else if dot > 0.999999 {
        return [0.0, 0.0, 0.0, 1.0];
    }
    let cx = -by;
    let cy = bx;
    let cz = 0.0;
    let mut out = [cx, cy, cz, 1.0 + dot];
    // Normalize
    let len_sq = out[0] * out[0] + out[1] * out[1] + out[2] * out[2] + out[3] * out[3];
    if len_sq > 0.0 {
        let len = len_sq.sqrt();
        out[0] /= len;
        out[1] /= len;
        out[2] /= len;
        out[3] /= len;
    }
    out
}

fn set_axis_angle(axis: [f64; 3], rad: f64) -> Quat {
    let half = rad * 0.5;
    let s = half.sin();
    [s * axis[0], s * axis[1], s * axis[2], half.cos()]
}

/// Transform a 3-vector by a unit quaternion (q * v * q^-1).
fn transform_quat(v: [f64; 3], q: Quat) -> [f64; 3] {
    let [qx, qy, qz, qw] = q;
    let [vx, vy, vz] = v;

    let qconj_x = -qx;
    let qconj_y = -qy;
    let qconj_z = -qz;
    let qconj_w = qw;

    let t1_x = qw * vx + qy * vz - qz * vy;
    let t1_y = qw * vy + qz * vx - qx * vz;
    let t1_z = qw * vz + qx * vy - qy * vx;
    let t1_w = -qx * vx - qy * vy - qz * vz;

    let r_x = t1_w * qconj_x + t1_x * qconj_w + t1_y * qconj_z - t1_z * qconj_y;
    let r_y = t1_w * qconj_y + t1_y * qconj_w + t1_z * qconj_x - t1_x * qconj_z;
    let r_z = t1_w * qconj_z + t1_z * qconj_w + t1_x * qconj_y - t1_y * qconj_x;
    [r_x, r_y, r_z]
}
