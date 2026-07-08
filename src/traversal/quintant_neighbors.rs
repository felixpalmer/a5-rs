// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use crate::lattice::{s_to_cell, triple_in_bounds, triple_to_s, Orientation, Triple};
use crate::traversal::neighbors::NEIGHBOR_DELTAS;

/// Find within-quintant neighbors via the cell's pentagon flavor.
///
/// A cell's neighbors sit at fixed triple deltas determined by its flavor
/// (NEIGHBOR_DELTAS — 5 edge-sharing + 2 vertex-only), so no per-candidate
/// validation is needed: each in-bounds delta is a neighbor.
///
/// * `source_triple` - Triple coordinates of the source cell
/// * `source_flavor` - Pentagon flavor of the source cell (0-3)
/// * `source_s` - Source s-value to exclude from results
/// * `resolution` - Resolution level
/// * `orientation` - Curve orientation
/// * `edge_only` - If true, only the 5 edge-sharing neighbors
pub fn find_quintant_neighbor_s(
    source_triple: &Triple,
    source_flavor: u8,
    source_s: u64,
    resolution: usize,
    orientation: Orientation,
    edge_only: bool,
) -> Vec<u64> {
    let max_s = 4u64.pow(resolution as u32);
    let max_row = (1i32 << resolution) - 1;
    let deltas = &NEIGHBOR_DELTAS[source_flavor as usize];
    let mut neighbors: Vec<u64> = Vec::new();

    let list: &[Triple] = if edge_only { &deltas.edge } else { &deltas.all };
    for delta in list {
        let neighbor_triple = Triple::new(
            source_triple.x + delta.x,
            source_triple.y + delta.y,
            source_triple.z + delta.z,
        );
        if !triple_in_bounds(&neighbor_triple, max_row) {
            continue;
        }
        if let Some(neighbor_s) = triple_to_s(&neighbor_triple, resolution, orientation) {
            if neighbor_s < max_s && neighbor_s != source_s {
                neighbors.push(neighbor_s);
            }
        }
    }

    neighbors
}

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

    let mut result = find_quintant_neighbor_s(
        &cell.triple,
        cell.flavor,
        s,
        resolution,
        orientation,
        edge_only,
    );
    result.sort();
    result
}
