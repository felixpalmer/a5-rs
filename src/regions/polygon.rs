// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

use crate::coordinate_systems::{Cartesian, LonLat};
use crate::core::cell::{cell_to_spherical, lonlat_to_cell, spherical_to_cell};
use crate::core::cell_info::get_num_cells;
use crate::core::compact::compact;
use crate::core::coordinate_transforms::{from_lon_lat, to_cartesian, to_spherical};
use crate::core::origin::{get_origins, quintant_to_segment, segment_to_quintant};
use crate::core::serialization::{
    cell_to_children, cell_to_parent, deserialize, get_resolution, get_stride, is_first_child,
    serialize, FIRST_HILBERT_RESOLUTION, MAX_RESOLUTION, WORLD_CELL,
};
use crate::core::utils::A5Cell;
use crate::geometry::prepared_polygon::{
    point_in_prepared_polygon, prepare_polygon, PreparedPolygon,
};
use crate::geometry::spherical_polygon::{ring_winding_sign, spherical_triangle_area};
use crate::lattice::{s_to_triple, triple_flavor, triple_to_s, Orientation, Triple};
use crate::traversal::cap::estimate_cell_radius;
use crate::traversal::lattice_flood_fill::{triple_space_flood_fill, FloodInput};
use crate::traversal::neighbors::NEIGHBOR_DELTAS;
use crate::traversal::triple_cells::{
    cell_ids_to_triples, for_each_lattice_neighbor, for_each_triple_neighbor, triple_cell_center,
    triple_cell_key, triple_cell_to_id,
};
use crate::utils::great_circle::sample_great_circle_arc;

/// Maps each boundary cell to the indices of the ring segments that produced it.
/// Segment indices are global across rings (outer ring first, then holes).
/// Used by `classify_boundary_cells` to short-circuit PIP via segment-side dot
/// products, and to classify ring cells locally.
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

/// Whether `p` lies in the lune of the segment a->b (normal `n` = a × b): its
/// projection onto the great circle falls between a and b.
fn projects_onto_segment(p: Cartesian, a: Cartesian, b: Cartesian, n: Cartesian) -> bool {
    // n × a points along the arc from a towards b, b × n from b back towards a
    let from_a = p.x() * (n.y() * a.z() - n.z() * a.y())
        + p.y() * (n.z() * a.x() - n.x() * a.z())
        + p.z() * (n.x() * a.y() - n.y() * a.x());
    let from_b = p.x() * (b.y() * n.z() - b.z() * n.y())
        + p.y() * (b.z() * n.x() - b.x() * n.z())
        + p.z() * (b.x() * n.y() - b.y() * n.x());
    from_a > 0.0 && from_b > 0.0
}

/// Per-segment geometry, flattened across rings and indexed like the segment map.
struct Segments {
    starts: Vec<Cartesian>,
    ends: Vec<Cartesian>,
    normals: Vec<Cartesian>,
    signs: Vec<f64>,
}

/// Classify boundary cells by whether their center is inside the polygon.
///
/// For each cell we know which ring segment(s) sampled it. When all of those
/// segments place the cell on the same side (cheap signed-dot test), that
/// decides it. When they disagree (vertex / concave corner) or the cell wasn't
/// recorded, fall back to full PIP. Returns the centers too, for reuse.
fn classify_boundary_cells(
    boundary_cells: &[u64],
    segment_map: &SegmentMap,
    segs: &Segments,
    prep: &PreparedPolygon,
) -> Result<(Vec<bool>, Vec<Cartesian>), String> {
    let mut inside: Vec<bool> = Vec::with_capacity(boundary_cells.len());
    let mut centers: Vec<Cartesian> = Vec::with_capacity(boundary_cells.len());
    for &cell in boundary_cells {
        let cv = to_cartesian(cell_to_spherical(cell)?);
        centers.push(cv);
        let segments = match segment_map.get(&cell) {
            Some(s) => s,
            None => {
                inside.push(point_in_prepared_polygon(cv, prep));
                continue;
            }
        };
        let mut all_inside = true;
        let mut any_inside = false;
        let mut ambiguous = false;
        for &seg_idx in segments {
            let n = segs.normals[seg_idx];
            let dot = n.x() * cv.x() + n.y() * cv.y() + n.z() * cv.z();
            if dot.abs() < 1e-14 {
                ambiguous = true;
                break;
            }
            // The side of the segment's great circle only decides when the center
            // projects onto the segment itself, not beyond one of its endpoints
            if !projects_onto_segment(cv, segs.starts[seg_idx], segs.ends[seg_idx], n) {
                ambiguous = true;
                break;
            }
            if dot * segs.signs[seg_idx] > 0.0 {
                any_inside = true;
            } else {
                all_inside = false;
            }
        }
        if ambiguous || (any_inside && !all_inside) {
            inside.push(point_in_prepared_polygon(cv, prep));
        } else {
            inside.push(all_inside);
        }
    }
    Ok((inside, centers))
}

