// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use std::collections::{HashMap, HashSet};

use crate::core::origin::{get_origins, quintant_tables};
use crate::core::serialization::{deserialize, serialize, FIRST_HILBERT_RESOLUTION};
use crate::core::utils::{A5Cell, Origin};
use crate::lattice::{s_to_triple, triple_to_s, Orientation};

/// Per-quintant context needed to convert triples back to cell IDs.
#[derive(Debug, Clone)]
struct QuintantCtx {
    origin: &'static Origin,
    segment: usize,
    orientation: Orientation,
}

/// Per-quintant packed BFS state. Reusable across phases at the same resolution.
#[derive(Debug, Clone)]
pub struct QuintantState {
    ctx: QuintantCtx,
    visited: HashSet<i64>,
    frontier: Vec<i64>,
}

/// Packed flood-fill state, indexed by quintant.
pub type PackedFloodState = HashMap<usize, QuintantState>;

/// Input to `triple_space_flood_fill`: either a fresh bigint firewall (mutated
/// to include discoveries) or a reused `{state, delta}` from a previous call
/// (state reused, only `delta` cells converted).
pub enum FloodInput<'a> {
    Firewall(&'a mut HashSet<u64>),
    Reuse {
        state: PackedFloodState,
        delta: Vec<u64>,
    },
}

/// Result of `triple_space_flood_fill`.
pub struct FloodResult {
    pub interior_cells: Vec<u64>,
    pub frontier_cell_ids: Vec<u64>,
    pub state: PackedFloodState,
}

/// Pack a triple as a single integer key for fast set lookup.
/// Encoding: (x + max_row) * y_stride + y * 2 + parity, where parity = (x+y+z) ∈ {0,1}.
fn pack_triple_key(x: i32, y: i32, parity: i32, max_row: i32, y_stride: i32) -> i64 {
    ((x + max_row) as i64) * (y_stride as i64) + (y as i64) * 2 + (parity as i64)
}

/// Inverse of `pack_triple_key` — recover (x, y, z, parity) from a packed key.
fn unpack_triple_key(key: i64, max_row: i32, y_stride: i32) -> (i32, i32, i32, i32) {
    let parity = (key % 2) as i32;
    let y_part = (key - parity as i64) % (y_stride as i64);
    let y = (y_part / 2) as i32;
    let x = ((key - y_part - parity as i64) / (y_stride as i64)) as i32 - max_row;
    let z = parity - x - y;
    (x, y, z, parity)
}

/// Convert a packed triple key back to a cell ID, or None if it doesn't map to a valid cell.
fn packed_key_to_cell_id(
    key: i64,
    ctx: &QuintantCtx,
    hilbert_res: usize,
    max_row: i32,
    y_stride: i32,
    max_s: u64,
    resolution: i32,
) -> Option<u64> {
    let (x, y, z, _) = unpack_triple_key(key, max_row, y_stride);
    let triple = crate::lattice::Triple::new(x, y, z);
    let s = triple_to_s(&triple, hilbert_res, ctx.orientation)?;
    if s >= max_s {
        return None;
    }
    serialize(&A5Cell {
        origin_id: ctx.origin.id,
        segment: ctx.segment,
        s,
        resolution,
    })
    .ok()
}

/// Convert a cell ID into its quintant context and packed triple key.
fn cell_to_quintant_key(
    cell_id: u64,
    hilbert_res: usize,
    max_row: i32,
    y_stride: i32,
) -> Option<(usize, i64, QuintantCtx)> {
    let cell = deserialize(cell_id).ok()?;
    let origin = &get_origins()[cell.origin_id as usize];
    let orientation =
        quintant_tables().segment_to_orientation[cell.origin_id as usize * 5 + cell.segment];
    let triple = s_to_triple(cell.s, hilbert_res, orientation);
    let parity = triple.x + triple.y + triple.z; // 0 or 1
    let quintant_idx = (origin.id as usize) * 60 + cell.segment;
    let key = pack_triple_key(triple.x, triple.y, parity, max_row, y_stride);
    let ctx = QuintantCtx {
        origin,
        segment: cell.segment,
        orientation,
    };
    Some((quintant_idx, key, ctx))
}

