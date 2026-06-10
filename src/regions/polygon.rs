// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use std::collections::{HashMap, HashSet};

use crate::coordinate_systems::{Cartesian, LonLat};
use crate::core::cell::{cell_to_spherical, lonlat_to_cell, spherical_to_cell};
use crate::core::compact::compact;
use crate::core::coordinate_transforms::{from_lon_lat, to_cartesian, to_spherical};
use crate::core::serialization::{
    cell_to_children, cell_to_parent, FIRST_HILBERT_RESOLUTION, MAX_RESOLUTION,
};
use crate::geometry::spherical_polygon::{
    point_in_spherical_polygon, ring_segment_normals, ring_winding_sign,
};
use crate::traversal::cap::estimate_cell_radius;
use crate::traversal::lattice_flood_fill::{triple_space_flood_fill, FloodInput};
use crate::traversal::lattice_neighbors::get_lattice_neighbors;
use crate::utils::great_circle::sample_great_circle_arc;

/// Maps each boundary cell to the indices of the ring segments that produced it.
/// Segment indices are global across rings (outer ring first, then holes).
/// Used by `filter_boundary_cells` to short-circuit PIP via segment-side dot products.
type SegmentMap = HashMap<u64, Vec<usize>>;

struct DenseSampleResult {
    boundary_cells: Vec<u64>,
    boundary_set: HashSet<u64>,
    segment_map: SegmentMap,
}

/// Point-in-polygon for a polygon with holes: inside the outer ring and
/// outside every hole ring.
fn point_in_polygon_rings(point: Cartesian, ring_vecs_list: &[Vec<Cartesian>]) -> bool {
    if !point_in_spherical_polygon(point, &ring_vecs_list[0]) {
        return false;
    }
    for ring_vecs in &ring_vecs_list[1..] {
        if point_in_spherical_polygon(point, ring_vecs) {
            return false;
        }
    }
    true
}

/// Dense-sample boundary cells along every closed ring (outer + holes) at
/// `cell_radius * 0.4` spacing, calling `spherical_to_cell` per sample.
fn dense_sample_boundary(
    rings: &[&[LonLat]],
    ring_vecs_list: &[Vec<Cartesian>],
    resolution: i32,
) -> Result<DenseSampleResult, String> {
    let mut boundary_cells: Vec<u64> = Vec::new();
    let mut boundary_set: HashSet<u64> = HashSet::new();
    let mut segment_map: SegmentMap = HashMap::new();
    let cell_radius = estimate_cell_radius(resolution);
    let sample_interval = cell_radius * 0.4;

    let record_cell = |cell: u64,
                       seg_idx: usize,
                       boundary_cells: &mut Vec<u64>,
                       boundary_set: &mut HashSet<u64>,
                       segment_map: &mut SegmentMap| {
        if boundary_set.insert(cell) {
            boundary_cells.push(cell);
        }
        let entry = segment_map.entry(cell).or_default();
        if entry.last() != Some(&seg_idx) {
            entry.push(seg_idx);
        }
    };

    let mut seg_offset = 0;
    for (r, ring) in rings.iter().enumerate() {
        let ring_vecs = &ring_vecs_list[r];
        let n = ring.len();

        let mut vertex_cells: Vec<u64> = Vec::with_capacity(n);
        for v in ring.iter() {
            vertex_cells.push(lonlat_to_cell(*v, resolution)?);
        }

        for i in 0..n {
            let next_i = (i + 1) % n;
            record_cell(
                vertex_cells[i],
                seg_offset + i,
                &mut boundary_cells,
                &mut boundary_set,
                &mut segment_map,
            );

            // Skip the lonLat round-trip: samples are authalic-Cartesian already.
            let samples = sample_great_circle_arc(ring_vecs[i], ring_vecs[next_i], sample_interval);
            for s in samples {
                let cell = spherical_to_cell(to_spherical(s), resolution)?;
                record_cell(
                    cell,
                    seg_offset + i,
                    &mut boundary_cells,
                    &mut boundary_set,
                    &mut segment_map,
                );
            }
            record_cell(
                vertex_cells[next_i],
                seg_offset + i,
                &mut boundary_cells,
                &mut boundary_set,
                &mut segment_map,
            );
        }
        seg_offset += n;
    }

    Ok(DenseSampleResult {
        boundary_cells,
        boundary_set,
        segment_map,
    })
}

