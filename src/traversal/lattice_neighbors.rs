// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use crate::core::origin::{get_origins, segment_to_quintant};
use crate::core::serialization::{deserialize, serialize, FIRST_HILBERT_RESOLUTION};
use crate::core::utils::{A5Cell, Origin};
use crate::lattice::{
    anchor_to_triple, s_to_anchor, triple_in_bounds, triple_parity, triple_to_s, Orientation,
    Triple,
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
    let anchor = s_to_anchor(cell.s, hilbert_res, orientation);
    let triple = anchor_to_triple(&anchor);

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

/// Fast lattice-based neighbor finding for BFS in line tracing.
///
/// Unlike `get_global_cell_neighbors`, this skips `is_neighbor()` validation
/// for within-quintant candidates. The result is a SUPERSET of true neighbors —
/// it may include a few extra cells that share only a vertex point (not an edge).
///
/// This is safe for BFS contexts where candidates are validated by
/// `cell_intersects_segment` — false positives just fail that check.
///
/// For res < 2, falls back to `get_global_cell_neighbors` (rare).
///
/// `edge_only`: if true, restrict to Manhattan distance ≤ 2 (edge-sharing candidates).
pub fn get_lattice_neighbors(cell_id: u64, edge_only: bool) -> Vec<u64> {
    let src = match decode_source(cell_id) {
        Some(s) => s,
        None => return get_global_cell_neighbors(cell_id, edge_only),
    };

    let mut result: Vec<u64> = Vec::new();

    // Within-quintant: enumerate the 26-cube of ±1 deltas, skipping the source.
    for dx in -1i32..=1 {
        for dy in -1i32..=1 {
            for dz in -1i32..=1 {
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

                let candidate =
                    Triple::new(src.triple.x + dx, src.triple.y + dy, src.triple.z + dz);
                if !triple_in_bounds(&candidate, src.max_row) {
                    continue;
                }

                if let Some(candidate_s) = triple_to_s(&candidate, src.hilbert_res, src.orientation)
                {
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
        }
    }

    for c in get_boundary_neighbors(&boundary_context(&src), edge_only) {
        result.push(c);
    }
    result
}
