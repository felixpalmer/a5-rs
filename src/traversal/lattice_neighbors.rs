// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use crate::core::origin::{get_origins, segment_to_quintant};
use crate::core::serialization::{deserialize, serialize, FIRST_HILBERT_RESOLUTION};
use crate::core::utils::{A5Cell, Origin};
use crate::lattice::{
    s_to_triple, triple_in_bounds, triple_parity, triple_to_s, Orientation, Triple,
};
use crate::traversal::global_neighbors::get_global_cell_neighbors;
use crate::traversal::lattice_boundary::{get_boundary_neighbors, BoundaryContext};

/// Source-cell state used by the lattice neighbor finder.
struct LatticeSource<'a> {
    origin: &'a Origin,
    segment: usize,
    s: u64,
    resolution: i32,
    hilbert_res: usize,
    quintant: usize,
    orientation: Orientation,
    triple: Triple,
    max_s: u64,
    max_row: i32,
}

/// Deserialize and unpack into a LatticeSource. Returns None below FIRST_HILBERT_RESOLUTION.
fn decode_source(cell_id: u64) -> Option<LatticeSource<'static>> {
    let cell = deserialize(cell_id).ok()?;
    if cell.resolution < FIRST_HILBERT_RESOLUTION {
        return None;
    }
    let origin = &get_origins()[cell.origin_id as usize];
    let hilbert_res = (cell.resolution - FIRST_HILBERT_RESOLUTION + 1) as usize;
    let (quintant, orientation) = segment_to_quintant(cell.segment, origin);
    let triple = s_to_triple(cell.s, hilbert_res, orientation);

    Some(LatticeSource {
        origin,
        segment: cell.segment,
        s: cell.s,
        resolution: cell.resolution,
        hilbert_res,
        quintant,
        orientation,
        triple,
        max_s: 4u64.pow(hilbert_res as u32),
        max_row: (1i32 << hilbert_res) - 1,
    })
}

/// Build the BoundaryContext used by lattice-boundary helpers.
fn boundary_context<'a>(src: &'a LatticeSource<'a>) -> BoundaryContext<'a> {
    BoundaryContext {
        triple: src.triple,
        parity: triple_parity(&src.triple),
        source_quintant: src.quintant,
        origin: src.origin,
        hilbert_res: src.hilbert_res,
        max_s: src.max_s,
        max_row: src.max_row,
        resolution: src.resolution,
    }
}

type Delta = (i32, i32, i32);

/// All 26 non-zero ±1 moves in 3D — vertex- and edge-sharing within-quintant candidates.
const SUPERSET_DELTAS: &[Delta] = &[
    (-1, -1, -1),
    (-1, -1, 0),
    (-1, -1, 1),
    (-1, 0, -1),
    (-1, 0, 0),
    (-1, 0, 1),
    (-1, 1, -1),
    (-1, 1, 0),
    (-1, 1, 1),
    (0, -1, -1),
    (0, -1, 0),
    (0, -1, 1),
    (0, 0, -1),
    (0, 0, 1),
    (0, 1, -1),
    (0, 1, 0),
    (0, 1, 1),
    (1, -1, -1),
    (1, -1, 0),
    (1, -1, 1),
    (1, 0, -1),
    (1, 0, 0),
    (1, 0, 1),
    (1, 1, -1),
    (1, 1, 0),
    (1, 1, 1),
];

/// The 3 parity-valid single-axis moves matching `triple_space_flood_fill`'s edge connectivity.
const PARITY_EVEN_DELTAS: &[Delta] = &[(1, 0, 0), (0, 1, 0), (0, 0, 1)];
const PARITY_ODD_DELTAS: &[Delta] = &[(-1, 0, 0), (0, -1, 0), (0, 0, -1)];

/// Fast lattice-based neighbor finding. Skips `is_neighbor()` validation for
/// within-quintant candidates; falls back to `get_global_cell_neighbors` below res 2.
///
/// - `edge_only=false`: 26-cube ±1 superset (may include vertex-only touchers).
///   For BFS that re-validates candidates downstream (e.g. line tracing).
/// - `edge_only=true`: 3 parity-valid moves matching `triple_space_flood_fill` —
///   exact connectivity for shell-buffering the flood-fill firewall.
pub fn get_lattice_neighbors(cell_id: u64, edge_only: bool) -> Vec<u64> {
    let src = match decode_source(cell_id) {
        Some(s) => s,
        None => return get_global_cell_neighbors(cell_id, edge_only),
    };

    let deltas: &[Delta] = if edge_only {
        if triple_parity(&src.triple) == 0 {
            PARITY_EVEN_DELTAS
        } else {
            PARITY_ODD_DELTAS
        }
    } else {
        SUPERSET_DELTAS
    };

    let mut result: Vec<u64> = Vec::new();

    for &(dx, dy, dz) in deltas {
        let candidate = Triple::new(src.triple.x + dx, src.triple.y + dy, src.triple.z + dz);
        if !triple_in_bounds(&candidate, src.max_row) {
            continue;
        }
        if let Some(candidate_s) = triple_to_s(&candidate, src.hilbert_res, src.orientation) {
            if candidate_s < src.max_s && candidate_s != src.s {
                if let Ok(cell_id) = serialize(&A5Cell {
                    origin_id: src.origin.id,
                    segment: src.segment,
                    s: candidate_s,
                    resolution: src.resolution,
                }) {
                    result.push(cell_id);
                }
            }
        }
    }

    // Strict lattice connectivity (edge_only) doesn't traverse the [-max_row, max_row, 0]
    // vertex corner, so we skip it there too — keeping the firewall topology tight.
    for c in get_boundary_neighbors(&boundary_context(&src), edge_only, edge_only) {
        result.push(c);
    }
    result
}
