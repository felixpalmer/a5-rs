// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use crate::lattice::lsystem::triple_to_s_lattice;
use crate::lattice::types::{Orientation, Triple};

/// The parity of a triple (0 or 1), equal to x + y + z.
pub fn triple_parity(t: &Triple) -> i32 {
    t.x + t.y + t.z
}

/// Check if a triple is within valid quintant bounds.
pub fn triple_in_bounds(t: &Triple, max_row: i32) -> bool {
    let sum = t.x + t.y + t.z;
    if sum != 0 && sum != 1 {
        return false;
    }
    let limit = t.y - sum;
    t.x <= 0 && t.z <= 0 && t.y >= 0 && t.y <= max_row && t.x >= -limit && t.z >= -limit
}

/// Convert triple coordinates to an s-value on the A5 (L-system) curve.
/// The engine's `lattice::triple_to_s` is currently the compat alias; this is
/// the pure-curve form it swaps to at the canonical cutover (mirrors the other
/// ports' triple modules).
///
/// Returns None if the triple has invalid parity.
pub fn triple_to_s(t: &Triple, resolution: usize, orientation: Orientation) -> Option<u64> {
    let sum = t.x + t.y + t.z;
    if sum != 0 && sum != 1 {
        return None;
    }
    Some(triple_to_s_lattice(t, resolution, orientation))
}
