// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

// Cells handled in triple space — (origin_id, quintant, x, y, z) — by the
// traversal algorithms that walk many neighboring cells: they key and dedup
// cells as plain integers and encode a cell to its ID only when it is output.

use std::sync::LazyLock;

use crate::coordinate_systems::Spherical;
use crate::core::origin::{get_origins, quintant_to_segment, segment_to_quintant};
use crate::core::serialization::{deserialize, serialize, FIRST_HILBERT_RESOLUTION};
use crate::core::tiling::get_pentagon_center;
use crate::core::utils::A5Cell;
use crate::lattice::{
    s_to_triple, triple_flavor, triple_in_bounds, triple_to_s, Orientation, Triple,
};
use crate::projections::dodecahedron::DodecahedronProjection;
use crate::traversal::lattice_boundary::get_boundary_neighbor_triples;
use crate::traversal::neighbors::NEIGHBOR_DELTAS;

// A cell's key packs the quintant (origin.id * 5 + quintant, < 60), parity,
// and the low KEY_BITS bits of -x and -z (y follows). Up to Hilbert resolution
// 21 the coordinates fit whole; above it, two cells of one quintant share a key
// only if they are 2^22 rows apart, and a walk holding both would need ~2^21
// steps (~10^12 cells for a disk) — far past what fits in memory.
const KEY_BITS: u32 = 22;
const KEY_MASK: i64 = (1 << KEY_BITS) - 1;
const KEY_SIDE: i64 = 1 << KEY_BITS;

/// Segment and curve orientation of each of the 60 quintants, by origin.id * 5 + quintant.
static QUINTANT_SEGMENTS: LazyLock<Vec<(usize, Orientation)>> = LazyLock::new(|| {
    let mut out = Vec::with_capacity(60);
    for origin in get_origins() {
        for q in 0..5 {
            out.push(quintant_to_segment(q, origin));
        }
    }
    out
});

/// The integer key of a cell, unique among the cells of any one traversal.
pub fn triple_cell_key(cell: [i32; 5]) -> i64 {
    let [origin_id, quintant, x, y, z] = cell;
    ((-x as i64 & KEY_MASK) * KEY_SIDE + (-z as i64 & KEY_MASK)) * 2
        + (x + y + z) as i64
        + (origin_id * 5 + quintant) as i64 * 2 * KEY_SIDE * KEY_SIDE
}

/// The cell ID of a cell given in triple space.
pub fn triple_cell_to_id(
    cell: [i32; 5],
    hilbert_res: usize,
    resolution: i32,
) -> Result<u64, String> {
    let [origin_id, quintant, x, y, z] = cell;
    let (segment, orientation) = QUINTANT_SEGMENTS[(origin_id * 5 + quintant) as usize];
    let s = triple_to_s(&Triple::new(x, y, z), hilbert_res, orientation)
        .ok_or("triple_cell_to_id: invalid triple")?;
    serialize(&A5Cell {
        origin_id: origin_id as u8,
        segment,
        s,
        resolution,
    })
}

/// Decode cell IDs (each at resolution 1 or above) into triple space, appending
/// them to `out` as (origin_id, quintant, x, y, z).
pub fn cell_ids_to_triples(
    cell_ids: impl IntoIterator<Item = u64>,
    out: &mut Vec<[i32; 5]>,
) -> Result<(), String> {
    let origins = get_origins();
    for cell_id in cell_ids {
        let cell = deserialize(cell_id)?;
        let origin = &origins[cell.origin_id as usize];
        let (quintant, orientation) = segment_to_quintant(cell.segment, origin);
        let hilbert_res = (cell.resolution - FIRST_HILBERT_RESOLUTION + 1) as usize;
        let t = s_to_triple(cell.s, hilbert_res, orientation);
        out.push([origin.id as i32, quintant as i32, t.x, t.y, t.z]);
    }
    Ok(())
}

