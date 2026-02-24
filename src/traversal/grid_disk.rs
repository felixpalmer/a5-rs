// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use std::collections::HashSet;

use crate::core::compact::compact;
use crate::traversal::global_neighbors::get_global_cell_neighbors;

/// BFS grid disk with progressive compaction.
///
/// Uses a sliding-window dedup approach: only the previous and current frontier
/// rings are kept in memory for deduplication.
fn grid_disk_bfs(cell_id: u64, k: usize, edge_only: bool) -> Result<Vec<u64>, String> {
    if k == 0 {
        return Ok(vec![cell_id]);
    }

    let mut interior: Vec<u64> = Vec::new();
    let mut prev_frontier: HashSet<u64> = HashSet::new();
    let mut frontier: HashSet<u64> = HashSet::new();
    frontier.insert(cell_id);

    for _ring in 1..=k {
        let mut next_frontier: HashSet<u64> = HashSet::new();
        for &cid in &frontier {
            for neighbor in get_global_cell_neighbors(cid, edge_only) {
                if !prev_frontier.contains(&neighbor)
                    && !frontier.contains(&neighbor)
                    && !next_frontier.contains(&neighbor)
                {
                    next_frontier.insert(neighbor);
                }
            }
        }

        // Evict prevFrontier -- these cells are >=2 rings behind the new frontier
        for &cid in &prev_frontier {
            interior.push(cid);
        }

        // Progressively compact interior to reduce memory pressure
        if interior.len() > 100 {
            interior = compact(&interior)?;
        }

        prev_frontier = frontier;
        frontier = next_frontier;
    }

    // Merge remaining boundary rings with compacted interior
    for &cid in &prev_frontier {
        interior.push(cid);
    }
    for &cid in &frontier {
        interior.push(cid);
    }

    compact(&interior)
}

/// Compute the grid disk of edge-sharing neighbors within k hops.
/// Returns a sorted, compacted list of cell IDs including the center cell.
pub fn grid_disk(cell_id: u64, k: usize) -> Result<Vec<u64>, String> {
    grid_disk_bfs(cell_id, k, true)
}

/// Compute the grid disk of all neighbors (edge + vertex sharing) within k hops.
/// Returns a sorted, compacted list of cell IDs including the center cell.
pub fn grid_disk_vertex(cell_id: u64, k: usize) -> Result<Vec<u64>, String> {
    grid_disk_bfs(cell_id, k, false)
}
