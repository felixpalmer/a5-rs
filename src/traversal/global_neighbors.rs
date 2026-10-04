// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use std::collections::BTreeSet;

use crate::core::face_adjacency::FACE_ADJACENCY;
use crate::core::serialization::{deserialize, serialize, FIRST_HILBERT_RESOLUTION};
use crate::core::utils::A5Cell;
use crate::traversal::triple_cells::{
    cell_ids_to_triples, for_each_triple_neighbor, triple_cell_to_id,
};

/// Get all neighbors of a cell across quintant and face boundaries: within its
/// quintant the fixed per-flavor triple deltas, and across a quintant edge the
/// boundary delta tables (see `for_each_triple_neighbor`).
///
/// `edge_only`: if true, return only edge-sharing neighbors (5 per cell).
/// Default false returns all neighbors including vertex-only neighbors (6-8 per cell).
pub fn get_global_cell_neighbors(cell_id: u64, edge_only: bool) -> Vec<u64> {
    try_global_cell_neighbors(cell_id, edge_only).unwrap_or_default()
}

fn try_global_cell_neighbors(cell_id: u64, edge_only: bool) -> Result<Vec<u64>, String> {
    let cell = deserialize(cell_id)?;
    let mut neighbors = BTreeSet::new();
    if cell.resolution == 0 {
        // The cells are the 12 dodecahedron faces, adjacent across their edges
        for &(face, _) in &FACE_ADJACENCY[cell.origin_id as usize] {
            neighbors.insert(serialize(&A5Cell {
                origin_id: face,
                segment: 0,
                s: 0,
                resolution: 0,
            })?);
        }
    } else {
        let hilbert_res = (cell.resolution - FIRST_HILBERT_RESOLUTION + 1) as usize;
        let mut source: Vec<[i32; 5]> = Vec::with_capacity(1);
        cell_ids_to_triples([cell_id], &mut source)?;
        for_each_triple_neighbor(source[0], (1i32 << hilbert_res) - 1, edge_only, |n| {
            neighbors.insert(triple_cell_to_id(n, hilbert_res, cell.resolution)?);
            Ok(())
        })?;
    }
    Ok(neighbors.into_iter().collect())
}
