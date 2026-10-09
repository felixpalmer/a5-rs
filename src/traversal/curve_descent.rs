// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

// A region found by descending the cell hierarchy: a cell wholly inside is kept
// whole, one wholly outside is dropped, and the rest split, so the work follows
// the region's boundary rather than its area. The descent runs in curve order,
// stepping the curve one digit per level (see lattice curve_child), so the cells
// come out as sorted slot runs, with no cell IDs to encode and nothing to sort.

use crate::collections::slot_runs::append_slot_run;
use crate::collections::SlotRuns;
use crate::coordinate_systems::Face;
use crate::core::serialization::{FIRST_HILBERT_RESOLUTION, SLOT_COUNTS};
use crate::core::tiling::get_pentagon_center;
use crate::core::utils::OriginId;
use crate::lattice::{curve_child, triple_to_curve_node, CurveNode, Orientation, Triple};
use crate::traversal::triple_cells::QUINTANT_TABLES;

/// How a cell lies relative to the region (see `descend_in_curve_order`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurveDescentClass {
    /// No cell at the target resolution in the cell is in the region: drop it
    Outside,
    /// All its cells at the target resolution are in the region: keep it whole
    Inside,
    /// Descend into its children
    Split,
}

// The descent's inputs, shared by every level of `descend`
struct Descent<'a, F> {
    classify: F,
    /// Hilbert level of the target resolution
    target_level: usize,
    origin_id: OriginId,
    quintant: usize,
    orientation: Orientation,
    /// Output, as sorted slot runs
    runs: &'a mut SlotRuns,
}

/// Descend from `starts` — non-overlapping cells at one Hilbert level, as
/// triples (origin_id, quintant, x, y, z), in any order — to `resolution`,
/// appending the cells of the region `classify` describes to `runs`, as slot
/// runs in curve order.
///
/// `classify(origin_id, resolution, center, slot)` classifies a cell of the
/// descent by its center, given in its face's frame, and its first slot. At the
/// target resolution it must decide, `Inside` or `Outside`.
pub fn descend_in_curve_order<F>(
    starts: &[[i32; 5]],
    start_level: usize,
    resolution: i32,
    classify: F,
    runs: &mut SlotRuns,
) -> Result<(), String>
where
    F: FnMut(OriginId, i32, Face, u64) -> Result<CurveDescentClass, String>,
{
    let tables = &*QUINTANT_TABLES;
    let shift = 58 - 2 * start_level as u32;
    let mut states: Vec<(u64, u8, CurveNode, usize)> = Vec::with_capacity(starts.len());
    for (i, c) in starts.iter().enumerate() {
        let q = (c[0] * 5 + c[1]) as usize;
        let triple = Triple::new(c[2], c[3], c[4]);
        let (s, flavor, node) = triple_to_curve_node(&triple, start_level, tables.orientation[q]);
        states.push((tables.prefix[q] | (s << shift), flavor, node, i));
    }
    states.sort_unstable_by_key(|state| state.0);

    let mut d = Descent {
        classify,
        target_level: (resolution - FIRST_HILBERT_RESOLUTION + 1) as usize,
        origin_id: 0,
        quintant: 0,
        orientation: Orientation::UV,
        runs,
    };
    for &(slot, flavor, node, i) in &states {
        let c = starts[i];
        d.origin_id = c[0] as OriginId;
        d.quintant = c[1] as usize;
        d.orientation = tables.orientation[(c[0] * 5 + c[1]) as usize];
        let triple = Triple::new(c[2], c[3], c[4]);
        descend(&mut d, start_level, &triple, flavor, &node, slot)?;
    }
    Ok(())
}

/// Visit the cell at Hilbert `level`, with its triple, flavor, the descent state
/// below it and its first slot.
fn descend<F>(
    d: &mut Descent<F>,
    level: usize,
    triple: &Triple,
    flavor: u8,
    node: &CurveNode,
    slot: u64,
) -> Result<(), String>
where
    F: FnMut(OriginId, i32, Face, u64) -> Result<CurveDescentClass, String>,
{
    let resolution = level as i32 + FIRST_HILBERT_RESOLUTION - 1;
    let center = get_pentagon_center(level as i32, d.quintant, triple, flavor);
    match (d.classify)(d.origin_id, resolution, center, slot)? {
        CurveDescentClass::Inside => {
            append_slot_run(d.runs, slot, slot + SLOT_COUNTS[resolution as usize]);
            return Ok(());
        }
        CurveDescentClass::Outside => return Ok(()),
        CurveDescentClass::Split if level == d.target_level => return Ok(()),
        CurveDescentClass::Split => {}
    }
    // The children's slots follow one another in curve order
    let child_slots = SLOT_COUNTS[resolution as usize + 1];
    let mut child_slot = slot;
    for digit in 0..4 {
        let (child, below) = curve_child(node, digit, level + 1, d.orientation);
        descend(
            d,
            level + 1,
            &child.triple,
            child.flavor,
            &below,
            child_slot,
        )?;
        child_slot += child_slots;
    }
    Ok(())
}
