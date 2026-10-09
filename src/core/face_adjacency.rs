// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use crate::core::constants::TWO_PI_OVER_5;
use crate::core::pentagon::basis;
use crate::core::utils::OriginId;
use crate::lattice::Triple;

// Computed empirically from cell boundary vertex sharing at resolution 4.

// Face adjacency table: FACE_ADJACENCY[originId][quintant] = (adjOriginId, adjQuintant)
//
// For each quintant on each face, this gives the primary adjacent face/quintant
// that shares the base edge.
pub const FACE_ADJACENCY: [[(u8, usize); 5]; 12] = [
    [(1, 2), (4, 3), (5, 4), (6, 0), (11, 1)],  // origin 0
    [(2, 3), (4, 4), (0, 0), (11, 0), (10, 1)], // origin 1
    [(9, 2), (3, 0), (4, 0), (1, 0), (10, 0)],  // origin 2
    [(2, 1), (9, 1), (8, 1), (5, 1), (4, 1)],   // origin 3
    [(2, 2), (3, 4), (5, 0), (0, 1), (1, 1)],   // origin 4
    [(4, 2), (3, 3), (8, 0), (6, 1), (0, 2)],   // origin 5
    [(0, 3), (5, 3), (8, 4), (7, 1), (11, 2)],  // origin 6
    [(11, 3), (6, 3), (8, 3), (9, 4), (10, 3)], // origin 7
    [(5, 2), (3, 2), (9, 0), (7, 2), (6, 2)],   // origin 8
    [(8, 2), (3, 1), (2, 0), (10, 4), (7, 3)],  // origin 9
    [(2, 4), (1, 4), (11, 4), (7, 4), (9, 3)],  // origin 10
    [(1, 3), (0, 4), (6, 4), (7, 0), (10, 2)],  // origin 11
];

// The seam across a base edge. Unfolded about the edge, quintant q of a face
// and quintant FACE_ADJACENCY[origin_id][q].1 of its neighbor continue one
// lattice: in each quintant's own lattice coordinates (IJ, in face units) the
// point ij of one is the point (1, 1) - ij of the other, a half-turn about the
// edge's midpoint. `seam_transform` and `seam_triple` are this map in face
// coordinates and on cells.

/// The map from a face's frame into the frame of its neighbor across the base
/// edge of `quintant`, as [a, b, c, d, tx, ty] taking (x, y) to
/// (a x + c y + tx, b x + d y + ty). With R(k) the rotation of quintant k into
/// place and q' the neighbor's quintant, it is p -> R(q') (BASIS (1, 1) - R(-q) p).
pub fn seam_transform(origin_id: OriginId, quintant: usize) -> [f64; 6] {
    let angle = TWO_PI_OVER_5.get() * FACE_ADJACENCY[origin_id as usize][quintant].1 as f64;
    let (sin, cos) = (angle.sin(), angle.cos());
    // The linear part is R(q') R(-q) negated: a half-turn plus the change of quintant
    let turn = angle - TWO_PI_OVER_5.get() * quintant as f64;
    let b = basis();
    let ex = b.m00 + b.m01;
    let ey = b.m10 + b.m11;
    [
        -turn.cos(),
        -turn.sin(),
        turn.sin(),
        -turn.cos(),
        cos * ex - sin * ey,
        sin * ex + cos * ey,
    ]
}

/// A cell's image across the seam at its quintant's base edge (`max_row` is its
/// own), in the neighbor quintant's triples. The half-turn maps the lattice to
/// itself, so the image is a cell: its pentagon is the cell's own turned
/// half-way round, which flips the flavor's parity bit. It lies just outside
/// the neighbor quintant; its neighbors inside it are the cell's neighbors
/// across the seam.
#[inline]
pub fn seam_triple(t: &Triple, max_row: i32) -> Triple {
    Triple::new(-max_row - t.x, 2 * max_row + 1 - t.y, -max_row - t.z)
}

/// Breadth-first walk over the 12 dodecahedron faces (the resolution 0 cells),
/// adjacent across their edges. Starts from `seeds`, which are always expanded;
/// every other face is visited once and expanded only if `expand(face)` is true.
/// Stops after `max_rings` rings.
///
/// Returns every face reached, seeds first.
pub fn walk_faces(
    seeds: &[u8],
    mut expand: impl FnMut(u8) -> Result<bool, String>,
    max_rings: usize,
) -> Result<Vec<u8>, String> {
    let mut reached: Vec<u8> = Vec::with_capacity(12);
    for &seed in seeds {
        if !reached.contains(&seed) {
            reached.push(seed);
        }
    }
    let mut frontier = reached.clone();
    let mut ring = 0;
    while ring < max_rings && !frontier.is_empty() {
        let mut next: Vec<u8> = Vec::new();
        for &id in &frontier {
            for &(face, _) in &FACE_ADJACENCY[id as usize] {
                if reached.contains(&face) {
                    continue;
                }
                reached.push(face);
                if expand(face)? {
                    next.push(face);
                }
            }
        }
        frontier = next;
        ring += 1;
    }
    Ok(reached)
}
