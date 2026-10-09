// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use crate::core::origin::get_origins;
use crate::core::utils::A5Cell;

pub const FIRST_HILBERT_RESOLUTION: i32 = 2;
pub const MAX_RESOLUTION: i32 = 30;
// IDs below res 30 start with a 6-bit origin (res 0) or quintant, above S
pub const QUINTANT_SHIFT: u32 = 58;
pub const S_MASK: u64 = (1 << QUINTANT_SHIFT) - 1;

// Abstract cell that contains the whole world, has resolution -1 and 12 children,
// which are the res0 cells.
pub const WORLD_CELL: u64 = 0;

// Resolution 30 IDs have no room for a 6-bit quintant: its field is 5, 3 or 1
// bits wide, marked by the tag (lowest bits) ...1, ...100 or ...10000:
//   ...1     → 5-bit quintant (0-31),  58-bit S
//   ...100   → 3-bit quintant (32-39), 58-bit S
//   ...10000 → 1-bit quintant (40-41), 58-bit S
// Quintants 42-59 have no res-30 IDs.
/// The number of quintants (in ID order) with resolution 30 IDs.
pub const RES30_QUINTANTS: usize = 42;

/// The leaf slot of a res-30 ID: its quintant, then its 58-bit S (see Leaf slots below).
fn res30_to_slot(index: u64) -> u64 {
    if index & 1 != 0 {
        return ((index >> 59) << QUINTANT_SHIFT) | ((index >> 1) & S_MASK);
    }
    if index & 0b100 != 0 {
        return (((index >> 61) + 32) << QUINTANT_SHIFT) | ((index >> 3) & S_MASK);
    }
    (((index >> 63) + 40) << QUINTANT_SHIFT) | ((index >> 5) & S_MASK)
}

/// The res-30 ID of a leaf slot in quintants 0-41.
fn slot_to_res30(slot: u64) -> u64 {
    let q = slot >> QUINTANT_SHIFT;
    let s = slot & S_MASK;
    if q < 32 {
        (q << 59) | (s << 1) | 1
    } else if q < 40 {
        ((q - 32) << 61) | (s << 3) | 0b100
    } else {
        ((q - 40) << 63) | (s << 5) | 0b10000
    }
}

// Leaf slots. The A5 curve, at resolution 30, passes through every leaf
// (res-30) cell of the globe once: picture it as a line of slots, one per leaf
// cell, numbered in curve order from 0 to 60 * 4^29 - 1. A leaf slot is the
// 6-bit quintant (0-59) then the leaf's S, left-aligned below it.
//
// A cell at resolution r occupies 4^(30-r) consecutive slots, an aligned block
// starting at its first slot, and the cells of a resolution step along the
// slots in strides of that size, as the Hilbert curve does. Unlike cell IDs,
// whose layout differs at resolutions 0, 1 and 30, slots put every cell on one
// integer line in curve order, including all 60 quintants at resolution 30.
// A leaf slot is not a cell ID.

pub const QUINTANT_SLOTS: u64 = 1 << QUINTANT_SHIFT;
pub const ORIGIN_SLOTS: u64 = 5 * QUINTANT_SLOTS;
pub const WORLD_SLOTS: u64 = 60 * QUINTANT_SLOTS;

/// By resolution 0..30: the resolution tag (lowest set bit) of a cell below
/// res 30.
pub const RESOLUTION_TAGS: [u64; 31] = {
    let mut tags = [0; 31];
    let mut r = 0;
    while r <= 30 {
        tags[r] = match r {
            0 => 1 << 57,
            1 => 1 << 56,
            30 => 1,
            _ => 1 << (59 - 2 * r),
        };
        r += 1;
    }
    tags
};

/// By resolution 0..30: the number of slots a cell occupies.
pub const SLOT_COUNTS: [u64; 31] = {
    let mut counts = [0; 31];
    let mut r = 0;
    while r <= 30 {
        counts[r] = match r {
            0 => ORIGIN_SLOTS,
            1 => QUINTANT_SLOTS,
            _ => 1 << (60 - 2 * r),
        };
        r += 1;
    }
    counts
};

/// Res-30 IDs end in ...1, ...100 or ...10000: their tag has one of these bits
pub const RES30_TAG_BITS: u64 = 0b10101;

/// The tags of resolutions 2-29: the odd bits 55 down to 1
const HILBERT_TAG_BITS: u64 = {
    let mut bits = 0;
    let mut r = FIRST_HILBERT_RESOLUTION as usize;
    while r < MAX_RESOLUTION as usize {
        bits |= RESOLUTION_TAGS[r];
        r += 1;
    }
    bits
};

