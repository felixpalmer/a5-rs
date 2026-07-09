// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

// The ORIGINAL A5 curve (the shift_digits construction), expressed on top of the
// L-system machinery — bit-for-bit compatible with the pre-L-system library.
//
// The old construction is two layers, and both are preserved here exactly:
//
// 1. The base ordering is a simple two-motif L-system (the "original quaternary
//    curve" the A5 curve grew out of), verified index-for-index equal to the
//    old raw descent. Its native form uses ±60° turns
//    (X: X-Y+X++Y--  Y: Y+X-Y--X++, draws X -> E, Y -> -e+), but it re-gauges
//    into the 180°-only form the table compiler requires — the ±60° is
//    absorbed into Z's leaf gauge (a walk-identical re-gauging):
//      W: W+++Z---WZ   Z: Z+++W---ZW    (draws W -> E, Z -> +e-)
// 2. On top of it, the shift_digits digit recode (ported verbatim below), which
//    rearranges children so they overlap their parent cells — the "hierarchy
//    fix" that introduced the self-intersections the new curve removes.
//
// Orientations follow the old engine exactly: reverse remaps s -> N-1-s,
// flipIJ (uw/wu) selects the flipped pattern and mirrors the raw cell
// (x <-> z in triple space), invertJ (vw/wv) flips the quintant vertically
// ((x,y,z) -> (y-(n-1), x+(n-1), z), n = 2^res). Both maps are self-inverse.

use std::collections::HashMap;
use std::sync::LazyLock;

use crate::coordinate_systems::IJ;
use crate::lattice::lsystem::tables::{compile_grammar, CurveTables, POW2};
use crate::lattice::lsystem::{
    a5_triple_to_flavor, ab_to_triple, axiom_leaf_cell, axiom_target_to_s, triple_to_ab, Cell,
};
use crate::lattice::types::{Orientation, Triple};

/// The compiled two-motif grammar of the original curve (W/Z gauge).
static ORIGINAL: LazyLock<CurveTables> = LazyLock::new(|| {
    let rules: HashMap<char, String> = [('W', "W+++Z---WZ"), ('Z', "Z+++W---ZW")]
        .iter()
        .map(|(k, v)| (*k, v.to_string()))
        .collect();
    let draws: HashMap<char, String> = [('W', "E"), ('Z', "+e-")]
        .iter()
        .map(|(k, v)| (*k, v.to_string()))
        .collect();
    compile_grammar(&rules, &draws)
});

/// The W axiom motif index, resolved once (not per call).
static AXIOM_W: LazyLock<usize> = LazyLock::new(|| ORIGINAL.motif_idx[&'W']);

// ---------- shift_digits (ported verbatim from the original construction) ----------
// Patterns used to rearrange the cells when shifting. This adjusts the layout
// so that children always overlap with their parent cells.
fn reverse_pattern(pattern: &[usize; 8]) -> [usize; 8] {
    let mut out = [0usize; 8];
    for (i, slot) in out.iter_mut().enumerate() {
        *slot = pattern.iter().position(|&p| p == i).unwrap();
    }
    out
}

const PATTERN: [usize; 8] = [0, 1, 3, 4, 5, 6, 7, 2];
const PATTERN_FLIPPED: [usize; 8] = [0, 1, 2, 7, 3, 4, 5, 6];
static PATTERN_REVERSED: LazyLock<[usize; 8]> = LazyLock::new(|| reverse_pattern(&PATTERN));
static PATTERN_FLIPPED_REVERSED: LazyLock<[usize; 8]> =
    LazyLock::new(|| reverse_pattern(&PATTERN_FLIPPED));

fn shift_digits(
    digits: &mut [u8],
    i: usize,
    flips: [i32; 2],
    invert_j: bool,
    pattern: &[usize; 8],
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
    // The rule is flipped depending on the orientation, specifically on the value of invertJ
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
    digits[i - 1] = (dst % 4) as u8;
    digits[i] = ((parent_k as i32 + 4 + (dst / 4) as i32 - (src / 4) as i32) % 4) as u8;
}

// the flips product accumulates per digit exactly as quaternary_to_flips did:
// digit 1 flips the second component, digit 3 the first
fn apply_digit_flips(flips: &mut [i32; 2], d: u8) {
    if d == 1 {
        flips[1] = -flips[1];
    } else if d == 3 {
        flips[0] = -flips[0];
    }
}

/// old s digits -> geometric (X/Y curve) digits, in place. LSB-first array.
fn forward_shift(digits: &mut [u8], invert_j: bool, flip_ij: bool) {
    let pattern = if flip_ij { &PATTERN_FLIPPED } else { &PATTERN };
    let mut flips: [i32; 2] = [1, 1];
    for i in (0..digits.len()).rev() {
        shift_digits(digits, i, flips, invert_j, pattern);
        apply_digit_flips(&mut flips, digits[i]);
    }
}

/// geometric (X/Y curve) digits -> old s digits, in place. LSB-first array.
/// The flips state starts as the product over ALL digits and each iteration
/// cancels digit i's contribution — so at step i it holds the product of the
/// digits ABOVE i, matching the forward pass's state at the same level.
fn inverse_shift(digits: &mut [u8], invert_j: bool, flip_ij: bool) {
    let pattern: &[usize; 8] = if flip_ij {
        &PATTERN_FLIPPED_REVERSED
    } else {
        &PATTERN_REVERSED
    };
    let mut flips: [i32; 2] = [1, 1];
    for &d in digits.iter() {
        apply_digit_flips(&mut flips, d);
    }
    for i in 0..digits.len() {
        apply_digit_flips(&mut flips, digits[i]);
        shift_digits(digits, i, flips, invert_j, pattern);
    }
}

