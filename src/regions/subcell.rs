// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

// The spatial counterpart of the cell hierarchy. The index hierarchy nests
// cells by ID, but a cell's children do not tile it exactly: some stick out,
// and children of its neighbors poke in. Here a finer cell belongs to the
// coarser cell holding its center, so every resolution partitions every coarser
// one exactly.

use std::sync::LazyLock;

use crate::collections::slot_runs::{slot_runs_to_covering, to_covering};
use crate::collections::SlotRuns;
use crate::coordinate_systems::{Face, Spherical};
use crate::core::cell::{cell_to_spherical, get_pentagon, spherical_to_cell};
use crate::core::constants::TWO_PI_OVER_5;
use crate::core::face_adjacency::FACE_ADJACENCY;
use crate::core::serialization::{
    deserialize, get_resolution, slot_to_cell, FIRST_HILBERT_RESOLUTION, MAX_RESOLUTION,
    RES30_QUINTANTS, WORLD_CELL,
};
use crate::core::tiling::get_face_vertices;
use crate::core::utils::OriginId;
use crate::projections::dodecahedron::DodecahedronProjection;
use crate::traversal::curve_descent::{descend_in_curve_order, CurveDescentClass};

// How far the center of any descendant of a cell can lie from the cell's own
// center, in units of the cell's lattice spacing (face units · 2^hilbert_res). A
// child's center is within 0.342 of its parent's (the max over every flavor and
// child), and the offsets halve each level down, so all descendants lie within
// 2 · 0.342 (observed: 0.648).
const DESCENDANT_REACH: f64 = 0.7;

// Face-unit distance from the parent's edges below which a center is decided by
// `cell_to_supercell` itself, so ties and float noise resolve exactly as it
// does (fine cells at resolution 30 are ~2e-9 across).
const EDGE_EPS: f64 = 1e-12;

/// A convex pentagon's edge lines: inward unit normal and offset per edge, as
/// [nx, ny, offset] * 5 (see `signed_margin`).
type EdgeLines = [f64; 15];

/// The cell at a coarser `resolution` that contains the center of `cell`: the
/// spatial counterpart of `cell_to_parent`. Unlike the parent, the supercell
/// always contains (the center of) the cell, so aggregating by supercell
/// attributes each fine cell to the coarse cell it lies in.
///
/// # Arguments
///
/// * `cell` - The cell
/// * `resolution` - Target resolution, at most the cell's own
///
/// # Returns
///
/// The cell at `resolution` containing the center of `cell`
pub fn cell_to_supercell(cell: u64, resolution: i32) -> Result<u64, String> {
    let cell_resolution = get_resolution(cell);
    if resolution > cell_resolution {
        return Err(format!(
            "Target resolution ({}) must be equal to or less than current resolution ({})",
            resolution, cell_resolution
        ));
    }
    if resolution == cell_resolution {
        return Ok(cell);
    }
    spherical_to_cell(cell_to_spherical(cell)?, resolution)
}

