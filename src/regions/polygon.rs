// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use std::collections::{HashMap, HashSet};

use crate::coordinate_systems::{Cartesian, LonLat};
use crate::core::cell::{cell_to_spherical, lonlat_to_cell, spherical_to_cell};
use crate::core::compact::compact;
use crate::core::coordinate_transforms::{from_lon_lat, to_cartesian, to_spherical};
use crate::core::serialization::{
    cell_to_children, deserialize, get_resolution, serialize, FIRST_HILBERT_RESOLUTION,
    MAX_RESOLUTION, WORLD_CELL,
};
use crate::core::utils::A5Cell;
use crate::geometry::prepared_polygon::{
    point_in_prepared_polygon, prepare_polygon, PreparedPolygon,
};
use crate::geometry::spherical_polygon::ring_winding_sign;
use crate::traversal::cap::estimate_cell_radius;
use crate::traversal::lattice_flood_fill::{triple_space_flood_fill, FloodInput};
use crate::traversal::triple_cells::{
    cell_ids_to_triples, for_each_lattice_neighbor, triple_cell_center, triple_cell_key,
    triple_cells_to_ids, triple_children, triple_parent,
};
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
    prep: &PreparedPolygon,
) -> Result<Vec<u64>, String> {
    let mut out: Vec<u64> = Vec::new();
    for &cell in boundary_cells {
        let cv = to_cartesian(cell_to_spherical(cell)?);
        let segments = match segment_map.get(&cell) {
            Some(s) => s,
            None => {
                if point_in_prepared_polygon(cv, prep) {
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
            if point_in_prepared_polygon(cv, prep) {
                out.push(cell);
            }
        } else if all_inside {
            out.push(cell);
        }
    }
    Ok(out)
}

/// Buffer the boundary by one cell using lattice neighbors, in triple space
/// (cells as (origin_id, quintant, x, y, z)). The shell matches the
/// connectivity of `triple_space_flood_fill` so the firewall (boundary + exterior
/// shell) is a tight topological barrier for the subsequent flood.
fn expand_shell(boundary: &[[i32; 5]], max_row: i32) -> Result<Vec<[i32; 5]>, String> {
    let mut seen: HashSet<i64> = boundary.iter().map(|&c| triple_cell_key(c)).collect();
    let mut shell: Vec<[i32; 5]> = Vec::new();
    for &cell in boundary {
        for_each_lattice_neighbor(cell, max_row, |n| {
            if seen.insert(triple_cell_key(n)) {
                shell.push(n);
            }
            Ok(())
        })?;
    }
    Ok(shell)
}

/// Hierarchical flood fill from interior seed cells. Runs a few fine BFS layers
/// to clear the boundary, then a coarse-resolution BFS through the bulk, then
/// resumes fine BFS to fill gaps near the boundary. The coarse phase is skipped
/// when the polygon is too small to amortize its setup overhead.
///
/// All in triple space (cells as (origin_id, quintant, x, y, z)), moving between
/// resolutions with `triple_parent` / `triple_children`; only the cells emitted
/// are encoded.
fn flood_interior(
    seeds: &[[i32; 5]],
    boundary: &[[i32; 5]],
    exterior_shell: &[[i32; 5]],
    resolution: i32,
) -> Result<Vec<u64>, String> {
    let hilbert_res = (resolution - FIRST_HILBERT_RESOLUTION + 1) as usize;
    let firewall: Vec<[i32; 5]> = boundary.iter().chain(exterior_shell).copied().collect();

    // Isoperimetric bound: B² / (4π) is the max interior for B boundary cells.
    let boundary_size = boundary.len() as f64;
    let max_interior = boundary_size * boundary_size / (4.0 * std::f64::consts::PI);
    // res 30 has a different encoding the parent-emit optimization can't use.
    let use_coarse_phase = resolution > FIRST_HILBERT_RESOLUTION
        && resolution < MAX_RESOLUTION
        && max_interior > 1000.0;

    let mut out: Vec<u64> = Vec::new();
    if !use_coarse_phase {
        let result =
            triple_space_flood_fill(FloodInput::Firewall(&firewall), seeds, resolution, None);
        triple_cells_to_ids(seeds, hilbert_res, resolution, &mut out)?;
        triple_cells_to_ids(&result.interior, hilbert_res, resolution, &mut out)?;
        return Ok(out);
    }

    let parent_max_row = (1i32 << (hilbert_res - 1)) - 1;
    let parent = |cell: &[i32; 5]| triple_parent(*cell, parent_max_row);
    let coarse_firewall: Vec<[i32; 5]> = firewall.iter().chain(seeds).map(parent).collect();

    // Phase 1: short fine BFS to move the frontier off the boundary.
    let phase1 =
        triple_space_flood_fill(FloodInput::Firewall(&firewall), seeds, resolution, Some(3));

    // Phase 2: coarse BFS through the bulk interior, seeded by the parents of the
    // phase 1 frontier that aren't firewall parents.
    let mut seen: HashSet<i64> = coarse_firewall
        .iter()
        .map(|&c| triple_cell_key(c))
        .collect();
    let coarse_seeds: Vec<[i32; 5]> = phase1
        .frontier
        .iter()
        .map(parent)
        .filter(|&c| seen.insert(triple_cell_key(c)))
        .collect();
    let mut coarse_interior: Vec<[i32; 5]> = Vec::new();
    let mut phase3_delta: Vec<[i32; 5]> = Vec::new();
    if !coarse_seeds.is_empty() {
        let coarse_result = triple_space_flood_fill(
            FloodInput::Firewall(&coarse_firewall),
            &coarse_seeds,
            resolution - 1,
            None,
        );
        coarse_interior = coarse_seeds;
        coarse_interior.extend(coarse_result.interior);
        // Children become firewall for phase 3; the coarse parent represents
        // them in the output, so we don't emit them individually.
        for &cell in &coarse_interior {
            triple_children(cell, parent_max_row, &mut phase3_delta);
        }
    }

    // Phase 3: resume fine BFS, reusing phase 1's state.
    let phase3 = triple_space_flood_fill(
        FloodInput::Reuse {
            state: phase1.state,
            delta: phase3_delta,
        },
        &phase1.frontier,
        resolution,
        None,
    );

    // Emit fine cells only when not already covered by a coarse parent.
    let covered: HashSet<i64> = coarse_interior
        .iter()
        .map(|&c| triple_cell_key(c))
        .collect();
    let emitted: Vec<[i32; 5]> = seeds
        .iter()
        .chain(&phase1.interior)
        .filter(|cell| !covered.contains(&triple_cell_key(parent(cell))))
        .copied()
        .collect();
    triple_cells_to_ids(&emitted, hilbert_res, resolution, &mut out)?;
    triple_cells_to_ids(&phase3.interior, hilbert_res, resolution, &mut out)?;
    triple_cells_to_ids(&coarse_interior, hilbert_res - 1, resolution - 1, &mut out)?;
    Ok(out)
}

/// Quintants the polygon swallows whole. The flood fill never crosses a
/// quintant edge, so such a quintant gets no seeds from the boundary shell and
/// would be left empty. A quintant holding none of the boundary or shell cells
/// has none of the polygon's edge passing through it: its cells lie wholly
/// inside or wholly outside, and a single probe cell decides which. Inside
/// quintants are emitted as their resolution 1 cell (resolution 0 when that is
/// the target), which `compact` merges with the rest of the output.
fn swallowed_quintants(
    boundary: &[[i32; 5]],
    shell: &[[i32; 5]],
    resolution: i32,
    prep: &PreparedPolygon,
) -> Result<Vec<u64>, String> {
    // A swallowed quintant lies inside the polygon's bounding cap, so the cap
    // must have at least a quintant's area (4π/60: cells are equal-area)
    let pi = std::f64::consts::PI;
    if 2.0 * pi * (1.0 - prep.cap.min_dot) < (4.0 * pi) / 60.0 {
        return Ok(Vec::new());
    }
    // Quintants by origin.id * 5 + quintant, as the triples carry them
    let touched: HashSet<i32> = boundary
        .iter()
        .chain(shell)
        .map(|c| c[0] * 5 + c[1])
        .collect();

    let mut out: Vec<u64> = Vec::new();
    let quintant_cells = cell_to_children(WORLD_CELL, Some(FIRST_HILBERT_RESOLUTION - 1))?;
    let mut quintants: Vec<[i32; 5]> = Vec::with_capacity(quintant_cells.len());
    cell_ids_to_triples(quintant_cells.iter().copied(), &mut quintants)?;
    for (&quintant_cell, q) in quintant_cells.iter().zip(&quintants) {
        if touched.contains(&(q[0] * 5 + q[1])) {
            continue;
        }
        // Any cell of the quintant at the target resolution will do
        let probe = serialize(&A5Cell {
            s: 0,
            resolution,
            ..deserialize(quintant_cell)?
        })?;
        if point_in_prepared_polygon(to_cartesian(cell_to_spherical(probe)?), prep) {
            out.push(quintant_cell);
        }
    }
    Ok(out)
}

/// How a cell is judged to belong to the polygon.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Containment {
    /// Include a cell iff its center lies inside the polygon.
    #[default]
    Center,
    /// Additionally include every cell that overlaps the polygon boundary,
    /// giving gap-free coverage (a superset of [`Containment::Center`]).
    Overlapping,
}

/// Options for [`polygon_to_cells`].
#[derive(Debug, Clone, Copy, Default)]
pub struct PolygonToCellsOptions {
    /// Which cells to include relative to the polygon. Defaults to
    /// [`Containment::Center`].
    pub containment: Containment,
}

/// Find all cells within a polygon. The result is compacted — use `uncompact`
/// to expand to the input resolution.
///
/// `polygon` is GeoJSON-style rings `[outer, ...holes]` of `[longitude, latitude]`
/// vertices; cells inside a hole are excluded. Rings may be open or closed
/// (GeoJSON-style, first vertex repeated at the end) — closure is automatic
/// either way. Holes with fewer than 3 distinct vertices are ignored.
///
/// Pass `None` for `options` to use the defaults. `options.containment` selects
/// [`Containment::Center`] (default, cell center inside the polygon) or
/// [`Containment::Overlapping`] (any cell touching the polygon, for gap-free
/// coverage). Returns sorted, compacted cell IDs.
pub fn polygon_to_cells(
    polygon: &[Vec<LonLat>],
    resolution: i32,
    options: Option<PolygonToCellsOptions>,
) -> Result<Vec<u64>, String> {
    let options = options.unwrap_or_default();
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

    let prep = prepare_polygon(ring_vecs_list.clone());

    let DenseSampleResult {
        boundary_cells,
        boundary_set,
        segment_map,
    } = dense_sample_boundary(&rings, &ring_vecs_list, resolution)?;

    // Res 30 covers only quintants 0-41 (elsewhere A5 answers at res 29, see
    // serialize), so a polygon reaching past them is filled at res 29: mixing the
    // two lattices would leave the fill without a consistent grid.
    if resolution == MAX_RESOLUTION
        && boundary_cells
            .iter()
            .any(|&cell| get_resolution(cell) != resolution)
    {
        return polygon_to_cells(polygon, resolution - 1, Some(options));
    }

    // The boundary contribution to the output. In `Overlapping` mode every
    // densely-sampled boundary cell contains a point on the polygon boundary, so
    // it overlaps the polygon — keep them all, unfiltered. In `Center` mode we
    // filter down to those whose center lies inside.
    let boundary_out: Vec<u64> = if options.containment == Containment::Overlapping {
        boundary_cells.clone()
    } else {
        // Flattened per-segment normals and interior-side signs, indexed like the
        // segment map. The polygon interior lies on the *outside* of a hole ring,
        // so hole segments get the opposite sign.
        let mut seg_normals: Vec<Cartesian> = Vec::new();
        let mut seg_signs: Vec<f64> = Vec::new();
        for (r, ring_vecs) in ring_vecs_list.iter().enumerate() {
            let sign = (if r == 0 { 1 } else { -1 }) * ring_winding_sign(ring_vecs);
            for normal in &prep.ring_normals[r] {
                seg_normals.push(*normal);
                seg_signs.push(sign as f64);
            }
        }
        filter_boundary_cells(
            &boundary_cells,
            &segment_map,
            &seg_normals,
            &seg_signs,
            &prep,
        )?
    };

    // Resolutions 0 and 1 have no lattice to flood (a quintant is a single
    // cell): every cell off the boundary is in or out by its center, and there
    // are at most 60 of them.
    if resolution < FIRST_HILBERT_RESOLUTION {
        let mut out = boundary_out;
        for cell in cell_to_children(WORLD_CELL, Some(resolution))? {
            if !boundary_set.contains(&cell)
                && point_in_prepared_polygon(to_cartesian(cell_to_spherical(cell)?), &prep)
            {
                out.push(cell);
            }
        }
        return compact(&out);
    }

    // The rest runs in triple space: cells as (origin_id, quintant, x, y, z)
    let hilbert_res = (resolution - FIRST_HILBERT_RESOLUTION + 1) as usize;
    let max_row = (1i32 << hilbert_res) - 1;
    let mut boundary: Vec<[i32; 5]> = Vec::with_capacity(boundary_cells.len());
    cell_ids_to_triples(boundary_cells.iter().copied(), &mut boundary)?;

    // Dense sampling can leave gaps; the shell catches them, classifying each cell.
    let shell = expand_shell(&boundary, max_row)?;
    let swallowed = swallowed_quintants(&boundary, &shell, resolution, &prep)?;
    if shell.is_empty() {
        let mut combined = boundary_out;
        combined.extend(swallowed);
        return compact(&combined);
    }

    let mut seeds: Vec<[i32; 5]> = Vec::new();
    let mut exterior_shell: Vec<[i32; 5]> = Vec::new(); // exterior shell (and hole interiors) join the firewall
    for cell in shell {
        let center = triple_cell_center(cell, hilbert_res, max_row)?;
        if point_in_prepared_polygon(to_cartesian(center), &prep) {
            seeds.push(cell);
        } else {
            exterior_shell.push(cell);
        }
    }
    if seeds.is_empty() {
        let mut combined = boundary_out;
        combined.extend(swallowed);
        return compact(&combined);
    }

    let interior_cells = flood_interior(&seeds, &boundary, &exterior_shell, resolution)?;

    let mut combined: Vec<u64> =
        Vec::with_capacity(boundary_out.len() + interior_cells.len() + swallowed.len());
    combined.extend(boundary_out);
    combined.extend(interior_cells);
    combined.extend(swallowed);
    compact(&combined)
}
