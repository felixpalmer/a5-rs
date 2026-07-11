// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

// The neighbors of a cell, in triple space, are a fixed function of its
// pentagon flavor: pentagons tile edge-to-edge, so the arrangement around a
// pentagon is forced. Every cell has exactly 5 edge-sharing and 2 vertex-only
// neighbors, at the triple deltas below. No validation is needed — each delta
// IS a neighbor (bounds permitting).
//
// Derived geometrically (shared pentagon vertices) and verified exhaustively
// over all interior cells at res 4-5, all orientations, zero conflicts. The
// flavor-1/3 lists are the flavor-0/2 lists negated (they are the 180°-rotated
// shapes).

use crate::lattice::Triple;

pub struct NeighborDeltas {
    pub edge: [Triple; 5],   // 5 edge-sharing neighbors
    pub vertex: [Triple; 2], // 2 vertex-only neighbors
    pub all: [Triple; 7],    // edge ++ vertex, spelled out so this stays a pure data table
}

const fn d(x: i32, y: i32, z: i32) -> Triple {
    Triple { x, y, z }
}

#[rustfmt::skip]
pub static NEIGHBOR_DELTAS: [NeighborDeltas; 4] = [
    NeighborDeltas { // flavor 0
        edge:   [d(0, 0, 1), d(0, 1, -1), d(0, 1, 0), d(1, -1, 0), d(1, 0, 0)],
        vertex: [d(1, -1, 1), d(1, 1, -1)],
        all:    [d(0, 0, 1), d(0, 1, -1), d(0, 1, 0), d(1, -1, 0), d(1, 0, 0), d(1, -1, 1), d(1, 1, -1)],
    },
    NeighborDeltas { // flavor 1 (= flavor 0 rotated 180°: deltas negated)
        edge:   [d(0, 0, -1), d(0, -1, 1), d(0, -1, 0), d(-1, 1, 0), d(-1, 0, 0)],
        vertex: [d(-1, 1, -1), d(-1, -1, 1)],
        all:    [d(0, 0, -1), d(0, -1, 1), d(0, -1, 0), d(-1, 1, 0), d(-1, 0, 0), d(-1, 1, -1), d(-1, -1, 1)],
    },
    NeighborDeltas { // flavor 2
        edge:   [d(-1, 1, 0), d(0, -1, 1), d(0, 0, 1), d(0, 1, 0), d(1, 0, 0)],
        vertex: [d(-1, 1, 1), d(1, -1, 1)],
        all:    [d(-1, 1, 0), d(0, -1, 1), d(0, 0, 1), d(0, 1, 0), d(1, 0, 0), d(-1, 1, 1), d(1, -1, 1)],
    },
    NeighborDeltas { // flavor 3 (= flavor 2 rotated 180°: deltas negated)
        edge:   [d(1, -1, 0), d(0, 1, -1), d(0, 0, -1), d(0, -1, 0), d(-1, 0, 0)],
        vertex: [d(1, -1, -1), d(-1, 1, -1)],
        all:    [d(1, -1, 0), d(0, 1, -1), d(0, 0, -1), d(0, -1, 0), d(-1, 0, 0), d(1, -1, -1), d(-1, 1, -1)],
    },
];