/// The cells at a finer `resolution` whose centers lie in `cell`: the spatial
/// counterpart of `cell_to_children`, and the inverse of `cell_to_supercell` — a
/// cell is a subcell of exactly the supercell it maps to, so the subcells of all
/// the cells at one resolution partition every finer one. The result is a
/// covering: compacted, with a compaction marker recording the resolution — use
/// `uncompact` to expand it.
///
/// Resolution 30 covers only part of the world (see `lonlat_to_cell`); for a
/// cell reaching past it, the subcells are given at resolution 29.
///
/// # Arguments
///
/// * `cell` - The cell
/// * `resolution` - Target resolution, at least the cell's own
///
/// # Returns
///
/// Compacted cells sorted in curve order, then the compaction marker
pub fn cell_to_subcell(cell: u64, resolution: i32) -> Result<Vec<u64>, String> {
    let cell_resolution = get_resolution(cell);
    if resolution < cell_resolution {
        return Err(format!(
            "Target resolution ({}) must be equal to or greater than current resolution ({})",
            resolution, cell_resolution
        ));
    }
    if resolution > MAX_RESOLUTION {
        return Err(format!(
            "Target resolution ({}) exceeds maximum resolution ({})",
            resolution, MAX_RESOLUTION
        ));
    }
    if resolution == cell_resolution || cell == WORLD_CELL {
        return to_covering(&[cell], resolution);
    }

    // Cells along a dodecahedron edge interlock with the neighboring face's, so a
    // cell's subcells can come from the faces next to its own: search each face
    // the cell's pentagon reaches into, in that face's frame.
    let a5cell = deserialize(cell)?;
    let origin_id = a5cell.origin_id;
    let vertices = get_pentagon(&a5cell)?.get_vertices();
    let mut own = [0.0f64; 10];
    for i in 0..5 {
        own[2 * i] = vertices[i].x();
        own[2 * i + 1] = vertices[i].y();
    }
    let mut frames: Vec<(OriginId, [f64; 10])> = vec![(origin_id, own)];
    for q in 0..5 {
        let (adjacent_id, map) = &UNFOLDS[origin_id as usize * 5 + q];
        let mut mapped = [0.0f64; 10];
        let mut reaches = false;
        for i in (0..10).step_by(2) {
            mapped[i] = map[0] * own[i] + map[2] * own[i + 1] + map[4];
            mapped[i + 1] = map[1] * own[i] + map[3] * own[i + 1] + map[5];
            if signed_margin(&FACE_EDGES, mapped[i], mapped[i + 1]) > -EDGE_EPS {
                reaches = true;
            }
        }
        if reaches {
            frames.push((*adjacent_id, mapped));
        }
    }

    // Res-30 IDs only reach the first RES30_QUINTANTS quintants (in ID order)
    let mut target = resolution;
    if target == MAX_RESOLUTION
        && frames
            .iter()
            .any(|&(id, _)| 5 * id as usize + 5 > RES30_QUINTANTS)
    {
        target -= 1;
    }

    // Descend each face from its resolution-1 cells, classifying a cell by the
    // signed distance of its center from the pentagon's edges, in that face's frame
    let mut lines_by_origin: [EdgeLines; 12] = [*FACE_EDGES; 12];
    let mut starts: Vec<[i32; 5]> = Vec::with_capacity(5 * frames.len());
    for (id, frame_vertices) in &frames {
        lines_by_origin[*id as usize] = edge_lines(frame_vertices);
        // The resolution-1 cells: triple (0, 0, 0) of each quintant
        for q in 0..5 {
            starts.push([*id as i32, q, 0, 0, 0]);
        }
    }
    // By resolution: how far a cell's center must lie inside (or outside) for all
    // its descendants' to; none at the target, where each cell decides for itself
    let mut reaches = [0.0f64; 31];
    for (res, reach) in reaches.iter_mut().enumerate().take(target as usize).skip(1) {
        *reach = DESCENDANT_REACH / 2.0_f64.powi(res as i32 - FIRST_HILBERT_RESOLUTION + 1);
    }
    let mut runs = SlotRuns::new();
    descend_in_curve_order(
        &starts,
        0,
        target,
        |id, res, center: Face, slot| {
            let margin = signed_margin(&lines_by_origin[id as usize], center.x(), center.y());
            let reach = reaches[res as usize] + EDGE_EPS;
            if margin > reach {
                return Ok(CurveDescentClass::Inside);
            }
            if margin < -reach {
                return Ok(CurveDescentClass::Outside);
            }
            if res < target {
                return Ok(CurveDescentClass::Split);
            }
            // Within float noise of an edge: decide exactly as cell_to_supercell does
            Ok(
                if cell_to_supercell(slot_to_cell(slot, res), cell_resolution)? == cell {
                    CurveDescentClass::Inside
                } else {
                    CurveDescentClass::Outside
                },
            )
        },
        &mut runs,
    )?;
    Ok(slot_runs_to_covering(&runs, target))
}

/// The edge lines of the face pentagon.
static FACE_EDGES: LazyLock<EdgeLines> = LazyLock::new(|| {
    let vertices = get_face_vertices().get_vertices();
    let mut flat = [0.0f64; 10];
    for i in 0..5 {
        flat[2 * i] = vertices[i].x();
        flat[2 * i + 1] = vertices[i].y();
    }
    edge_lines(&flat)
});

