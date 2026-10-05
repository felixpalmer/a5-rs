// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use std::collections::HashSet;

use crate::coordinate_systems::{Cartesian, Face, LonLat, Spherical};
use crate::core::cell::{
    cell_intersects_segment, last_cell_shape, last_projection, lonlat_to_cell, spherical_to_cell,
    CellShape,
};
use crate::core::coordinate_transforms::{from_lon_lat, to_cartesian, to_lon_lat, to_spherical};
use crate::core::face_adjacency::walk_faces;
use crate::core::origin::get_origins;
use crate::core::serialization::{deserialize, serialize, FIRST_HILBERT_RESOLUTION};
use crate::core::tiling::get_pentagon_vertices;
use crate::geometry::pentagon::clip_segment;
use crate::core::utils::A5Cell;
use crate::lattice::{triple_flavor, Triple};
use crate::projections::dodecahedron::DodecahedronProjection;
use crate::traversal::cap::estimate_cell_radius;
use crate::traversal::triple_cells::{cell_ids_to_triples, triple_cell_to_id, walk_triple_cells};
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

// Tolerance on where (as a fraction of the sub-segment) the part in one cell
// ends and the part in the next begins, and how far (as a fraction of the
// edge) the crossing must be from the edge's ends for no third cell to touch it
const SHARED_EDGE_EPS: f64 = 1e-9;
const SHARED_EDGE_MARGIN: f64 = 1e-6;

/// One end of the current sub-segment: the point, its cell's shape and the
/// point's own projection onto that cell's face, as the cell's lookup made them.
struct SubsegmentEnd {
    point: Spherical,
    shape: Option<CellShape>,
    face: Option<Face>,
}

/// The sub-segment being traced, with both ends projected onto each face, filled on demand.
struct Subsegment {
    a: SubsegmentEnd,
    b: SubsegmentEnd,
    faces: Vec<Option<(Face, Face)>>,
}

impl Subsegment {
    /// Both ends on `origin_id`'s face, reusing an end's own projection when it has one there.
    fn project(&mut self, origin_id: u8) -> Result<(Face, Face), String> {
        if let Some(projected) = self.faces[origin_id as usize] {
            return Ok(projected);
        }
        let dodecahedron = DodecahedronProjection::get_thread_local();
        let mut end_face = |end: &SubsegmentEnd| -> Result<Face, String> {
            match (&end.shape, end.face) {
                (Some(shape), Some(face)) if shape.origin_id == origin_id => Ok(face),
                _ => dodecahedron.forward(end.point, origin_id),
            }
        };
        let projected = (end_face(&self.a)?, end_face(&self.b)?);
        self.faces[origin_id as usize] = Some(projected);
        Ok(projected)
    }
}

/// Whether the sub-segment, clipped to each cell it passes through in turn (see
/// `PentagonShape::clip_segment`), hands over cleanly: from a (in the first part)
/// to b (in the last), each part ends where the next begins, at a point well
/// inside an edge, so no other cell meets the sub-segment there.
fn hands_over(parts: &[Option<(f64, f64, f64)>]) -> bool {
    let mut prev_end = 0.0;
    for (i, part) in parts.iter().enumerate() {
        let Some((start, end, exit_edge_t)) = *part else {
            return false;
        };
        let mismatch = if i == 0 {
            start > SHARED_EDGE_EPS
        } else {
            (start - prev_end).abs() > SHARED_EDGE_EPS
        };
        if mismatch {
            return false;
        }
        if i == parts.len() - 1 {
            return end >= 1.0 - SHARED_EDGE_EPS;
        }
        if exit_edge_t <= SHARED_EDGE_MARGIN || exit_edge_t >= 1.0 - SHARED_EDGE_MARGIN {
            return false;
        }
        prev_end = end;
    }
    false
}

