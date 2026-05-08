// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use crate::coordinate_systems::Cartesian;
use crate::core::constants::AUTHALIC_RADIUS_EARTH;
use crate::utils::vector::{precompute_slerp, slerp_ctx};

/// Great-circle distance in meters between two unit vectors on the authalic sphere.
pub fn great_circle_distance(a: Cartesian, b: Cartesian) -> f64 {
    let dot = (a.x() * b.x() + a.y() * b.y() + a.z() * b.z()).clamp(-1.0, 1.0);
    dot.acos() * AUTHALIC_RADIUS_EARTH
}

/// Sample interior points along the great-circle arc from `a` to `b` at roughly
/// `sample_interval` meters spacing. Endpoints are NOT included — the caller
/// already has them. Returned vectors live on the authalic unit sphere.
pub fn sample_great_circle_arc(a: Cartesian, b: Cartesian, sample_interval: f64) -> Vec<Cartesian> {
    let dist = great_circle_distance(a, b);
    let num_segments = ((dist / sample_interval).ceil() as usize).max(1);
    let mut samples: Vec<Cartesian> = Vec::new();
    if num_segments <= 1 {
        return samples;
    }
    let ctx = precompute_slerp(a, b);
    for j in 1..num_segments {
        let t = j as f64 / num_segments as f64;
        samples.push(slerp_ctx(a, b, t, Some(ctx)));
    }
    samples
}
