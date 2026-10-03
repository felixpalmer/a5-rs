// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use crate::lattice::lsystem::triple_to_s_lattice;
use crate::lattice::types::{Orientation, Triple};

/// The parity of a triple (0 or 1), equal to x + y + z.
pub fn triple_parity(t: &Triple) -> i32 {
    t.x + t.y + t.z
}

// The pentagon flavor is a CLOSED FORM of the triple. The A5 tiling is gyro
// applied to the square grid R left when every lattice edge parallel to the
// quintant's dodecahedron edge is deleted (A5 = g o^r D in Conway notation).
// Bit 0 is the triangle's parity (which half of its rhombus it is); bit 1 is
// the colour of its apex in the 2-colouring of R, coloured from a dodecahedron
// vertex. The apex of either triangle in unit square (m, n) has colour
// (m + n + max_row + 1) & 1, and x + z = -(m + n). For max_row + 1 even (every
// resolution but 0) face centres and vertices share a colour, so bit 1 is just
// (x + z) & 1; at resolution 0 they differ, which gives the corner cell flavor 2.
// Verified against the descent's flavor over all cells (tests/lattice_curve.rs).

/// The pentagon flavor (0-3) of a triple's cell — orientation-independent.
pub fn triple_flavor(t: &Triple, max_row: i32) -> u8 {
    ((t.x + t.y + t.z) | (((max_row + 1 + t.x + t.z) & 1) << 1)) as u8
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