/// The first slot a cell occupies. Errors if the value is not an A5 cell ID:
/// its tag (lowest set bit) must be a resolution tag, and its origin (res 0) or
/// quintant (res 1-29) must exist. Every res-30 pattern decodes to an existing
/// quintant (0-41).
#[inline]
pub fn cell_first_slot(cell: u64) -> Result<u64, String> {
    // The resolution tag: the lowest set bit (0 for the world cell)
    let tag = cell & cell.wrapping_neg();
    if tag < RESOLUTION_TAGS[1] {
        if tag & RES30_TAG_BITS != 0 {
            return Ok(res30_to_slot(cell));
        }
        // Resolutions 2-29, in quintants 0-59: the first slot is the ID without its tag
        if tag & HILBERT_TAG_BITS != 0 && cell < WORLD_SLOTS {
            return Ok(cell - tag);
        }
        if tag == 0 {
            return Ok(0);
        }
    } else {
        // Resolution 0 (tag bit 57) starts its origin's 5 quintants, 1 (bit 56) its quintant
        let top = cell >> QUINTANT_SHIFT;
        if tag == RESOLUTION_TAGS[0] && top < 12 {
            return Ok((5 * top) << QUINTANT_SHIFT);
        }
        if tag == RESOLUTION_TAGS[1] && top < 60 {
            return Ok(top << QUINTANT_SHIFT);
        }
    }
    Err(invalid_cell(cell))
}

/// The first slot of a cell, without checking that the value is an A5 cell ID:
/// for searches, which check the cell they land on. A value that is not a cell
/// gives a meaningless slot, but never panics.
pub fn cell_first_slot_unchecked(cell: u64) -> u64 {
    let tag = cell & cell.wrapping_neg();
    if tag == 0 {
        return 0;
    }
    if tag >= RESOLUTION_TAGS[1] {
        let top = cell >> QUINTANT_SHIFT;
        let quintant = if tag == RESOLUTION_TAGS[0] {
            top.wrapping_mul(5)
        } else {
            top
        };
        return quintant << QUINTANT_SHIFT;
    }
    if tag & RES30_TAG_BITS != 0 {
        return res30_to_slot(cell);
    }
    cell - tag
}

/// The resolution of a cell, as `get_resolution` gives it, but an error if the
/// value is not an A5 cell ID (see `cell_first_slot` for what that requires).
#[inline]
pub fn checked_resolution(cell: u64) -> Result<i32, String> {
    if cell == 0 {
        return Ok(-1);
    }
    let bit = cell.trailing_zeros() as i32;
    if bit < 56 {
        if bit % 2 == 1 && cell < WORLD_SLOTS {
            return Ok((59 - bit) >> 1);
        }
        if bit <= 4 && bit % 2 == 0 {
            return Ok(MAX_RESOLUTION);
        }
    } else {
        let top = cell >> QUINTANT_SHIFT;
        if bit == 57 && top < 12 {
            return Ok(0);
        }
        if bit == 56 && top < 60 {
            return Ok(1);
        }
    }
    Err(invalid_cell(cell))
}

#[cold]
#[inline(never)]
fn invalid_cell(cell: u64) -> String {
    format!("Invalid cell: {:#x}", cell)
}

/// The number of slots a cell occupies.
pub fn cell_slot_count(cell: u64) -> u64 {
    // The resolution tag: the lowest set bit (0 for the world cell)
    let tag = cell & cell.wrapping_neg();
    if tag == 0 {
        return WORLD_SLOTS;
    }
    if tag == RESOLUTION_TAGS[0] {
        return ORIGIN_SLOTS;
    }
    if tag == RESOLUTION_TAGS[1] {
        return QUINTANT_SLOTS;
    }
    if tag & RES30_TAG_BITS != 0 {
        return 1;
    }
    // Resolutions 2-29: the slots are symmetric about the ID
    tag << 1
}

/// The res-r cell whose block of slots starts at `slot`.
pub fn slot_to_cell(slot: u64, resolution: i32) -> u64 {
    if resolution < 0 {
        return WORLD_CELL;
    }
    if resolution > 0 && resolution < MAX_RESOLUTION {
        return slot + RESOLUTION_TAGS[resolution as usize];
    }
    if resolution == 0 {
        return (((slot >> QUINTANT_SHIFT) / 5) << QUINTANT_SHIFT) | RESOLUTION_TAGS[0];
    }
    slot_to_res30(slot)
}

