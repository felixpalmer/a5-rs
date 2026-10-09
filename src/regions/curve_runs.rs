// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

// Polygon fill by curve runs. Within a quintant consecutive cells on the curve
// are neighbors, or at most a step over one or two cells. So the band of
// boundary cells plus one ring of their neighbors splits each quintant's
// stretch of the curve (a range of slots) into runs that lie wholly inside or
// wholly outside the polygon: a step over the boundary would have to land in
// the band. One probe classifies a run, and an inside run is emitted whole, as
// a slot run (see collections/slot_runs), so the interior costs O(boundary), not
// O(area).

use std::collections::HashMap;

use crate::collections::slot_runs::append_slot_run;
use crate::collections::SlotRuns;
use crate::core::cell::cell_to_spherical;
use crate::core::coordinate_transforms::to_cartesian;
use crate::core::serialization::{
    cell_first_slot, slot_to_cell, FIRST_HILBERT_RESOLUTION, QUINTANT_SHIFT, SLOT_COUNTS, S_MASK,
};
use crate::geometry::prepared_polygon::point_in_prepared_polygon;
use crate::lattice::{s_to_triple, triple_flavor, triple_to_s, Triple};
use crate::traversal::neighbors::NEIGHBOR_DELTAS;
use crate::traversal::triple_cells::{
    for_each_triple_neighbor, triple_cell_center, QUINTANT_TABLES,
};

use super::polygon_boundary::{boundary_neighbors, Boundary};

// Cells are ordered on the curve by the leaf slots they occupy (see core/serialization).

/// The slot of a cell given in triple space.
fn triple_slot(cell: [i32; 5], hilbert_res: usize, unit_shift: u32) -> Result<u64, String> {
    let tables = &*QUINTANT_TABLES;
    let q = (cell[0] * 5 + cell[1]) as usize;
    let s = triple_to_s(
        &Triple::new(cell[2], cell[3], cell[4]),
        hilbert_res,
        tables.orientation[q],
    )
    .ok_or("triple_slot: invalid triple")?;
    Ok(tables.prefix[q] | (s << unit_shift))
}

/// Fill a polygon by curve runs, given its classified boundary and the boundary
/// cells in triple space. `cap_holds_quintant` says whether the polygon might
/// swallow a quintant whole (one holding no band cells at all). Returns the
/// cells inside as sorted slot runs.
pub(super) fn fill_by_curve_runs(
    boundary: &Boundary,
    triples: &[[i32; 5]],
    resolution: i32,
    overlapping: bool,
    cap_holds_quintant: bool,
) -> Result<SlotRuns, String> {
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

    let unit = SLOT_COUNTS[resolution as usize];
    let unit_shift = 58 - 2 * hilbert_res as u32;

    // Band slots carry two flags below the slot: EMIT (the cell is in the output)
    // and RING. A u128 always has room for them, at resolution 30 too.
    const EMIT: u128 = 1;
    const RING: u128 = 2;

    let mut slots: Vec<u128> = Vec::with_capacity(boundary.cells.len() + ring.len());
    for (i, &cell) in boundary.cells.iter().enumerate() {
        let emit = boundary.emits(i, overlapping);
        slots.push((cell_first_slot(cell)? as u128) << 2 | if emit { EMIT } else { 0 });
    }
    // Ring cells by flagged slot (as their index into `ring`), with their class
    let mut ring_by_slot: HashMap<u128, usize> = HashMap::with_capacity(ring.len());
    let mut ring_inside: Vec<bool> = Vec::with_capacity(ring.len());
    for (r, &(cell, parent)) in ring.iter().enumerate() {
        let center = to_cartesian(triple_cell_center(cell, hilbert_res, max_row)?);
        let inside = boundary.inside_next_to(center, parent);
        ring_inside.push(inside);
        let slot = (triple_slot(cell, hilbert_res, unit_shift)? as u128) << 2
            | if inside { EMIT | RING } else { RING };
        slots.push(slot);
        ring_by_slot.insert(slot, r);
    }
    slots.sort_unstable();

    // The class of a run cell from a ring cell next to it on the curve, when the
    // two are lattice neighbors: any boundary cell near the run cell would have
    // put it in the ring, so nothing between them can cross the boundary.
    let class_from_ring = |slot: u64, ring_slot: u128| -> Option<bool> {
        if ring_slot & RING == 0 {
            return None;
        }
        let r = ring_by_slot[&ring_slot];
        let [_, _, x, y, z] = ring[r].0;
        let tables = &*QUINTANT_TABLES;
        let q = (slot >> QUINTANT_SHIFT) as usize;
        let t = s_to_triple(
            (slot & S_MASK) >> unit_shift,
            hilbert_res,
            tables.orientation[tables.triple_quintant_by_id_order[q]],
        );
        let flavor = triple_flavor(&Triple::new(x, y, z), max_row) as usize;
        NEIGHBOR_DELTAS[flavor]
            .all
            .iter()
            .any(|d| t.x - x == d.x && t.y - y == d.y && t.z - z == d.z)
            .then(|| ring_inside[r])
    };

    // Walk each quintant's slots in curve order, emitting the inside band cells and
    // runs as they come, so the output is sorted.
    let mut out = SlotRuns::new();
    let probe_run = |lo: u64,
                     hi: u64,
                     prev: Option<u128>,
                     next: Option<u128>,
                     out: &mut SlotRuns|
     -> Result<(), String> {
        let mut inside = prev.and_then(|p| class_from_ring(lo, p));
        if inside.is_none() {
            inside = next.and_then(|n| class_from_ring(hi - unit, n));
        }
        let inside = match inside {
            Some(inside) => inside,
            None => point_in_prepared_polygon(
                to_cartesian(cell_to_spherical(slot_to_cell(lo, resolution))?),
                boundary.prep,
            ),
        };
        if inside {
            append_slot_run(out, lo, hi);
        }
        Ok(())
    };
    let n_band = slots.len();
    let mut i = 0;
    let mut q: u64 = 0;
    while q < 60 {
        // Skip straight to the next quintant holding band cells, unless whole ones may be inside
        if !cap_holds_quintant {
            if i >= n_band {
                break;
            }
            q = (slots[i] >> (QUINTANT_SHIFT + 2)) as u64;
        }
        let q_end = (q + 1) << QUINTANT_SHIFT;
        let mut cursor = q << QUINTANT_SHIFT;
        if i >= n_band || (slots[i] >> 2) as u64 >= q_end {
            if cap_holds_quintant {
                probe_run(cursor, q_end, None, None, &mut out)?;
            }
            q += 1;
            continue;
        }
        let mut prev: Option<u128> = None;
        while i < n_band && ((slots[i] >> 2) as u64) < q_end {
            let flagged = slots[i];
            let slot = (flagged >> 2) as u64;
            if slot > cursor {
                probe_run(cursor, slot, prev, Some(flagged), &mut out)?;
            }
            if flagged & EMIT != 0 {
                append_slot_run(&mut out, slot, slot + unit);
            }
            prev = Some(flagged);
            cursor = slot + unit;
            i += 1;
        }
        if cursor < q_end {
            probe_run(cursor, q_end, prev, None, &mut out)?;
        }
        q += 1;
    }

    Ok(out)
}