/// Filter boundary cells to those whose center is inside the polygon.
///
/// For each cell we know which ring segment(s) sampled it. When all of those
/// segments place the cell on the interior side (cheap signed-dot test), we
/// accept immediately. When they disagree (vertex / concave corner) or the
/// cell wasn't recorded, fall back to full PIP.
fn filter_boundary_cells(
    boundary_cells: &[u64],
    segment_map: &SegmentMap,
    seg_normals: &[Cartesian],
    seg_signs: &[f64],
    ring_vecs_list: &[Vec<Cartesian>],
) -> Result<Vec<u64>, String> {
    let mut out: Vec<u64> = Vec::new();
    for &cell in boundary_cells {
        let cv = to_cartesian(cell_to_spherical(cell)?);
        let segments = match segment_map.get(&cell) {
            Some(s) => s,
            None => {
                if point_in_polygon_rings(cv, ring_vecs_list) {
                    out.push(cell);
                }
                continue;
            }
        };
        let mut all_inside = true;
        let mut any_inside = false;
        let mut ambiguous = false;
        for &seg_idx in segments {
            let n = seg_normals[seg_idx];
            let dot = n.x() * cv.x() + n.y() * cv.y() + n.z() * cv.z();
            if dot.abs() < 1e-14 {
                ambiguous = true;
                break;
            }
            if dot * seg_signs[seg_idx] > 0.0 {
                any_inside = true;
            } else {
                all_inside = false;
            }
        }
        if ambiguous || (any_inside && !all_inside) {
            if point_in_polygon_rings(cv, ring_vecs_list) {
                out.push(cell);
            }
        } else if all_inside {
            out.push(cell);
        }
    }
    Ok(out)
}

/// Buffer the boundary by one cell using 3-edge lattice neighbors. The shell
/// matches the connectivity of `triple_space_flood_fill` so the firewall (boundary
/// + exterior shell) is a tight topological barrier for the subsequent flood.
fn expand_shell(boundary_cells: &[u64], boundary_set: &HashSet<u64>) -> Vec<u64> {
    let mut shell_cells: Vec<u64> = Vec::new();
    let mut shell_set: HashSet<u64> = HashSet::new();
    for &cell in boundary_cells {
        for neighbor in get_lattice_neighbors(cell, true) {
            if boundary_set.contains(&neighbor) {
                continue;
            }
            if shell_set.insert(neighbor) {
                shell_cells.push(neighbor);
            }
        }
    }
    shell_cells
}

