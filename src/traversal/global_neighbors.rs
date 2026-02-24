// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use std::collections::BTreeSet;

use crate::core::face_adjacency::FACE_ADJACENCY;
use crate::core::origin::{get_origins, quintant_to_segment, segment_to_quintant};
use crate::core::serialization::{deserialize, serialize, FIRST_HILBERT_RESOLUTION};
use crate::core::utils::Origin;
use crate::lattice::{
    anchor_to_triple, s_to_anchor, triple_in_bounds, triple_parity, triple_to_anchor, triple_to_s,
    Orientation, Triple,
};
use crate::traversal::quintant_neighbors::find_quintant_neighbor_s;

// Neighbor delta: (dx, dy, dz, is_edge_sharing)
type NeighborDelta = (i32, i32, i32, bool);

// Cross-quintant left-edge deltas (source z=0), indexed by parity * 2 + (y_odd ? 1 : 0)
const LEFT_EDGE_DELTAS: [&[NeighborDelta]; 4] = [
    // parity=0, yEven
    &[(0, 0, 0, true), (0, 0, 1, false)],
    // parity=0, yOdd
    &[
        (0, 0, 0, true),
        (0, 1, 0, true),
        (0, -1, 1, false),
        (0, 1, -1, false),
    ],
    // parity=1, yEven
    &[],
    // parity=1, yOdd
    &[(0, -1, 0, true), (0, 0, -1, false)],
];

// Cross-quintant right-edge deltas (source x=0), indexed by parity * 2 + (y_odd ? 1 : 0)
const RIGHT_EDGE_DELTAS: [&[NeighborDelta]; 4] = [
    // parity=0, yEven
    &[
        (0, 0, 0, true),
        (0, 1, 0, true),
        (-1, 1, 0, false),
        (1, -1, 0, false),
    ],
    // parity=0, yOdd
    &[(0, 0, 0, true), (1, 0, 0, false)],
    // parity=1, yEven
    &[(0, -1, 0, true), (-1, 0, 0, false)],
    // parity=1, yOdd
    &[],
];

// Cross-face base-edge deltas (source y=maxRow), indexed by parity
const CROSS_FACE_DELTAS: [&[NeighborDelta]; 2] = [
    // parity=0
    &[(0, 0, 0, true), (1, 0, 0, true), (1, 0, -1, false)],
    // parity=1
    &[(0, 0, -1, true), (0, 0, 0, false)],
];

struct NeighborContext {
    hilbert_res: usize,
    resolution: i32,
    max_s: u64,
    max_row: i32,
    edge_only: bool,
    neighbor_set: BTreeSet<u64>,
}

impl NeighborContext {
    fn new(hilbert_res: usize, resolution: i32, edge_only: bool) -> Self {
        Self {
            hilbert_res,
            resolution,
            max_s: 4u64.pow(hilbert_res as u32),
            max_row: (1i32 << hilbert_res) - 1,
            edge_only,
            neighbor_set: BTreeSet::new(),
        }
    }
}

fn add_neighbor(
    ctx: &mut NeighborContext,
    neighbor_triple: &Triple,
    orientation: Orientation,
    neighbor_origin: &Origin,
    neighbor_segment: usize,
) {
    if let Some(s) = triple_to_s(neighbor_triple, ctx.hilbert_res, orientation) {
        if s < ctx.max_s {
            if let Ok(cell_id) = serialize(&crate::core::utils::A5Cell {
                origin_id: neighbor_origin.id,
                segment: neighbor_segment,
                s,
                resolution: ctx.resolution,
            }) {
                ctx.neighbor_set.insert(cell_id);
            }
        }
    }
}

fn add_delta_neighbors(
    ctx: &mut NeighborContext,
    base: &Triple,
    deltas: &[NeighborDelta],
    orientation: Orientation,
    neighbor_origin: &Origin,
    neighbor_segment: usize,
) {
    for &(dx, dy, dz, is_edge) in deltas {
        if ctx.edge_only && !is_edge {
            continue;
        }
        let neighbor_triple = Triple::new(base.x + dx, base.y + dy, base.z + dz);
        if !triple_in_bounds(&neighbor_triple, ctx.max_row) {
            continue;
        }
        add_neighbor(
            ctx,
            &neighbor_triple,
            orientation,
            neighbor_origin,
            neighbor_segment,
        );
    }
}

