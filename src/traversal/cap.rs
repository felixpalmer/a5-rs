// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use crate::coordinate_systems::Spherical;
use crate::core::cell::cell_to_spherical;
use crate::core::cell_info::cell_area;
use crate::core::constants::AUTHALIC_RADIUS_EARTH;
use crate::core::face_adjacency::FACE_ADJACENCY;
use crate::core::origin::{get_origins, haversine, segment_to_quintant};
use crate::core::serialization::{
    cell_to_children, cell_to_parent, deserialize, get_resolution, serialize,
    FIRST_HILBERT_RESOLUTION,
};
use crate::core::tiling::get_pentagon_center;
use crate::core::utils::A5Cell;
use crate::lattice::{s_to_triple, triple_flavor, Triple};
use crate::projections::dodecahedron::DodecahedronProjection;
use crate::traversal::triple_cells::{
    for_each_triple_neighbor, triple_cell_key, triple_cell_to_id,
};
use std::collections::HashSet;

/// Safety factor applied to equal-area circle radius to get conservative circumradius estimate
const CELL_RADIUS_SAFETY_FACTOR: f64 = 2.0;

/// Minimum cells in the cap before hierarchical subdivision is worthwhile
const MIN_CELLS_FOR_SUBDIVISION: f64 = 20.0;

// Pre-compute cell radii.
//
// Derived from: cellRadius = SAFETY * sqrt(cellArea / PI)
//             = SAFETY * sqrt(4*PI*R² / (numCells * PI))
//             = SAFETY * 2R / sqrt(numCells)
//
// For r >= 1: numCells = 60 * 4^(r-1), so sqrt(numCells) = 2*sqrt(15) * 2^(r-1)
// giving: cellRadius(r) = BASE / 2^(r-1) — halves at each resolution level.
lazy_static::lazy_static! {
    static ref CELL_RADIUS: Vec<f64> = {
        let base = CELL_RADIUS_SAFETY_FACTOR * AUTHALIC_RADIUS_EARTH / 15_f64.sqrt();
        let mut radii = Vec::with_capacity(31);
        radii.push(CELL_RADIUS_SAFETY_FACTOR * AUTHALIC_RADIUS_EARTH / 3_f64.sqrt());
        for r in 1..31 {
            radii.push(base / (1_u64 << (r - 1)) as f64);
        }
        radii
    };
}

/// Convert a distance in meters to a haversine threshold value.
/// Since haversine h = sin^2(d/2R) is monotonic in d for d in [0, piR],
/// comparing h <= threshold is equivalent to comparing dist <= radius
/// but avoids the asin/sqrt per point.
pub fn meters_to_h(meters: f64) -> f64 {
    let s = (meters / (2.0 * AUTHALIC_RADIUS_EARTH)).sin();
    s * s
}

/// Estimate a conservative cell circumradius in meters for a given resolution.
pub fn estimate_cell_radius(resolution: i32) -> f64 {
    CELL_RADIUS[resolution as usize]
}

/// Pick the coarsest resolution where the cap contains enough cells
/// to make hierarchical subdivision worthwhile.
pub fn pick_coarse_resolution(radius: f64, target_res: i32) -> i32 {
    // Spherical cap area in m²: 2πR²(1 − cos(r/R)) computed as 4πR²·sin²(r/2R),
    // which keeps full precision for small radii where 1 − cos cancels
    let half_angle_sin = (radius / (2.0 * AUTHALIC_RADIUS_EARTH)).sin();
    let cap_area_m2 = 4.0
        * std::f64::consts::PI
        * AUTHALIC_RADIUS_EARTH
        * AUTHALIC_RADIUS_EARTH
        * half_angle_sin
        * half_angle_sin;

    for res in FIRST_HILBERT_RESOLUTION..=target_res {
        let c_area = cell_area(res);
        if cap_area_m2 / c_area >= MIN_CELLS_FOR_SUBDIVISION {
            return res;
        }
    }
    target_res // No coarsening benefit
}

