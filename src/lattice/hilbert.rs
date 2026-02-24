// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use crate::coordinate_systems::{IJ, KJ};
use crate::lattice::basis::kj_to_ij;
use crate::lattice::quaternary::{ij_to_quaternary, quaternary_to_flips, quaternary_to_kj, ZERO};
use crate::lattice::shift_digits::{
    shift_digits, PATTERN, PATTERN_FLIPPED, PATTERN_FLIPPED_REVERSED, PATTERN_REVERSED,
};
use crate::lattice::types::{Anchor, Flip, Orientation, Quaternary, NO, YES};

const FLIP_SHIFT: IJ = IJ(crate::coordinate_systems::vec2::Vec2 { x: -1.0, y: 1.0 });

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

    let q = digits.first().copied().unwrap_or(0);

    Anchor {
        flips,
        q,
        offset: kj_to_ij(offset),
    }
}

/// Get the number of digits needed to represent the offset
/// As we don't know the flips we need to add 2 to include the next row
pub fn get_required_digits(offset: IJ) -> usize {
    let index_sum = offset.x().ceil() + offset.y().ceil();
    if index_sum == 0.0 {
        return 1;
    }
    1 + (index_sum.log2().floor() as usize)
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

/// Compute flip states from IJ coordinates.
pub fn ij_to_flips(input: IJ, resolution: usize) -> [Flip; 2] {
    let num_digits = resolution;
    let mut flips = [NO, NO];
    let mut pivot = IJ::new(0.0, 0.0);

    for i in (0..num_digits).rev() {
        let relative_offset = IJ::new(input.x() - pivot.x(), input.y() - pivot.y());
        let scale = 1.0 / (1u64 << i) as f64;
        let scaled_offset = IJ::new(relative_offset.x() * scale, relative_offset.y() * scale);

        let digit = ij_to_quaternary(scaled_offset, flips);

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

    flips
}

// Precomputed probe offsets for anchor_to_s(), indexed by flip combination.
// Index = (1 - flip0) + (1 - flip1) / 2
const PROBE_R: f64 = 0.1;

lazy_static::lazy_static! {
    static ref PROBE_OFFSETS: [(f64, f64); 4] = {
        let angles = [45.0_f64, 113.0, 293.0, 225.0];
        let mut offsets = [(0.0, 0.0); 4];
        for (i, &angle_deg) in angles.iter().enumerate() {
            let angle_rad = angle_deg * std::f64::consts::PI / 180.0;
            offsets[i] = (PROBE_R * angle_rad.cos(), PROBE_R * angle_rad.sin());
        }
        offsets
    };
}

/// Convert an anchor to an s-value using a single targeted fractional offset probe.
pub fn anchor_to_s(anchor: &Anchor, resolution: usize, orientation: Orientation) -> u64 {
    let i = anchor.offset.x();
    let j = anchor.offset.y();
    let probe_idx = ((1 - anchor.flips[0]) + (1 - anchor.flips[1]) / 2) as usize;
    let probe_offset = PROBE_OFFSETS[probe_idx];
    ij_to_s(
        IJ::new(i + probe_offset.0, j + probe_offset.1),
        resolution,
        orientation,
    )
}
