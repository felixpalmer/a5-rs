// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use crate::coordinate_systems::IJ;
use crate::lattice::anchor::offset_flips_to_anchor;
use crate::lattice::hilbert::{anchor_to_s, ij_to_flips, ij_to_s, s_to_anchor};
use crate::lattice::types::{Anchor, Orientation, NO, YES};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Triple {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl Triple {
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }
}

/// The parity of a triple (0 or 1), equal to x + y + z.
pub fn triple_parity(t: &Triple) -> i32 {
    t.x + t.y + t.z
}

/// Check if a triple is within valid quintant bounds.
pub fn triple_in_bounds(t: &Triple, max_row: i32) -> bool {
    let s = t.x + t.y + t.z;
    if s != 0 && s != 1 {
        return false;
    }
    let limit = t.y - s;
    t.x <= 0 && t.z <= 0 && t.y >= 0 && t.y <= max_row && t.x >= -limit && t.z >= -limit
}

/// Convert triple coordinates to an s-value (Hilbert index).
/// Returns None if the triple has invalid parity.
pub fn triple_to_s(t: &Triple, resolution: usize, orientation: Orientation) -> Option<u64> {
    let anchor = triple_to_anchor(t, resolution, orientation)?;
    Some(anchor_to_s(&anchor, resolution, orientation))
}

/// Compute triple coordinates from an anchor.
///
/// Maps the pentagonal A5 grid to a triangular grid coordinate system where
/// neighbors differ by +/-1 in exactly one coordinate while the other two stay constant.
pub fn anchor_to_triple(anchor: &Anchor) -> Triple {
    // Start with shift in IJ space
    let mut shift_i: f64 = 0.25;
    let mut shift_j: f64 = 0.25;
    let flip0 = anchor.flips[0];
    let flip1 = anchor.flips[1];

    // First check for [1, -1] rotation
    if flip0 == NO && flip1 == YES {
        // Rotate 180 degrees
        shift_i = -shift_i;
        shift_j = -shift_j;
    }

    // Then apply additional adjustments
    if flip0 == YES && flip1 == YES {
        // Rotate 180 degrees
        shift_i = -shift_i;
        shift_j = -shift_j;
    } else if flip0 == YES {
        // Shift left (subtract w = [0, 1])
        shift_j -= 1.0;
    } else if flip1 == YES {
        // Shift right (add w = [0, 1])
        shift_j += 1.0;
    }

    // Compute center
    let i = anchor.offset.x() + shift_i;
    let j = anchor.offset.y() + shift_j;

    // Compute row and column in triangular grid
    let r = (i + j) - 0.5;
    let c = (i - j) + r;

    // Compute triple coordinates (all integers for valid anchors)
    let x = ((c + 1.0) / 2.0 - r).floor() as i32;
    let y = r as i32;
    let z = ((1.0 - c) / 2.0).floor() as i32;

    Triple::new(x, y, z)
}

/// Convert triple coordinates to an Anchor.
///
/// This is the inverse of anchor_to_triple().
pub fn triple_to_anchor(t: &Triple, resolution: usize, orientation: Orientation) -> Option<Anchor> {
    let (x, y, z) = (t.x, t.y, t.z);

    // Verify parity constraint
    let s = x + y + z;
    if s != 0 && s != 1 {
        return None;
    }

    // Compute r and c from triple coordinates
    let r = y;
    let c_min_f = (2 * x + 2 * r - 1) as f64;
    let c_max_cand = (-2 * z - 1) as f64 + 0.0001;
    let c_min = if c_min_f > c_max_cand {
        c_min_f
    } else {
        c_max_cand
    };

    let c_max_f1 = (2 * x + 2 * r + 1) as f64 - 0.0001;
    let c_max_f2 = (1 - 2 * z) as f64;
    let c_max = if c_max_f1 < c_max_f2 {
        c_max_f1
    } else {
        c_max_f2
    };
    let c = ((c_min + c_max) / 2.0).round();

    // Compute center IJ coordinates from r and c
    let center_i = (c + 0.5) / 2.0;
    let center_j = r as f64 - c / 2.0 + 0.25;

    // Fast path for uv/vu: use ij_to_flips directly (works in raw IJ space)
    if matches!(orientation, Orientation::UV | Orientation::VU) {
        let flips = ij_to_flips(IJ::new(center_i, center_j), resolution);

        // Compute shift from flips (inverse of anchor_to_triple logic)
        let mut shift_i: f64 = 0.25;
        let mut shift_j: f64 = 0.25;
        if flips[0] == NO && flips[1] == YES {
            shift_i = -shift_i;
            shift_j = -shift_j;
        }
        if flips[0] == YES && flips[1] == YES {
            shift_i = -shift_i;
            shift_j = -shift_j;
        } else if flips[0] == YES {
            shift_j -= 1.0;
        } else if flips[1] == YES {
            shift_j += 1.0;
        }

        let offset = IJ::new((center_i - shift_i).round(), (center_j - shift_j).round());
        return Some(offset_flips_to_anchor(offset, flips, orientation));
    }

    // General path: ij_to_s -> s_to_anchor (handles all orientation transforms)
    let s_val = ij_to_s(IJ::new(center_i, center_j), resolution, orientation);
    Some(s_to_anchor(s_val, resolution, orientation))
}
