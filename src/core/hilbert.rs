// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use crate::coordinate_systems::{IJ, KJ};

pub type Quaternary = u8; // 0, 1, 2, 3

pub const YES: i8 = -1;
pub const NO: i8 = 1;
pub type Flip = i8;

#[derive(Debug, Clone, PartialEq)]
pub struct Anchor {
    pub k: Quaternary,
    pub offset: IJ,
    pub flips: [Flip; 2],
}

/// Anchor offset is specified in ij units, the eigenbasis of the Hilbert curve
/// Define k as the vector i + j, as it means vectors u & v are of unit length
pub fn ij_to_kj(ij: IJ) -> KJ {
    KJ::new(ij.x() + ij.y(), ij.y())
}

pub fn kj_to_ij(kj: KJ) -> IJ {
    IJ::new(kj.x() - kj.y(), kj.y())
}

/// Orientation of the Hilbert curve. The curve fills a space defined by the triangle with vertices
/// u, v & w. The orientation describes which corner the curve starts and ends at, e.g. wv is a
/// curve that starts at w and ends at v.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Orientation {
    UV,
    VU,
    UW,
    WU,
    VW,
    WV,
}

impl Orientation {
    #[allow(dead_code)]
    fn from_str(s: &str) -> Option<Self> {
        match s {
            "uv" => Some(Self::UV),
            "vu" => Some(Self::VU),
            "uw" => Some(Self::UW),
            "wu" => Some(Self::WU),
            "vw" => Some(Self::VW),
            "wv" => Some(Self::WV),
            _ => None,
        }
    }
}

// Using KJ allows simplification of definitions
const K_POS: KJ = KJ(crate::coordinate_systems::vec2::Vec2 { x: 1.0, y: 0.0 }); // k
const J_POS: KJ = KJ(crate::coordinate_systems::vec2::Vec2 { x: 0.0, y: 1.0 }); // j
const K_NEG: KJ = KJ(crate::coordinate_systems::vec2::Vec2 { x: -1.0, y: 0.0 });
const J_NEG: KJ = KJ(crate::coordinate_systems::vec2::Vec2 { x: 0.0, y: -1.0 });
const ZERO: KJ = KJ(crate::coordinate_systems::vec2::Vec2 { x: 0.0, y: 0.0 });

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

const FLIP_SHIFT: IJ = IJ(crate::coordinate_systems::vec2::Vec2 { x: -1.0, y: 1.0 });

// Patterns used to rearrange the cells when shifting. This adjusts the layout so that
// children always overlap with their parent cells.
fn reverse_pattern(pattern: &[usize]) -> Vec<usize> {
    let mut result = vec![0; pattern.len()];
    for (i, &val) in pattern.iter().enumerate() {
        result[val] = i;
    }
    result
}

const PATTERN: [usize; 8] = [0, 1, 3, 4, 5, 6, 7, 2];
const PATTERN_FLIPPED: [usize; 8] = [0, 1, 2, 7, 3, 4, 5, 6];

lazy_static::lazy_static! {
    static ref PATTERN_REVERSED: Vec<usize> = reverse_pattern(&PATTERN);
    static ref PATTERN_FLIPPED_REVERSED: Vec<usize> = reverse_pattern(&PATTERN_FLIPPED);
}