const CROSSING_EPS: f64 = 1e-14;

/// Parity of the crossings of the short arc p->q with the given ring segments
/// (proper crossings, by the signs of four triple products), or `None` on a
/// near-degenerate sign.
fn arc_crossing_parity(
    p: Cartesian,
    q: Cartesian,
    segments: &[usize],
    segs: &Segments,
) -> Option<bool> {
    let abx = p.y() * q.z() - p.z() * q.y();
    let aby = p.z() * q.x() - p.x() * q.z();
    let abz = p.x() * q.y() - p.y() * q.x();
    let mut odd = false;
    for &seg in segments {
        let c = segs.starts[seg];
        let d = segs.ends[seg];
        let acb = -(abx * c.x() + aby * c.y() + abz * c.z());
        let bda = abx * d.x() + aby * d.y() + abz * d.z();
        if acb.abs() < CROSSING_EPS || bda.abs() < CROSSING_EPS {
            return None;
        }
        if acb * bda < 0.0 {
            continue;
        }
        let cd = segs.normals[seg];
        let cbd = -(cd.x() * q.x() + cd.y() * q.y() + cd.z() * q.z());
        let dac = cd.x() * p.x() + cd.y() * p.y() + cd.z() * p.z();
        if cbd.abs() < CROSSING_EPS || dac.abs() < CROSSING_EPS {
            return None;
        }
        if acb * cbd > 0.0 && acb * dac > 0.0 {
            odd = !odd;
        }
    }
    Some(odd)
}

// Cells are ordered on the curve by a 64-bit key: the 6-bit quintant (as in
// the ID's top bits) then S, left-aligned below it. Below resolution 30 that is
// the cell ID without its resolution marker; at resolution 30 S fills all 58
// bits. A cell at resolution r < 30 is its aligned key plus the marker.
const QUINTANT_SHIFT: u32 = 58;
const S_MASK: u64 = (1 << QUINTANT_SHIFT) - 1;

/// Curve orientation of each quintant by its 6-bit key prefix.
static PREFIX_ORIENTATION: LazyLock<Vec<Orientation>> = LazyLock::new(|| {
    let origins = get_origins();
    (0..60)
        .map(|q| {
            let origin = &origins[q / 5];
            segment_to_quintant((q + origin.first_quintant) % 5, origin).1
        })
        .collect()
});

/// Key prefix and curve orientation by triple quintant (origin.id * 5 + quintant).
static TRIPLE_PREFIX: LazyLock<Vec<(u64, Orientation)>> = LazyLock::new(|| {
    let mut out = Vec::with_capacity(60);
    for origin in get_origins() {
        for quintant in 0..5 {
            let (segment, orientation) = quintant_to_segment(quintant, origin);
            let q = 5 * origin.id as usize + (segment + 5 - origin.first_quintant) % 5;
            out.push(((q as u64) << QUINTANT_SHIFT, orientation));
        }
    }
    out
});

