// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use std::collections::BTreeSet;

use crate::core::face_adjacency::FACE_ADJACENCY;
use crate::core::origin::{get_origins, segment_to_quintant};
use crate::core::serialization::{deserialize, serialize, FIRST_HILBERT_RESOLUTION};
use crate::core::utils::{A5Cell, Origin};
use crate::lattice::{s_to_cell, triple_parity};
use crate::traversal::lattice_boundary::{get_boundary_neighbors, BoundaryContext};
use crate::traversal::quintant_neighbors::find_quintant_neighbor_s;

/// Get neighbors of a resolution 0 cell (dodecahedron face).
fn get_res0_neighbors(origin: &Origin) -> Vec<u64> {
    let origins = get_origins();
    let mut neighbor_set = BTreeSet::new();
    for q in 0..5 {
        let (adjacent_face_id, _) = FACE_ADJACENCY[origin.id as usize][q];
        let adjacent_origin = &origins[adjacent_face_id as usize];
        if let Ok(cell_id) = serialize(&A5Cell {
            origin_id: adjacent_origin.id,
            segment: 0,
            s: 0,
            resolution: 0,
        }) {
            neighbor_set.insert(cell_id);
        }
    }
    neighbor_set.into_iter().collect()
}

/// Get all neighbors of a cell across quintant and face boundaries.
///
/// Within-quintant neighbors come from the fixed per-flavor triple deltas
/// (via `find_quintant_neighbor_s`). Cross-quintant, cross-face, apex, and
/// corner neighbors are emitted by the shared `get_boundary_neighbors` helper
/// using fixed delta tables — see `lattice_boundary.rs`.
///
/// `edge_only`: if true, return only edge-sharing neighbors (5 per cell).
/// Default false returns all neighbors including vertex-only neighbors (6-8 per cell).
pub fn get_global_cell_neighbors(cell_id: u64, edge_only: bool) -> Vec<u64> {
    let cell = match deserialize(cell_id) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let origins = get_origins();
    let origin = &origins[cell.origin_id as usize];
    let resolution = cell.resolution;

    if resolution == 0 {
        return get_res0_neighbors(origin);
    }

    let hilbert_res = (resolution - FIRST_HILBERT_RESOLUTION + 1) as usize;
    let (source_quintant, source_orientation) = segment_to_quintant(cell.segment, origin);

    // Triple coordinates are orientation-independent
    let source_cell = s_to_cell(cell.s, hilbert_res, source_orientation);
    let triple = source_cell.triple;

    let mut neighbor_set: BTreeSet<u64> = BTreeSet::new();

    // --- Within-quintant: fixed per-flavor triple deltas ---
    let within_neighbors = find_quintant_neighbor_s(
        &triple,
        source_cell.flavor,
        cell.s,
        hilbert_res,
        source_orientation,
        edge_only,
    );
    for neighbor_s in within_neighbors {
        if let Ok(neighbor_cell_id) = serialize(&A5Cell {
            origin_id: cell.origin_id,
            segment: cell.segment,
            s: neighbor_s,
            resolution,
        }) {
            neighbor_set.insert(neighbor_cell_id);
        }
    }

    // --- Cross-quintant / cross-face / apex / corner: shared lattice-boundary helper ---
    let ctx = BoundaryContext {
        triple,
        parity: triple_parity(&triple),
        source_quintant,
        origin,
        hilbert_res,
        max_s: 4u64.pow(hilbert_res as u32),
        max_row: (1i32 << hilbert_res) - 1,
        resolution,
    };
    for cell_id in get_boundary_neighbors(&ctx, edge_only, false) {
        neighbor_set.insert(cell_id);
    }

    neighbor_set.into_iter().collect()
}
