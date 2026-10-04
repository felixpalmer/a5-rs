// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use std::collections::HashSet;

use crate::coordinate_systems::{Face, LonLat};
use crate::core::cell::{cell_intersects_segment, lonlat_to_cell};
use crate::core::coordinate_transforms::{from_lon_lat, to_cartesian, to_lon_lat, to_spherical};
use crate::core::face_adjacency::walk_faces;
use crate::core::origin::get_origins;
use crate::core::serialization::{deserialize, serialize, FIRST_HILBERT_RESOLUTION};
use crate::core::tiling::get_pentagon_vertices;
use crate::core::utils::A5Cell;
use crate::lattice::{triple_flavor, Triple};
use crate::projections::dodecahedron::DodecahedronProjection;
use crate::traversal::cap::estimate_cell_radius;
use crate::traversal::triple_cells::{
    cell_ids_to_triples, for_each_triple_neighbor, triple_cell_key, triple_cell_to_id,
};
use crate::utils::great_circle::sample_great_circle_arc;

/// Resolution 0 version of the sub-segment BFS below: the cells are the 12
/// dodecahedron faces, adjacent across their edges.
fn trace_faces(
    cell_a: u64,
    cell_b: u64,
    a: LonLat,
    b: LonLat,
    mut add_cell: impl FnMut(u64),
) -> Result<(), String> {
    let seeds = [
        deserialize(cell_a)?.origin_id,
        deserialize(cell_b)?.origin_id,
    ];
    walk_faces(
        &seeds,
        |face| {
            let cell = serialize(&A5Cell {
                origin_id: face,
                segment: 0,
                s: 0,
                resolution: 0,
            })?;
            if !cell_intersects_segment(cell, a, b)? {
                return Ok(false);
            }
            add_cell(cell);
            Ok(true)
        },
        usize::MAX,
    )?;
    Ok(())
}

/// Trace cells along a polyline defined by a sequence of waypoints.
///
/// Consecutive waypoints are connected with great-circle arcs. Each arc is
/// sampled at half-cell-radius intervals; for each consecutive pair of samples,
/// a strict local BFS finds every cell whose pentagon is touched by the
/// straight 2D segment between the two samples (projected onto each candidate
/// cell's Face). Cells at waypoint junctions are deduplicated.
///
/// The BFS runs in triple space: a cell's neighbors come from its flavor's
/// triple deltas plus the boundary delta tables, and its pentagon straight from
/// its triple, so a candidate is never decoded and only touched cells are
/// encoded.
///
/// Pass `[start, end]` for a simple two-point line segment.
///
/// Returns unique cell IDs along the polyline, in order.
pub fn line_string_to_cells(waypoints: &[LonLat], resolution: i32) -> Result<Vec<u64>, String> {
    if waypoints.is_empty() {
        return Ok(Vec::new());
    }
    if waypoints.len() == 1 {
        return Ok(vec![lonlat_to_cell(waypoints[0], resolution)?]);
    }

    let mut seen: HashSet<u64> = HashSet::new();
    let mut result: Vec<u64> = Vec::new();
    let cell_radius = estimate_cell_radius(resolution);
    let sample_interval = cell_radius * 0.5;
    let hilbert_res = (resolution - FIRST_HILBERT_RESOLUTION + 1).max(0) as usize;
    let max_row = (1i32 << hilbert_res) - 1;
    let origins = get_origins();
    let dodecahedron = DodecahedronProjection::get_thread_local();

    let mut add_cell = |cell: u64| {
        if seen.insert(cell) {
            result.push(cell);
        }
    };

    // The current sub-segment, projected onto each face it is tested against
    let mut faces: Vec<Option<(Face, Face)>> = vec![None; origins.len()];
    for i in 0..waypoints.len() - 1 {
        let start = waypoints[i];
        let end = waypoints[i + 1];
        let start_vec = to_cartesian(from_lon_lat(start));
        let end_vec = to_cartesian(from_lon_lat(end));

        // Sample the great-circle at half-cell-radius spacing. Endpoints are
        // always included; even for short hops we get the start→end pair.
        let interior = sample_great_circle_arc(start_vec, end_vec, sample_interval);
        let num_subsegments = interior.len() + 1;
        let mut samples: Vec<LonLat> = vec![start; num_subsegments + 1];
        samples[num_subsegments] = end;
        for (j, v) in interior.iter().enumerate() {
            samples[j + 1] = to_lon_lat(to_spherical(*v));
        }
        // Each sample's cell, as its ID and in triple space as (origin_id, quintant, x, y, z)
        let mut sample_cells: Vec<u64> = Vec::with_capacity(samples.len());
        let mut sample_triples: Vec<[i32; 5]> = Vec::with_capacity(samples.len());
        for s in &samples {
            sample_cells.push(lonlat_to_cell(*s, resolution)?);
        }
        if resolution > 0 {
            cell_ids_to_triples(sample_cells.iter().copied(), &mut sample_triples)?;
        }

        // Walk pairwise. Each (P_j, P_{j+1}) sub-segment is short enough that its
        // projection onto any nearby cell's Face is essentially straight, so we
        // can use exact 2D segment-vs-pentagon intersection.
        for j in 0..num_subsegments {
            let a = samples[j];
            let b = samples[j + 1];
            let cell_a = sample_cells[j];
            let cell_b = sample_cells[j + 1];

            add_cell(cell_a);
            add_cell(cell_b);
            if cell_a == cell_b {
                continue;
            }
            if resolution == 0 {
                trace_faces(cell_a, cell_b, a, b, &mut add_cell)?;
                continue;
            }
            faces.fill(None);

            // Strict local BFS: expand neighbors of every cell known to touch this
            // sub-segment, keeping anything whose pentagon the sub-segment crosses.
            // Terminates as soon as no new touching cells are found — typically 1–2
            // hops, since a sub-segment ≤ cell_radius/2 reaches at most a couple of
            // cells beyond its endpoint cells.
            let mut visited: HashSet<i64> = HashSet::new();
            visited.insert(triple_cell_key(sample_triples[j]));
            visited.insert(triple_cell_key(sample_triples[j + 1]));
            let mut frontier: Vec<[i32; 5]> = vec![sample_triples[j], sample_triples[j + 1]];
            while !frontier.is_empty() {
                let mut next: Vec<[i32; 5]> = Vec::new();
                let mut visit = |cell: [i32; 5]| -> Result<(), String> {
                    if !visited.insert(triple_cell_key(cell)) {
                        return Ok(());
                    }
                    let [origin_id, quintant, x, y, z] = cell;
                    let (a_face, b_face) = match faces[origin_id as usize] {
                        Some(projected) => projected,
                        None => {
                            let id = origin_id as u8;
                            let projected = (
                                dodecahedron.forward(from_lon_lat(a), id)?,
                                dodecahedron.forward(from_lon_lat(b), id)?,
                            );
                            faces[origin_id as usize] = Some(projected);
                            projected
                        }
                    };
                    let triple = Triple::new(x, y, z);
                    let pentagon = get_pentagon_vertices(
                        hilbert_res as i32,
                        quintant as usize,
                        &triple,
                        triple_flavor(&triple, max_row),
                    );
                    if pentagon.intersects_segment(a_face, b_face) {
                        add_cell(triple_cell_to_id(cell, hilbert_res, resolution)?);
                        next.push(cell);
                    }
                    Ok(())
                };
                for &cell in &frontier {
                    for_each_triple_neighbor(cell, max_row, false, &mut visit)?;
                }
                frontier = next;
            }
        }
    }

    Ok(result)
}
