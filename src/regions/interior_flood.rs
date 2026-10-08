// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

// Polygon fill by flooding the interior: cheaper than curve runs when the
// interior is small, as the flood costs about boundary + interior cells while
// the runs sort a band of boundary plus ring slots.

use crate::coordinate_systems::Cartesian;
use crate::core::cell_info::get_num_cells;
use crate::core::coordinate_transforms::to_cartesian;
use crate::core::serialization::FIRST_HILBERT_RESOLUTION;
use crate::traversal::lattice_flood_fill::{triple_space_flood_fill, FloodInput};
use crate::traversal::triple_cells::{
    for_each_lattice_neighbor, triple_cell_center, triple_cell_to_id,
};

use super::polygon_boundary::{boundary_neighbors, polygon_area, Boundary};

/// Below this many estimated interior cells per boundary cell, flooding the
/// interior beats splitting the curve into runs (measured crossover: ~3.3).
const FLOOD_INTERIOR_PER_BOUNDARY: f64 = 3.0;

/// Whether to fill by flooding: the interior is small, and the polygon can't
/// swallow a quintant whole (`cap_holds_quintant` false), which the flood, never
/// crossing a quintant edge from the boundary, would miss.
pub(super) fn prefers_flood(
    ring_vecs_list: &[Vec<Cartesian>],
    boundary_count: usize,
    resolution: i32,
    cap_holds_quintant: bool,
) -> bool {
    !cap_holds_quintant
        && polygon_area(ring_vecs_list) / (4.0 * std::f64::consts::PI)
            * (get_num_cells(resolution) as f64)
            < FLOOD_INTERIOR_PER_BOUNDARY * boundary_count as f64
}

/// Fill a polygon by flooding its interior, given its classified boundary and
/// the boundary cells in triple space. Returns the cells inside, uncompacted and
/// unsorted.
pub(super) fn fill_by_flood(
    boundary: &Boundary,
    triples: &[[i32; 5]],
    resolution: i32,
    overlapping: bool,
) -> Result<Vec<u64>, String> {
    let hilbert_res = (resolution - FIRST_HILBERT_RESOLUTION + 1) as usize;
    let max_row = (1i32 << hilbert_res) - 1;
    let mut out = boundary.output(overlapping);

    // The shell: the flood's own moves out of the boundary (each an edge
    // neighbor), split into seeds inside and firewall outside
    let shell = boundary_neighbors(
        triples,
        &[&|cell, visit| for_each_lattice_neighbor(cell, max_row, visit)],
    )?;
    let mut seeds: Vec<[i32; 5]> = Vec::new();
    let mut firewall: Vec<[i32; 5]> = triples.to_vec();
    for &(cell, parent) in &shell {
        let center = to_cartesian(triple_cell_center(cell, hilbert_res, max_row)?);
        if boundary.inside_next_to(center, parent) {
            seeds.push(cell);
        } else {
            firewall.push(cell);
        }
    }
    if !seeds.is_empty() {
        for &seed in &seeds {
            out.push(triple_cell_to_id(seed, hilbert_res, resolution)?);
        }
        let flood =
            triple_space_flood_fill(FloodInput::Firewall(&firewall), &seeds, resolution, None)?;
        out.extend(flood.interior_cells);
    }
    Ok(out)
}
