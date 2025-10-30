// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

//! Optimized implementation of compact/uncompact functions for A5 DGGS.
//!
//! This version uses cell_to_children for expansion and stride-based sibling detection
//! for compaction.

use std::collections::HashSet;

use crate::core::cell_info::get_num_children;
use crate::core::serialization::{
    cell_to_children, cell_to_parent, get_resolution, get_stride, is_first_child,
    FIRST_HILBERT_RESOLUTION,
};

/// Expands a set of A5 cells to a target resolution by generating all descendant cells.
///
/// # Arguments
///
/// * `cells` - Slice of A5 cell identifiers to uncompact
/// * `target_resolution` - The target resolution level for all output cells
///
/// # Returns
///
/// Vector of cell identifiers, all at the target resolution
///
/// # Errors
///
/// Returns an error if any cell is at a resolution higher than the target resolution
pub fn uncompact(cells: &[u64], target_resolution: i32) -> Result<Vec<u64>, String> {
    // First calculate how much space is needed
    let mut n = 0;
    let mut resolutions = Vec::with_capacity(cells.len());

    for &cell in cells {
        let resolution = get_resolution(cell);
        let resolution_diff = target_resolution - resolution;
        if resolution_diff < 0 {
            return Err(format!(
                "Cannot uncompact cell at resolution {} to lower resolution {}",
                resolution, target_resolution
            ));
        }

        resolutions.push(resolution);
        n += get_num_children(resolution, target_resolution);
    }

    // Write directly into pre-allocated vec
    let mut result = Vec::with_capacity(n);

    for (i, &cell) in cells.iter().enumerate() {
        let resolution = resolutions[i];
        let num_children = get_num_children(resolution, target_resolution);

        if num_children == 1 {
            result.push(cell);
        } else {
            let children = cell_to_children(cell, Some(target_resolution))?;
            result.extend(children);
        }
    }

    Ok(result)
}

/// Compacts a set of A5 cells by replacing complete groups of sibling cells with their parent cells.
///
/// # Arguments
///
/// * `cells` - Slice of A5 cell identifiers to compact
///
/// # Returns
///
/// Vector of compacted cell identifiers (typically smaller than input)
pub fn compact(cells: &[u64]) -> Result<Vec<u64>, String> {
    if cells.is_empty() {
        return Ok(Vec::new());
    }

    // Single sort and dedup
    let unique_cells: HashSet<u64> = cells.iter().copied().collect();
    let mut current_cells: Vec<u64> = unique_cells.into_iter().collect();
    current_cells.sort_unstable();

    // Compact until no more changes
    // No re-sorting needed - parents maintain sorted order!
    let mut changed = true;
    while changed {
        changed = false;
        let mut result = Vec::new();
        let mut i = 0;

        while i < current_cells.len() {
            let cell = current_cells[i];
            let resolution = get_resolution(cell);

            // Can't compact below resolution 0
            if resolution < 0 {
                result.push(cell);
                i += 1;
                continue;
            }

            // Check for complete sibling group using unified stride-based approach
            let expected_children = if resolution >= FIRST_HILBERT_RESOLUTION {
                4 // Hilbert levels have 4 siblings
            } else if resolution == 0 {
                12 // First level has 12 siblings
            } else {
                5 // Second level has 5 siblings
            };

            if i + expected_children <= current_cells.len() {
                let mut has_all_siblings = true;

                // Use stride-based checking for all resolutions
                // First check if this cell is a first child (at a sibling group boundary)
                if is_first_child(cell, Some(resolution)) {
                    let stride = get_stride(resolution);

                    // Check that all expected siblings are present with correct stride
                    for j in 1..expected_children {
                        let expected_cell = cell + (j as u64) * stride;
                        if current_cells[i + j] != expected_cell {
                            has_all_siblings = false;
                            break;
                        }
                    }
                } else {
                    // First cell is not at a sibling group boundary
                    has_all_siblings = false;
                }

                if has_all_siblings {
                    // Compute parent only once when needed
                    let parent = cell_to_parent(cell, None)?;
                    result.push(parent);
                    i += expected_children;
                    changed = true;
                    continue;
                }
            }

            result.push(cell);
            i += 1;
        }

        current_cells = result;
    }

    Ok(current_cells)
}