/// Hierarchical flood fill from interior seed cells. Runs a few fine BFS layers
/// to clear the boundary, then a coarse-resolution BFS through the bulk, then
/// resumes fine BFS to fill gaps near the boundary. The coarse phase is skipped
/// when the polygon is too small to amortize its setup overhead.
fn flood_interior(
    interior_seeds: &[u64],
    visited: &mut HashSet<u64>,
    boundary_size: usize,
    resolution: i32,
) -> Result<Vec<u64>, String> {
    for &cell in interior_seeds {
        visited.insert(cell);
    }

    // Isoperimetric bound: B² / (4π) is the max interior for B boundary cells.
    let max_interior =
        (boundary_size as f64) * (boundary_size as f64) / (4.0 * std::f64::consts::PI);
    // res 30 has a different encoding the parent-emit optimization can't use.
    let use_coarse_phase = resolution > FIRST_HILBERT_RESOLUTION
        && resolution < MAX_RESOLUTION
        && max_interior > 1000.0;

    if !use_coarse_phase {
        let result = triple_space_flood_fill(
            FloodInput::Firewall(visited),
            interior_seeds,
            resolution,
            None,
        );
        let mut out: Vec<u64> =
            Vec::with_capacity(interior_seeds.len() + result.interior_cells.len());
        out.extend_from_slice(interior_seeds);
        out.extend(result.interior_cells);
        return Ok(out);
    }

    let parent_res = resolution - 1;
    let mut coarse_firewall: HashSet<u64> = HashSet::new();
    for &cell in visited.iter() {
        coarse_firewall.insert(cell_to_parent(cell, Some(parent_res))?);
    }

    // Phase 1: short fine BFS to move the frontier off the boundary.
    let phase1 = triple_space_flood_fill(
        FloodInput::Firewall(visited),
        interior_seeds,
        resolution,
        Some(3),
    );

    // Phase 2: coarse BFS through the bulk interior.
    let mut coarse_interior_set: Option<HashSet<u64>> = None;
    let mut phase3_delta: Vec<u64> = Vec::new();
    let mut coarse_interior_cells: Vec<u64> = Vec::new();
    if !phase1.frontier_cell_ids.is_empty() {
        let mut coarse_seeds: HashSet<u64> = HashSet::new();
        for &cell in &phase1.frontier_cell_ids {
            let parent = cell_to_parent(cell, Some(parent_res))?;
            if !coarse_firewall.contains(&parent) {
                coarse_seeds.insert(parent);
            }
        }

        if !coarse_seeds.is_empty() {
            let mut coarse_visited: HashSet<u64> = coarse_firewall.clone();
            for &seed in &coarse_seeds {
                coarse_visited.insert(seed);
            }
            let coarse_seed_vec: Vec<u64> = coarse_seeds.iter().copied().collect();
            let coarse_result = triple_space_flood_fill(
                FloodInput::Firewall(&mut coarse_visited),
                &coarse_seed_vec,
                parent_res,
                None,
            );
            let mut coarse_interior: Vec<u64> =
                Vec::with_capacity(coarse_seed_vec.len() + coarse_result.interior_cells.len());
            coarse_interior.extend(coarse_seed_vec);
            coarse_interior.extend(coarse_result.interior_cells);
            let mut coarse_set: HashSet<u64> = HashSet::with_capacity(coarse_interior.len());
            for &c in &coarse_interior {
                coarse_set.insert(c);
            }
            coarse_interior_cells.extend(coarse_interior.iter().copied());
            coarse_interior_set = Some(coarse_set);

            // Children become firewall for phase 3; the coarse parent represents
            // them in the output, so we don't emit them individually.
            for coarse_cell in coarse_interior {
                for child in cell_to_children(coarse_cell, Some(resolution))? {
                    if !visited.contains(&child) {
                        visited.insert(child);
                        phase3_delta.push(child);
                    }
                }
            }
        }
    }

    // Emit fine cells only when not already covered by a coarse parent.
    let mut interior_cells: Vec<u64> = Vec::new();
    if coarse_interior_set.is_none() {
        interior_cells.extend_from_slice(interior_seeds);
        interior_cells.extend(phase1.interior_cells.iter().copied());
    } else {
        let coarse_set = coarse_interior_set.as_ref().unwrap();
        for &cell in interior_seeds {
            let parent = cell_to_parent(cell, Some(parent_res))?;
            if !coarse_set.contains(&parent) {
                interior_cells.push(cell);
            }
        }
        for &cell in &phase1.interior_cells {
            let parent = cell_to_parent(cell, Some(parent_res))?;
            if !coarse_set.contains(&parent) {
                interior_cells.push(cell);
            }
        }
        interior_cells.extend(coarse_interior_cells);
    }

    // Phase 3: resume fine BFS, reusing phase 1's packed state.
    let phase3 = triple_space_flood_fill(
        FloodInput::Reuse {
            state: phase1.state,
            delta: phase3_delta,
        },
        &phase1.frontier_cell_ids,
        resolution,
        None,
    );
    interior_cells.extend(phase3.interior_cells);

    Ok(interior_cells)
}