/// Get all neighbors of a cell across quintant and face boundaries.
pub fn get_global_cell_neighbors(cell_id: u64, edge_only: bool) -> Vec<u64> {
    let cell = match deserialize(cell_id) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let origins = get_origins();
    let origin = &origins[cell.origin_id as usize];
    let resolution = cell.resolution;

    if resolution < FIRST_HILBERT_RESOLUTION {
        return Vec::new(); // No neighbors for res 0-1
    }

    let hilbert_res = (resolution - FIRST_HILBERT_RESOLUTION + 1) as usize;
    let (source_quintant, source_orientation) = segment_to_quintant(cell.segment, origin);
    let anchor = s_to_anchor(cell.s, hilbert_res, source_orientation);

    // Triple coordinates are orientation-independent
    let triple = anchor_to_triple(&anchor);

    // Get uv anchor for is_neighbor validation (within-quintant)
    let uv_source_anchor = triple_to_anchor(&triple, hilbert_res, Orientation::UV);

    let mut ctx = NeighborContext::new(hilbert_res, resolution, edge_only);

    // --- Within-quintant neighbors ---
    let within_neighbors = find_quintant_neighbor_s(
        &triple,
        uv_source_anchor.as_ref(),
        cell.s,
        hilbert_res,
        source_orientation,
        edge_only,
    );
    for neighbor_s in within_neighbors {
        if let Ok(neighbor_cell_id) = serialize(&crate::core::utils::A5Cell {
            origin_id: cell.origin_id,
            segment: cell.segment,
            s: neighbor_s,
            resolution,
        }) {
            ctx.neighbor_set.insert(neighbor_cell_id);
        }
    }

    // --- Cross-quintant neighbors ---
    let parity = triple_parity(&triple);
    let y_odd = triple.y % 2 != 0;
    let delta_index = (parity * 2 + if y_odd { 1 } else { 0 }) as usize;

    // Left edge (z=0): neighbor in previous quintant at swapped [0, y, x]
    if triple.z == 0 {
        let target_quintant = (source_quintant + 4) % 5; // (source_quintant - 1 + 5) % 5
        let (target_segment, target_orientation) = quintant_to_segment(target_quintant, origin);
        let swapped_base = Triple::new(0, triple.y, triple.x);
        add_delta_neighbors(
            &mut ctx,
            &swapped_base,
            LEFT_EDGE_DELTAS[delta_index],
            target_orientation,
            origin,
            target_segment,
        );
    }

    // Right edge (x=0): neighbor in next quintant at swapped [z, y, 0]
    if triple.x == 0 {
        let target_quintant = (source_quintant + 1) % 5;
        let (target_segment, target_orientation) = quintant_to_segment(target_quintant, origin);
        let swapped_base = Triple::new(triple.z, triple.y, 0);
        add_delta_neighbors(
            &mut ctx,
            &swapped_base,
            RIGHT_EDGE_DELTAS[delta_index],
            target_orientation,
            origin,
            target_segment,
        );
    }

    // --- Cross-face neighbors ---
    if triple.y == ctx.max_row {
        let (adj_face_id, adj_quintant) = FACE_ADJACENCY[origin.id as usize][source_quintant];
        let adj_origin = &origins[adj_face_id as usize];
        let (adj_segment, adj_orientation) = quintant_to_segment(adj_quintant, adj_origin);
        let mirrored_base = Triple::new(triple.z, ctx.max_row, triple.x);
        add_delta_neighbors(
            &mut ctx,
            &mirrored_base,
            CROSS_FACE_DELTAS[parity as usize],
            adj_orientation,
            adj_origin,
            adj_segment,
        );
    }

    // Apex: [0,0,0] cells from all 5 quintants meet at the face center
    if triple.x == 0 && triple.y == 0 && triple.z == 0 {
        for q in 0..5usize {
            if q == source_quintant {
                continue;
            }
            // Adjacent quintants (distance=1) share an edge; non-adjacent (distance=2) share only a vertex
            let distance =
                std::cmp::min((q + 5 - source_quintant) % 5, (source_quintant + 5 - q) % 5);
            if edge_only && distance != 1 {
                continue;
            }
            let (target_segment, target_orientation) = quintant_to_segment(q, origin);
            add_neighbor(
                &mut ctx,
                &triple,
                target_orientation,
                origin,
                target_segment,
            );
        }
    }

    // Special case: base-left corner cells
    if triple.x == -ctx.max_row && triple.y == ctx.max_row && triple.z == 0 {
        // Vertex neighbor 1: across the previous quintant's base edge
        let prev_quintant = (source_quintant + 4) % 5;
        let (prev_adj_face_id, prev_adj_quintant) =
            FACE_ADJACENCY[origin.id as usize][prev_quintant];
        let prev_adj_origin = &origins[prev_adj_face_id as usize];
        let (prev_adj_segment, prev_adj_orientation) =
            quintant_to_segment(prev_adj_quintant, prev_adj_origin);
        add_neighbor(
            &mut ctx,
            &triple,
            prev_adj_orientation,
            prev_adj_origin,
            prev_adj_segment,
        );

        // Vertex neighbor 2: adjacent quintant on the primary cross-face
        let (cross_face_id, cross_quintant) = FACE_ADJACENCY[origin.id as usize][source_quintant];
        let cross_origin = &origins[cross_face_id as usize];
        let next_cross_quintant = (cross_quintant + 1) % 5;
        let (cross_segment, cross_orientation) =
            quintant_to_segment(next_cross_quintant, cross_origin);
        add_neighbor(
            &mut ctx,
            &triple,
            cross_orientation,
            cross_origin,
            cross_segment,
        );
    }

    ctx.neighbor_set.into_iter().collect()
}
