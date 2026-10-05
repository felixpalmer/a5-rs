// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

// The boundary of a polygon in cells: sampled densely along every ring,
// classified by whether each cell's center is inside, and used to classify the
// cells next to it without a full point-in-polygon test.

use std::collections::{HashMap, HashSet};

use crate::coordinate_systems::{Cartesian, LonLat};
use crate::core::cell::cell_to_spherical;
use crate::core::coordinate_transforms::to_cartesian;
use crate::geometry::prepared_polygon::{point_in_prepared_polygon, PreparedPolygon};
use crate::geometry::spherical_polygon::{ring_winding_sign, spherical_triangle_area};
use crate::traversal::line::trace_path;
use crate::traversal::triple_cells::triple_cell_key;

/// Maps each boundary cell to the indices of the ring segments that produced it.
/// Segment indices are global across rings (outer ring first, then holes).
type SegmentMap = HashMap<u64, Vec<usize>>;

/// Every ring segment, flattened across rings and indexed like the segment map:
/// endpoints, great-circle normal, and the side the polygon interior lies on.
struct Segments {
    starts: Vec<Cartesian>,
    ends: Vec<Cartesian>,
    normals: Vec<Cartesian>,
    signs: Vec<f64>,
}

/// The boundary cells as sampled, before classification.
pub(super) struct SampledBoundary {
    /// Cell IDs in the order they were sampled
    pub cells: Vec<u64>,
    pub set: HashSet<u64>,
    segment_map: SegmentMap,
}

/// The polygon's boundary cells, classified, with what's needed to classify their neighbors.
pub(super) struct Boundary<'a> {
    /// Cell IDs in the order they were sampled
    pub cells: Vec<u64>,
    pub set: HashSet<u64>,
    /// Whether the cell's center is inside the polygon, by index into `cells`
    inside: Vec<bool>,
    /// Cell centers, by index into `cells`
    centers: Vec<Cartesian>,
    segment_map: SegmentMap,
    segments: Segments,
    pub prep: &'a PreparedPolygon,
}

/// The boundary cells, each recorded with the ring segments (outer ring and
/// holes) that reached it: with `exact`, every cell a segment touches; without,
/// the cells holding samples along the segments at half-cell-radius spacing,
/// which can miss a cell whose corner a segment clips between samples.
pub(super) fn sample_boundary(
    rings: &[&[LonLat]],
    resolution: i32,
    exact: bool,
) -> Result<SampledBoundary, String> {
    let mut sampled = SampledBoundary {
        cells: Vec::new(),
        set: HashSet::new(),
        segment_map: HashMap::new(),
    };
    let mut seg_offset = 0;
    for ring in rings {
        trace_path(
            ring,
            true,
            resolution,
            |cell, arc| {
                let seg_idx = seg_offset + arc;
                if sampled.set.insert(cell) {
                    sampled.cells.push(cell);
                }
                let entry = sampled.segment_map.entry(cell).or_default();
                if entry.last() != Some(&seg_idx) {
                    entry.push(seg_idx);
                }
            },
            exact,
        )?;
        seg_offset += ring.len();
    }
    Ok(sampled)
}

