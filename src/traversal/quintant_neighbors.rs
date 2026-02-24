// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use crate::lattice::{
    anchor_to_triple, s_to_anchor, triple_in_bounds, triple_to_anchor, triple_to_s, Anchor,
    Orientation, Triple,
};
use crate::traversal::neighbors::is_neighbor;

/// Find within-quintant neighbors via triple coordinate search.
///
/// Generates +/-1 candidate triples, validates with is_neighbor() in uv space,
/// and converts validated triples to s-values in the requested orientation.
pub fn find_quintant_neighbor_s(
    source_triple: &Triple,
    uv_source_anchor: Option<&Anchor>,
    source_s: u64,
    resolution: usize,
    orientation: Orientation,
    edge_only: bool,
) -> Vec<u64> {
    let max_s = 4u64.pow(resolution as u32);
    let max_row = (1i32 << resolution) - 1;
    let mut neighbors: Vec<u64> = Vec::new();

    for dx in -1_i32..=1 {
        for dy in -1_i32..=1 {
            for dz in -1_i32..=1 {
                if dx == 0 && dy == 0 && dz == 0 {
                    continue;
                }
                let manhattan = dx.abs() + dy.abs() + dz.abs();
                if manhattan > 3 {
                    continue;
                }
                if edge_only && manhattan > 2 {
                    continue;
                }

                let neighbor_triple = Triple::new(
                    source_triple.x + dx,
                    source_triple.y + dy,
                    source_triple.z + dz,
                );
                if !triple_in_bounds(&neighbor_triple, max_row) {
                    continue;
                }

                // Validate in uv space where is_neighbor is known to work
                let uv_neighbor_anchor =
                    triple_to_anchor(&neighbor_triple, resolution, Orientation::UV);
                let uv_source = uv_source_anchor;
                if let (Some(uv_neighbor), Some(uv_src)) = (uv_neighbor_anchor.as_ref(), uv_source)
                {
                    if !is_neighbor(uv_src, uv_neighbor) {
                        continue;
                    }
                } else {
                    continue;
                }

                if let Some(neighbor_s) = triple_to_s(&neighbor_triple, resolution, orientation) {
                    if neighbor_s < max_s && neighbor_s != source_s {
                        neighbors.push(neighbor_s);
                    }
                }
            }
        }
    }

    neighbors
}

/// Fast neighbor finding using triple coordinates.
pub fn get_cell_neighbors(
    s: u64,
    resolution: usize,
    orientation: Orientation,
    edge_only: bool,
) -> Vec<u64> {
    let anchor = s_to_anchor(s, resolution, orientation);
    let triple = anchor_to_triple(&anchor);
    let uv_source_anchor = triple_to_anchor(&triple, resolution, Orientation::UV);

    let mut result = find_quintant_neighbor_s(
        &triple,
        uv_source_anchor.as_ref(),
        s,
        resolution,
        orientation,
        edge_only,
    );
    result.sort();
    result
}
