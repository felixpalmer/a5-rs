// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

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
