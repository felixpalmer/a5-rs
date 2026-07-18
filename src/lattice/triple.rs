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
// The pentagon flavor is a CLOSED FORM of the triple: it depends only on the
// parity and y mod 2 (the Cairo-like tiling repeats its four orientations with
// period 2). Verified exhaustively against the descent's flavor over all cells
// (see tests/curve.rs); the descent's leaf flavor agrees because both describe
// the same fixed tiling.
const FLAVOR_LUT: [u8; 4] = [0, 2, 3, 1]; // index = parity << 1 | (y & 1)

/// The pentagon flavor (0-3) of a triple's cell — orientation-independent.
pub fn triple_flavor(t: &Triple) -> u8 {
    FLAVOR_LUT[(((t.x + t.y + t.z) << 1) | (t.y & 1)) as usize]
}

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
