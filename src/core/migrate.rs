// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use crate::core::hilbert::Orientation;
use crate::core::origin::{face_step, quintant_to_segment};
use crate::core::serialization::{deserialize, serialize, FIRST_HILBERT_RESOLUTION};
use crate::core::utils::A5Cell;
use crate::lattice::{compat_s_to_triple, triple_to_s};

// The v0 face layouts, by origin id (curve order): the quintant orientations
// and first quintant of every face. Windings are the same as in v1.
const FAN: [Orientation; 5] = [
    Orientation::VU,
    Orientation::UW,
    Orientation::VW,
    Orientation::VW,
    Orientation::VW,
];
const COUNTER_STEP: [Orientation; 5] = [
    Orientation::WU,
    Orientation::UV,
    Orientation::WV,
    Orientation::WU,
    Orientation::UW,
];
const COUNTER_JUMP: [Orientation; 5] = [
    Orientation::VU,
    Orientation::UV,
    Orientation::WV,
    Orientation::WU,
    Orientation::UW,
];
const CLOCKWISE_STEP: [Orientation; 5] = [
    Orientation::WU,
    Orientation::UW,
    Orientation::VW,
    Orientation::VU,
    Orientation::UW,
];
const V0_LAYOUTS: [([Orientation; 5], usize); 12] = [
    (FAN, 4),
    (COUNTER_JUMP, 2),
    (COUNTER_STEP, 3),
    (COUNTER_STEP, 0),
    (CLOCKWISE_STEP, 2),
    (COUNTER_JUMP, 4),
    (CLOCKWISE_STEP, 2),
    (CLOCKWISE_STEP, 2),
    (COUNTER_STEP, 3),
    (COUNTER_JUMP, 0),
    (COUNTER_JUMP, 3),
    (CLOCKWISE_STEP, 0),
];

/// Migrates a cell id from the v0 index (a5 <= 0.10) to the v1 index. Both
/// versions share the same cells; the v1 index threads the curve differently:
/// a new curve within each quintant, and a new quintant order on some faces.
/// So the cell is located in v0 terms (quintant + lattice triple, via the
/// original curve) and re-encoded in v1 terms.
///
/// # Arguments
///
/// * `cell` - A cell id in the v0 index
///
/// # Returns
///
/// The id of the same cell in the v1 index
pub fn migrate(cell: u64) -> Result<u64, String> {
    let data = deserialize(cell)?;
    if data.resolution < FIRST_HILBERT_RESOLUTION - 1 {
        return Ok(cell);
    }

    // Locate the cell's quintant in the v0 layout. Windings are unchanged, so
    // the v0 face shares the v1 face's direction of travel.
    let origin = data.origin();
    let (v0_orientation, v0_first_quintant) = V0_LAYOUTS[data.origin_id as usize];
    let step = face_step(origin);
    let face_relative_quintant = (data.segment + 5 - origin.first_quintant) % 5;
    let quintant =
        (v0_first_quintant as i32 + step * face_relative_quintant as i32).rem_euclid(5) as usize;
    let (segment, orientation) = quintant_to_segment(quintant, origin);
    if data.resolution == FIRST_HILBERT_RESOLUTION - 1 {
        return serialize(&A5Cell {
            origin_id: data.origin_id,
            segment,
            s: 0,
            resolution: data.resolution,
        });
    }

    let hilbert_resolution = (1 + data.resolution - FIRST_HILBERT_RESOLUTION) as usize;
    let triple = compat_s_to_triple(
        data.s,
        hilbert_resolution,
        v0_orientation[face_relative_quintant],
    );
    let s = triple_to_s(&triple, hilbert_resolution, orientation)
        .ok_or_else(|| format!("Invalid triple for cell {cell}"))?;
    serialize(&A5Cell {
        origin_id: data.origin_id,
        segment,
        s,
        resolution: data.resolution,
    })
}
