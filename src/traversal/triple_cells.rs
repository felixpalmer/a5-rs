// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

// Cells handled in triple space — (origin_id, quintant, x, y, z) — by the
// traversal algorithms that walk many neighboring cells: they key and dedup
// cells as plain integers and encode a cell to its ID only when it is output.

use std::sync::LazyLock;

use crate::core::origin::{get_origins, quintant_to_segment};
use crate::core::serialization::serialize;
use crate::core::utils::A5Cell;
use crate::lattice::{triple_to_s, Orientation, Triple};

// A cell's key packs the quintant (origin.id * 5 + quintant, < 60), parity,
// and the low KEY_BITS bits of -x and -z (y follows). Up to Hilbert resolution
// 21 the coordinates fit whole; above it, two cells of one quintant share a key
// only if they are 2^22 rows apart, and a walk holding both would need ~2^21
// steps (~10^12 cells for a disk) — far past what fits in memory.
const KEY_BITS: u32 = 22;
const KEY_MASK: i64 = (1 << KEY_BITS) - 1;
const KEY_SIDE: i64 = 1 << KEY_BITS;

/// Segment and curve orientation of each of the 60 quintants, by origin.id * 5 + quintant.
static QUINTANT_SEGMENTS: LazyLock<Vec<(usize, Orientation)>> = LazyLock::new(|| {
    let mut out = Vec::with_capacity(60);
    for origin in get_origins() {
        for q in 0..5 {
            out.push(quintant_to_segment(q, origin));
        }
    }
    out
});

/// The integer key of a cell, unique among the cells of any one traversal.
pub fn triple_cell_key(cell: [i32; 5]) -> i64 {
    let [origin_id, quintant, x, y, z] = cell;
    ((-x as i64 & KEY_MASK) * KEY_SIDE + (-z as i64 & KEY_MASK)) * 2
        + (x + y + z) as i64
        + (origin_id * 5 + quintant) as i64 * 2 * KEY_SIDE * KEY_SIDE
}

/// The cell ID of a cell given in triple space.
pub fn triple_cell_to_id(
    cell: [i32; 5],
    hilbert_res: usize,
    resolution: i32,
) -> Result<u64, String> {
    let [origin_id, quintant, x, y, z] = cell;
    let (segment, orientation) = QUINTANT_SEGMENTS[(origin_id * 5 + quintant) as usize];
    let s = triple_to_s(&Triple::new(x, y, z), hilbert_res, orientation)
        .ok_or("triple_cell_to_id: invalid triple")?;
    serialize(&A5Cell {
        origin_id: origin_id as u8,
        segment,
        s,
        resolution,
    })
}
