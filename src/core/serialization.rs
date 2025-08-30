// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use crate::core::origin::get_origins;
use crate::core::utils::{A5Cell, OriginId};

pub const FIRST_HILBERT_RESOLUTION: i32 = 2;
pub const MAX_RESOLUTION: i32 = 30;
pub const HILBERT_START_BIT: u32 = 58; // 64 - 6 bits for origin & segment

// First 6 bits 0, remaining 58 bits 1
pub const REMOVAL_MASK: u64 = 0x03ffffffffffffff;

// First 6 bits 1, remaining 58 bits 0
pub const ORIGIN_SEGMENT_MASK: u64 = 0xfc00000000000000;

// All 64 bits 1
pub const ALL_ONES: u64 = 0xffffffffffffffff;

// Abstract cell that contains the whole world, has resolution -1 and 12 children,
// which are the res0 cells.
pub const WORLD_CELL: u64 = 0;

pub fn get_resolution(index: u64) -> i32 {
    // Find resolution from position of first non-00 bits from the right
    let mut resolution = MAX_RESOLUTION - 1;
    let mut shifted = index >> 1; // TODO check if non-zero for point level

    while resolution > -1 && (shifted & 0b1) == 0 {
        resolution -= 1;
        // For non-Hilbert resolutions, resolution marker moves by 1 bit per resolution
        // For Hilbert resolutions, resolution marker moves by 2 bits per resolution
        shifted >>= if resolution < FIRST_HILBERT_RESOLUTION {
            1
        } else {
            2
        };
    }

    resolution
}

pub fn deserialize(index: u64) -> Result<A5Cell, String> {
    let resolution = get_resolution(index);

    // Technically not a resolution, but can be useful to think of as an
    // abstract cell that contains the whole world
    if resolution == -1 {
        return Ok(A5Cell {
            origin_id: 0,
            segment: 0,
            s: 0,
            resolution,
        });
    }

    // Extract origin*segment from top 6 bits
    let top6_bits = (index >> 58) as usize;

    // Find origin and segment that multiply to give this product
    let (origin_id, segment) = if resolution == 0 {
        let origin_id = top6_bits;
        let origins = get_origins();
        if origin_id >= origins.len() {
            return Err(format!("Could not parse origin: {}", top6_bits));
        }
        (origin_id as OriginId, 0)
    } else {
        let origin_id = top6_bits / 5;
        let origins = get_origins();
        if origin_id >= origins.len() {
            return Err(format!("Could not parse origin: {}", top6_bits));
        }
        let origin = &origins[origin_id];
        let segment = (top6_bits + origin.first_quintant) % 5;
        (origin_id as OriginId, segment)
    };

    if resolution < FIRST_HILBERT_RESOLUTION {
        return Ok(A5Cell {
            origin_id,
            segment,
            s: 0,
            resolution,
        });
    }

    // Mask away origin & segment and shift away resolution and 00 bits
    let hilbert_levels = resolution - FIRST_HILBERT_RESOLUTION + 1;
    let hilbert_bits = 2 * hilbert_levels as u32;
    let shift = HILBERT_START_BIT - hilbert_bits;
    let s = (index & REMOVAL_MASK) >> shift;

    Ok(A5Cell {
        origin_id,
        segment,
        s: s,
        resolution,
    })
}

pub fn serialize(cell: &A5Cell) -> Result<u64, String> {
    let A5Cell {
        origin_id,
        segment,
        s,
        resolution,
    } = cell;

    if *resolution > MAX_RESOLUTION {
        return Err(format!("Resolution ({}) is too large", resolution));
    }

    if *resolution == -1 {
        return Ok(WORLD_CELL);
    }

    // Position of resolution marker as bit shift from LSB
    let r = if *resolution < FIRST_HILBERT_RESOLUTION {
        // For non-Hilbert resolutions, resolution marker moves by 1 bit per resolution
        *resolution as u32 + 1
    } else {
        // For Hilbert resolutions, resolution marker moves by 2 bits per resolution
        let hilbert_resolution = 1 + *resolution - FIRST_HILBERT_RESOLUTION;
        2 * hilbert_resolution as u32 + 1
    };

    // First 6 bits are the origin id and the segment
    let origin = &crate::core::origin::get_origins()[*origin_id as usize];
    let segment_n = (*segment + 5 - origin.first_quintant) % 5;

    let mut index = if *resolution == 0 {
        (*origin_id as u64) << 58
    } else {
        ((5 * (*origin_id as usize) + segment_n) as u64) << 58
    };

    if *resolution >= FIRST_HILBERT_RESOLUTION {
        // Number of bits required for S Hilbert curve
        let hilbert_levels = *resolution - FIRST_HILBERT_RESOLUTION + 1;
        let hilbert_bits = 2 * hilbert_levels as u32;

        // Check if S fits in the required bits
        let max_s = 1u64 << hilbert_bits;
        if *s >= max_s {
            return Err(format!(
                "S ({}) is too large for resolution level {}",
                s, resolution
            ));
        }

        // S is already u64
        let s_u64 = *s;

        // Next (2 * hilbertResolution) bits are S (hilbert index within segment)
        index += s_u64 << (HILBERT_START_BIT - hilbert_bits);
    }

    // Resolution is encoded by position of the least significant 1
    index |= 1u64 << (HILBERT_START_BIT - r);

    Ok(index)
}