/// BFS at the cap's coarse resolution from `start_cell` through every cell whose
/// center lies within `h_expanded` of `center`, returning every cell reached: the
/// cells within, plus the ring just outside (the subdivision classifies them).
///
/// Runs in triple space: neighbors (edge and vertex) come from the per-flavor
/// triple deltas plus the boundary delta tables, and a cell's center straight
/// from its triple, so no cell is decoded and each is encoded once.
fn coarse_cap_cells(
    start_cell: u64,
    center: Spherical,
    h_expanded: f64,
) -> Result<Vec<u64>, String> {
    let cell = deserialize(start_cell)?;
    let origins = get_origins();
    if cell.resolution == 0 {
        // The cells are the 12 dodecahedron faces, adjacent across their edges
        let face_cell = |id: u8| {
            serialize(&A5Cell {
                origin_id: id,
                segment: 0,
                s: 0,
                resolution: 0,
            })
        };
        let mut visited: Vec<u8> = vec![cell.origin_id];
        let mut frontier: Vec<u8> = vec![cell.origin_id];
        while !frontier.is_empty() {
            let mut next: Vec<u8> = Vec::new();
            for &id in &frontier {
                for q in 0..5 {
                    let face = FACE_ADJACENCY[id as usize][q].0;
                    if visited.contains(&face) {
                        continue;
                    }
                    visited.push(face);
                    if haversine(center, cell_to_spherical(face_cell(face)?)?) <= h_expanded {
                        next.push(face);
                    }
                }
            }
            frontier = next;
        }
        return visited.into_iter().map(face_cell).collect();
    }

    let origin = &origins[cell.origin_id as usize];
    let hilbert_res = (cell.resolution - FIRST_HILBERT_RESOLUTION + 1) as usize;
    let max_row = (1i32 << hilbert_res) - 1;
    let (quintant, orientation) = segment_to_quintant(cell.segment, origin);
    let seed = s_to_triple(cell.s, hilbert_res, orientation);
    let seed_cell = [origin.id as i32, quintant as i32, seed.x, seed.y, seed.z];
    let mut visited: HashSet<i64> = HashSet::from([triple_cell_key(seed_cell)]);
    let mut cells: Vec<u64> = vec![start_cell];
    let mut frontier: Vec<[i32; 5]> = vec![seed_cell];
    let dodecahedron = DodecahedronProjection::get_thread_local();

    while !frontier.is_empty() {
        let mut next: Vec<[i32; 5]> = Vec::new();
        let mut visit = |c: [i32; 5]| -> Result<(), String> {
            if !visited.insert(triple_cell_key(c)) {
                return Ok(());
            }
            cells.push(triple_cell_to_id(c, hilbert_res, cell.resolution)?);
            let triple = Triple::new(c[2], c[3], c[4]);
            let face = get_pentagon_center(
                hilbert_res as i32,
                c[1] as usize,
                &triple,
                triple_flavor(&triple, max_row),
            );
            if haversine(center, dodecahedron.inverse(face, c[0] as u8)?) <= h_expanded {
                next.push(c);
            }
            Ok(())
        };
        for &cell in &frontier {
            for_each_triple_neighbor(cell, max_row, false, &mut visit)?;
        }
        frontier = next;
    }
    Ok(cells)
}

/// Compute all cells within a great-circle radius, returning a naturally
/// compacted result (mix of resolutions).
///
/// Uses hierarchical BFS: starts at a coarse resolution and recursively
/// subdivides boundary cells, keeping interior cells at coarser resolutions.
/// Only cells whose centers fall within the radius are included.
pub fn spherical_cap(cell_id: u64, radius: f64) -> Result<Vec<u64>, String> {
    let target_res = get_resolution(cell_id);
    let coarse_res = pick_coarse_resolution(radius, target_res);
    let center = cell_to_spherical(cell_id)?;

    // Pre-compute haversine threshold for the exact radius
    let h_radius = meters_to_h(radius);

    // BFS at coarse resolution with expanded radius to capture all overlapping cells.
    let start_cell = if coarse_res < target_res {
        cell_to_parent(cell_id, Some(coarse_res))?
    } else {
        cell_id
    };
    let coarse_cell_radius = estimate_cell_radius(coarse_res);
    let h_expanded = meters_to_h(radius + coarse_cell_radius);
    let coarse_cells = coarse_cap_cells(start_cell, center, h_expanded)?;

    // Recursive subdivision from coarseRes to targetRes.
    let mut result: Vec<u64> = Vec::new();
    let mut boundary: Vec<u64> = coarse_cells;

    for res in coarse_res..target_res {
        let cell_radius_val = estimate_cell_radius(res);
        let h_inner = if radius > cell_radius_val {
            meters_to_h(radius - cell_radius_val)
        } else {
            -1.0
        };
        let h_outer = meters_to_h(radius + cell_radius_val);
        let mut next_boundary: Vec<u64> = Vec::new();

        for &cell in &boundary {
            let h = haversine(center, cell_to_spherical(cell)?);
            if h <= h_inner {
                result.push(cell);
            } else if h > h_outer {
                // Cell's entire extent is outside the cap -- discard
            } else {
                for child in cell_to_children(cell, Some(res + 1))? {
                    next_boundary.push(child);
                }
            }
        }

        boundary = next_boundary;
    }

    // Final target resolution: strict haversine check
    for &cell in &boundary {
        if haversine(center, cell_to_spherical(cell)?) <= h_radius {
            result.push(cell);
        }
    }

    result.sort();
    Ok(result)
}
