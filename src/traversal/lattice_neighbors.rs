// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use crate::core::serialization::{get_resolution, FIRST_HILBERT_RESOLUTION};
use crate::traversal::global_neighbors::get_global_cell_neighbors;
use crate::traversal::triple_cells::{
    cell_ids_to_triples, for_each_lattice_neighbor, triple_cell_to_id,
};

/// Fast lattice-based neighbor finding over triple-space deltas: the 3
/// parity-valid moves — strict triple-lattice edge connectivity, the
/// connectivity `triple_space_flood_fill` uses — plus the edge-sharing
/// neighbors across a quintant edge (see `for_each_lattice_neighbor`). Falls
/// back to `get_global_cell_neighbors` below res 2.
pub fn get_lattice_neighbors(cell_id: u64) -> Vec<u64> {
    let resolution = get_resolution(cell_id);
    if resolution < FIRST_HILBERT_RESOLUTION {
        return get_global_cell_neighbors(cell_id, true);
    }

    let hilbert_res = (resolution - FIRST_HILBERT_RESOLUTION + 1) as usize;
    let mut cells: Vec<[i32; 5]> = Vec::with_capacity(1);
    if cell_ids_to_triples([cell_id], &mut cells).is_err() {
        return Vec::new();
    }
    let mut result: Vec<u64> = Vec::new();
    let _ = for_each_lattice_neighbor(cells[0], (1i32 << hilbert_res) - 1, |n| {
        result.push(triple_cell_to_id(n, hilbert_res, resolution)?);
        Ok(())
    });
    result
}