/// The key of a cell given in triple space.
fn triple_key(cell: [i32; 5], hilbert_res: usize, unit_shift: u32) -> Result<u64, String> {
    let (prefix, orientation) = TRIPLE_PREFIX[(cell[0] * 5 + cell[1]) as usize];
    let s = triple_to_s(
        &Triple::new(cell[2], cell[3], cell[4]),
        hilbert_res,
        orientation,
    )
    .ok_or("triple_key: invalid triple")?;
    Ok(prefix | (s << unit_shift))
}

fn marker_bit(resolution: i32) -> u64 {
    if resolution == 1 {
        1 << 56
    } else {
        1 << (59 - 2 * resolution)
    }
}

fn cell_to_key(cell: u64, resolution: i32) -> Result<u64, String> {
    if resolution < MAX_RESOLUTION {
        return Ok(cell - marker_bit(resolution));
    }
    let c = deserialize(cell)?;
    let origin = &get_origins()[c.origin_id as usize];
    let q = 5 * origin.id as u64 + ((c.segment + 5 - origin.first_quintant) % 5) as u64;
    Ok((q << QUINTANT_SHIFT) | c.s)
}

fn key_to_cell(key: u64, resolution: i32) -> Result<u64, String> {
    if resolution < MAX_RESOLUTION {
        return Ok(key + marker_bit(resolution));
    }
    let q = (key >> QUINTANT_SHIFT) as usize;
    let origin = &get_origins()[q / 5];
    serialize(&A5Cell {
        origin_id: origin.id,
        segment: (q + origin.first_quintant) % 5,
        s: key & S_MASK,
        resolution,
    })
}

/// Append the cells covering the key range [lo, hi) at `resolution`, as the
/// coarsest aligned blocks (a block of 4^k cells is their resolution - k parent).
fn emit_range(mut lo: u64, hi: u64, resolution: i32, out: &mut Vec<u64>) -> Result<(), String> {
    let hilbert_res = (resolution - FIRST_HILBERT_RESOLUTION + 1) as u32;
    let unit_shift = 58 - 2 * hilbert_res;
    while lo < hi {
        let mut k = 0;
        while k < hilbert_res {
            // A whole quintant (k + 1 = hilbert_res) is 2^58: still fits in u64
            let size = 1u64 << (unit_shift + 2 * (k + 1));
            if lo & (size - 1) != 0 || lo + size > hi {
                break;
            }
            k += 1;
        }
        out.push(key_to_cell(lo, resolution - k as i32)?);
        lo += 1u64 << (unit_shift + 2 * k);
    }
    Ok(())
}

/// The ring of neighbors (edge and vertex, across quintant edges too) around
/// the boundary cells. Each ring cell comes with a boundary cell next to it
/// (its parent, an index into the boundary): one it shares an edge with when
/// there is one, as edge neighbors are visited first. The arc between their
/// centers then crosses no other cell holding boundary samples: a ring cell
/// found by a vertex has no boundary cell across any of its edges, which covers
/// every other cell around that vertex.
fn grow_ring(boundary: &[[i32; 5]], max_row: i32) -> Result<Vec<([i32; 5], usize)>, String> {
    let mut seen: HashSet<i64> = boundary.iter().map(|&c| triple_cell_key(c)).collect();
    let mut ring: Vec<([i32; 5], usize)> = Vec::new();
    for edge_only in [true, false] {
        for (parent, &cell) in boundary.iter().enumerate() {
            for_each_triple_neighbor(cell, max_row, edge_only, |n| {
                if seen.insert(triple_cell_key(n)) {
                    ring.push((n, parent));
                }
                Ok(())
            })?;
        }
    }
    Ok(ring)
}

/// Area of the polygon (outer ring minus holes) on the unit sphere, in steradians.
fn polygon_area(ring_vecs_list: &[Vec<Cartesian>]) -> f64 {
    let mut total = 0.0;
    for (r, ring) in ring_vecs_list.iter().enumerate() {
        // Signed fan from the first vertex: concave rings come out right too
        let mut area = 0.0;
        for i in 1..ring.len().saturating_sub(1) {
            area += spherical_triangle_area(ring[0], ring[i], ring[i + 1]).get();
        }
        total += if r == 0 { area.abs() } else { -area.abs() };
    }
    total
}