/// Visit every cell a path of great-circle arcs touches, arc by arc and in order
/// along each arc, with the index of the arc (a cell may be visited more than
/// once). The path joins consecutive `points`, and the last back to the first
/// when `closed`. With `exact` false only the cells holding the samples are
/// visited, which can miss a cell whose corner an arc clips between samples.
///
/// Each arc is sampled at half-cell-radius intervals. A pair of consecutive
/// samples within one cell needs nothing more: cells are convex and the
/// sub-segment between them is short enough to be straight (projected onto the
/// cell's Face). Between two cells, clipping the sub-segment to their pentagons
/// usually shows it crossing straight from one into the other, or clipping one
/// cell between them; otherwise a strict local search finds every cell whose
/// pentagon it touches.
///
/// The search runs in triple space: a cell's neighbors come from its flavor's
/// triple deltas plus the boundary delta tables, and its pentagon straight from
/// its triple, so a candidate is never decoded and only touched cells are
/// encoded.
pub fn trace_path(
    points: &[LonLat],
    closed: bool,
    resolution: i32,
    mut visit: impl FnMut(u64, usize),
    exact: bool,
) -> Result<(), String> {
    let sample_interval = estimate_cell_radius(resolution) * 0.5;
    let hilbert_res = (resolution - FIRST_HILBERT_RESOLUTION + 1).max(0) as usize;
    let max_row = (1i32 << hilbert_res) - 1;
    let origins = get_origins();

    // Each point once: on the sphere, as a vector, and its cell, with the cell's
    // pentagon and origin and the point's projection there, as its lookup made them
    let n = points.len();
    let point_spherical: Vec<Spherical> = points.iter().map(|&p| from_lon_lat(p)).collect();
    let point_vecs: Vec<Cartesian> = point_spherical.iter().map(|&p| to_cartesian(p)).collect();
    let mut point_cells: Vec<u64> = Vec::with_capacity(n);
    let mut point_ends: Vec<(Option<CellShape>, Option<Face>)> = Vec::with_capacity(n);
    // Only the exact trace uses the shapes, so only it pays to copy them
    let capture = |cell: u64, point: Spherical| {
        if !exact {
            return (None, None);
        }
        let shape = last_cell_shape(cell);
        let face = shape.as_ref().and_then(|s| last_projection(point, s.origin_id));
        (shape, face)
    };
    for &p in &point_spherical {
        let cell = spherical_to_cell(p, resolution)?;
        point_cells.push(cell);
        point_ends.push(capture(cell, p));
    }
    let point_end = |i: usize| SubsegmentEnd {
        point: point_spherical[i],
        shape: point_ends[i].0,
        face: point_ends[i].1,
    };

    let mut sub = Subsegment {
        a: point_end(0),
        b: point_end(0),
        faces: vec![None; origins.len()],
    };
    let arcs = if closed { n } else { n - 1 };
    for arc in 0..arcs {
        let end = (arc + 1) % n;
        // Sample the great-circle at half-cell-radius spacing, endpoints included
        let interior = sample_great_circle_arc(point_vecs[arc], point_vecs[end], sample_interval);
        let last = interior.len() + 1;

        let mut cell_a = point_cells[arc];
        // The first step moves this end to `a`
        sub.b = point_end(arc);
        visit(cell_a, arc);
        // Walk pairwise. Each (P_j, P_{j+1}) sub-segment is short enough that its
        // projection onto any nearby cell's Face is essentially straight, so we
        // can use exact 2D segment-vs-pentagon intersection.
        for j in 1..=last {
            let (cell_b, next_b) = if j == last {
                (point_cells[end], point_end(end))
            } else {
                let point = to_spherical(interior[j - 1]);
                let cell = spherical_to_cell(point, resolution)?;
                let (shape, face) = capture(cell, point);
                (cell, SubsegmentEnd { point, shape, face })
            };
            sub.a = std::mem::replace(&mut sub.b, next_b);
            visit(cell_b, arc);
            if cell_a != cell_b && exact {
                if resolution == 0 {
                    trace_faces(
                        cell_a,
                        cell_b,
                        to_lon_lat(sub.a.point),
                        to_lon_lat(sub.b.point),
                        |cell| visit(cell, arc),
                    )?;
                } else {
                    sub.faces.fill(None);
                    if !settle(&mut sub, cell_a, cell_b, resolution, arc, &mut visit)? {
                        search_subsegment(
                            &mut sub,
                            cell_a,
                            cell_b,
                            hilbert_res,
                            max_row,
                            resolution,
                            arc,
                            &mut visit,
                        )?;
                    }
                }
            }
            cell_a = cell_b;
        }
    }
    Ok(())
}