/// The polygon's ring segments, flattened. The polygon interior lies on the
/// *outside* of a hole ring, so hole segments get the opposite sign.
fn ring_segments(ring_vecs_list: &[Vec<Cartesian>], prep: &PreparedPolygon) -> Segments {
    let mut segments = Segments {
        starts: Vec::new(),
        ends: Vec::new(),
        normals: Vec::new(),
        signs: Vec::new(),
    };
    for (r, ring_vecs) in ring_vecs_list.iter().enumerate() {
        let sign = (if r == 0 { 1 } else { -1 }) * ring_winding_sign(ring_vecs);
        for (i, normal) in prep.ring_normals[r].iter().enumerate() {
            segments.starts.push(ring_vecs[i]);
            segments.ends.push(ring_vecs[(i + 1) % ring_vecs.len()]);
            segments.normals.push(*normal);
            segments.signs.push(sign as f64);
        }
    }
    segments
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

/// Classify the sampled boundary cells by whether their center is inside the
/// polygon.
///
/// For each cell we know which ring segment(s) sampled it. When all of those
/// segments place the cell on the same side (cheap signed-dot test), that
/// decides it. When they disagree (vertex / concave corner) or the cell wasn't
/// recorded, fall back to full PIP.
pub(super) fn classify_boundary<'a>(
    sampled: SampledBoundary,
    ring_vecs_list: &[Vec<Cartesian>],
    prep: &'a PreparedPolygon,
) -> Result<Boundary<'a>, String> {
    let segments = ring_segments(ring_vecs_list, prep);
    let mut inside: Vec<bool> = Vec::with_capacity(sampled.cells.len());
    let mut centers: Vec<Cartesian> = Vec::with_capacity(sampled.cells.len());
    for &cell in &sampled.cells {
        let cv = to_cartesian(cell_to_spherical(cell)?);
        centers.push(cv);
        let segs = match sampled.segment_map.get(&cell) {
            Some(s) => s,
            None => {
                inside.push(point_in_prepared_polygon(cv, prep));
                continue;
            }
        };
        let mut all_inside = true;
        let mut any_inside = false;
        let mut ambiguous = false;
        for &seg_idx in segs {
            let n = segments.normals[seg_idx];
            let dot = n.x() * cv.x() + n.y() * cv.y() + n.z() * cv.z();
            if dot.abs() < 1e-14 {
                ambiguous = true;
                break;
            }
            // The side of the segment's great circle only decides when the center
            // projects onto the segment itself, not beyond one of its endpoints
            if !projects_onto_segment(cv, segments.starts[seg_idx], segments.ends[seg_idx], n) {
                ambiguous = true;
                break;
            }
            if dot * segments.signs[seg_idx] > 0.0 {
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
    Ok(Boundary {
        cells: sampled.cells,
        set: sampled.set,
        inside,
        centers,
        segment_map: sampled.segment_map,
        segments,
        prep,
    })
}

impl Boundary<'_> {
    /// Whether boundary cell `c` is in the output. In `Overlapping` mode every
    /// densely-sampled boundary cell contains a point on the polygon boundary, so
    /// it overlaps the polygon — keep them all. In `Center` mode keep those whose
    /// center lies inside.
    pub fn emits(&self, c: usize, overlapping: bool) -> bool {
        overlapping || self.inside[c]
    }

    /// The boundary cells in the output (see [`Boundary::emits`]), as a new vector.
    pub fn output(&self, overlapping: bool) -> Vec<u64> {
        (0..self.cells.len())
            .filter(|&c| self.emits(c, overlapping))
            .map(|c| self.cells[c])
            .collect()
    }

    /// Whether a point next to boundary cell `parent` (see [`boundary_neighbors`])
    /// is inside the polygon: the parent's class, flipped by each of its ring
    /// segments crossed on the way. Full PIP only on a near-degenerate crossing.
    pub fn inside_next_to(&self, center: Cartesian, parent: usize) -> bool {
        let seg_idxs = &self.segment_map[&self.cells[parent]];
        match arc_crossing_parity(center, self.centers[parent], seg_idxs, &self.segments) {
            Some(odd) => self.inside[parent] != odd,
            None => point_in_prepared_polygon(center, self.prep),
        }
    }
}

const CROSSING_EPS: f64 = 1e-14;

/// Parity of the crossings of the short arc p->q with the given ring segments
/// (proper crossings, by the signs of four triple products), or `None` on a
/// near-degenerate sign.
fn arc_crossing_parity(
    p: Cartesian,
    q: Cartesian,
    seg_idxs: &[usize],
    segments: &Segments,
) -> Option<bool> {
    let abx = p.y() * q.z() - p.z() * q.y();
    let aby = p.z() * q.x() - p.x() * q.z();
    let abz = p.x() * q.y() - p.y() * q.x();
    let mut odd = false;
    for &seg in seg_idxs {
        let c = segments.starts[seg];
        let d = segments.ends[seg];
        let acb = -(abx * c.x() + aby * c.y() + abz * c.z());
        let bda = abx * d.x() + aby * d.y() + abz * d.z();
        if acb.abs() < CROSSING_EPS || bda.abs() < CROSSING_EPS {
            return None;
        }
        if acb * bda < 0.0 {
            continue;
        }
        let cd = segments.normals[seg];
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

/// Calls the visitor for neighbors of a cell given in triple space.
pub(super) type NeighborWalk<'a> =
    &'a dyn Fn([i32; 5], &mut dyn FnMut([i32; 5]) -> Result<(), String>) -> Result<(), String>;

/// The cells next to the boundary, found by `walks` in turn, each with the
/// boundary cell it was first found from (its parent, an index into the
/// boundary). Listing edge-neighbor walks first gives every cell an
/// edge-sharing parent when it has one, which [`Boundary::inside_next_to`]
/// needs: the arc between their centers then crosses no other cell holding
/// boundary samples. A cell found only by a vertex has no boundary cell across
/// any of its edges, which covers every other cell around that vertex.
pub(super) fn boundary_neighbors(
    boundary: &[[i32; 5]],
    walks: &[NeighborWalk],
) -> Result<Vec<([i32; 5], usize)>, String> {
    let mut seen: HashSet<i64> = boundary.iter().map(|&c| triple_cell_key(c)).collect();
    let mut cells: Vec<([i32; 5], usize)> = Vec::new();
    for walk in walks {
        for (parent, &cell) in boundary.iter().enumerate() {
            walk(cell, &mut |n| {
                if seen.insert(triple_cell_key(n)) {
                    cells.push((n, parent));
                }
                Ok(())
            })?;
        }
    }
    Ok(cells)
}

/// Area of the polygon (outer ring minus holes) on the unit sphere, in steradians.
pub(super) fn polygon_area(ring_vecs_list: &[Vec<Cartesian>]) -> f64 {
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
