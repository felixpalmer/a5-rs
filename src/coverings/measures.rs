// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use super::resolution::covering_resolution;
use crate::core::cell_info::{cell_area, get_num_children};
use crate::core::compaction_marker::is_compaction_marker;
use crate::core::serialization::checked_resolution;

/// The number of cells in a set at its resolution: the length of the
/// uncompacted set. This differs from the length of a compacted array. Each
/// cell given is counted, so overlapping cells are counted more than once; use
/// `union` to merge them first.
///
/// # Arguments
///
/// * `cells` - Set of cells (compacted or not)
///
/// # Returns
///
/// Number of cells at the set's resolution
///
/// # Errors
///
/// If a value is neither an A5 cell ID nor a compaction marker, or the count
/// exceeds `u64` (overlapping cells counted many times)
pub fn count(cells: &[u64]) -> Result<u64, String> {
    let resolution = covering_resolution(cells);
    let mut total: u64 = 0;
    for &cell in cells {
        if is_compaction_marker(cell) {
            continue;
        }
        let children = get_num_children(checked_resolution(cell)?, resolution) as u64;
        total = total.checked_add(children).ok_or("Count exceeds u64")?;
    }
    Ok(total)
}

/// The area of a set of cells, in square meters. Exact, as A5 cells are
/// equal-area. Overlapping cells each add their area; use `union` to merge them
/// first.
///
/// # Arguments
///
/// * `cells` - Set of cells (compacted or not)
///
/// # Returns
///
/// Area in square meters
///
/// # Errors
///
/// If a value is neither an A5 cell ID nor a compaction marker
pub fn area(cells: &[u64]) -> Result<f64, String> {
    let mut total = 0.0;
    for &cell in cells {
        if is_compaction_marker(cell) {
            continue;
        }
        total += cell_area(checked_resolution(cell)?);
    }
    Ok(total)
}