/// Below this many estimated interior cells per boundary cell, flooding the
/// interior beats splitting the curve into runs (measured crossover: ~3.3).
const FLOOD_INTERIOR_PER_BOUNDARY: f64 = 3.0;

/// Compact cells that are already sorted and disjoint, in one pass: a stack
/// whose top is merged into its parent whenever it ends in a full sibling group.
fn compact_sorted(cells: &[u64]) -> Result<Vec<u64>, String> {
    let mut stack: Vec<u64> = Vec::with_capacity(cells.len());
    for &cell in cells {
        stack.push(cell);
        loop {
            let top = stack.len() - 1;
            let resolution = get_resolution(stack[top]);
            if resolution < 0 {
                break;
            }
            let n = if resolution >= FIRST_HILBERT_RESOLUTION {
                4
            } else if resolution == 0 {
                12
            } else {
                5
            };
            if stack.len() < n {
                break;
            }
            let first = stack[top + 1 - n];
            if !is_first_child(first, Some(resolution)) {
                break;
            }
            let stride = get_stride(resolution);
            if (1..n).any(|j| stack[top + 1 - n + j] != first + j as u64 * stride) {
                break;
            }
            stack.truncate(top + 1 - n);
            stack.push(cell_to_parent(first, None)?);
        }
    }
    Ok(stack)
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

    // Flattened per-segment endpoints, normals and interior-side signs, indexed
    // like the segment map. The polygon interior lies on the *outside* of a hole
    // ring, so hole segments get the opposite sign.
    let mut segs = Segments {
        starts: Vec::new(),
        ends: Vec::new(),
        normals: Vec::new(),
        signs: Vec::new(),
    };
    for (r, ring_vecs) in ring_vecs_list.iter().enumerate() {
        let sign = (if r == 0 { 1 } else { -1 }) * ring_winding_sign(ring_vecs);
        for (i, normal) in prep.ring_normals[r].iter().enumerate() {
            segs.starts.push(ring_vecs[i]);
            segs.ends.push(ring_vecs[(i + 1) % ring_vecs.len()]);
            segs.normals.push(*normal);
            segs.signs.push(sign as f64);
        }
    }
    let (boundary_inside, boundary_centers) =
        classify_boundary_cells(&boundary_cells, &segment_map, &segs, &prep)?;

    // In `Overlapping` mode every densely-sampled boundary cell contains a point
    // on the polygon boundary, so it overlaps the polygon — keep them all. In
    // `Center` mode keep those whose center lies inside.
    let overlapping = options.containment == Containment::Overlapping;

    // Resolutions 0 and 1 have no lattice (a quintant is a single cell): every
    // cell off the boundary is in or out by its center, and there are at most 60
    // of them.
    if resolution < FIRST_HILBERT_RESOLUTION {
        let mut out: Vec<u64> = Vec::new();
        for (c, &cell) in boundary_cells.iter().enumerate() {
            if overlapping || boundary_inside[c] {
                out.push(cell);
            }
        }
        for cell in cell_to_children(WORLD_CELL, Some(resolution))? {
            if !boundary_set.contains(&cell)
                && point_in_prepared_polygon(to_cartesian(cell_to_spherical(cell)?), &prep)
            {
                out.push(cell);
            }
        }
        return compact(&out);
    }

    // The rest relies on the curve. Within a quintant consecutive cells are
    // neighbors, or at most a step over one or two cells. So the band of boundary
    // cells plus one ring of their neighbors splits each quintant's stretch of the
    // curve (a range of keys) into runs that lie wholly inside or wholly outside
    // the polygon: a step over the boundary would have to land in the band. One
    // probe classifies a run, and an inside run is emitted directly as the
    // coarsest cells covering it, so the interior costs O(boundary), not O(area).
    let hilbert_res = (resolution - FIRST_HILBERT_RESOLUTION + 1) as usize;
    let max_row = (1i32 << hilbert_res) - 1;
    let mut boundary: Vec<[i32; 5]> = Vec::with_capacity(boundary_cells.len());
    cell_ids_to_triples(boundary_cells.iter().copied(), &mut boundary)?;

    // A quintant without band cells is wholly inside or outside; it can only be
    // inside when the polygon's bounding cap holds a quintant's area (4π/60)
    let pi = std::f64::consts::PI;
    let cap_holds_quintant = 2.0 * pi * (1.0 - prep.cap.min_dot) >= (4.0 * pi) / 60.0;

    // A small interior is cheaper to flood than to split into curve runs: the
    // flood costs about boundary + interior cells, the runs a sorted band of
    // boundary plus ring keys. The flood can't reach a quintant the polygon
    // swallows whole, which a polygon smaller than its bounding cap never does.
    if !cap_holds_quintant
        && polygon_area(&ring_vecs_list) / (4.0 * pi) * (get_num_cells(resolution) as f64)
            < FLOOD_INTERIOR_PER_BOUNDARY * boundary_cells.len() as f64
    {
        let mut out: Vec<u64> = Vec::new();
        for (c, &cell) in boundary_cells.iter().enumerate() {
            if overlapping || boundary_inside[c] {
                out.push(cell);
            }
        }
        // The shell: the flood's own moves out of the boundary, each cell classified
        // from the boundary cell it was found from (they share an edge)
        let mut seen: HashSet<i64> = boundary.iter().map(|&c| triple_cell_key(c)).collect();
        let mut seeds: Vec<[i32; 5]> = Vec::new();
        let mut firewall: Vec<[i32; 5]> = boundary.clone();
        for (parent, &cell) in boundary.iter().enumerate() {
            for_each_lattice_neighbor(cell, max_row, |n| {
                if !seen.insert(triple_cell_key(n)) {
                    return Ok(());
                }
                let center = to_cartesian(triple_cell_center(n, hilbert_res, max_row)?);
                let segments = &segment_map[&boundary_cells[parent]];
                let inside =
                    match arc_crossing_parity(center, boundary_centers[parent], segments, &segs) {
                        Some(odd) => boundary_inside[parent] != odd,
                        None => point_in_prepared_polygon(center, &prep),
                    };
                if inside {
                    seeds.push(n);
                } else {
                    firewall.push(n);
                }
                Ok(())
            })?;
        }
        if !seeds.is_empty() {
            for &seed in &seeds {
                out.push(triple_cell_to_id(seed, hilbert_res, resolution)?);
            }
            let flood =
                triple_space_flood_fill(FloodInput::Firewall(&firewall), &seeds, resolution, None)?;
            out.extend(flood.interior_cells);
        }
        return compact(&out);
    }

    let ring = grow_ring(&boundary, max_row)?;

    let unit_shift = 58 - 2 * hilbert_res as u32;
    let unit = 1u64 << unit_shift;

    // Band keys carry two flags below the key: EMIT (the cell is in the output)
    // and RING. A u128 always has room for them, at resolution 30 too.
    const EMIT: u128 = 1;
    const RING: u128 = 2;

    let mut keys: Vec<u128> = Vec::with_capacity(boundary_cells.len() + ring.len());
    for (i, &cell) in boundary_cells.iter().enumerate() {
        let emit = overlapping || boundary_inside[i];
        keys.push((cell_to_key(cell, resolution)? as u128) << 2 | if emit { EMIT } else { 0 });
    }
    // Ring cells by flagged key (as their index into `ring`), with their class
    let mut ring_by_key: HashMap<u128, usize> = HashMap::with_capacity(ring.len());
    let mut ring_inside: Vec<bool> = Vec::with_capacity(ring.len());
    for (r, &(cell, parent)) in ring.iter().enumerate() {
        let center = to_cartesian(triple_cell_center(cell, hilbert_res, max_row)?);
        // Locally: the parent's class, flipped by each ring segment crossed on the
        // way (full PIP only on a near-degenerate crossing)
        let segments = &segment_map[&boundary_cells[parent]];
        let inside = match arc_crossing_parity(center, boundary_centers[parent], segments, &segs) {
            Some(odd) => boundary_inside[parent] != odd,
            None => point_in_prepared_polygon(center, &prep),
        };
        ring_inside.push(inside);
        let key = (triple_key(cell, hilbert_res, unit_shift)? as u128) << 2
            | if inside { EMIT | RING } else { RING };
        keys.push(key);
        ring_by_key.insert(key, r);
    }
    keys.sort_unstable();

    // The class of a run cell from a ring cell next to it on the curve, when the
    // two are lattice neighbors: any boundary cell near the run cell would have
    // put it in the ring, so nothing between them can cross the boundary.
    let class_from_ring = |key: u64, ring_key: u128| -> Option<bool> {
        if ring_key & RING == 0 {
            return None;
        }
        let r = ring_by_key[&ring_key];
        let [_, _, x, y, z] = ring[r].0;
        let q = (key >> QUINTANT_SHIFT) as usize;
        let t = s_to_triple(
            (key & S_MASK) >> unit_shift,
            hilbert_res,
            PREFIX_ORIENTATION[q],
        );
        let flavor = triple_flavor(&Triple::new(x, y, z), max_row) as usize;
        NEIGHBOR_DELTAS[flavor]
            .all
            .iter()
            .any(|d| t.x - x == d.x && t.y - y == d.y && t.z - z == d.z)
            .then(|| ring_inside[r])
    };

    // Walk each quintant's keys in curve order, emitting the inside band cells and
    // runs as they come, so the output is sorted.
    let mut out: Vec<u64> = Vec::new();
    let probe_run = |lo: u64,
                     hi: u64,
                     prev: Option<u128>,
                     next: Option<u128>,
                     out: &mut Vec<u64>|
     -> Result<(), String> {
        let mut inside = prev.and_then(|p| class_from_ring(lo, p));
        if inside.is_none() {
            inside = next.and_then(|n| class_from_ring(hi - unit, n));
        }
        let inside = match inside {
            Some(inside) => inside,
            None => point_in_prepared_polygon(
                to_cartesian(cell_to_spherical(key_to_cell(lo, resolution)?)?),
                &prep,
            ),
        };
        if inside {
            emit_range(lo, hi, resolution, out)?;
        }
        Ok(())
    };
    let n_band = keys.len();
    let mut i = 0;
    let mut q: u64 = 0;
    while q < 60 {
        // Skip straight to the next quintant holding band cells, unless whole ones may be inside
        if !cap_holds_quintant {
            if i >= n_band {
                break;
            }
            q = (keys[i] >> (QUINTANT_SHIFT + 2)) as u64;
        }
        let q_end = (q + 1) << QUINTANT_SHIFT;
        let mut cursor = q << QUINTANT_SHIFT;
        if i >= n_band || (keys[i] >> 2) as u64 >= q_end {
            if cap_holds_quintant {
                probe_run(cursor, q_end, None, None, &mut out)?;
            }
            q += 1;
            continue;
        }
        let mut prev: Option<u128> = None;
        while i < n_band && ((keys[i] >> 2) as u64) < q_end {
            let flagged = keys[i];
            let key = (flagged >> 2) as u64;
            if key > cursor {
                probe_run(cursor, key, prev, Some(flagged), &mut out)?;
            }
            if flagged & EMIT != 0 {
                out.push(key_to_cell(key, resolution)?);
            }
            prev = Some(flagged);
            cursor = key + unit;
            i += 1;
        }
        if cursor < q_end {
            probe_run(cursor, q_end, prev, None, &mut out)?;
        }
        q += 1;
    }

    // Resolution 30 IDs don't sort like their keys (the quintant field varies in width)
    if resolution == MAX_RESOLUTION {
        compact(&out)
    } else {
        compact_sorted(&out)
    }
}
