// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use std::collections::HashSet;

use crate::core::serialization::FIRST_HILBERT_RESOLUTION;
use crate::lattice::{triple_in_bounds, Triple};
use crate::traversal::triple_cells::{triple_cell_key, triple_cell_to_id};

/// Flood state, reusable across calls at one resolution: the keys of every cell visited so far.
#[derive(Debug, Clone, Default)]
pub struct FloodState {
    visited: HashSet<i64>,
}

/// Input to `triple_space_flood_fill`: the cells the flood may not enter, or a
/// reused `{state, delta}` from a previous call (state reused, `delta` cells
/// joining its firewall).
pub enum FloodInput<'a> {
    Firewall(&'a [[i32; 5]]),
    Reuse {
        state: FloodState,
        delta: Vec<[i32; 5]>,
    },
}

/// Result of `triple_space_flood_fill`.
pub struct FloodResult {
    /// The cells discovered by this call (seeds excluded)
    pub interior_cells: Vec<u64>,
    /// The final frontier, as cell IDs and in triple space
    pub frontier_cell_ids: Vec<u64>,
    pub frontier: Vec<[i32; 5]>,
    /// State for a follow-up call
    pub state: FloodState,
}

/// Triple-space flood fill over the 3 parity-valid lattice moves. Those never
/// cross a quintant edge, so each quintant floods independently. All cells —
/// firewall, seeds, the returned frontier — are (origin_id, quintant, x, y, z),
/// so nothing is decoded; discovered cells are encoded once, on output.
///
/// Seeds are always added to the frontier, even if already visited — reusing
/// state with the same seeds restarts BFS. `max_layers` limits the BFS layers;
/// `None` = run to convergence.
pub fn triple_space_flood_fill(
    firewall: FloodInput,
    seeds: &[[i32; 5]],
    resolution: i32,
    max_layers: Option<usize>,
) -> Result<FloodResult, String> {
    let hilbert_res = (resolution - FIRST_HILBERT_RESOLUTION + 1) as usize;
    let max_row = (1i32 << hilbert_res) - 1;

    let (mut state, delta) = match firewall {
        FloodInput::Firewall(cells) => (FloodState::default(), cells.to_vec()),
        FloodInput::Reuse { state, delta } => (state, delta),
    };
    for &cell in delta.iter().chain(seeds) {
        state.visited.insert(triple_cell_key(cell));
    }

    let mut discovered: Vec<[i32; 5]> = Vec::new();
    let mut frontier: Vec<[i32; 5]> = seeds.to_vec();
    let mut layers = 0;
    while !frontier.is_empty() && max_layers.is_none_or(|max| layers < max) {
        let mut next: Vec<[i32; 5]> = Vec::new();
        for &[origin_id, quintant, x, y, z] in &frontier {
            // +1 on one axis from a parity 0 triple, -1 from parity 1
            let step = if x + y + z == 0 { 1 } else { -1 };
            for (dx, dy, dz) in [(step, 0, 0), (0, step, 0), (0, 0, step)] {
                let (nx, ny, nz) = (x + dx, y + dy, z + dz);
                if !triple_in_bounds(&Triple::new(nx, ny, nz), max_row) {
                    continue;
                }
                let cell = [origin_id, quintant, nx, ny, nz];
                if state.visited.insert(triple_cell_key(cell)) {
                    discovered.push(cell);
                    next.push(cell);
                }
            }
        }
        frontier = next;
        layers += 1;
    }

    let to_ids = |cells: &[[i32; 5]]| -> Result<Vec<u64>, String> {
        cells
            .iter()
            .map(|&c| triple_cell_to_id(c, hilbert_res, resolution))
            .collect()
    };
    Ok(FloodResult {
        interior_cells: to_ids(&discovered)?,
        frontier_cell_ids: to_ids(&frontier)?,
        frontier,
        state,
    })
}
