// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use super::resolution::get_compaction_resolution;
use super::slot_runs::{append_slot_run, slot_runs_to_collection, to_slot_runs};
use super::SlotRuns;
use crate::core::compaction_marker::is_compaction_marker;
use crate::core::serialization::{
    cell_first_slot, cell_first_slot_unchecked, cell_slot_count, checked_resolution,
};

/// Merge two lists of slot runs, keeping slots in either.
fn union_slot_runs(a: &[u64], b: &[u64]) -> SlotRuns {
    let mut out = SlotRuns::new();
    let (mut i, mut j) = (0, 0);
    while i < a.len() || j < b.len() {
        if j >= b.len() || (i < a.len() && a[i] <= b[j]) {
            append_slot_run(&mut out, a[i], a[i + 1]);
            i += 2;
        } else {
            append_slot_run(&mut out, b[j], b[j + 1]);
            j += 2;
        }
    }
    out
}

/// Slots in both lists of slot runs.
fn intersect_slot_runs(a: &[u64], b: &[u64]) -> SlotRuns {
    let mut out = SlotRuns::new();
    let (mut i, mut j) = (0, 0);
    while i < a.len() && j < b.len() {
        let lo = a[i].max(b[j]);
        let hi = a[i + 1].min(b[j + 1]);
        if lo < hi {
            out.extend([lo, hi]);
        }
        if a[i + 1] < b[j + 1] {
            i += 2;
        } else {
            j += 2;
        }
    }
    out
}

/// Slots in the first list of slot runs but not the second.
fn difference_slot_runs(a: &[u64], b: &[u64]) -> SlotRuns {
    let mut out = SlotRuns::new();
    let mut j = 0;
    for run in a.chunks_exact(2) {
        let (mut lo, hi) = (run[0], run[1]);
        while j < b.len() && b[j + 1] <= lo {
            j += 2;
        }
        let mut k = j;
        while k < b.len() && b[k] < hi {
            if b[k] > lo {
                out.extend([lo, b[k]]);
            }
            if b[k + 1] > lo {
                lo = b[k + 1];
            }
            k += 2;
        }
        if lo < hi {
            out.extend([lo, hi]);
        }
    }
    out
}

/// The resolution of two sets of cells, which must be the same: A5 resolutions
/// don't nest geometrically, so combining sets at different ones has no meaning.
fn same_resolution(a: &[u64], b: &[u64]) -> Result<i32, String> {
    let resolution_a = get_compaction_resolution(a);
    let resolution_b = get_compaction_resolution(b);
    if resolution_a != resolution_b {
        return Err(format!(
            "Cannot combine cells at resolution {} with cells at resolution {}",
            resolution_a, resolution_b
        ));
    }
    Ok(resolution_a)
}

/// Combine two sets of cells as slot runs, compacted at their resolution.
fn combine(
    a: &[u64],
    b: &[u64],
    operation: fn(&[u64], &[u64]) -> SlotRuns,
) -> Result<Vec<u64>, String> {
    let resolution = same_resolution(a, b)?;
    Ok(slot_runs_to_collection(
        &operation(&to_slot_runs(a)?, &to_slot_runs(b)?),
        resolution,
    ))
}

/// The union of two sets of cells: cells in either. Both sets must be at the
/// same resolution, and the result is compacted at it.
///
/// # Returns
///
/// Compacted cells, with a compaction marker recording the resolution
///
/// # Errors
///
/// If the sets are at different resolutions, or a value is neither an A5 cell ID
/// nor a compaction marker
pub fn union(a: &[u64], b: &[u64]) -> Result<Vec<u64>, String> {
    combine(a, b, union_slot_runs)
}

/// The intersection of two sets of cells: cells in both. Both sets must be at
/// the same resolution, and the result is compacted at it.
///
/// # Returns
///
/// Compacted cells, with a compaction marker recording the resolution
///
/// # Errors
///
/// If the sets are at different resolutions, or a value is neither an A5 cell ID
/// nor a compaction marker
pub fn intersect(a: &[u64], b: &[u64]) -> Result<Vec<u64>, String> {
    combine(a, b, intersect_slot_runs)
}

/// The difference of two sets of cells: cells in `a` but not in `b`. Both sets
/// must be at the same resolution, and the result is compacted at it.
///
/// # Returns
///
/// Compacted cells, with a compaction marker recording the resolution
///
/// # Errors
///
/// If the sets are at different resolutions, or a value is neither an A5 cell ID
/// nor a compaction marker
pub fn difference(a: &[u64], b: &[u64]) -> Result<Vec<u64>, String> {
    combine(a, b, difference_slot_runs)
}

/// Check whether two sets of cells share any cell. Both sets must be at the same
/// resolution.
///
/// # Errors
///
/// If the sets are at different resolutions, or a value is neither an A5 cell ID
/// nor a compaction marker
pub fn overlaps(a: &[u64], b: &[u64]) -> Result<bool, String> {
    same_resolution(a, b)?;
    Ok(!intersect_slot_runs(&to_slot_runs(a)?, &to_slot_runs(b)?).is_empty())
}

/// Check whether a cell is in a set of cells. The cell must be at the set's
/// resolution: A5 cells don't nest geometrically across resolutions, so for a
/// point-in-polygon test pass `lonlat_to_cell(point, resolution)` with the
/// resolution of the set. Uses a binary search, so `cells` must be sorted in
/// curve order, as returned by `compact` and the other A5 functions.
///
/// # Errors
///
/// If the cell is at a different resolution from the set, or the cell, or the
/// one the search lands on, is not an A5 cell ID. Only those two are checked
pub fn contains(cells: &[u64], cell: u64) -> Result<bool, String> {
    let resolution = get_compaction_resolution(cells);
    let cell_resolution = checked_resolution(cell)?;
    if cell_resolution != resolution {
        return Err(format!(
            "Cannot test a cell at resolution {} against cells at resolution {}",
            cell_resolution, resolution
        ));
    }
    let slot = cell_first_slot(cell)?;
    let cells = match cells.split_last() {
        Some((&last, rest)) if is_compaction_marker(last) => rest,
        _ => cells,
    };

    // The last cell starting at or before the cell's first slot is the only one
    // that can hold it. Only that cell is checked to be a cell: the search steps
    // just need an order
    let found = cells.partition_point(|&c| cell_first_slot_unchecked(c) <= slot);
    if found == 0 {
        return Ok(false);
    }
    let candidate = cells[found - 1];
    Ok(slot < cell_first_slot(candidate)? + cell_slot_count(candidate))
}
