// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use crate::core::face_adjacency::FACE_ADJACENCY;
use crate::core::origin::{get_origins, quintant_to_segment};
use crate::core::serialization::serialize;
use crate::core::utils::{A5Cell, Origin};
use crate::lattice::{triple_in_bounds, triple_to_s, Triple};

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

/// If the triple is a valid cell, append it to `out` as (origin_id, quintant, x, y, z).
fn push_triple(out: &mut Vec<i32>, triple: Triple, origin_id: u8, quintant: usize, max_row: i32) {
    if !triple_in_bounds(&triple, max_row) {
        return;
    }
    out.extend_from_slice(&[
        origin_id as i32,
        quintant as i32,
        triple.x,
        triple.y,
        triple.z,
    ]);
}

/// Apply a delta table to a base triple, appending each valid cell.
fn push_deltas(
    out: &mut Vec<i32>,
    base: &Triple,
    deltas: &[NeighborDelta],
    edge_only: bool,
    origin_id: u8,
    quintant: usize,
    max_row: i32,
) {
    for &(dx, dy, dz, is_edge) in deltas {
        if edge_only && !is_edge {
            continue;
        }
        let neighbor = Triple::new(base.x + dx, base.y + dy, base.z + dz);
        push_triple(out, neighbor, origin_id, quintant, max_row);
    }
}

/// Every neighbor that lies outside the source cell's quintant, appended to
/// `out` as flat (origin_id, quintant, x, y, z) quintuples: cross-quintant
/// lateral edges, cross-face base edge, apex (face center), and (when not
/// `skip_corners`) the `[-max_row, max_row, 0]` vertex corner. The
/// within-quintant ±1 candidates are NOT covered here — callers generate those
/// directly.
///
/// Only cells on a quintant edge (x = 0, z = 0 or y = max_row) have any. The
/// result may contain duplicates; callers deduplicate.
///
/// `edge_only` drops apex non-adjacent quintants and other vertex-only neighbors.
/// `skip_corners` drops the `[-max_row, max_row, 0]` corner — used when the caller's
/// connectivity (e.g. lattice ±1 moves) doesn't traverse that vertex.
#[allow(clippy::too_many_arguments)]
pub fn get_boundary_neighbor_triples(
    triple: Triple,
    parity: i32,
    source_quintant: usize,
    origin: &Origin,
    max_row: i32,
    edge_only: bool,
    skip_corners: bool,
    out: &mut Vec<i32>,
) {
    let y_odd = triple.y % 2 != 0;
    let delta_index = (parity * 2 + if y_odd { 1 } else { 0 }) as usize;

    // Left edge (z=0): neighbor in previous quintant at swapped [0, y, x]
    if triple.z == 0 {
        let target_quintant = (source_quintant + 4) % 5;
        let base = Triple::new(0, triple.y, triple.x);
        push_deltas(
            out,
            &base,
            LEFT_EDGE_DELTAS[delta_index],
            edge_only,
            origin.id,
            target_quintant,
            max_row,
        );
    }

    // Right edge (x=0): neighbor in next quintant at swapped [z, y, 0]
    if triple.x == 0 {
        let target_quintant = (source_quintant + 1) % 5;
        let base = Triple::new(triple.z, triple.y, 0);
        push_deltas(
            out,
            &base,
            RIGHT_EDGE_DELTAS[delta_index],
            edge_only,
            origin.id,
            target_quintant,
            max_row,
        );
    }

    // Base edge (y=max_row): neighbor on adjacent face at mirrored [z, max_row, x]
    if triple.y == max_row {
        let (adj_face_id, adj_quintant) = FACE_ADJACENCY[origin.id as usize][source_quintant];
        let base = Triple::new(triple.z, max_row, triple.x);
        push_deltas(
            out,
            &base,
            CROSS_FACE_DELTAS[parity as usize],
            edge_only,
            adj_face_id,
            adj_quintant,
            max_row,
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
            push_triple(out, triple, origin.id, q, max_row);
        }
    }

    // Base-left corner [-max_row, max_row, 0]: 3 dodecahedron faces meet at this vertex.
    // The symmetric base-right corner is implicitly covered: its cross-quintant and
    // cross-face paths land on the [-max_row, max_row, 0] cell of neighboring quintants.
    if !skip_corners && triple.x == -max_row && triple.y == max_row && triple.z == 0 {
        // Vertex neighbor 1: across the previous quintant's base edge
        let prev_quintant = (source_quintant + 4) % 5;
        let (prev_adj_face_id, prev_adj_quintant) =
            FACE_ADJACENCY[origin.id as usize][prev_quintant];
        push_triple(out, triple, prev_adj_face_id, prev_adj_quintant, max_row);

        // Vertex neighbor 2: adjacent quintant on the primary cross-face
        let (cross_face_id, cross_quintant) = FACE_ADJACENCY[origin.id as usize][source_quintant];
        push_triple(
            out,
            triple,
            cross_face_id,
            (cross_quintant + 1) % 5,
            max_row,
        );
    }
}

/// The neighbors outside the source cell's quintant (see
/// `get_boundary_neighbor_triples`), as cell IDs.
///
/// The result may contain duplicates and the order is not stable; callers
/// deduplicate (via Set) or accept duplicates if their downstream pipeline tolerates them.
pub fn get_boundary_neighbors(
    ctx: &BoundaryContext,
    edge_only: bool,
    skip_corners: bool,
) -> Vec<u64> {
    let mut triples: Vec<i32> = Vec::new();
    get_boundary_neighbor_triples(
        ctx.triple,
        ctx.parity,
        ctx.source_quintant,
        ctx.origin,
        ctx.max_row,
        edge_only,
        skip_corners,
        &mut triples,
    );
    let origins = get_origins();
    let mut out: Vec<u64> = Vec::new();
    for t in triples.chunks_exact(5) {
        let origin = &origins[t[0] as usize];
        let (segment, orientation) = quintant_to_segment(t[1] as usize, origin);
        let triple = Triple::new(t[2], t[3], t[4]);
        if let Some(s) = triple_to_s(&triple, ctx.hilbert_res, orientation) {
            if s >= ctx.max_s {
                continue;
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
    out
}