/// Triple-space flood fill in packed integer coordinates — no per-step bigint ops.
/// Uses the 3 parity-valid ±1 moves; since those never cross quintant boundaries,
/// each quintant is flooded independently.
///
/// `seed_cell_ids` BFS seeds. Always added to the frontier, even if already
/// visited — reusing state with the same seeds restarts BFS.
///
/// `max_layers` Maximum BFS layers; `None` = run to convergence.
pub fn triple_space_flood_fill(
    firewall: FloodInput,
    seed_cell_ids: &[u64],
    resolution: i32,
    max_layers: Option<usize>,
) -> FloodResult {
    let hilbert_res = (resolution - FIRST_HILBERT_RESOLUTION + 1) as usize;
    let max_row = (1i32 << hilbert_res) - 1;
    let y_stride = (max_row + 1) * 2;
    let max_s = 4u64.pow(hilbert_res as u32);

    let (mut quintants, bigint_firewall): (PackedFloodState, Option<&mut HashSet<u64>>) =
        match firewall {
            FloodInput::Firewall(fw) => {
                let mut q: PackedFloodState = HashMap::new();
                for &cell_id in fw.iter() {
                    if let Some((quintant_idx, key, ctx)) =
                        cell_to_quintant_key(cell_id, hilbert_res, max_row, y_stride)
                    {
                        let entry = q.entry(quintant_idx).or_insert_with(|| QuintantState {
                            ctx,
                            visited: HashSet::new(),
                            frontier: Vec::new(),
                        });
                        entry.visited.insert(key);
                    }
                }
                (q, Some(fw))
            }
            FloodInput::Reuse { mut state, delta } => {
                // Stale frontier from prior call — clear so seeds drive this BFS.
                for q in state.values_mut() {
                    q.frontier.clear();
                }
                for cell_id in delta {
                    if let Some((quintant_idx, key, ctx)) =
                        cell_to_quintant_key(cell_id, hilbert_res, max_row, y_stride)
                    {
                        let entry = state.entry(quintant_idx).or_insert_with(|| QuintantState {
                            ctx,
                            visited: HashSet::new(),
                            frontier: Vec::new(),
                        });
                        entry.visited.insert(key);
                    }
                }
                (state, None)
            }
        };

    // Seed the frontier
    for &cell_id in seed_cell_ids {
        if let Some((quintant_idx, key, ctx)) =
            cell_to_quintant_key(cell_id, hilbert_res, max_row, y_stride)
        {
            let entry = quintants
                .entry(quintant_idx)
                .or_insert_with(|| QuintantState {
                    ctx,
                    visited: HashSet::new(),
                    frontier: Vec::new(),
                });
            entry.visited.insert(key);
            entry.frontier.push(key);
        }
    }

    // Discovered keys per quintant for THIS call (excludes prior-call discoveries).
    let mut discovered_per_q: HashMap<usize, Vec<i64>> = HashMap::new();

    let mut layers: usize = 0;
    let mut has_work = true;
    while has_work && max_layers.is_none_or(|m| layers < m) {
        has_work = false;
        // Iterate quintant indices snapshot to allow mutating frontier in place.
        let q_indices: Vec<usize> = quintants.keys().copied().collect();
        for q_idx in q_indices {
            let frontier_len = quintants.get(&q_idx).map(|q| q.frontier.len()).unwrap_or(0);
            if frontier_len == 0 {
                continue;
            }
            // Take ownership of current frontier; collect next frontier.
            let current_frontier: Vec<i64> = {
                let q = quintants.get_mut(&q_idx).unwrap();
                std::mem::take(&mut q.frontier)
            };
            let mut next_frontier: Vec<i64> = Vec::new();

            for key in current_frontier {
                let parity = (key % 2) as i32;
                let y_part = (key - parity as i64) % (y_stride as i64);
                let y = (y_part / 2) as i32;
                let x = ((key - y_part - parity as i64) / (y_stride as i64)) as i32 - max_row;
                let step: i32 = if parity == 0 { 1 } else { -1 };
                let new_parity = 1 - parity;
                let y_limit = y - new_parity;

                // Move in x: triple becomes (x+step, y, z); z = parity - x - y is unchanged.
                let nx = x + step;
                let nz_x = parity - x - y;
                if nx <= 0 && nz_x <= 0 && nx >= -y_limit && nz_x >= -y_limit {
                    let nk = ((nx + max_row) as i64) * (y_stride as i64)
                        + (y as i64) * 2
                        + new_parity as i64;
                    let q = quintants.get_mut(&q_idx).unwrap();
                    if q.visited.insert(nk) {
                        discovered_per_q.entry(q_idx).or_default().push(nk);
                        next_frontier.push(nk);
                    }
                }

                // Move in y: triple becomes (x, y+step, z); z is unchanged.
                let ny = y + step;
                let nz_y = parity - x - y;
                let ny_limit = ny - new_parity;
                if ny >= 0 && ny <= max_row && nz_y <= 0 && x >= -ny_limit && nz_y >= -ny_limit {
                    let nk = ((x + max_row) as i64) * (y_stride as i64)
                        + (ny as i64) * 2
                        + new_parity as i64;
                    let q = quintants.get_mut(&q_idx).unwrap();
                    if q.visited.insert(nk) {
                        discovered_per_q.entry(q_idx).or_default().push(nk);
                        next_frontier.push(nk);
                    }
                }

                // Move in z: triple becomes (x, y, z+step); the packed key shape (x, y, parity)
                // is identical to the x and y moves' starting point apart from parity flip.
                let z = parity - x - y;
                let nz = z + step;
                if nz <= 0 && x >= -y_limit && nz >= -y_limit {
                    let nk = ((x + max_row) as i64) * (y_stride as i64)
                        + (y as i64) * 2
                        + new_parity as i64;
                    let q = quintants.get_mut(&q_idx).unwrap();
                    if q.visited.insert(nk) {
                        discovered_per_q.entry(q_idx).or_default().push(nk);
                        next_frontier.push(nk);
                    }
                }
            }

            let q = quintants.get_mut(&q_idx).unwrap();
            q.frontier = next_frontier;
            if !q.frontier.is_empty() {
                has_work = true;
            }
        }
        layers += 1;
    }

    // Convert results back to cell IDs.
    let mut interior_cells: Vec<u64> = Vec::new();
    let mut frontier_cell_ids: Vec<u64> = Vec::new();

    for (q_idx, q) in &quintants {
        if let Some(discovered) = discovered_per_q.get(q_idx) {
            for &key in discovered {
                if let Some(cell_id) = packed_key_to_cell_id(
                    key,
                    &q.ctx,
                    hilbert_res,
                    max_row,
                    y_stride,
                    max_s,
                    resolution,
                ) {
                    interior_cells.push(cell_id);
                }
            }
        }
        for &key in &q.frontier {
            if let Some(cell_id) = packed_key_to_cell_id(
                key,
                &q.ctx,
                hilbert_res,
                max_row,
                y_stride,
                max_s,
                resolution,
            ) {
                frontier_cell_ids.push(cell_id);
            }
        }
    }

    if let Some(fw) = bigint_firewall {
        for &cell in &interior_cells {
            fw.insert(cell);
        }
    }

    FloodResult {
        interior_cells,
        frontier_cell_ids,
        state: quintants,
    }
}
