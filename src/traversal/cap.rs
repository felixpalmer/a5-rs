// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use crate::collections::slot_runs::to_covering;
use crate::coordinate_systems::Spherical;
use crate::core::cell::cell_to_spherical;
use crate::core::cell_info::cell_area;
use crate::core::constants::AUTHALIC_RADIUS_EARTH;
use crate::core::face_adjacency::walk_faces;
use crate::core::origin::haversine;
use crate::core::serialization::{
    cell_to_parent, deserialize, get_resolution, serialize, FIRST_HILBERT_RESOLUTION,
};
use crate::core::utils::A5Cell;
use crate::traversal::triple_cells::{
    cell_ids_to_triples, triple_cell_center, triple_cell_to_id, triple_children, walk_triple_cells,
};

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

/// BFS at the cap's coarse resolution (1 or above) from `start_cell` through
/// every cell whose center lies within `h_expanded` of `center`, returning every
/// cell reached: the cells within, plus the ring just outside (the subdivision
/// classifies them).
///
/// Runs in triple space (cells as (origin_id, quintant, x, y, z)): neighbors
/// (edge and vertex) come from the per-flavor triple deltas plus the boundary
/// delta tables, and a cell's center straight from its triple.
fn coarse_cap_cells(
    start_cell: u64,
    center: Spherical,
    h_expanded: f64,
) -> Result<Vec<[i32; 5]>, String> {
    let hilbert_res = (get_resolution(start_cell) - FIRST_HILBERT_RESOLUTION + 1) as usize;
    let max_row = (1i32 << hilbert_res) - 1;
    let mut cells: Vec<[i32; 5]> = Vec::new();
    cell_ids_to_triples([start_cell], &mut cells)?;
    walk_triple_cells(cells.clone(), max_row, |c| {
        cells.push(c);
        Ok(haversine(center, triple_cell_center(c, hilbert_res, max_row)?) <= h_expanded)
    })?;
    Ok(cells)
}

/// Compute all cells within a great-circle radius, returning a compacted result
/// (mix of resolutions), sorted in curve order, then a compaction marker recording
/// the resolution.
///
/// Uses hierarchical BFS: starts at a coarse resolution and recursively
/// subdivides boundary cells, keeping interior cells at coarser resolutions.
/// Only cells whose centers fall within the radius are included.
pub fn spherical_cap(cell_id: u64, radius: f64) -> Result<Vec<u64>, String> {
    let target_res = get_resolution(cell_id);
    let coarse_res = pick_coarse_resolution(radius, target_res);
    let center = cell_to_spherical(cell_id)?;

    // Pre-compute haversine thresholds: the exact radius, and the radius expanded
    // so the coarse BFS captures every overlapping cell
    let h_radius = meters_to_h(radius);
    let h_expanded = meters_to_h(radius + estimate_cell_radius(coarse_res));
    let start_cell = if coarse_res < target_res {
        cell_to_parent(cell_id, Some(coarse_res))?
    } else {
        cell_id
    };
    let mut result: Vec<u64> = Vec::new();

    if coarse_res == 0 {
        // The target is resolution 0: the cells are the 12 dodecahedron faces
        let face_cell = |face: u8| {
            serialize(&A5Cell {
                origin_id: face,
                segment: 0,
                s: 0,
                resolution: 0,
            })
        };
        let near = |face: u8, h: f64| -> Result<bool, String> {
            Ok(haversine(center, cell_to_spherical(face_cell(face)?)?) <= h)
        };
        let seed = deserialize(start_cell)?.origin_id;
        for face in walk_faces(&[seed], |face| near(face, h_expanded), usize::MAX)? {
            if near(face, h_radius)? {
                result.push(face_cell(face)?);
            }
        }
    } else {
        // Recursive subdivision from coarse_res to target_res, in triple space.
        //
        // Each cell is classified by comparing haversine(center, cell) against
        // pre-computed h thresholds:
        // - Interior (h <= h_inner): keep compacted, all descendants inside
        // - Outside  (h > h_outer): discard, no descendants inside
        // - Boundary: subdivide children to next level
        // At the target resolution both thresholds are the exact radius.
        let mut cells = coarse_cap_cells(start_cell, center, h_expanded)?;
        for res in coarse_res..=target_res {
            let hilbert_res = (res - FIRST_HILBERT_RESOLUTION + 1) as usize;
            let max_row = (1i32 << hilbert_res) - 1;
            let cell_radius = estimate_cell_radius(res);
            let last = res == target_res;
            let h_inner = if last {
                h_radius
            } else if radius > cell_radius {
                meters_to_h(radius - cell_radius)
            } else {
                -1.0
            };
            let h_outer = if last {
                h_radius
            } else {
                meters_to_h(radius + cell_radius)
            };
            let mut children: Vec<[i32; 5]> = Vec::new();
            for &cell in &cells {
                let h = haversine(center, triple_cell_center(cell, hilbert_res, max_row)?);
                if h <= h_inner {
                    result.push(triple_cell_to_id(cell, hilbert_res, res)?);
                } else if h <= h_outer {
                    triple_children(cell, max_row, &mut children);
                }
            }
            cells = children;
        }
    }

    to_covering(&result, target_res)
}