pub fn cell_to_children(index: u64, child_resolution: Option<i32>) -> Result<Vec<u64>, String> {
    let cell = deserialize(index)?;
    let A5Cell {
        origin_id,
        segment,
        s,
        resolution: current_resolution,
    } = cell;
    let new_resolution = child_resolution.unwrap_or(current_resolution + 1);

    if new_resolution < current_resolution {
        return Err(format!(
            "Target resolution ({}) must be equal to or greater than current resolution ({})",
            new_resolution, current_resolution
        ));
    }

    if new_resolution > MAX_RESOLUTION {
        return Err(format!(
            "Target resolution ({}) exceeds maximum resolution ({})",
            new_resolution, MAX_RESOLUTION
        ));
    }

    // If target resolution equals current resolution, return the original cell
    if new_resolution == current_resolution {
        return Ok(vec![index]);
    }

    let mut new_origin_ids = vec![origin_id];
    let mut new_segments = vec![segment];

    if current_resolution == -1 {
        new_origin_ids = (0..12).collect();
    }

    if (current_resolution == -1 && new_resolution > 0) || current_resolution == 0 {
        new_segments = vec![0, 1, 2, 3, 4];
    }

    let resolution_diff =
        new_resolution - std::cmp::max(current_resolution, FIRST_HILBERT_RESOLUTION - 1);
    let children_count = if resolution_diff <= 0 {
        1
    } else if resolution_diff > 20 {
        // Prevent overflow
        return Err("Resolution difference too large".to_string());
    } else {
        4_usize.pow(resolution_diff as u32)
    };
    let mut children = Vec::new();
    let shifted_s = if resolution_diff > 0 {
        s << (2 * resolution_diff)
    } else {
        s
    };

    for &new_origin_id in &new_origin_ids {
        for &new_segment in &new_segments {
            for i in 0..children_count {
                let new_s = shifted_s + i as u64;
                let new_cell = A5Cell {
                    origin_id: new_origin_id,
                    segment: new_segment,
                    s: new_s,
                    resolution: new_resolution,
                };
                children.push(serialize(&new_cell)?);
            }
        }
    }

    Ok(children)
}

pub fn cell_to_parent(index: u64, parent_resolution: Option<i32>) -> Result<u64, String> {
    let cell = deserialize(index)?;
    let A5Cell {
        origin_id,
        segment,
        s,
        resolution: current_resolution,
    } = cell;
    let new_resolution = parent_resolution.unwrap_or(current_resolution - 1);

    if new_resolution < 0 {
        return Err(format!(
            "Target resolution ({}) cannot be negative",
            new_resolution
        ));
    }

    if new_resolution > current_resolution {
        return Err(format!(
            "Target resolution ({}) must be equal to or less than current resolution ({})",
            new_resolution, current_resolution
        ));
    }

    if new_resolution == current_resolution {
        return Ok(index);
    }

    let resolution_diff = current_resolution - new_resolution;
    let shifted_s = &s >> (2 * resolution_diff);
    let new_cell = A5Cell {
        origin_id,
        segment,
        s: shifted_s,
        resolution: new_resolution,
    };

    serialize(&new_cell)
}

/// Returns resolution 0 cells of the A5 system, which serve as a starting point
/// for all higher-resolution subdivisions in the hierarchy.
///
/// Returns Array of 12 cell indices
pub fn get_res0_cells() -> Result<Vec<u64>, String> {
    cell_to_children(WORLD_CELL, Some(0))
}
