// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use crate::lattice::types::{Flip, Quaternary};

// Patterns used to rearrange the cells when shifting. This adjusts the layout so that
// children always overlap with their parent cells.
pub fn reverse_pattern(pattern: &[usize]) -> Vec<usize> {
    let mut result = vec![0; pattern.len()];
    for (i, &val) in pattern.iter().enumerate() {
        result[val] = i;
    }
    result
}

pub const PATTERN: [usize; 8] = [0, 1, 3, 4, 5, 6, 7, 2];
pub const PATTERN_FLIPPED: [usize; 8] = [0, 1, 2, 7, 3, 4, 5, 6];

lazy_static::lazy_static! {
    pub static ref PATTERN_REVERSED: Vec<usize> = reverse_pattern(&PATTERN);
    pub static ref PATTERN_FLIPPED_REVERSED: Vec<usize> = reverse_pattern(&PATTERN_FLIPPED);
}

pub fn shift_digits(
    digits: &mut [Quaternary],
    i: usize,
    flips: [Flip; 2],
    invert_j: bool,
    pattern: &[usize],
) {
    if i == 0 {
        return;
    }

    let parent_k = digits[i];
    let child_k = digits[i - 1];
    let f = flips[0] + flips[1];

    // Detect when cells need to be shifted
    let needs_shift: bool;
    let first: bool;

    // The value of F which cells need to be shifted
    // The rule is flipped depending on the orientation, specifically on the value of invert_j
    if invert_j != (f == 0) {
        needs_shift = parent_k == 1 || parent_k == 2; // Second & third pentagons only
        first = parent_k == 1; // Second pentagon is first
    } else {
        needs_shift = parent_k < 2; // First two pentagons only
        first = parent_k == 0; // First pentagon is first
    }

    if !needs_shift {
        return;
    }

    // Apply the pattern by setting the digits based on the value provided
    let src = if first {
        child_k as usize
    } else {
        child_k as usize + 4
    };
    let dst = pattern[src];
    digits[i - 1] = (dst % 4) as Quaternary;
    digits[i] = ((parent_k as usize + 4 + dst / 4 - src / 4) % 4) as Quaternary;
}