pub fn get_resolution(index: u64) -> i32 {
    // The resolution tag: the lowest set bit (none for the world cell). Its position
    // gives the resolution: bit 57 is res 0, 56 res 1, 59 - 2r res r (2-29), and
    // res 30 uses the patterns ...1, ...100 and ...10000 (bits 0, 2 and 4).
    if index == 0 {
        return -1;
    }
    let bit = index.trailing_zeros() as i32;
    match bit {
        57 => 0,
        56 => 1,
        _ if bit <= 4 && bit % 2 == 0 => MAX_RESOLUTION,
        _ => (59 - bit) >> 1,
    }
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

    // The cell's first slot holds its quintant, then its S above the slots of one cell
    let slot = cell_first_slot(index)?;
    let quintant = (slot >> QUINTANT_SHIFT) as usize;
    let origin = &get_origins()[quintant / 5];
    if resolution == 0 {
        return Ok(A5Cell {
            origin_id: origin.id,
            segment: 0,
            s: 0,
            resolution,
        });
    }
    let segment = (quintant + origin.first_quintant) % 5;
    let s = if resolution < FIRST_HILBERT_RESOLUTION {
        0
    } else {
        (slot & S_MASK) / SLOT_COUNTS[resolution as usize]
    };
    Ok(A5Cell {
        origin_id: origin.id,
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
    } = *cell;

    if resolution > MAX_RESOLUTION {
        return Err(format!("Resolution ({}) is too large", resolution));
    }

    if resolution == -1 {
        return Ok(WORLD_CELL);
    }
    if resolution == 0 {
        return Ok(slot_to_cell(5 * origin_id as u64 * QUINTANT_SLOTS, 0));
    }

    // The cell's first slot: its quintant, then S cells of this resolution into it
    let offset = if resolution >= FIRST_HILBERT_RESOLUTION {
        let count = SLOT_COUNTS[resolution as usize];
        match s.checked_mul(count) {
            Some(offset) if offset < QUINTANT_SLOTS => offset,
            _ => {
                return Err(format!(
                    "S ({}) is too large for resolution level {}",
                    s, resolution
                ))
            }
        }
    } else {
        0
    };

    let origin = &get_origins()[origin_id as usize];
    let quintant = 5 * origin_id as usize + (segment + 5 - origin.first_quintant) % 5;
    // Quintants past RES30_QUINTANTS have no res-30 IDs: fall back to res 29
    if resolution == MAX_RESOLUTION && quintant >= RES30_QUINTANTS {
        return serialize(&A5Cell {
            origin_id,
            segment,
            s: s >> 2,
            resolution: MAX_RESOLUTION - 1,
        });
    }
    Ok(slot_to_cell(
        ((quintant as u64) << QUINTANT_SHIFT) + offset,
        resolution,
    ))
}

/// The children of a cell at `child_resolution` (default: the next resolution),
/// in ascending ID order.
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

    if current_resolution == -1 {
        new_origin_ids = (0..12).collect();
    }

    let all_segments = (current_resolution == -1 && new_resolution > 0) || current_resolution == 0;

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

    let origins = get_origins();
    for &new_origin_id in &new_origin_ids {
        // An origin's quintants in ID order: the n-th is segment (n + first_quintant) % 5
        let first_quintant = origins[new_origin_id as usize].first_quintant;
        let new_segments: Vec<usize> = if all_segments {
            (0..5).map(|n| (n + first_quintant) % 5).collect()
        } else {
            vec![segment]
        };
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

/// Whether a cell is at resolution 30: its tag is one of ...1, ...100 or ...10000.
fn is_max_resolution(index: u64) -> bool {
    index & index.wrapping_neg() & RES30_TAG_BITS != 0
}

/// Re-pack a res-30 cell into the standard res-29 bit layout (6-bit quintant
/// in [63..58], 56-bit S in [57..2], tag at bit 1). The 58-bit res-30 S is
/// truncated by 2 bits, exactly as `cell_to_parent(_, 29)` would.
fn normalize_res30(index: u64) -> u64 {
    // The res-29 parent starts at the same slot, rounded down to its 4 children
    (res30_to_slot(index) & !3) | 0b10
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
        // Hilbert-range parent: clear bits below the parent tag, set the tag.
        // Identity (parent res === child res) falls out for free: the tag lands
        // in the same position and bits below the keep cut are already zero.
        let keep_shift = (60 - 2 * parent_resolution) as u32;
        return Ok(
            ((c >> keep_shift) << keep_shift) | (1u64 << (59 - 2 * parent_resolution) as u32)
        );
    }

    if parent_resolution == 1 {
        // Top 6 bits already encode 5*originId + segmentN; only the tag moves.
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

/// The 12 resolution-0 cells (dodecahedron faces) — a constant, computed once.
static RES0_CELLS: std::sync::LazyLock<Vec<u64>> =
    std::sync::LazyLock::new(|| cell_to_children(WORLD_CELL, Some(0)).expect("res 0 cells"));

/// Returns resolution 0 cells of the A5 system, which serve as a starting point
/// for all higher-resolution subdivisions in the hierarchy.
///
/// Returns Array of 12 cell indices
pub fn get_res0_cells() -> Result<Vec<u64>, String> {
    Ok(RES0_CELLS.clone())
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
    // + 2(P-1) Hilbert bits. Bit (59-2P) is the tag, below that is zero.
    // Shifting both right by (60-2P) keeps exactly those identifying bits and
    // discards the tag, so a descendant matches iff the high bits match.
    let shift = (60 - 2 * parent_resolution) as u32;
    (child >> shift) == (parent >> shift)
}