// Quaternary digits of `s`, LSB-first, into a stack buffer (no heap alloc on
// the hot path). At most 32 base-4 digits fit a u64; resolution stays <= 31.
fn digits_of(s: u64, resolution: usize) -> ([u8; 33], usize) {
    let mut digits = [0u8; 33];
    let mut v = s;
    let mut n = 0;
    while v > 0 || n < resolution {
        digits[n] = (v & 3) as u8;
        v >>= 2;
        n += 1;
    }
    (digits, n)
}

fn pack_digits(digits: &[u8]) -> u64 {
    let mut s = 0u64;
    for i in (0..digits.len()).rev() {
        s = (s << 2) | digits[i] as u64;
    }
    s
}

// ---------- orientations (as in the old engine) ----------
struct CompatRecipe {
    reverse: bool,
    invert_j: bool,
    flip_ij: bool,
}

fn compat_orient(o: Orientation) -> CompatRecipe {
    match o {
        Orientation::UV => CompatRecipe {
            reverse: false,
            invert_j: false,
            flip_ij: false,
        },
        Orientation::VU => CompatRecipe {
            reverse: true,
            invert_j: false,
            flip_ij: false,
        },
        Orientation::UW => CompatRecipe {
            reverse: false,
            invert_j: false,
            flip_ij: true,
        },
        Orientation::WU => CompatRecipe {
            reverse: true,
            invert_j: false,
            flip_ij: true,
        },
        Orientation::VW => CompatRecipe {
            reverse: true,
            invert_j: true,
            flip_ij: false,
        },
        Orientation::WV => CompatRecipe {
            reverse: false,
            invert_j: true,
            flip_ij: false,
        },
    }
}

/// Old-curve position `s` -> triple coordinate, via the ORIGINAL (W/Z) forward
/// descent + shiftDigits recode. No flavor (that needs a second, A5, descent).
pub fn compat_s_to_triple(s: u64, resolution: usize, orientation: Orientation) -> Triple {
    let rec = compat_orient(orientation);
    let v = if rec.reverse {
        (1u64 << (2 * resolution)) - 1 - s
    } else {
        s
    };
    let (mut digits, len) = digits_of(v, resolution);
    forward_shift(&mut digits[..len], rec.invert_j, rec.flip_ij);
    let raw = axiom_leaf_cell(&ORIGINAL, pack_digits(&digits[..len]), resolution, *AXIOM_W);
    let mut triple = ab_to_triple(raw.a, raw.b);
    if rec.flip_ij {
        triple = Triple::new(triple.z, triple.y, triple.x);
    }
    if rec.invert_j {
        let n1 = POW2[resolution] as i32 - 1;
        triple = Triple::new(triple.y - n1, triple.x + n1, triple.z);
    }
    triple
}

/// Old-curve position `s` -> cell (triple + pentagon flavor).
pub fn compat_s_to_cell(s: u64, resolution: usize, orientation: Orientation) -> Cell {
    let triple = compat_s_to_triple(s, resolution, orientation);
    // The X/Y walk hosts every cell via a diagonal (E/e) segment, so its leaf
    // state cannot distinguish all four pentagon flavors — that missing bit is
    // exactly why the original engine carried its fractal flips field. The
    // flavor is a per-cell geometric property, so read it off the A5 descent.
    let flavor = a5_triple_to_flavor(&triple, resolution);
    Cell { triple, flavor }
}

/// Triple -> old-curve position `s`, or None if the triple has invalid parity.
pub fn compat_triple_to_s(t: &Triple, resolution: usize, orientation: Orientation) -> Option<u64> {
    let sum = t.x + t.y + t.z;
    if sum != 0 && sum != 1 {
        return None;
    }
    let rec = compat_orient(orientation);
    let n = 1u64 << (2 * resolution);
    let mut raw = *t;
    if rec.invert_j {
        let n1 = POW2[resolution] as i32 - 1;
        raw = Triple::new(raw.y - n1, raw.x + n1, raw.z);
    }
    if rec.flip_ij {
        raw = Triple::new(raw.z, raw.y, raw.x);
    }
    let (ab_a, ab_b) = triple_to_ab(&raw);
    let s_geo = axiom_target_to_s(&ORIGINAL, ab_a, ab_b, resolution, *AXIOM_W, true).0;
    let (mut digits, len) = digits_of(s_geo, resolution);
    inverse_shift(&mut digits[..len], rec.invert_j, rec.flip_ij);
    let v = pack_digits(&digits[..len]);
    Some(if rec.reverse { n - 1 - v } else { v })
}

/// Fractional IJ point -> old-curve position `s` of the containing cell.
pub fn compat_ij_to_s(ij: IJ, resolution: usize, orientation: Orientation) -> u64 {
    let rec = compat_orient(orientation);
    let n = 1u64 << (2 * resolution);
    let mut i = ij.x();
    let mut j = ij.y();
    if rec.flip_ij {
        std::mem::swap(&mut i, &mut j);
    }
    if rec.invert_j {
        j = POW2[resolution] - (i + j);
    }
    let s_geo = axiom_target_to_s(
        &ORIGINAL,
        12.0 * (i + j),
        -12.0 * j,
        resolution,
        *AXIOM_W,
        false,
    )
    .0;
    let (mut digits, len) = digits_of(s_geo, resolution);
    inverse_shift(&mut digits[..len], rec.invert_j, rec.flip_ij);
    let v = pack_digits(&digits[..len]);
    if rec.reverse {
        n - 1 - v
    } else {
        v
    }
}
