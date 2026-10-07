// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

// The collection engine. Each cell covers a block of leaf slots (see
// core/serialization), so a set of cells is a list of sorted, disjoint slot runs:
// cells are turned into slot runs, set operations merge runs, and runs are
// turned back into the coarsest cells covering them. Nothing is uncompacted.

use super::SlotRuns;
use crate::core::compaction_marker::{compaction_marker, is_compaction_marker};
use crate::core::serialization::{
    cell_first_slot, cell_slot_count, slot_to_cell, ORIGIN_SLOTS, QUINTANT_SHIFT, QUINTANT_SLOTS,
    RES30_TAG_BITS, RESOLUTION_TAGS, SLOT_COUNTS, S_MASK, WORLD_CELL, WORLD_SLOTS,
};

/// Merge cells, in the given order, into sorted and disjoint slot runs in `out`.
/// With `check_order`, stop and return false at the first cell starting before
/// the one before it.
fn merge_cells(cells: &[u64], out: &mut SlotRuns, check_order: bool) -> Result<bool, String> {
    let mut previous_lo = 0;
    for &cell in cells {
        if is_compaction_marker(cell) {
            continue;
        }
        let lo = cell_first_slot(cell)?;
        let hi = lo + cell_slot_count(cell);
        if check_order && lo < previous_lo {
            return Ok(false);
        }
        previous_lo = lo;
        // Drop earlier runs this one contains, then merge with the one before it
        while out.len() >= 2 && out[out.len() - 2] >= lo && out[out.len() - 1] <= hi {
            out.truncate(out.len() - 2);
        }
        append_slot_run(out, lo, hi);
    }
    Ok(true)
}

/// Whether a cell's ID lies inside its own slot run, so IDs sort like runs: res 1-29.
fn id_inside_run(cell: u64) -> bool {
    let tag = cell & cell.wrapping_neg();
    cell != WORLD_CELL && tag != RESOLUTION_TAGS[0] && tag & RES30_TAG_BITS == 0
}

/// Cells sorted by a slot inside each cell's run: then disjoint runs come out
/// in order, and a run can only be preceded by runs it contains or that
/// contain it. Below res 30 and above res 0 the ID itself is such a slot, so a
/// native sort of the IDs does it.
fn sort_cells(cells: &[u64]) -> Result<Vec<u64>, String> {
    if cells.iter().all(|&cell| id_inside_run(cell)) {
        let mut sorted = cells.to_vec();
        sorted.sort_unstable();
        return Ok(sorted);
    }
    let mut starts = Vec::with_capacity(cells.len());
    for &cell in cells {
        let start = if is_compaction_marker(cell) {
            WORLD_SLOTS
        } else {
            cell_first_slot(cell)?
        };
        starts.push((start, cell));
    }
    starts.sort_by_key(|&(start, _)| start);
    Ok(starts.into_iter().map(|(_, cell)| cell).collect())
}

/// The slot runs covered by a set of cells, sorted and merged. Compaction
/// markers are skipped.
///
/// Collections come sorted in curve order, so the cells are first merged as
/// given, checking the order as they go; only input found out of order is
/// sorted, and merged again. Errors if a value is neither an A5 cell ID nor a
/// compaction marker.
pub fn to_slot_runs(cells: &[u64]) -> Result<SlotRuns, String> {
    let mut runs = SlotRuns::new();
    if !merge_cells(cells, &mut runs, true)? {
        runs.clear();
        merge_cells(&sort_cells(cells)?, &mut runs, false)?;
    }
    Ok(runs)
}

/// Append a slot run [lo, hi) to sorted runs starting at or before lo, merging if they touch or overlap.
pub fn append_slot_run(runs: &mut SlotRuns, lo: u64, hi: u64) {
    match runs.last_mut() {
        Some(last) if *last >= lo => {
            if hi > *last {
                *last = hi;
            }
        }
        _ => runs.extend([lo, hi]),
    }
}

/// The coarsest cells covering the slot runs, in curve order. Runs built from
/// cells at resolution r or coarser are aligned to res-r cells, so no cell finer
/// than r is needed.
pub fn slot_runs_to_cells(runs: &[u64]) -> Vec<u64> {
    let mut out = Vec::new();
    for run in runs.chunks_exact(2) {
        let (mut lo, hi) = (run[0], run[1]);
        while lo < hi {
            if lo == 0 && hi == WORLD_SLOTS {
                out.push(WORLD_CELL);
                break;
            }
            if lo & S_MASK == 0 {
                // Whole origins, then whole quintants
                if (lo >> QUINTANT_SHIFT) % 5 == 0 && lo + ORIGIN_SLOTS <= hi {
                    out.push(slot_to_cell(lo, 0));
                    lo += ORIGIN_SLOTS;
                    continue;
                }
                if lo + QUINTANT_SLOTS <= hi {
                    out.push(slot_to_cell(lo, 1));
                    lo += QUINTANT_SLOTS;
                    continue;
                }
            }
            // The coarsest Hilbert-level cell that starts at lo and fits: its span is
            // 4^k slots, at most the alignment of lo and the length of the run
            let offset = lo & S_MASK;
            let alignment = if offset == 0 {
                56
            } else {
                offset.trailing_zeros()
            };
            let fit = (hi - lo).ilog2();
            let bits = alignment.min(fit) as i32;
            let r = 2.max((60 - bits + 1) / 2) as usize;
            out.push(slot_to_cell(lo, r as i32));
            lo += SLOT_COUNTS[r];
        }
    }
    out
}

/// The coarsest cells covering slot runs, in curve order, then the compaction marker for `resolution`.
pub fn slot_runs_to_collection(runs: &[u64], resolution: i32) -> Vec<u64> {
    let mut cells = slot_runs_to_cells(runs);
    if resolution >= 0 {
        cells.push(compaction_marker(resolution));
    }
    cells
}

/// Compact cells without appending a compaction marker: the coarsest cells covering
/// them, sorted in curve order. For internal use on intermediate results.
pub fn compact_cells(cells: &[u64]) -> Result<Vec<u64>, String> {
    Ok(slot_runs_to_cells(&to_slot_runs(cells)?))
}

/// Compact cells, at resolution `resolution` or coarser, into a collection: the
/// coarsest cells covering them, sorted in curve order, then the compaction
/// marker for `resolution`. The resolution is given, so an empty fill still
/// records it.
pub fn to_collection(cells: &[u64], resolution: i32) -> Result<Vec<u64>, String> {
    Ok(slot_runs_to_collection(&to_slot_runs(cells)?, resolution))
}
