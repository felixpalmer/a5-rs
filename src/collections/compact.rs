// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

//! compact/uncompact for A5 DGGS. A compacted set of cells is a collection: its
//! cells sorted in curve order, then a compaction marker recording the resolution
//! they stand for (see `core::compaction_marker`).

use super::resolution::get_compaction_resolution;
use super::slot_runs::to_collection;
use crate::core::cell_info::get_num_children;
use crate::core::compaction_marker::is_compaction_marker;
use crate::core::serialization::{cell_to_children, checked_resolution, get_resolution};

/// Expand a set of cells to all their cells at its resolution: the resolution
/// of its compaction marker, or of its finest cell when it has none.
///
/// **Ordering property**: If the input is sorted in curve order (as `compact`
/// returns it), the output is too. All children of a cell form a contiguous,
/// ordered block on the curve, so `children(A) < children(B)` whenever `A < B`.
///
/// # Arguments
///
/// * `cells` - Set of cells, compacted or not
///
/// # Returns
///
/// Vector of cell identifiers, all at the set's resolution
///
/// # Errors
///
/// If a value is neither an A5 cell ID nor a compaction marker
pub fn uncompact(cells: &[u64]) -> Result<Vec<u64>, String> {
    let target_resolution = get_compaction_resolution(cells);

    // First calculate how much space is needed
    let mut n = 0;
    for &cell in cells {
        if is_compaction_marker(cell) {
            continue;
        }
        n = get_num_children(checked_resolution(cell)?, target_resolution)
            .checked_add(n)
            .ok_or("Too many cells to uncompact")?;
    }

    // Write directly into pre-allocated vec
    let mut result = Vec::new();
    result
        .try_reserve_exact(n)
        .map_err(|_| format!("Too many cells to uncompact: {}", n))?;
    for &cell in cells {
        if is_compaction_marker(cell) {
            continue;
        }
        if get_num_children(get_resolution(cell), target_resolution) == 1 {
            result.push(cell);
        } else {
            result.extend(cell_to_children(cell, Some(target_resolution))?);
        }
    }

    Ok(result)
}

/// Compact a set of cells: replace every complete group of siblings by their
/// parent, recursively, and append a compaction marker recording the resolution of
/// the input's finest cell, which `uncompact` expands back to.
///
/// # Arguments
///
/// * `cells` - Slice of A5 cell identifiers to compact
///
/// # Returns
///
/// Compacted cells sorted in curve order, then the compaction marker
///
/// # Errors
///
/// If a value is neither an A5 cell ID nor a compaction marker
pub fn compact(cells: &[u64]) -> Result<Vec<u64>, String> {
    to_collection(cells, get_compaction_resolution(cells))
}
