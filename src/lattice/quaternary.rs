// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use crate::coordinate_systems::{IJ, KJ};
use crate::lattice::types::{Flip, Quaternary, NO, YES};

// Using KJ allows simplification of definitions
pub const K_POS: KJ = KJ(crate::coordinate_systems::vec2::Vec2 { x: 1.0, y: 0.0 }); // k
pub const J_POS: KJ = KJ(crate::coordinate_systems::vec2::Vec2 { x: 0.0, y: 1.0 }); // j
pub const K_NEG: KJ = KJ(crate::coordinate_systems::vec2::Vec2 { x: -1.0, y: 0.0 });
pub const J_NEG: KJ = KJ(crate::coordinate_systems::vec2::Vec2 { x: 0.0, y: -1.0 });
pub const ZERO: KJ = KJ(crate::coordinate_systems::vec2::Vec2 { x: 0.0, y: 0.0 });

pub fn quaternary_to_kj(n: Quaternary, flips: [Flip; 2]) -> KJ {
    let [flip_x, flip_y] = flips;

    // Indirection to allow for flips
    let (p, q) = match (flip_x, flip_y) {
        (NO, NO) => (K_POS, J_POS),
        (YES, NO) => (J_NEG, K_NEG),  // Swap and negate
        (NO, YES) => (J_POS, K_POS),  // Swap only
        (YES, YES) => (K_NEG, J_NEG), // Negate only
        _ => panic!("Invalid flip values"),
    };

    match n {
        0 => ZERO,                                              // Length 0
        1 => p,                                                 // Length 1
        2 => KJ::new(q.x() + p.x(), q.y() + p.y()),             // Length SQRT2
        3 => KJ::new(q.x() + 2.0 * p.x(), q.y() + 2.0 * p.y()), // Length SQRT5
        _ => panic!("Invalid Quaternary value: {}", n),
    }
}

pub fn quaternary_to_flips(n: Quaternary) -> [Flip; 2] {
    match n {
        0 => [NO, NO],
        1 => [NO, YES],
        2 => [NO, NO],
        3 => [YES, NO],
        _ => panic!("Invalid Quaternary value: {}", n),
    }
}

/// This function uses the ij basis, unlike its inverse!
pub fn ij_to_quaternary(ij: IJ, flips: [Flip; 2]) -> Quaternary {
    let u = ij.x();
    let v = ij.y();

    // Boundaries to compare against
    let a = if flips[0] == YES { -(u + v) } else { u + v };
    let b = if flips[1] == YES { -u } else { u };
    let c = if flips[0] == YES { -v } else { v };

    // Only one flip
    if flips[0] + flips[1] == 0 {
        if c < 1.0 {
            0
        } else if b > 1.0 {
            3
        } else if a > 1.0 {
            2
        } else {
            1
        }
    // No flips or both
    } else if a < 1.0 {
        0
    } else if b > 1.0 {
        3
    } else if c > 1.0 {
        2
    } else {
        1
    }
}
