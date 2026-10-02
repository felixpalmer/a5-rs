// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use crate::core::origin::segment_to_quintant;
use crate::core::serialization::{deserialize, serialize, FIRST_HILBERT_RESOLUTION};
use crate::core::utils::A5Cell;
use crate::lattice::{compat_s_to_triple, triple_to_s};

/// Migrates a cell id from the v0 index (a5 <= 0.10, original curve) to the v1
/// index (non-self-intersecting curve). Both versions share the same cells and
/// the same origin/segment/resolution bits; only the curve position S within
/// the quintant differs, so the old S is decoded to its lattice triple and
/// re-encoded along the new curve.
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
    if data.resolution < FIRST_HILBERT_RESOLUTION {
        return Ok(cell);
    }

    let (_, orientation) = segment_to_quintant(data.segment, data.origin());
    let hilbert_resolution = (1 + data.resolution - FIRST_HILBERT_RESOLUTION) as usize;
    let triple = compat_s_to_triple(data.s, hilbert_resolution, orientation);
    let s = triple_to_s(&triple, hilbert_resolution, orientation)
        .ok_or_else(|| format!("Invalid triple for cell {cell}"))?;
    serialize(&A5Cell {
        origin_id: data.origin_id,
        segment: data.segment,
        s,
        resolution: data.resolution,
    })
}
