// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use crate::core::origin::get_origins;
use crate::core::utils::{A5Cell, OriginId};

pub const FIRST_HILBERT_RESOLUTION: i32 = 2;
pub const MAX_RESOLUTION: i32 = 30;
pub const HILBERT_START_BIT: u32 = 58; // 64 - 6 bits for origin & segment

// Abstract cell that contains the whole world, has resolution -1 and 12 children,
// which are the res0 cells.
pub const WORLD_CELL: u64 = 0;

pub fn get_resolution(index: u64) -> i32 {
    if index == 0 {
        return -1;
    }

    // Resolution 30 uses three encoding patterns:
    //   ...1     -> 5-bit quintant (0-31),  58-bit S
    //   ...100   -> 3-bit quintant (32-39), 58-bit S
    //   ...10000 -> 1-bit quintant (40-41), 58-bit S
    if (index & 1) != 0 || (index & 0b111) == 0b100 || (index & 0b11111) == 0b10000 {
        return MAX_RESOLUTION;
    }

    let mut resolution = MAX_RESOLUTION - 1;
    let mut shifted = index >> 1;
    if shifted == 0 {
        return -1;
    }

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

    // For res 30, quintant bits are fewer to make room for S:
    //   ...1     marker (1 bit)  -> 5-bit quintant (0-31)
    //   ...100   marker (3 bits) -> 3-bit quintant + 32 (32-39)
    //   ...10000 marker (5 bits) -> 1-bit quintant + 40 (40-41)
    let mut quintant_shift = HILBERT_START_BIT;
    let mut quintant_offset: usize = 0;
    if resolution == MAX_RESOLUTION {
        let marker_bits: u32 = if (index & 1) != 0 {
            1
        } else if (index & 0b100) != 0 {
            3
        } else {
            5
        };
        quintant_shift = HILBERT_START_BIT + marker_bits;
        quintant_offset = match marker_bits {
            1 => 0,
            3 => 32,
            _ => 40,
        };
    }

    // Extract origin*segment from top bits
    let top_bits = (index >> quintant_shift) as usize + quintant_offset;

    // Find origin and segment
    let origins = get_origins();
    let (origin_id, segment) = if resolution == 0 {
        if top_bits >= origins.len() {
            return Err(format!("Could not parse origin: {}", top_bits));
        }
        (top_bits as OriginId, 0)
    } else {
        let origin_id = top_bits / 5;
        if origin_id >= origins.len() {
            return Err(format!("Could not parse origin: {}", top_bits));
        }
        let origin = &origins[origin_id];
        let segment = (top_bits + origin.first_quintant) % 5;
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

    // Mask away origin & segment and shift away resolution and marker bits
    let hilbert_levels = resolution - FIRST_HILBERT_RESOLUTION + 1;
    let hilbert_bits = 2 * hilbert_levels as u32;
    let removal_mask = (1u64 << quintant_shift) - 1;
    let s = (index & removal_mask) >> (quintant_shift - hilbert_bits);

    Ok(A5Cell {
        origin_id,
        segment,
        s,
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

    // For res 30, quintant bits are fewer to make room for S:
    //   quintant 0-31:  ...1     marker -> 5-bit quintant
    //   quintant 32-39: ...100   marker -> 3-bit quintant + 32
    //   quintant 40-41: ...10000 marker -> 1-bit quintant + 40
    //   quintant 42+:   fall back to res 29
    let mut quintant_shift = HILBERT_START_BIT;

    // Position of resolution marker as bit shift from LSB
    let r = if *resolution < FIRST_HILBERT_RESOLUTION {
        *resolution as u32 + 1
    } else {
        let hilbert_resolution = 1 + *resolution - FIRST_HILBERT_RESOLUTION;
        2 * hilbert_resolution as u32 + 1
    };

    // Top bits encode the origin id and segment
    let origin = &crate::core::origin::get_origins()[*origin_id as usize];
    let segment_n = (*segment + 5 - origin.first_quintant) % 5;

    let mut index = if *resolution == 0 {
        (*origin_id as u64) << quintant_shift
    } else {
        let quintant = 5 * (*origin_id as usize) + segment_n;
        if *resolution == MAX_RESOLUTION {
            let quintant_value;
            if quintant <= 31 {
                quintant_shift = HILBERT_START_BIT + 1;
                quintant_value = quintant;
            } else if quintant <= 39 {
                quintant_shift = HILBERT_START_BIT + 3;
                quintant_value = quintant - 32;
            } else if quintant <= 41 {
                quintant_shift = HILBERT_START_BIT + 5;
                quintant_value = quintant - 40;
            } else {
                return serialize(&A5Cell {
                    origin_id: *origin_id,
                    segment: *segment,
                    s: *s >> 2,
                    resolution: MAX_RESOLUTION - 1,
                });
            }
            (quintant_value as u64) << quintant_shift
        } else {
            (quintant as u64) << quintant_shift
        }
    };

    if *resolution >= FIRST_HILBERT_RESOLUTION {
        let hilbert_levels = *resolution - FIRST_HILBERT_RESOLUTION + 1;
        let hilbert_bits = 2 * hilbert_levels as u32;
        let max_s = 1u64 << hilbert_bits;
        if *s >= max_s {
            return Err(format!(
                "S ({}) is too large for resolution level {}",
                s, resolution
            ));
        }
        index += *s << (quintant_shift - hilbert_bits);
    }

    // Resolution is encoded by position of the least significant 1
    index |= 1u64 << (quintant_shift - r);

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

/// Cheap predicate that mirrors the first three checks in `get_resolution`:
/// res-30 cells are exactly those whose low bits match one of the three
/// variable-width quintant marker patterns.
fn is_max_resolution(index: u64) -> bool {
    (index & 1) != 0 || (index & 0b111) == 0b100 || (index & 0b11111) == 0b10000
}

/// Re-pack a res-30 cell into the standard res-29 bit layout (6-bit quintant
/// in [63..58], 56-bit S in [57..2], marker at bit 1). The 58-bit res-30 S is
/// truncated by 2 bits, exactly as `cell_to_parent(_, 29)` would.
fn normalize_res30(index: u64) -> u64 {
    let (q_shift, q_offset, marker_bits): (u32, u64, u32) = if (index & 1) != 0 {
        (59, 0, 1)
    } else if (index & 0b100) != 0 {
        (61, 32, 3)
    } else {
        (63, 40, 5)
    };
    let quintant = (index >> q_shift) + q_offset;
    let s58 = (index >> marker_bits) & ((1u64 << 58) - 1);
    (quintant << 58) | ((s58 >> 2) << 2) | (1u64 << 1)
}

/// Walk a cell up the hierarchy to a coarser resolution.
///
/// Implemented as pure bit ops over the encoded index — no deserialize /
/// serialize round-trip. The three encoding regimes (non-Hilbert res 0/1,
/// Hilbert res 2..29, variable-width res 30) all reduce to the same shape
/// after a small amount of normalization.
pub fn cell_to_parent(index: u64, parent_resolution: Option<i32>) -> Result<u64, String> {
    let parent_resolution = parent_resolution.unwrap_or_else(|| get_resolution(index) - 1);

    // Special case: parent of resolution 0 cells is the world cell
    if parent_resolution == -1 {
        return Ok(WORLD_CELL);
    }
    if !(-1..=MAX_RESOLUTION).contains(&parent_resolution) {
        return Err(format!(
            "Target resolution ({}) is out of range",
            parent_resolution
        ));
    }
    if index == WORLD_CELL {
        return Err(format!(
            "Target resolution ({}) must be equal to or less than current resolution (-1)",
            parent_resolution
        ));
    }

    // Normalize res-30 children to the standard res-29 layout. After this,
    // the fast paths below treat the cell as a Hilbert-range cell.
    let mut c = index;
    if is_max_resolution(index) {
        if parent_resolution == MAX_RESOLUTION {
            return Ok(index); // identity (already res 30)
        }
        c = normalize_res30(index);
        if parent_resolution == MAX_RESOLUTION - 1 {
            return Ok(c);
        }
    }

    if parent_resolution >= FIRST_HILBERT_RESOLUTION {
        // Hilbert-range parent: clear bits below the parent marker, set the marker.
        // Identity (parent res === child res) falls out for free: the marker lands
        // in the same position and bits below the keep cut are already zero.
        let keep_shift = (60 - 2 * parent_resolution) as u32;
        return Ok(
            ((c >> keep_shift) << keep_shift) | (1u64 << (59 - 2 * parent_resolution) as u32)
        );
    }

    if parent_resolution == 1 {
        // Top 6 bits already encode 5*originId + segmentN; only the marker moves.
        // Identity (cell already at res 1) is preserved.
        return Ok(((c >> 58) << 58) | (1u64 << 56));
    }

    // parent_resolution == 0: top 6 bits change from quintant (0-59) to originId (0-11).
    // Identity (cell already at res 0) needs an explicit guard since dividing
    // an originId by 5 would corrupt it. A res-0 cell has bit 57 set with all
    // lower bits zero — equivalently, all bottom 57 bits are zero.
    if (c & ((1u64 << 57) - 1)) == 0 {
        return Ok(c);
    }
    Ok((((c >> 58) / 5) << 58) | (1u64 << 57))
}

/// Returns resolution 0 cells of the A5 system, which serve as a starting point
/// for all higher-resolution subdivisions in the hierarchy.
///
/// Returns Array of 12 cell indices
pub fn get_res0_cells() -> Result<Vec<u64>, String> {
    cell_to_children(WORLD_CELL, Some(0))
}

/// Check whether index corresponds to first child of its parent
pub fn is_first_child(index: u64, resolution: Option<i32>) -> bool {
    let resolution = resolution.unwrap_or_else(|| get_resolution(index));

    if resolution < 2 {
        // For resolution 0: first child is origin 0 (child count = 12)
        // For resolution 1: first children are at multiples of 5 (child count = 5)
        let top6_bits = (index >> HILBERT_START_BIT) as usize;
        let child_count = if resolution == 0 { 12 } else { 5 };
        return top6_bits % child_count == 0;
    }

    if resolution == MAX_RESOLUTION {
        // S's 2 LSBs sit just above the marker bits
        let marker_bits: u32 = if (index & 1) != 0 {
            1
        } else if (index & 0b100) != 0 {
            3
        } else {
            5
        };
        return (index & (3u64 << marker_bits)) == 0;
    }

    let s_position = 2 * (MAX_RESOLUTION - resolution) as u32;
    let s_mask = 3u64 << s_position; // Mask for the 2 LSBs of S
    (index & s_mask) == 0
}

/// Bit-level descendant test: is `child` the same cell as `parent`, or one of
/// its descendants at any deeper resolution? Compares the high (quintant +
/// parent's Hilbert) bits in a single shift, no deserialize needed.
///
/// Restricted to the Hilbert range: `parent_resolution` must be in
/// [FIRST_HILBERT_RESOLUTION .. MAX_RESOLUTION - 1], and `child` must not be
/// a resolution-30 cell (whose encoding uses a variable quintant shift).
/// Callers handling those cases should fall back to `cell_to_parent` equality.
pub fn is_child_of(child: u64, parent: u64, parent_resolution: i32) -> bool {
    // Parent's identifying bits occupy positions 63..(60-2P): 6 quintant bits
    // + 2(P-1) Hilbert bits. Bit (59-2P) is the marker, below that is zero.
    // Shifting both right by (60-2P) keeps exactly those identifying bits and
    // discards the marker, so a descendant matches iff the high bits match.
    let shift = (60 - 2 * parent_resolution) as u32;
    (child >> shift) == (parent >> shift)
}

/// Difference between two neighbouring sibling cells at a given resolution
pub fn get_stride(resolution: i32) -> u64 {
    // Both level 0 & 1 just write values 0-11 or 0-59 to the first 6 bits
    if resolution < 2 {
        return 1u64 << HILBERT_START_BIT;
    }

    // For res 30, S is shifted left by 1 (marker bit at position 0)
    if resolution == MAX_RESOLUTION {
        return 2;
    }

    // For hilbert levels, the position shifts by 2 bits per resolution level
    let s_position = 2 * (MAX_RESOLUTION - resolution) as u32;
    1u64 << s_position
}
