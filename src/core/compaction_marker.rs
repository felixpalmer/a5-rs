// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

// The compaction marker: the value a covering carries as its last
// element, recording the resolution its cells stand for. It is a value no cell
// can take: quintant 60 (only 0-59 exist), the resolution in bits 55-48, and the
// marker tag 1000000 in bits 6-0. Cell IDs end in a 1 followed by an odd number
// of zeros, or in one of the res-30 patterns ...1, ...100, ...10000, never in a 1
// followed by 6 zeros. The other bits carry no meaning yet: they are written as
// 0 and ignored when read.
//
//   63-58    57-56   55-48        47-7          6-0
//   111100   00      resolution   reserved (0)  1000000

use crate::core::serialization::{MAX_RESOLUTION, QUINTANT_SHIFT};

const COMPACTION_MARKER_PREFIX: u64 = 60 << QUINTANT_SHIFT;
const COMPACTION_MARKER_END: u64 = 61 << QUINTANT_SHIFT;
const COMPACTION_MARKER_RESOLUTION_SHIFT: u32 = 48;
const COMPACTION_MARKER_TAG: u64 = 0b1000000;
const LOW_7_BITS: u64 = 0b1111111;

/// The compaction marker recording `resolution`.
pub fn compaction_marker(resolution: i32) -> u64 {
    COMPACTION_MARKER_PREFIX
        | ((resolution as u64) << COMPACTION_MARKER_RESOLUTION_SHIFT)
        | COMPACTION_MARKER_TAG
}

/// The resolution a compaction marker records.
pub fn compaction_marker_resolution(value: u64) -> i32 {
    ((value >> COMPACTION_MARKER_RESOLUTION_SHIFT) & 0xff) as i32
}

/// Check whether a value is a compaction marker: the value a covering
/// carries, as its last element, to record its resolution. It is not a cell.
pub fn is_compaction_marker(value: u64) -> bool {
    (COMPACTION_MARKER_PREFIX..COMPACTION_MARKER_END).contains(&value)
        && value & LOW_7_BITS == COMPACTION_MARKER_TAG
        && compaction_marker_resolution(value) <= MAX_RESOLUTION
}
