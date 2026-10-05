// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

// Polygon fill by curve runs. Within a quintant consecutive cells on the curve
// are neighbors, or at most a step over one or two cells. So the band of
// boundary cells plus one ring of their neighbors splits each quintant's
// stretch of the curve (a range of keys) into runs that lie wholly inside or
// wholly outside the polygon: a step over the boundary would have to land in
// the band. One probe classifies a run, and an inside run is emitted directly
// as the coarsest cells covering it, so the interior costs O(boundary), not
// O(area).

use std::collections::HashMap;
use std::sync::LazyLock;

use crate::core::cell::cell_to_spherical;
use crate::core::compact::compact;
use crate::core::coordinate_transforms::to_cartesian;
use crate::core::origin::{get_origins, quintant_to_segment, segment_to_quintant};
use crate::core::serialization::{
    cell_to_parent, deserialize, get_resolution, get_stride, is_first_child, serialize,
    FIRST_HILBERT_RESOLUTION, MAX_RESOLUTION,
};
use crate::core::utils::A5Cell;
use crate::geometry::prepared_polygon::point_in_prepared_polygon;
use crate::lattice::{s_to_triple, triple_flavor, triple_to_s, Orientation, Triple};
use crate::traversal::neighbors::NEIGHBOR_DELTAS;
use crate::traversal::triple_cells::{for_each_triple_neighbor, triple_cell_center};

use super::polygon_boundary::{boundary_neighbors, Boundary};

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

/// Fill a polygon by curve runs, given its classified boundary and the boundary
/// cells in triple space. `cap_holds_quintant` says whether the polygon might
/// swallow a quintant whole (one holding no band cells at all).
pub(super) fn fill_by_curve_runs(
    boundary: &Boundary,
    triples: &[[i32; 5]],
    resolution: i32,
    overlapping: bool,
    cap_holds_quintant: bool,
) -> Result<Vec<u64>, String> {
    let hilbert_res = (resolution - FIRST_HILBERT_RESOLUTION + 1) as usize;
    let max_row = (1i32 << hilbert_res) - 1;
    // One ring of neighbors (edge and vertex, across quintant edges too), edge neighbors first
    let ring = boundary_neighbors(
        triples,
        &[
            &|cell, visit| for_each_triple_neighbor(cell, max_row, true, visit),
            &|cell, visit| for_each_triple_neighbor(cell, max_row, false, visit),
        ],
    )?;

    let unit_shift = 58 - 2 * hilbert_res as u32;
    let unit = 1u64 << unit_shift;

    // Band keys carry two flags below the key: EMIT (the cell is in the output)
    // and RING. A u128 always has room for them, at resolution 30 too.
    const EMIT: u128 = 1;
    const RING: u128 = 2;

    let mut keys: Vec<u128> = Vec::with_capacity(boundary.cells.len() + ring.len());
    for (i, &cell) in boundary.cells.iter().enumerate() {
        let emit = boundary.emits(i, overlapping);
        keys.push((cell_to_key(cell, resolution)? as u128) << 2 | if emit { EMIT } else { 0 });
    }
    // Ring cells by flagged key (as their index into `ring`), with their class
    let mut ring_by_key: HashMap<u128, usize> = HashMap::with_capacity(ring.len());
    let mut ring_inside: Vec<bool> = Vec::with_capacity(ring.len());
    for (r, &(cell, parent)) in ring.iter().enumerate() {
        let center = to_cartesian(triple_cell_center(cell, hilbert_res, max_row)?);
        let inside = boundary.inside_next_to(center, parent);
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
                boundary.prep,
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
