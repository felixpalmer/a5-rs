// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use crate::core::face_adjacency::FACE_ADJACENCY;
use crate::core::origin::{get_origins, quintant_to_segment};
use crate::core::serialization::serialize;
use crate::core::utils::{A5Cell, Origin};
use crate::lattice::{triple_in_bounds, triple_to_s, Orientation, Triple};

/// Neighbor delta: (dx, dy, dz, is_edge_sharing)
pub type NeighborDelta = (i32, i32, i32, bool);

/// Cross-quintant left-edge deltas (source z=0), indexed by `parity * 2 + (y_odd ? 1 : 0)`.
/// Applied to the swapped base triple [0, y, x] in the previous quintant.
pub const LEFT_EDGE_DELTAS: [&[NeighborDelta]; 4] = [
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

/// Cross-quintant right-edge deltas (source x=0), indexed by `parity * 2 + (y_odd ? 1 : 0)`.
/// Applied to the swapped base triple [z, y, 0] in the next quintant.
pub const RIGHT_EDGE_DELTAS: [&[NeighborDelta]; 4] = [
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

/// Cross-face base-edge deltas (source y=max_row), indexed by parity.
/// Applied to the mirrored position [z, max_row, x] on the adjacent face.
pub const CROSS_FACE_DELTAS: [&[NeighborDelta]; 2] = [
    // parity=0
    &[(0, 0, 0, true), (1, 0, 0, true), (1, 0, -1, false)],
    // parity=1
    &[(0, 0, -1, true), (0, 0, 0, false)],
];

/// Source-cell context shared by all boundary-neighbor cases.
pub struct BoundaryContext<'a> {
    pub triple: Triple,
    pub parity: i32,
    pub source_quintant: usize,
    pub origin: &'a Origin,
    pub hilbert_res: usize,
    pub max_s: u64,
    pub max_row: i32,
    pub resolution: i32,
}

/// If the triple maps to a valid cell, append its cell ID to `out`.
fn push_triple(
    out: &mut Vec<u64>,
    triple: &Triple,
    orientation: Orientation,
    origin: &Origin,
    segment: usize,
    ctx: &BoundaryContext,
) {
    if !triple_in_bounds(triple, ctx.max_row) {
        return;
    }
    if let Some(s) = triple_to_s(triple, ctx.hilbert_res, orientation) {
        if s >= ctx.max_s {
            return;
        }
        if let Ok(cell_id) = serialize(&A5Cell {
            origin_id: origin.id,
            segment,
            s,
            resolution: ctx.resolution,
        }) {
            out.push(cell_id);
        }
    }
}

/// Apply a delta table to a base triple, appending each valid cell.
#[allow(clippy::too_many_arguments)]
fn push_deltas(
    out: &mut Vec<u64>,
    base: &Triple,
    deltas: &[NeighborDelta],
    edge_only: bool,
    orientation: Orientation,
    origin: &Origin,
    segment: usize,
    ctx: &BoundaryContext,
) {
    for &(dx, dy, dz, is_edge) in deltas {
        if edge_only && !is_edge {
            continue;
        }
        let neighbor = Triple::new(base.x + dx, base.y + dy, base.z + dz);
        push_triple(out, &neighbor, orientation, origin, segment, ctx);
    }
}

/// Return every neighbor that lies outside the source cell's quintant: cross-quintant
/// lateral edges, cross-face base edge, apex (face center), and the [-max_row, max_row, 0]
/// vertex corner. The within-quintant ±1 candidates are NOT covered here — callers
/// generate those directly.
///
/// The result may contain duplicates and the order is not stable; callers
/// deduplicate (via Set) or accept duplicates if their downstream pipeline tolerates them.
pub fn get_boundary_neighbors(ctx: &BoundaryContext, edge_only: bool) -> Vec<u64> {
    let mut out: Vec<u64> = Vec::new();
    let triple = ctx.triple;
    let parity = ctx.parity;
    let source_quintant = ctx.source_quintant;
    let origin = ctx.origin;
    let max_row = ctx.max_row;
    let y_odd = triple.y % 2 != 0;
    let delta_index = (parity * 2 + if y_odd { 1 } else { 0 }) as usize;

    let origins = get_origins();

    // Left edge (z=0): neighbor in previous quintant at swapped [0, y, x]
    if triple.z == 0 {
        let target_quintant = (source_quintant + 4) % 5;
        let (segment, orientation) = quintant_to_segment(target_quintant, origin);
        let base = Triple::new(0, triple.y, triple.x);
        push_deltas(
            &mut out,
            &base,
            LEFT_EDGE_DELTAS[delta_index],
            edge_only,
            orientation,
            origin,
            segment,
            ctx,
        );
    }

    // Right edge (x=0): neighbor in next quintant at swapped [z, y, 0]
    if triple.x == 0 {
        let target_quintant = (source_quintant + 1) % 5;
        let (segment, orientation) = quintant_to_segment(target_quintant, origin);
        let base = Triple::new(triple.z, triple.y, 0);
        push_deltas(
            &mut out,
            &base,
            RIGHT_EDGE_DELTAS[delta_index],
            edge_only,
            orientation,
            origin,
            segment,
            ctx,
        );
    }

    // Base edge (y=max_row): neighbor on adjacent face at mirrored [z, max_row, x]
    if triple.y == max_row {
        let (adj_face_id, adj_quintant) = FACE_ADJACENCY[origin.id as usize][source_quintant];
        let adj_origin = &origins[adj_face_id as usize];
        let (segment, orientation) = quintant_to_segment(adj_quintant, adj_origin);
        let base = Triple::new(triple.z, max_row, triple.x);
        push_deltas(
            &mut out,
            &base,
            CROSS_FACE_DELTAS[parity as usize],
            edge_only,
            orientation,
            adj_origin,
            segment,
            ctx,
        );
    }

    // Apex [0,0,0]: cells from all 5 quintants meet at the face center
    if triple.x == 0 && triple.y == 0 && triple.z == 0 {
        for q in 0..5usize {
            if q == source_quintant {
                continue;
            }
            let distance =
                std::cmp::min((q + 5 - source_quintant) % 5, (source_quintant + 5 - q) % 5);
            if edge_only && distance != 1 {
                continue;
            }
            let (segment, orientation) = quintant_to_segment(q, origin);
            push_triple(&mut out, &triple, orientation, origin, segment, ctx);
        }
    }

    // Base-left corner [-max_row, max_row, 0]: 3 dodecahedron faces meet at this vertex.
    // The symmetric base-right corner is implicitly covered: its cross-quintant and
    // cross-face paths land on the [-max_row, max_row, 0] cell of neighboring quintants.
    if triple.x == -max_row && triple.y == max_row && triple.z == 0 {
        // Vertex neighbor 1: across the previous quintant's base edge
        let prev_quintant = (source_quintant + 4) % 5;
        let (prev_adj_face_id, prev_adj_quintant) =
            FACE_ADJACENCY[origin.id as usize][prev_quintant];
        let prev_adj_origin = &origins[prev_adj_face_id as usize];
        let (prev_adj_segment, prev_adj_orientation) =
            quintant_to_segment(prev_adj_quintant, prev_adj_origin);
        push_triple(
            &mut out,
            &triple,
            prev_adj_orientation,
            prev_adj_origin,
            prev_adj_segment,
            ctx,
        );

        // Vertex neighbor 2: adjacent quintant on the primary cross-face
        let (cross_face_id, cross_quintant) = FACE_ADJACENCY[origin.id as usize][source_quintant];
        let cross_origin = &origins[cross_face_id as usize];
        let next_cross_quintant = (cross_quintant + 1) % 5;
        let (cross_segment, cross_orientation) =
            quintant_to_segment(next_cross_quintant, cross_origin);
        push_triple(
            &mut out,
            &triple,
            cross_orientation,
            cross_origin,
            cross_segment,
            ctx,
        );
    }

    out
}