/// Settle the sub-segment from cell A to cell B without the full search. It
/// usually runs straight from A into B; failing that, it usually clips one
/// cell C between them, found at the middle of the gap and then visited.
/// `false` sends the sub-segment to the full search.
fn settle(
    sub: &mut Subsegment,
    cell_a: u64,
    cell_b: u64,
    resolution: i32,
    arc: usize,
    visit: &mut impl FnMut(u64, usize),
) -> Result<bool, String> {
    let (Some(shape_a), Some(shape_b)) = (sub.a.shape, sub.b.shape) else {
        return Ok(false);
    };
    let origin_id = shape_a.origin_id;
    if shape_b.origin_id != origin_id {
        return Ok(false);
    }
    let (fa, fb) = sub.project(origin_id)?;
    let in_a = clip_segment(&shape_a.pentagon, fa, fb);
    let in_b = clip_segment(&shape_b.pentagon, fa, fb);
    if hands_over(&[in_a, in_b]) {
        return Ok(true);
    }
    let (Some(a_part), Some(b_part)) = (in_a, in_b) else {
        return Ok(false);
    };
    if b_part.0 <= a_part.1 {
        return Ok(false);
    }
    let t = (a_part.1 + b_part.0) / 2.0;
    let av = to_cartesian(sub.a.point);
    let bv = to_cartesian(sub.b.point);
    let m = [
        av.x() + (bv.x() - av.x()) * t,
        av.y() + (bv.y() - av.y()) * t,
        av.z() + (bv.z() - av.z()) * t,
    ];
    let length = (m[0] * m[0] + m[1] * m[1] + m[2] * m[2]).sqrt();
    let mid = Cartesian::new(m[0] / length, m[1] / length, m[2] / length);
    let cell_c = spherical_to_cell(to_spherical(mid), resolution)?;
    let Some(shape_c) = last_cell_shape(cell_c) else {
        return Ok(false);
    };
    if shape_c.origin_id != origin_id || cell_c == cell_a || cell_c == cell_b {
        return Ok(false);
    }
    if !hands_over(&[in_a, clip_segment(&shape_c.pentagon, fa, fb), in_b]) {
        return Ok(false);
    }
    visit(cell_c, arc);
    Ok(true)
}

/// Strict local search: walk out from A and B, keeping every cell whose
/// pentagon the sub-segment crosses. Terminates as soon as no new touching
/// cells are found — typically 1–2 hops, since a sub-segment ≤ cell_radius/2
/// reaches at most a couple of cells beyond its endpoint cells.
#[allow(clippy::too_many_arguments)]
fn search_subsegment(
    sub: &mut Subsegment,
    cell_a: u64,
    cell_b: u64,
    hilbert_res: usize,
    max_row: i32,
    resolution: i32,
    arc: usize,
    visit: &mut impl FnMut(u64, usize),
) -> Result<(), String> {
    let mut ends: Vec<[i32; 5]> = Vec::with_capacity(2);
    cell_ids_to_triples([cell_a, cell_b], &mut ends)?;
    walk_triple_cells(ends, max_row, |cell| {
        let [origin_id, quintant, x, y, z] = cell;
        let (a_face, b_face) = sub.project(origin_id as u8)?;
        let triple = Triple::new(x, y, z);
        let pentagon = get_pentagon_vertices(
            hilbert_res as i32,
            quintant as usize,
            &triple,
            triple_flavor(&triple, max_row),
        );
        if !pentagon.intersects_segment(a_face, b_face) {
            return Ok(false);
        }
        visit(triple_cell_to_id(cell, hilbert_res, resolution)?, arc);
        Ok(true)
    })
}

/// Trace cells along a polyline defined by a sequence of waypoints.
///
/// Consecutive waypoints are connected with great-circle arcs, traced by
/// [`trace_path`]: every cell whose pentagon an arc touches. Cells at waypoint
/// junctions are deduplicated.
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
    trace_path(
        waypoints,
        false,
        resolution,
        |cell, _arc| {
            if seen.insert(cell) {
                result.push(cell);
            }
        },
        true,
    )?;
    Ok(result)
}
