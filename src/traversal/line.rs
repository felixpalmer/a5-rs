// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use std::collections::HashSet;

use crate::coordinate_systems::LonLat;
use crate::core::cell::{cell_intersects_segment, lonlat_to_cell};
use crate::core::coordinate_transforms::{from_lon_lat, to_cartesian, to_lon_lat, to_spherical};
use crate::traversal::cap::estimate_cell_radius;
use crate::traversal::lattice_neighbors::get_lattice_neighbors;
use crate::utils::great_circle::sample_great_circle_arc;

/// Trace cells along a polyline defined by a sequence of waypoints.
///
/// Consecutive waypoints are connected with great-circle arcs. Each arc is
/// sampled at half-cell-radius intervals; for each consecutive pair of samples,
/// a strict local BFS finds every cell whose pentagon is touched by the
/// straight 2D segment between the two samples (projected onto each candidate
/// cell's Face). Cells at waypoint junctions are deduplicated.
///
/// Pass `[start, end]` for a simple two-point line segment.
///
/// Returns a vector of unique cell IDs along the polyline, in order.
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

    let add_cell = |cell: u64, seen: &mut HashSet<u64>, result: &mut Vec<u64>| {
        if seen.insert(cell) {
            result.push(cell);
        }
    };

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
        let mut sample_cells: Vec<u64> = Vec::with_capacity(samples.len());
        for s in &samples {
            sample_cells.push(lonlat_to_cell(*s, resolution)?);
        }

        // Walk pairwise. Each (P_j, P_{j+1}) sub-segment is short enough that its
        // projection onto any nearby cell's Face is essentially straight, so we
        // can use exact 2D segment-vs-pentagon intersection.
        for j in 0..num_subsegments {
            let a = samples[j];
            let b = samples[j + 1];
            let cell_a = sample_cells[j];
            let cell_b = sample_cells[j + 1];

            add_cell(cell_a, &mut seen, &mut result);
            add_cell(cell_b, &mut seen, &mut result);
            if cell_a == cell_b {
                continue;
            }

            // Strict local BFS: expand neighbors of every cell known to touch this
            // sub-segment, keeping anything whose pentagon the sub-segment crosses.
            // Terminates as soon as no new touching cells are found — typically 1–2
            // hops, since a sub-segment ≤ cell_radius/2 reaches at most a couple of
            // cells beyond its endpoint cells.
            let mut visited: HashSet<u64> = HashSet::new();
            visited.insert(cell_a);
            visited.insert(cell_b);
            let mut frontier: Vec<u64> = vec![cell_a, cell_b];
            while !frontier.is_empty() {
                let mut next: Vec<u64> = Vec::new();
                for cell in &frontier {
                    for neighbor in get_lattice_neighbors(*cell, false) {
                        if !visited.insert(neighbor) {
                            continue;
                        }
                        if cell_intersects_segment(neighbor, a, b)? {
                            add_cell(neighbor, &mut seen, &mut result);
                            next.push(neighbor);
                        }
                    }
                }
                frontier = next;
            }
        }
    }

    Ok(result)
}