/// Find all cells within a polygon using center-point containment: a cell is
/// included iff its center lies inside the polygon. The result is compacted —
/// use `uncompact` to expand to the input resolution.
///
/// `polygon` is GeoJSON-style rings `[outer, ...holes]` of `[longitude, latitude]`
/// vertices; cells inside a hole are excluded. Rings may be open or closed
/// (GeoJSON-style, first vertex repeated at the end) — closure is automatic
/// either way. Holes with fewer than 3 distinct vertices are ignored.
/// Returns sorted, compacted cell IDs whose centers lie inside the polygon.
pub fn polygon_to_cells(polygon: &[Vec<LonLat>], resolution: i32) -> Result<Vec<u64>, String> {
    // GeoJSON rings repeat the first vertex at the end — drop the duplicate.
    fn strip_closing(ring: &[LonLat]) -> &[LonLat] {
        if ring.len() > 1 && ring[0] == ring[ring.len() - 1] {
            &ring[..ring.len() - 1]
        } else {
            ring
        }
    }

    if polygon.is_empty() {
        return Ok(Vec::new());
    }
    let outer = strip_closing(&polygon[0]);
    if outer.len() < 3 {
        return Ok(Vec::new());
    }
    let mut rings: Vec<&[LonLat]> = vec![outer];
    for hole in &polygon[1..] {
        let hole = strip_closing(hole);
        if hole.len() >= 3 {
            rings.push(hole);
        }
    }

    // Authalic-sphere ring vectors — A5's internal sphere, so cell centers
    // compare directly with no geodetic↔authalic round-trip.
    let mut ring_vecs_list: Vec<Vec<Cartesian>> = Vec::with_capacity(rings.len());
    for ring in &rings {
        let mut ring_vecs: Vec<Cartesian> = Vec::with_capacity(ring.len());
        for v in *ring {
            ring_vecs.push(to_cartesian(from_lon_lat(*v)));
        }
        ring_vecs_list.push(ring_vecs);
    }

    let DenseSampleResult {
        boundary_cells,
        boundary_set,
        segment_map,
    } = dense_sample_boundary(&rings, &ring_vecs_list, resolution)?;

    // Flattened per-segment normals and interior-side signs, indexed like the
    // segment map. The polygon interior lies on the *outside* of a hole ring,
    // so hole segments get the opposite sign.
    let mut seg_normals: Vec<Cartesian> = Vec::new();
    let mut seg_signs: Vec<f64> = Vec::new();
    for (r, ring_vecs) in ring_vecs_list.iter().enumerate() {
        let sign = (if r == 0 { 1 } else { -1 }) * ring_winding_sign(ring_vecs);
        for normal in ring_segment_normals(ring_vecs) {
            seg_normals.push(normal);
            seg_signs.push(sign as f64);
        }
    }

    let filtered_boundary = filter_boundary_cells(
        &boundary_cells,
        &segment_map,
        &seg_normals,
        &seg_signs,
        &ring_vecs_list,
    )?;

    // Dense sampling can leave gaps; the shell catches them, classifying each cell.
    let shell_cells = expand_shell(&boundary_cells, &boundary_set);
    if shell_cells.is_empty() {
        return compact(&filtered_boundary);
    }

    let mut interior_seeds: Vec<u64> = Vec::new();
    let mut visited: HashSet<u64> = boundary_set.clone();
    for cell in shell_cells {
        let cv = to_cartesian(cell_to_spherical(cell)?);
        if point_in_polygon_rings(cv, &ring_vecs_list) {
            interior_seeds.push(cell);
        } else {
            visited.insert(cell); // exterior shell (and hole interiors) join the firewall
        }
    }
    if interior_seeds.is_empty() {
        return compact(&filtered_boundary);
    }

    let interior_cells = flood_interior(
        &interior_seeds,
        &mut visited,
        boundary_set.len(),
        resolution,
    )?;

    let mut combined: Vec<u64> = Vec::with_capacity(filtered_boundary.len() + interior_cells.len());
    combined.extend(filtered_boundary);
    combined.extend(interior_cells);
    compact(&combined)
}