fn shift_digits(
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

pub fn s_to_anchor(s: u64, resolution: usize, orientation: Orientation) -> Anchor {
    let input = s;
    let reverse = matches!(
        orientation,
        Orientation::VU | Orientation::WU | Orientation::VW
    );
    let invert_j = matches!(orientation, Orientation::WV | Orientation::VW);
    let flip_ij = matches!(orientation, Orientation::WU | Orientation::UW);

    let adjusted_input = if reverse {
        (1u64 << (2 * resolution)) - input - 1
    } else {
        input
    };

    let mut anchor = s_to_anchor_internal(adjusted_input, resolution, invert_j, flip_ij);

    if flip_ij {
        let i = anchor.offset.x();
        let j = anchor.offset.y();
        anchor.offset = IJ::new(j, i);

        // The flips moved the origin of the cell, shift to compensate
        if anchor.flips[0] == YES {
            anchor.offset = IJ::new(
                anchor.offset.x() + FLIP_SHIFT.x(),
                anchor.offset.y() + FLIP_SHIFT.y(),
            );
        }
        if anchor.flips[1] == YES {
            anchor.offset = IJ::new(
                anchor.offset.x() - FLIP_SHIFT.x(),
                anchor.offset.y() - FLIP_SHIFT.y(),
            );
        }
    }

    if invert_j {
        let i = anchor.offset.x();
        let j = anchor.offset.y();
        let new_j = (1 << resolution) as f64 - (i + j);
        anchor.flips[0] = -anchor.flips[0];
        anchor.offset = IJ::new(i, new_j);
    }

    anchor
}

pub fn s_to_anchor_internal(s: u64, resolution: usize, invert_j: bool, flip_ij: bool) -> Anchor {
    let mut offset = ZERO;
    let mut flips = [NO, NO];
    let mut input = s;

    // Get all quaternary digits first
    let mut digits = Vec::new();
    while input > 0 || digits.len() < resolution {
        digits.push((input % 4) as Quaternary);
        input >>= 2;
    }

    let pattern = if flip_ij { &PATTERN_FLIPPED } else { &PATTERN };

    // Process digits from left to right (most significant first)
    for i in (0..digits.len()).rev() {
        shift_digits(&mut digits, i, flips, invert_j, pattern);
        let next_flips = quaternary_to_flips(digits[i]);
        flips[0] *= next_flips[0];
        flips[1] *= next_flips[1];
    }

    flips = [NO, NO]; // Reset flips for the next loop
    for i in (0..digits.len()).rev() {
        // Scale up existing anchor
        offset = KJ::new(offset.x() * 2.0, offset.y() * 2.0);

        // Get child anchor and combine with current anchor
        let child_offset = quaternary_to_kj(digits[i], flips);
        offset = KJ::new(offset.x() + child_offset.x(), offset.y() + child_offset.y());

        let next_flips = quaternary_to_flips(digits[i]);
        flips[0] *= next_flips[0];
        flips[1] *= next_flips[1];
    }

    let k = digits.first().copied().unwrap_or(0);

    Anchor {
        flips,
        k,
        offset: kj_to_ij(offset),
    }
}

/// Get the number of digits needed to represent the offset
/// As we don't know the flips we need to add 2 to include the next row
pub fn get_required_digits(offset: IJ) -> usize {
    let index_sum = offset.x().ceil() + offset.y().ceil(); // TODO perhaps use floor instead
    if index_sum == 0.0 {
        return 1;
    }
    1 + (index_sum.log2().floor() as usize)
}

/// This function uses the ij basis, unlike its inverse!
pub fn ij_to_quaternary(ij: IJ, flips: [Flip; 2]) -> Quaternary {
    let u = ij.x();
    let v = ij.y();
    let digit: Quaternary;

    // Boundaries to compare against
    let a = if flips[0] == YES { -(u + v) } else { u + v };
    let b = if flips[1] == YES { -u } else { u };
    let c = if flips[0] == YES { -v } else { v };

    // Only one flip
    if flips[0] + flips[1] == 0 {
        if c < 1.0 {
            digit = 0;
        } else if b > 1.0 {
            digit = 3;
        } else if a > 1.0 {
            digit = 2;
        } else {
            digit = 1;
        }
    // No flips or both
    } else if a < 1.0 {
        digit = 0;
    } else if b > 1.0 {
        digit = 3;
    } else if c > 1.0 {
        digit = 2;
    } else {
        digit = 1;
    }

    digit
}

pub fn ij_to_s(input: IJ, resolution: usize, orientation: Orientation) -> u64 {
    let reverse = matches!(
        orientation,
        Orientation::VU | Orientation::WU | Orientation::VW
    );
    let invert_j = matches!(orientation, Orientation::WV | Orientation::VW);
    let flip_ij = matches!(orientation, Orientation::WU | Orientation::UW);

    let mut ij = input;
    if flip_ij {
        ij = IJ::new(input.y(), input.x());
    }
    if invert_j {
        let i = ij.x();
        let j = ij.y();
        ij = IJ::new(i, (1 << resolution) as f64 - (i + j));
    }

    let s = ij_to_s_internal(ij, invert_j, flip_ij, resolution);
    if reverse {
        (1u64 << (2 * resolution)) - s - 1
    } else {
        s
    }
}

pub fn ij_to_s_internal(input: IJ, invert_j: bool, flip_ij: bool, resolution: usize) -> u64 {
    // Get number of digits we need to process
    let num_digits = resolution;
    let mut digits = vec![0u8; num_digits];

    let mut flips = [NO, NO];
    let mut pivot = IJ::new(0.0, 0.0);

    // Process digits from left to right (most significant first)
    for i in (0..num_digits).rev() {
        let relative_offset = IJ::new(input.x() - pivot.x(), input.y() - pivot.y());

        let scale = 1.0 / (1u64 << i) as f64;
        let scaled_offset = IJ::new(relative_offset.x() * scale, relative_offset.y() * scale);

        let digit = ij_to_quaternary(scaled_offset, flips);
        digits[i] = digit;

        // Update running state
        let child_offset = kj_to_ij(quaternary_to_kj(digit, flips));
        let upscaled_child_offset = IJ::new(
            child_offset.x() * (1u64 << i) as f64,
            child_offset.y() * (1u64 << i) as f64,
        );
        pivot = IJ::new(
            pivot.x() + upscaled_child_offset.x(),
            pivot.y() + upscaled_child_offset.y(),
        );

        let next_flips = quaternary_to_flips(digit);
        flips[0] *= next_flips[0];
        flips[1] *= next_flips[1];
    }

    let pattern: &[usize] = if flip_ij {
        &PATTERN_FLIPPED_REVERSED
    } else {
        &PATTERN_REVERSED
    };

    for i in 0..digits.len() {
        let next_flips = quaternary_to_flips(digits[i]);
        flips[0] *= next_flips[0];
        flips[1] *= next_flips[1];
        shift_digits(&mut digits, i, flips, invert_j, pattern);
    }

    let mut output = 0u64;
    for (i, &digit) in digits.iter().enumerate().rev() {
        let scale = 1u64 << (2 * i);
        output += (digit as u64) * scale;
    }

    output
}