/// The center of a cell given in triple space, on the sphere.
pub fn triple_cell_center(
    cell: [i32; 5],
    hilbert_res: usize,
    max_row: i32,
) -> Result<Spherical, String> {
    let triple = Triple::new(cell[2], cell[3], cell[4]);
    let face = get_pentagon_center(
        hilbert_res as i32,
        cell[1] as usize,
        &triple,
        triple_flavor(&triple, max_row),
    );
    DodecahedronProjection::get_thread_local().inverse(face, cell[0] as u8)
}

/// Visit every neighbor of a cell given in triple space: within its quintant the
/// fixed per-flavor triple deltas, and, for a cell on a quintant edge (x = 0,
/// z = 0 or y = max_row), the boundary delta tables. `edge_only` restricts to the
/// 5 edge-sharing neighbors; otherwise the vertex-only neighbors come too. A
/// neighbor may be visited more than once; visitors deduplicate.
pub fn for_each_triple_neighbor(
    cell: [i32; 5],
    max_row: i32,
    edge_only: bool,
    mut visit: impl FnMut([i32; 5]) -> Result<(), String>,
) -> Result<(), String> {
    let [origin_id, quintant, x, y, z] = cell;
    let triple = Triple::new(x, y, z);

    // Within the quintant: the fixed per-flavor deltas
    let flavor = triple_flavor(&triple, max_row) as usize;
    let deltas: &[Triple] = if edge_only {
        &NEIGHBOR_DELTAS[flavor].edge
    } else {
        &NEIGHBOR_DELTAS[flavor].all
    };
    for d in deltas {
        let neighbor = Triple::new(x + d.x, y + d.y, z + d.z);
        if triple_in_bounds(&neighbor, max_row) {
            visit([origin_id, quintant, neighbor.x, neighbor.y, neighbor.z])?;
        }
    }

    // Across a quintant edge: the boundary delta tables
    if x == 0 || z == 0 || y == max_row {
        visit_boundary(cell, max_row, edge_only, false, &mut visit)?;
    }
    Ok(())
}

/// Visit every lattice neighbor of a cell given in triple space: the 3
/// parity-valid single-axis moves within its quintant (the connectivity
/// `triple_space_flood_fill` floods by), and, for a cell on a quintant edge, its
/// edge-sharing boundary neighbors — but not the vertex corner, which the
/// lattice moves don't traverse either. A neighbor may be visited more than
/// once; visitors deduplicate.
pub fn for_each_lattice_neighbor(
    cell: [i32; 5],
    max_row: i32,
    mut visit: impl FnMut([i32; 5]) -> Result<(), String>,
) -> Result<(), String> {
    let [origin_id, quintant, x, y, z] = cell;

    // Within the quintant: +1 on one axis from a parity 0 triple, -1 from parity 1
    let step = if x + y + z == 0 { 1 } else { -1 };
    for (dx, dy, dz) in [(step, 0, 0), (0, step, 0), (0, 0, step)] {
        if triple_in_bounds(&Triple::new(x + dx, y + dy, z + dz), max_row) {
            visit([origin_id, quintant, x + dx, y + dy, z + dz])?;
        }
    }

    // Across a quintant edge: the boundary delta tables
    if x == 0 || z == 0 || y == max_row {
        visit_boundary(cell, max_row, true, true, &mut visit)?;
    }
    Ok(())
}

/// Visit the neighbors of a cell on a quintant edge that lie across it.
fn visit_boundary(
    cell: [i32; 5],
    max_row: i32,
    edge_only: bool,
    skip_corners: bool,
    visit: &mut impl FnMut([i32; 5]) -> Result<(), String>,
) -> Result<(), String> {
    let [origin_id, quintant, x, y, z] = cell;
    let mut boundary: Vec<i32> = Vec::with_capacity(20);
    get_boundary_neighbor_triples(
        Triple::new(x, y, z),
        x + y + z,
        quintant as usize,
        &get_origins()[origin_id as usize],
        max_row,
        edge_only,
        skip_corners,
        &mut boundary,
    );
    for b in boundary.chunks_exact(5) {
        visit([b[0], b[1], b[2], b[3], b[4]])?;
    }
    Ok(())
}
