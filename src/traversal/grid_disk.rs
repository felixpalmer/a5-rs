// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use std::collections::HashSet;
use std::sync::LazyLock;

use crate::core::compact::compact;
use crate::core::face_adjacency::FACE_ADJACENCY;
use crate::core::origin::{get_origins, quintant_to_segment, segment_to_quintant};
use crate::core::serialization::{deserialize, serialize, FIRST_HILBERT_RESOLUTION};
use crate::core::utils::A5Cell;
use crate::lattice::{
    s_to_triple, triple_flavor, triple_in_bounds, triple_to_s, Orientation, Triple,
};
use crate::traversal::lattice_boundary::get_boundary_neighbor_triples;
use crate::traversal::neighbors::NEIGHBOR_DELTAS;

// Cells are deduplicated by one integer key: the quintant (origin.id * 5 +
// quintant, < 60), parity, and the low KEY_BITS bits of -x and -z (y follows).
// Up to Hilbert resolution 21 the coordinates fit whole; above it, two cells of
// one quintant share a key only if they are 2^22 rows apart, and a disk holding
// both would need k ≈ 2^21 (~10^12 cells) — far past what fits in memory.
const KEY_BITS: u32 = 22;
const KEY_MASK: i64 = (1 << KEY_BITS) - 1;
const KEY_SIDE: i64 = 1 << KEY_BITS;

/// Segment and curve orientation of each of the 60 quintants, by origin.id * 5 + quintant.
static QUINTANT_SEGMENTS: LazyLock<Vec<(usize, Orientation)>> = LazyLock::new(|| {
    let mut out = Vec::with_capacity(60);
    for origin in get_origins() {
        for q in 0..5 {
            out.push(quintant_to_segment(q, origin));
        }
    }
    out
});

/// One BFS ring: its dedup keys, and its cells as flat (origin_id, quintant, x, y, z).
#[derive(Default)]
struct Ring {
    keys: HashSet<i64>,
    cells: Vec<i32>,
}

/// Add a cell to `next` unless it is already in one of the three live rings.
fn add_cell(next: &mut Ring, prev: &Ring, current: &Ring, cell: [i32; 5]) {
    let [origin_id, quintant, x, y, z] = cell;
    let key = ((-x as i64 & KEY_MASK) * KEY_SIDE + (-z as i64 & KEY_MASK)) * 2
        + (x + y + z) as i64
        + (origin_id * 5 + quintant) as i64 * 2 * KEY_SIDE * KEY_SIDE;
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
        let (segment, orientation) = QUINTANT_SEGMENTS[(c[0] * 5 + c[1]) as usize];
        let s = triple_to_s(&Triple::new(c[2], c[3], c[4]), hilbert_res, orientation)
            .ok_or("grid_disk: invalid triple")?;
        out.push(serialize(&A5Cell {
            origin_id: c[0] as u8,
            segment,
            s,
            resolution,
        })?);
    }
    Ok(())
}

/// Resolution 0: the cells are the 12 dodecahedron faces, adjacent across their edges.
fn grid_disk_faces(origin_id: u8, k: usize) -> Result<Vec<u64>, String> {
    let mut disk: Vec<u8> = vec![origin_id];
    let mut ring = 0;
    while ring < k && disk.len() < 12 {
        for i in 0..disk.len() {
            for q in 0..5 {
                let face = FACE_ADJACENCY[disk[i] as usize][q].0;
                if !disk.contains(&face) {
                    disk.push(face);
                }
            }
        }
        ring += 1;
    }
    let cells = disk
        .iter()
        .map(|&id| {
            serialize(&A5Cell {
                origin_id: id,
                segment: 0,
                s: 0,
                resolution: 0,
            })
        })
        .collect::<Result<Vec<u64>, String>>()?;
    compact(&cells)
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
    if k == 0 {
        return Ok(vec![cell_id]);
    }
    let cell = deserialize(cell_id)?;
    if cell.resolution == 0 {
        return grid_disk_faces(cell.origin_id, k);
    }
    let origins = get_origins();
    let origin = &origins[cell.origin_id as usize];
    let hilbert_res = (cell.resolution - FIRST_HILBERT_RESOLUTION + 1) as usize;
    let max_row = (1i32 << hilbert_res) - 1;
    let (quintant, orientation) = segment_to_quintant(cell.segment, origin);
    let seed = s_to_triple(cell.s, hilbert_res, orientation);

    // The seed is `cell_id` already, so it goes straight to the output
    let mut interior: Vec<u64> = vec![cell_id];
    let mut prev_frontier = Ring::default();
    let mut frontier = Ring::default();
    add_cell(
        &mut frontier,
        &Ring::default(),
        &Ring::default(),
        [origin.id as i32, quintant as i32, seed.x, seed.y, seed.z],
    );
    let mut boundary: Vec<i32> = Vec::new();

    for ring in 1..=k {
        let mut next_frontier = Ring::default();
        for c in frontier.cells.chunks_exact(5) {
            let [origin_id, q, x, y, z] = [c[0], c[1], c[2], c[3], c[4]];
            let triple = Triple::new(x, y, z);

            // Within the quintant: the fixed per-flavor deltas
            let flavor = triple_flavor(&triple, max_row) as usize;
            let deltas: &[Triple] = if edge_only {
                &NEIGHBOR_DELTAS[flavor].edge
            } else {
                &NEIGHBOR_DELTAS[flavor].all
            };
            for d in deltas {
                let neighbor = Triple::new(x + d.x, y + d.y, z + d.z);
                if !triple_in_bounds(&neighbor, max_row) {
                    continue;
                }
                add_cell(
                    &mut next_frontier,
                    &prev_frontier,
                    &frontier,
                    [origin_id, q, neighbor.x, neighbor.y, neighbor.z],
                );
            }

            // Across a quintant edge: the boundary delta tables
            if x == 0 || z == 0 || y == max_row {
                boundary.clear();
                get_boundary_neighbor_triples(
                    triple,
                    x + y + z,
                    q as usize,
                    &origins[origin_id as usize],
                    max_row,
                    edge_only,
                    false,
                    &mut boundary,
                );
                for b in boundary.chunks_exact(5) {
                    add_cell(
                        &mut next_frontier,
                        &prev_frontier,
                        &frontier,
                        [b[0], b[1], b[2], b[3], b[4]],
                    );
                }
            }
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
            interior = compact(&interior)?;
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
