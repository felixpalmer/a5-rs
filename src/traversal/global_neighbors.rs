// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use std::collections::BTreeSet;

use crate::core::face_adjacency::FACE_ADJACENCY;
use crate::core::origin::{get_origins, quintant_to_segment, segment_to_quintant};
use crate::core::serialization::{deserialize, serialize, FIRST_HILBERT_RESOLUTION};
use crate::core::utils::{A5Cell, Origin};
use crate::lattice::{anchor_to_triple, s_to_anchor, triple_parity, triple_to_anchor, Orientation};
use crate::traversal::lattice_boundary::{get_boundary_neighbors, BoundaryContext};
use crate::traversal::quintant_neighbors::find_quintant_neighbor_s;

/// Serialize a res 1 cell from origin and quintant.
fn serialize_res1(origin: &Origin, quintant: usize) -> u64 {
    let (segment, _) = quintant_to_segment(quintant, origin);
    serialize(&A5Cell {
        origin_id: origin.id,
        segment,
        s: 0,
        resolution: 1,
    })
    .unwrap()
}

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

/// Get neighbors of a resolution 1 cell (quintant).
fn get_res1_neighbors(origin: &Origin, segment: usize, edge_only: bool) -> Vec<u64> {
    let origins = get_origins();
    let (quintant, _) = segment_to_quintant(segment, origin);
    let mut neighbor_set = BTreeSet::new();

    // Left and right quintant on the same face (A, B)
    let left_q = (quintant + 4) % 5;
    let right_q = (quintant + 1) % 5;
    neighbor_set.insert(serialize_res1(origin, left_q));
    neighbor_set.insert(serialize_res1(origin, right_q));

    // Adjacent quintant on adjacent face (C)
    let (adjacent_face_id, adjacent_quintant) = FACE_ADJACENCY[origin.id as usize][quintant];
    let adjacent_origin = &origins[adjacent_face_id as usize];
    neighbor_set.insert(serialize_res1(adjacent_origin, adjacent_quintant));

    if edge_only {
        return neighbor_set.into_iter().collect();
    }

    // Remaining neighbors on face
    neighbor_set.insert(serialize_res1(origin, (quintant + 3) % 5));
    neighbor_set.insert(serialize_res1(origin, (quintant + 2) % 5));

    // Left & right quintant neighbors of C
    neighbor_set.insert(serialize_res1(adjacent_origin, (adjacent_quintant + 4) % 5));
    neighbor_set.insert(serialize_res1(adjacent_origin, (adjacent_quintant + 1) % 5));

    // Two neighbors each from adjacent faces of A & B
    let (left_adjacent_face_id, left_adjacent_quintant) =
        FACE_ADJACENCY[origin.id as usize][left_q];
    let left_adjacent_origin = &origins[left_adjacent_face_id as usize];
    neighbor_set.insert(serialize_res1(left_adjacent_origin, left_adjacent_quintant));
    neighbor_set.insert(serialize_res1(
        left_adjacent_origin,
        (left_adjacent_quintant + 4) % 5,
    ));

    let (right_adjacent_face_id, right_adjacent_quintant) =
        FACE_ADJACENCY[origin.id as usize][right_q];
    let right_adjacent_origin = &origins[right_adjacent_face_id as usize];
    neighbor_set.insert(serialize_res1(
        right_adjacent_origin,
        right_adjacent_quintant,
    ));
    neighbor_set.insert(serialize_res1(
        right_adjacent_origin,
        (right_adjacent_quintant + 1) % 5,
    ));

    neighbor_set.into_iter().collect()
}

/// Get all neighbors of a cell across quintant and face boundaries.
///
/// Within-quintant candidates are validated with `is_neighbor()` in uv space
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
    if resolution == 1 {
        return get_res1_neighbors(origin, cell.segment, edge_only);
    }

    let hilbert_res = (resolution - FIRST_HILBERT_RESOLUTION + 1) as usize;
    let (source_quintant, source_orientation) = segment_to_quintant(cell.segment, origin);
    let anchor = s_to_anchor(cell.s, hilbert_res, source_orientation);

    // Triple coordinates are orientation-independent
    let triple = anchor_to_triple(&anchor);

    // Get uv anchor for is_neighbor validation (within-quintant)
    let uv_source_anchor = triple_to_anchor(&triple, hilbert_res, Orientation::UV);

    let mut neighbor_set: BTreeSet<u64> = BTreeSet::new();

    // --- Within-quintant: validated by is_neighbor() in uv space ---
    let within_neighbors = find_quintant_neighbor_s(
        &triple,
        uv_source_anchor.as_ref(),
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
    for cell_id in get_boundary_neighbors(&ctx, edge_only) {
        neighbor_set.insert(cell_id);
    }

    neighbor_set.into_iter().collect()
}
