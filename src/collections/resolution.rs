// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use crate::core::compaction_marker::{compaction_marker_resolution, is_compaction_marker};
use crate::core::serialization::get_resolution;

/// The resolution of a set of cells: the resolution of its compaction marker, or of
/// its finest cell when it has none. A compacted collection stands for all its
/// cells at this resolution. Returns -1 for an empty set (or the world cell).
///
/// # Arguments
///
/// * `cells` - Cells, as returned by `compact`, `polygon_to_cells` etc.
///
/// # Returns
///
/// Resolution (-1 to 30)
pub fn get_compaction_resolution(cells: &[u64]) -> i32 {
    // A collection ends in its compaction marker, which records the resolution
    if let Some(&last) = cells.last() {
        if is_compaction_marker(last) {
            return compaction_marker_resolution(last);
        }
    }

    // Otherwise the finest cell
    let mut finest = -1;
    for &cell in cells {
        let resolution = if is_compaction_marker(cell) {
            compaction_marker_resolution(cell)
        } else {
            get_resolution(cell)
        };
        finest = finest.max(resolution);
    }
    finest
}
