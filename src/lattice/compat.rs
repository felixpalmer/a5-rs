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
    ab_to_triple, axiom_leaf_cell, axiom_target_to_s, triple_to_ab, Cell,
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
/// Returns the final flips product over the shifted digits — the old engine's
/// anchor `flips` state, from which the pentagon flavor follows in closed form
/// (see `compat_flavor`).
fn forward_shift(digits: &mut [u8], invert_j: bool, flip_ij: bool) -> [i32; 2] {
    let pattern = if flip_ij { &PATTERN_FLIPPED } else { &PATTERN };
    let mut flips: [i32; 2] = [1, 1];
    for i in (0..digits.len()).rev() {
        shift_digits(digits, i, flips, invert_j, pattern);
        apply_digit_flips(&mut flips, digits[i]);
    }
    flips
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

/// Pentagon flavor from the old engine's anchor state: the flips product over
/// the (shifted) digits and the leaf digit `q`. Ported from the old
/// `get_pentagon_vertices` orientation logic: flavor bit 0 (180° rotation)
/// fired iff `flips[1] == YES`; bit 1 (Y reflection) on the `(f, q)` predicate
/// below. This is why the compat decode needs no second (A5) descent — the
/// old engine's own fractal flips field carries the missing flavor bit.
fn compat_flavor(flips: [i32; 2], q: u8) -> u8 {
    let rotate = flips[1] == -1;
    let f = flips[0] + flips[1];
    // Orient last two pentagons when both or neither flips are set,
    // first & last pentagons when exactly one is.
    let reflect = if f == 0 {
        q == 0 || q == 3
    } else {
        q == 2 || q == 3
    };
    (rotate as u8) | ((reflect as u8) << 1)
}

/// Shared forward descent: old s digits -> (triple, anchor flips, leaf digit).
fn compat_descend(s: u64, resolution: usize, rec: &CompatRecipe) -> (Triple, [i32; 2], u8) {
    let v = if rec.reverse {
        (1u64 << (2 * resolution)) - 1 - s
    } else {
        s
    };
    let (mut digits, len) = digits_of(v, resolution);
    let flips = forward_shift(&mut digits[..len], rec.invert_j, rec.flip_ij);
    let raw = axiom_leaf_cell(&ORIGINAL, pack_digits(&digits[..len]), resolution, *AXIOM_W);
    let mut triple = ab_to_triple(raw.a, raw.b);
    if rec.flip_ij {
        triple = Triple::new(triple.z, triple.y, triple.x);
    }
    if rec.invert_j {
        let n1 = POW2[resolution] as i32 - 1;
        triple = Triple::new(triple.y - n1, triple.x + n1, triple.z);
    }
    (triple, flips, digits[0])
}

/// Old-curve position `s` -> triple coordinate, via the ORIGINAL (W/Z) forward
/// descent + shiftDigits recode.
pub fn compat_s_to_triple(s: u64, resolution: usize, orientation: Orientation) -> Triple {
    let rec = compat_orient(orientation);
    compat_descend(s, resolution, &rec).0
}

/// Old-curve position `s` -> cell (triple + pentagon flavor).
pub fn compat_s_to_cell(s: u64, resolution: usize, orientation: Orientation) -> Cell {
    let rec = compat_orient(orientation);
    let (triple, mut flips, q) = compat_descend(s, resolution, &rec);
    // As in the old engine's s_to_anchor: invertJ flips the first component
    // (flipIJ leaves the flips untouched).
    if rec.invert_j {
        flips[0] = -flips[0];
    }
    Cell {
        triple,
        flavor: compat_flavor(flips, q),
    }
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

// ---------- fractional point-location (ported verbatim from the original engine) ----------
// The old engine located a fractional point with a few sign tests per level
// (ij_to_quaternary) — far cheaper than the L-system's per-level hull scan
// (~10-15x less work), and bit-identical by construction including its boundary
// tie-breaks. The resulting digit stream is the geometric (X/Y curve) digit
// stream, so the same inverse_shift recode applies on top.

/// Which of the 4 children contains the scaled offset, under the current flips
/// (the old engine's `ij_to_quaternary`, verbatim).
fn ij_to_quaternary(u: f64, v: f64, flips: [i32; 2]) -> u8 {
    // Boundaries to compare against
    let a = if flips[0] == -1 { -(u + v) } else { u + v };
    let b = if flips[1] == -1 { -u } else { u };
    let c = if flips[0] == -1 { -v } else { v };

    if flips[0] + flips[1] == 0 {
        // Only one flip
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

/// Child anchor offsets in IJ units, indexed by [flip combination][digit]
/// (= the old engine's kj_to_ij(quaternary_to_kj(digit, flips))).
/// Flip index = (flips[0] == YES) | (flips[1] == YES) << 1.
const CHILD_OFFSET_IJ: [[(f64, f64); 4]; 4] = [
    [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)], // (NO, NO):  p = k, q = j
    [(0.0, 0.0), (1.0, -1.0), (0.0, -1.0), (1.0, -2.0)], // (YES, NO): p = -j, q = -k
    [(0.0, 0.0), (-1.0, 1.0), (0.0, 1.0), (-1.0, 2.0)], // (NO, YES): p = j, q = k
    [(0.0, 0.0), (-1.0, 0.0), (0.0, -1.0), (-1.0, -1.0)], // (YES, YES): p = -k, q = -j
];

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

    // Geometric digits by direct point-location, most significant first.
    let mut digits = [0u8; 33];
    let mut flips: [i32; 2] = [1, 1];
    let mut pivot_i = 0.0f64;
    let mut pivot_j = 0.0f64;
    for lvl in (0..resolution).rev() {
        let scale = 1.0 / (1u64 << lvl) as f64;
        let digit = ij_to_quaternary((i - pivot_i) * scale, (j - pivot_j) * scale, flips);
        digits[lvl] = digit;

        let fi = ((flips[0] == -1) as usize) | (((flips[1] == -1) as usize) << 1);
        let (di, dj) = CHILD_OFFSET_IJ[fi][digit as usize];
        let up = (1u64 << lvl) as f64;
        pivot_i += di * up;
        pivot_j += dj * up;
        apply_digit_flips(&mut flips, digit);
    }

    inverse_shift(&mut digits[..resolution], rec.invert_j, rec.flip_ij);
    let v = pack_digits(&digits[..resolution]);
    if rec.reverse {
        n - 1 - v
    } else {
        v
    }
}
