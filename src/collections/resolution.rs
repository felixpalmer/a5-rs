// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use crate::core::compaction_marker::{compaction_marker_resolution, is_compaction_marker};
use crate::core::serialization::{get_resolution, MAX_RESOLUTION, RES30_TAG_BITS, RESOLUTION_TAGS};

/// The resolution of a set of cells: the resolution of its compaction marker, or of
/// its finest cell when it has none. A covering stands for all its
/// cells at this resolution. Returns -1 for an empty set (or the world cell).
///
/// # Arguments
///
/// * `cells` - Cells, as returned by `compact`, `polygon_to_cells` etc.
///
/// # Returns
///
/// Resolution (-1 to 30)
pub fn covering_resolution(cells: &[u64]) -> i32 {
    // A covering ends in its compaction marker, which records the resolution
    if let Some(&last) = cells.last() {
        if is_compaction_marker(last) {
            return compaction_marker_resolution(last);
        }
    }

    // Otherwise the finest cell: finer cells have smaller resolution tags (the
    // lowest set bit), except at res 30, whose tags are recognised separately
    let mut finest_tag: u64 = 0;
    for &cell in cells {
        let tag = if is_compaction_marker(cell) {
            let resolution = compaction_marker_resolution(cell);
            if resolution == MAX_RESOLUTION {
                return MAX_RESOLUTION;
            }
            RESOLUTION_TAGS[resolution as usize]
        } else {
            let tag = cell & cell.wrapping_neg();
            if tag & RES30_TAG_BITS != 0 {
                return MAX_RESOLUTION;
            }
            tag
        };
        if tag != 0 && (finest_tag == 0 || tag < finest_tag) {
            finest_tag = tag;
        }
    }
    if finest_tag == 0 {
        -1
    } else {
        get_resolution(finest_tag)
    }
}
