// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use crate::core::cell::cell_to_spherical;
use crate::core::cell_info::cell_area;
use crate::core::constants::AUTHALIC_RADIUS_EARTH;
use crate::core::origin::haversine;
use crate::core::serialization::{
    cell_to_children, cell_to_parent, get_resolution, FIRST_HILBERT_RESOLUTION,
};
use crate::traversal::global_neighbors::get_global_cell_neighbors;
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
    let cap_area_m2 = 2.0
        * std::f64::consts::PI
        * AUTHALIC_RADIUS_EARTH
        * AUTHALIC_RADIUS_EARTH
        * (1.0 - (radius / AUTHALIC_RADIUS_EARTH).cos());

    for res in FIRST_HILBERT_RESOLUTION..=target_res {
        let c_area = cell_area(res);
        if cap_area_m2 / c_area >= MIN_CELLS_FOR_SUBDIVISION {
            return res;
        }
    }
    target_res // No coarsening benefit
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
    let mut coarse_visited: HashSet<u64> = HashSet::new();
    coarse_visited.insert(start_cell);
    let mut coarse_frontier: HashSet<u64> = HashSet::new();
    coarse_frontier.insert(start_cell);

    while !coarse_frontier.is_empty() {
        let mut next_frontier: HashSet<u64> = HashSet::new();
        for &cid in &coarse_frontier {
            for neighbor in get_global_cell_neighbors(cid, false) {
                if coarse_visited.contains(&neighbor) {
                    continue;
                }
                coarse_visited.insert(neighbor);
                if haversine(center, cell_to_spherical(neighbor)?) <= h_expanded {
                    next_frontier.insert(neighbor);
                }
            }
        }
        coarse_frontier = next_frontier;
    }

    // Recursive subdivision from coarseRes to targetRes.
    let mut result: Vec<u64> = Vec::new();
    let mut boundary: Vec<u64> = coarse_visited.into_iter().collect();

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
