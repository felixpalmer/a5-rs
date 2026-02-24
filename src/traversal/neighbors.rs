// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use crate::core::tiling::get_pentagon_flavor;
use crate::lattice::types::{Anchor, Flip};

// [di, dj, flip0, flip1]
type NeighborPattern = (i32, i32, Flip, Flip);

pub const NEIGHBORS: [&[NeighborPattern]; 8] = [
    // Flavor 0
    &[
        (0, -2, -1, 1),
        (0, -2, -1, -1),
        (0, -1, 1, -1),
        (0, -1, -1, -1),
        (0, -1, 1, 1),
        (1, -2, -1, -1),
        (1, -1, -1, 1),
        (1, -1, 1, -1),
        (1, 0, 1, -1),
        (2, -1, 1, -1),
        (2, -2, -1, -1),
    ],
    // Flavor 1
    &[
        (-1, -1, -1, 1),
        (0, -2, -1, -1),
        (0, -1, -1, -1),
        (0, -1, 1, -1),
        (0, 0, -1, 1),
        (0, 0, -1, -1),
        (0, 1, 1, -1),
        (0, 1, 1, 1),
        (1, -2, -1, -1),
        (1, -1, 1, -1),
        (1, -1, -1, -1),
        (1, 0, 1, -1),
    ],
    // Flavor 2
    &[
        (-2, 2, -1, -1),
        (-2, 1, 1, -1),
        (-1, 0, 1, -1),
        (-1, 1, 1, -1),
        (-1, 1, -1, 1),
        (-1, 2, -1, -1),
        (0, 1, -1, -1),
        (0, 1, 1, -1),
        (0, 1, 1, 1),
        (0, 2, -1, -1),
        (0, 2, -1, 1),
    ],
    // Flavor 3
    &[
        (-1, 0, 1, -1),
        (-1, 1, 1, -1),
        (-1, 1, -1, -1),
        (-1, 2, -1, -1),
        (0, -1, 1, -1),
        (0, -1, 1, 1),
        (0, 0, -1, -1),
        (0, 0, -1, 1),
        (0, 1, -1, -1),
        (0, 1, 1, -1),
        (0, 2, -1, -1),
        (1, 1, -1, 1),
    ],
    // Flavor 4
    &[
        (0, -1, 1, -1),
        (0, -1, 1, 1),
        (0, 0, -1, -1),
        (0, 0, -1, 1),
        (0, 1, -1, -1),
        (1, 0, -1, -1),
        (1, 0, 1, -1),
        (1, -1, 1, -1),
        (1, 1, -1, 1),
        (2, -1, 1, -1),
        (2, 0, -1, -1),
    ],
    // Flavor 5
    &[
        (-1, 1, -1, 1),
        (0, -1, 1, -1),
        (0, 0, -1, -1),
        (0, 1, -1, -1),
        (0, 1, 1, -1),
        (0, 1, 1, 1),
        (0, 2, -1, -1),
        (0, 2, -1, 1),
        (1, -1, 1, -1),
        (1, 0, -1, -1),
        (1, 0, 1, -1),
        (1, 1, -1, -1),
    ],
    // Flavor 6
    &[
        (-2, 0, -1, -1),
        (-2, 1, 1, -1),
        (-1, -1, -1, 1),
        (-1, 0, -1, -1),
        (-1, 0, 1, -1),
        (-1, 1, 1, -1),
        (0, -1, -1, -1),
        (0, 0, -1, -1),
        (0, 0, -1, 1),
        (0, 1, 1, -1),
        (0, 1, 1, 1),
    ],
    // Flavor 7
    &[
        (-1, -1, -1, -1),
        (-1, 0, -1, -1),
        (-1, 0, 1, -1),
        (-1, 1, 1, -1),
        (0, -2, -1, -1),
        (0, -2, -1, 1),
        (0, -1, -1, -1),
        (0, -1, 1, -1),
        (0, -1, 1, 1),
        (0, 0, -1, -1),
        (0, 1, 1, -1),
        (1, -1, -1, 1),
    ],
];

/// Check if two anchors are neighbors in uv/raw IJ space.
pub fn is_neighbor(origin: &Anchor, candidate: &Anchor) -> bool {
    let origin_flavor = get_pentagon_flavor(origin) as usize;
    let candidate_flavor = get_pentagon_flavor(candidate) as usize;
    if origin_flavor == candidate_flavor {
        return false;
    }
    let neighbors = NEIGHBORS[origin_flavor];
    let relative = (
        (candidate.offset.x() - origin.offset.x()) as i32,
        (candidate.offset.y() - origin.offset.y()) as i32,
        candidate.flips[0] * origin.flips[0],
        candidate.flips[1] * origin.flips[1],
    );

    for pattern in neighbors {
        if relative.0 == pattern.0
            && relative.1 == pattern.1
            && relative.2 == pattern.2
            && relative.3 == pattern.3
        {
            return true;
        }
    }

    false
}
