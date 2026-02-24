// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use crate::coordinate_systems::IJ;
use crate::lattice::types::{Anchor, Flip, Orientation, Quaternary};

fn is_group2_orientation(orientation: Orientation) -> bool {
    matches!(orientation, Orientation::UW | Orientation::WU)
}

/// Deduce the quaternary value q from offset and flip values.
///
/// Uses the discovered invariant that q can be deterministically computed
/// from offset parity and flip values.
pub fn compute_q(offset: IJ, flips: [Flip; 2], orientation: Orientation) -> Quaternary {
    let i = offset.x() as i32;
    let j = offset.y() as i32;
    let imod2 = (i & 1) as usize;
    let jmod2 = (j & 1) as usize;
    let f0idx = ((flips[0] + 1) >> 1) as usize; // Map: YES (-1) -> 0, NO (1) -> 1
    let f1idx = ((flips[1] + 1) >> 1) as usize;

    if is_group2_orientation(orientation) {
        // 4D lookup: [imod2][jmod2][f0idx][f1idx]
        let group2_lookup: [[[[Quaternary; 2]; 2]; 2]; 2] = [
            [[[0, 3], [3, 0]], [[3, 2], [2, 3]]],
            [[[2, 1], [1, 2]], [[1, 0], [0, 1]]],
        ];
        group2_lookup[imod2][jmod2][f0idx][f1idx]
    } else if imod2 == 0 {
        if jmod2 == 0 {
            0
        } else {
            2
        }
    } else {
        let odd_i_lookup: [[[Quaternary; 2]; 2]; 2] = [[[3, 1], [1, 3]], [[1, 3], [3, 1]]];
        odd_i_lookup[jmod2][f0idx][f1idx]
    }
}

/// Create a complete Anchor by deducing q from offset and flips.
pub fn offset_flips_to_anchor(offset: IJ, flips: [Flip; 2], orientation: Orientation) -> Anchor {
    let q = compute_q(offset, flips, orientation);
    Anchor { q, offset, flips }
}
