// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use std::collections::HashSet;

use crate::core::face_adjacency::walk_faces;
use crate::core::serialization::{deserialize, serialize, FIRST_HILBERT_RESOLUTION};
use crate::core::utils::A5Cell;
use crate::coverings::slot_runs::{compact_cells, to_covering};
use crate::traversal::triple_cells::{
    cell_ids_to_triples, for_each_triple_neighbor, triple_cell_key, triple_cell_to_id,
};

/// One BFS ring: its dedup keys, and its cells as flat (origin_id, quintant, x, y, z).
#[derive(Default)]
struct Ring {
    keys: HashSet<i64>,
    cells: Vec<i32>,
}

/// Add a cell to `next` unless it is already in one of the three live rings.
fn add_cell(next: &mut Ring, prev: &Ring, current: &Ring, cell: [i32; 5]) {
    let key = triple_cell_key(cell);
    if prev.keys.contains(&key) || current.keys.contains(&key) || !next.keys.insert(key) {
        return;
    }
    next.cells.extend_from_slice(&cell);
}

/// Encode a ring's cells as cell IDs, appending them to `out`.
fn push_cell_ids(
    out: &mut Vec<u64>,
    cells: &[i32],
    hilbert_res: usize,
    resolution: i32,
) -> Result<(), String> {
    for c in cells.chunks_exact(5) {
        out.push(triple_cell_to_id(
            [c[0], c[1], c[2], c[3], c[4]],
            hilbert_res,
            resolution,
        )?);
    }
    Ok(())
}

/// BFS grid disk in triple space, with progressive compaction.
///
/// Neighbors come from the per-flavor triple deltas, plus the boundary delta
/// tables for cells on a quintant edge, so no cell is decoded and each is
/// encoded exactly once, when it leaves the window.
///
/// Uses a sliding-window dedup approach: only the previous and current frontier
/// rings are kept in memory for deduplication (BFS guarantees cells >=2 rings
/// behind the frontier can never be re-discovered). Evicted interior cells are
/// periodically compacted to reduce memory pressure.
fn grid_disk_bfs(cell_id: u64, k: usize, edge_only: bool) -> Result<Vec<u64>, String> {
    let cell = deserialize(cell_id)?;
    if k == 0 {
        return to_covering(&[cell_id], cell.resolution);
    }
    if cell.resolution == 0 {
        // The cells are the 12 dodecahedron faces
        let faces = walk_faces(&[cell.origin_id], |_| Ok(true), k)?;
        let cells = faces
            .into_iter()
            .map(|face| {
                serialize(&A5Cell {
                    origin_id: face,
                    segment: 0,
                    s: 0,
                    resolution: 0,
                })
            })
            .collect::<Result<Vec<u64>, String>>()?;
        return to_covering(&cells, 0);
    }
    let hilbert_res = (cell.resolution - FIRST_HILBERT_RESOLUTION + 1) as usize;
    let max_row = (1i32 << hilbert_res) - 1;
    let mut seed: Vec<[i32; 5]> = Vec::with_capacity(1);
    cell_ids_to_triples([cell_id], &mut seed)?;

    // The seed is `cell_id` already, so it goes straight to the output
    let mut interior: Vec<u64> = vec![cell_id];
    let mut prev_frontier = Ring::default();
    let mut frontier = Ring::default();
    add_cell(&mut frontier, &Ring::default(), &Ring::default(), seed[0]);

    for ring in 1..=k {
        let mut next_frontier = Ring::default();
        for c in frontier.cells.chunks_exact(5) {
            for_each_triple_neighbor([c[0], c[1], c[2], c[3], c[4]], max_row, edge_only, |n| {
                add_cell(&mut next_frontier, &prev_frontier, &frontier, n);
                Ok(())
            })?;
        }

        // The seed ring is expanded; drop its cell so it isn't encoded again (its key stays)
        if ring == 1 {
            frontier.cells.clear();
        }

        // Evict prev_frontier -- these cells are >=2 rings behind the new frontier
        // and can never be re-discovered by BFS
        push_cell_ids(
            &mut interior,
            &prev_frontier.cells,
            hilbert_res,
            cell.resolution,
        )?;

        // Progressively compact interior to reduce memory pressure
        if interior.len() > 100 {
            interior = compact_cells(&interior)?;
        }

        prev_frontier = frontier;
        frontier = next_frontier;
    }

    // Merge remaining boundary rings with compacted interior
    push_cell_ids(
        &mut interior,
        &prev_frontier.cells,
        hilbert_res,
        cell.resolution,
    )?;
    push_cell_ids(&mut interior, &frontier.cells, hilbert_res, cell.resolution)?;

    to_covering(&interior, cell.resolution)
}

/// Compute the grid disk of edge-sharing neighbors within k hops.
/// Returns compacted cell IDs including the center cell, sorted in curve
/// order, then a compaction marker recording the resolution.
pub fn grid_disk(cell_id: u64, k: usize) -> Result<Vec<u64>, String> {
    grid_disk_bfs(cell_id, k, true)
}

/// Compute the grid disk of all neighbors (edge + vertex sharing) within k hops.
/// Returns compacted cell IDs including the center cell, sorted in curve
/// order, then a compaction marker recording the resolution.
pub fn grid_disk_vertex(cell_id: u64, k: usize) -> Result<Vec<u64>, String> {
    grid_disk_bfs(cell_id, k, false)
}