/// A convex pentagon ([x0, y0, ..., x4, y4]) as its edge lines: inward unit
/// normal and offset, so a point's margin (`signed_margin`) is its signed
/// distance to the nearest edge line, positive inside.
fn edge_lines(vertices: &[f64; 10]) -> EdgeLines {
    let mut cx = 0.0;
    let mut cy = 0.0;
    for i in (0..10).step_by(2) {
        cx += vertices[i] / 5.0;
        cy += vertices[i + 1] / 5.0;
    }
    let mut lines = [0.0f64; 15];
    for i in 0..5 {
        let x1 = vertices[2 * i];
        let y1 = vertices[2 * i + 1];
        let j = if i == 4 { 0 } else { 2 * i + 2 };
        let length = (vertices[j] - x1).hypot(vertices[j + 1] - y1);
        let mut nx = (y1 - vertices[j + 1]) / length;
        let mut ny = (vertices[j] - x1) / length;
        if nx * (cx - x1) + ny * (cy - y1) < 0.0 {
            nx = -nx;
            ny = -ny;
        }
        lines[3 * i] = nx;
        lines[3 * i + 1] = ny;
        lines[3 * i + 2] = nx * x1 + ny * y1;
    }
    lines
}

#[inline]
fn signed_margin(lines: &EdgeLines, x: f64, y: f64) -> f64 {
    let mut margin = f64::INFINITY;
    for i in (0..15).step_by(3) {
        let d = lines[i] * x + lines[i + 1] * y - lines[i + 2];
        if d < margin {
            margin = d;
        }
    }
    margin
}

// By origin.id * 5 + quintant: the face across that quintant's edge, and the
// map from this face's frame into that face's, as [a, b, c, d, tx, ty] taking
// (x, y) to (a x + c y + tx, b x + d y + ty). Beyond its edges a face's frame
// extends into the neighboring face by unfolding the dodecahedron about the
// shared edge, so the map is rigid; it is fitted from three points of the
// neighbor's quintant on that edge.
static UNFOLDS: LazyLock<Vec<(OriginId, [f64; 6])>> = LazyLock::new(|| {
    // Its own projection rather than the thread-local one, which a caller may hold
    let mut dodecahedron = DodecahedronProjection::new().expect("dodecahedron projection");
    let mut unfolds = Vec::with_capacity(60);
    for (o, adjacency) in FACE_ADJACENCY.iter().enumerate() {
        for &(adjacent_id, adjacent_quintant) in adjacency {
            // Points of the neighbor's quintant (in its frame), and where they land in this one
            let mut to = [0.0f64; 6];
            let mut from = [0.0f64; 6];
            for (k, (r, angle)) in [(0.3, 0.0), (0.55, -0.4), (0.55, 0.4)]
                .into_iter()
                .enumerate()
            {
                let gamma = adjacent_quintant as f64 * TWO_PI_OVER_5.get() + angle;
                let point = Face::new(r * gamma.cos(), r * gamma.sin());
                let spherical: Spherical = dodecahedron
                    .inverse(point, adjacent_id)
                    .expect("unfold inverse");
                let landed = dodecahedron
                    .forward(spherical, o as OriginId)
                    .expect("unfold forward");
                to[2 * k] = point.x();
                to[2 * k + 1] = point.y();
                from[2 * k] = landed.x();
                from[2 * k + 1] = landed.y();
            }
            // Solve [to1 - to0, to2 - to0] = M [from1 - from0, from2 - from0]
            let f1x = from[2] - from[0];
            let f1y = from[3] - from[1];
            let f2x = from[4] - from[0];
            let f2y = from[5] - from[1];
            let det = f1x * f2y - f2x * f1y;
            let t1x = to[2] - to[0];
            let t1y = to[3] - to[1];
            let t2x = to[4] - to[0];
            let t2y = to[5] - to[1];
            let a = (t1x * f2y - t2x * f1y) / det;
            let c = (t2x * f1x - t1x * f2x) / det;
            let b = (t1y * f2y - t2y * f1y) / det;
            let d = (t2y * f1x - t1y * f2x) / det;
            let map = [
                a,
                b,
                c,
                d,
                to[0] - a * from[0] - c * from[1],
                to[1] - b * from[0] - d * from[1],
            ];
            unfolds.push((adjacent_id, map));
        }
    }
    unfolds
});
