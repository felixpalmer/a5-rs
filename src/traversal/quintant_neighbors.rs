// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use crate::lattice::{s_to_cell, triple_in_bounds, triple_to_s, Orientation, Triple};
use crate::traversal::neighbors::NEIGHBOR_DELTAS;

/// Neighbor finding via triple coordinates and pentagon flavor.
///
/// Triple coordinates are orientation-independent — the same geometric cell
/// always has the same triple coords regardless of curve orientation. Only the
/// s-value changes between orientations, so neighbors are found in triple space
/// and converted back to the requested orientation.
pub fn get_cell_neighbors(
    s: u64,
    resolution: usize,
    orientation: Orientation,
    edge_only: bool,
) -> Vec<u64> {
    let cell = s_to_cell(s, resolution, orientation);
    let max_row = (1i32 << resolution) - 1;
    let deltas = &NEIGHBOR_DELTAS[cell.flavor as usize];
    let list: &[Triple] = if edge_only { &deltas.edge } else { &deltas.all };
    let mut neighbors: Vec<u64> = list
        .iter()
        .map(|d| {
            Triple::new(
                cell.triple.x + d.x,
                cell.triple.y + d.y,
                cell.triple.z + d.z,
            )
        })
        .filter(|neighbor| triple_in_bounds(neighbor, max_row))
        .filter_map(|neighbor| triple_to_s(&neighbor, resolution, orientation))
        .collect();
    neighbors.sort();
    neighbors
}
