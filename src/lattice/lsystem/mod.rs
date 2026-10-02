// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

// The A5 space-filling curve, a turtle L-system on the triangular lattice.
//
// This replaces the earlier `shift_digits` Hilbert construction. shift_digits was
// an approximation of this curve: they agree exactly through resolution 4, but
// shift_digits self-intersects from resolution 5 on, whereas this curve never
// crosses itself at any resolution while tiling the exact same cells with the same
// metacell hierarchy. (The old curve remains available bit-for-bit via compat.rs,
// which runs the original two-motif grammar + the shift_digits digit recode through
// the same descents below.)
//
// The curve is a vertex-to-vertex turtle L-system on the triangular lattice: 7
// self-referential motifs (A B C M P Q R), each a clean A5 unit (2 parallelograms
// + 2 triangles). The symbolic grammar lives in grammar.rs and is compiled to flat
// tables in tables.rs; this module evaluates it as an O(resolution) digit
// transducer:
//   forward  s -> cell   : descend the quaternary digits, accumulating a turtle
//            position + heading, then map (a,b) -> A5 triple via a fixed
//            similarity; the leaf state also yields the cell's pentagon flavor.
//   inverse  triple -> s : descend picking, at each level, the child whose convex
//            footprint (triforce / parallelogram) contains the target cell.
//
// Every turn in every rule is 180° (see tables.rs), so the descent tracks
// orientation as a single flip bit; for the A5 grammar that invariant is also
// what keeps every parallelogram cell on-axis.

pub mod grammar;
pub mod tables;
pub mod turtle;

use std::sync::LazyLock;

use crate::lattice::types::{Orientation, Triple};

use grammar::{draws, rules};
use tables::{compile_grammar, CurveTables, BSP_EPS, POW2};

/// Branchless child pick: 3 separator dot products form a 3-bit pattern that
/// indexes the per-state lookup table. No data-dependent branches (the tree
/// walk's mispredictions are what made the branchy form lose to the 4-hull scan
/// in native code).
#[inline]
fn classify(t: &CurveTables, state: usize, rel_a: f64, rel_b: f64, scale: f64) -> usize {
    let s = &t.class_sep;
    let b = state * 9;
    let thr = -BSP_EPS * scale;
    let b0 = (s[b] * rel_a + s[b + 1] * rel_b + s[b + 2] * scale >= thr) as usize;
    let b1 = (s[b + 3] * rel_a + s[b + 4] * rel_b + s[b + 5] * scale >= thr) as usize;
    let b2 = (s[b + 6] * rel_a + s[b + 7] * rel_b + s[b + 8] * scale >= thr) as usize;
    t.class_lut[state * 8 + (b0 | (b1 << 1) | (b2 << 2))] as usize
}

/// The compiled A5 grammar.
static A5: LazyLock<CurveTables> = LazyLock::new(|| compile_grammar(&rules(), &draws()));

/// A cell as the descent identifies it: its triple + its pentagon flavor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cell {
    pub triple: Triple,
    pub flavor: u8,
}

// The quaternary digits of `s` are read directly with native u64 bit ops
// (digit L-1 is `(s >> (2*(L-1))) & 3`) — u64 covers all 60 bits at resolution
// 30, so no BigInt/float split is needed (unlike the TypeScript port).

// ---------- exact (a,b) corner-sum <-> A5 triple ----------
// The turtle (a,b) lattice and A5's triple frame are two views of the same
// triangular grid. Composing them, the √3 from each basis cancels, leaving an
// exact rational map: from a cell's corner sum (= 3·centroid),
//   y - z      = (2·sum.a + sum.b - 12) / 12
//   2x - y - z = (sum.b + 4) / 4
// and the parity x+y+z ∈ {0,1} pins x, y, z. No floating point.
pub fn ab_to_triple(sum_a: f64, sum_b: f64) -> Triple {
    let sa = sum_a as i64;
    let sb = sum_b as i64;
    if (2 * sa + sb).rem_euclid(12) != 0 || sb.rem_euclid(4) != 0 {
        panic!("ab_to_triple: off-lattice corner sum ({},{})", sum_a, sum_b);
    }
    let yz = (2 * sa + sb - 12) / 12; // y - z
    let e = (sb + 4) / 4; // 2x - y - z
    for parity in [0i64, 1] {
        if (e + parity).rem_euclid(3) != 0 {
            continue;
        }
        let x = (e + parity) / 3;
        let r = parity - x; // = y + z
        if (r + yz).rem_euclid(2) != 0 {
            continue;
        }
        return Triple::new(x as i32, ((r + yz) / 2) as i32, ((r - yz) / 2) as i32);
    }
    panic!("ab_to_triple: no integer triple for ({},{})", sum_a, sum_b);
}

pub fn triple_to_ab(t: &Triple) -> (f64, f64) {
    let x = t.x as i64;
    let y = t.y as i64;
    let z = t.z as i64;
    let b = 4 * (2 * x - y - z) - 4;
    let a = (12 * (y - z) + 12 - b) / 2;
    (a as f64, b as f64)
}

/// Result of the forward leaf descent: host cell corner sum + pentagon flavor.
pub struct LeafCell {
    pub a: f64,
    pub b: f64,
    pub flavor: u8,
}

// ---------- forward: s -> leaf host cell (corner sum + flavor) ----------
// A child placed at (parent-relative) off_unit under a `flip` frame has its
// offset negated when flipped (180°); the child's own frame is
// `flip XOR child.flip`. Internal; also used by compat.rs.
pub fn axiom_leaf_cell(t: &CurveTables, s: u64, r: usize, axiom: usize) -> LeafCell {
    let mut motif = axiom;
    let mut flip: u8 = 0;
    let mut pos_a = 0.0f64;
    let mut pos_b = 0.0f64;
    let mut level = r;
    while level >= 2 {
        let idx = level - 1;
        let d = ((s >> (idx * 2)) & 3) as usize;
        let ci = motif * 4 + d;
        let scale = if flip == 1 {
            -POW2[level - 2]
        } else {
            POW2[level - 2]
        };
        pos_a += t.child_off_a[ci] * scale;
        pos_b += t.child_off_b[ci] * scale;
        flip ^= t.child_flip[ci];
        motif = t.child_token[ci] as usize;
        level -= 1;
    }
    // level 1: leaf walk (from heading 0 or 3), take the d0-th host cell
    let d0 = if r >= 1 { (s & 3) as usize } else { 0 };
    let base = motif * 2 + flip as usize;
    LeafCell {
        a: 3.0 * pos_a + t.leaf_sum[base * 8 + d0 * 2],
        b: 3.0 * pos_b + t.leaf_sum[base * 8 + d0 * 2 + 1],
        flavor: t.leaf_flavor[base * 4 + d0],
    }
}

// ---------- inverse: descend by the branchless child classifier ----------
// Shared descent: the target is the corner sum of a real cell, which is
// strictly interior at every level, so the branchless classifier is provably
// the containing child (and the leaf resolves by exact sum match). Fractional
// point location no longer descends at all — ij_to_s rounds to a triple first
// (see curve.rs round_to_triple). Internal; also used by compat.rs.
pub fn axiom_target_to_s(t: &CurveTables, ta: f64, tb: f64, r: usize, axiom: usize) -> (u64, u8) {
    let mut motif = axiom;
    let mut flip: u8 = 0;
    let mut pos_a = 0.0f64;
    let mut pos_b = 0.0f64;
    let mut s_val: u64 = 0;
    let mut level = r;
    while level >= 2 {
        let scale = POW2[level - 2];
        let sign = if flip == 1 { -scale } else { scale };
        let best_d = classify(
            t,
            motif * 2 + flip as usize,
            ta - 3.0 * pos_a,
            tb - 3.0 * pos_b,
            scale,
        );
        let ci = motif * 4 + best_d;
        pos_a += t.child_off_a[ci] * sign;
        pos_b += t.child_off_b[ci] * sign;
        flip ^= t.child_flip[ci];
        motif = t.child_token[ci] as usize;
        s_val |= (best_d as u64) << (2 * (level - 1));
        level -= 1;
    }
    // level 1: pick the leaf cell by exact corner-sum match
    let base = motif * 2 + flip as usize;
    let mut d0 = 0usize;
    let rel_a = ta - 3.0 * pos_a;
    let rel_b = tb - 3.0 * pos_b;
    let mut found = false;
    for d in 0..4 {
        if t.leaf_sum[base * 8 + d * 2] == rel_a && t.leaf_sum[base * 8 + d * 2 + 1] == rel_b {
            d0 = d;
            found = true;
            break;
        }
    }
    if !found {
        panic!(
            "lsystem inverse: no leaf match for corner sum ({},{})",
            ta, tb
        );
    }
    (s_val | d0 as u64, t.leaf_flavor[base * 4 + d0])
}

// ---------- orientation = which triforce motif is the axiom ----------
// Each orientation is one of the three triforce motifs used as the axiom
// (uv->A, uw->C, wv->B), with the reverse orientations (vu, wu, vw) walking the
// same curve backward (s -> N-1-s). The axiom motif index is resolved once at
// init here (not per call) — motif_idx is a HashMap, so the per-call lookup was
// pure overhead on this hot path.
struct OrientRecipe {
    axiom: usize,
    reverse: bool,
    is_b: bool,
}

#[inline]
fn orient_index(o: Orientation) -> usize {
    match o {
        Orientation::UV => 0,
        Orientation::VU => 1,
        Orientation::UW => 2,
        Orientation::WU => 3,
        Orientation::VW => 4,
        Orientation::WV => 5,
    }
}

static A5_ORIENT: LazyLock<[OrientRecipe; 6]> = LazyLock::new(|| {
    let a = A5.motif_idx[&'A'];
    let b = A5.motif_idx[&'B'];
    let c = A5.motif_idx[&'C'];
    [
        OrientRecipe {
            axiom: a,
            reverse: false,
            is_b: false,
        }, // uv
        OrientRecipe {
            axiom: a,
            reverse: true,
            is_b: false,
        }, // vu
        OrientRecipe {
            axiom: c,
            reverse: false,
            is_b: false,
        }, // uw
        OrientRecipe {
            axiom: c,
            reverse: true,
            is_b: false,
        }, // wu
        OrientRecipe {
            axiom: b,
            reverse: true,
            is_b: true,
        }, // vw
        OrientRecipe {
            axiom: b,
            reverse: false,
            is_b: true,
        }, // wv
    ]
});

/// The A5 curve position `s` -> cell (triple coordinate + pentagon flavor), for
/// a given resolution and orientation. The triple is bijective with
/// `triple_to_s_lattice`.
pub fn s_to_cell(s: u64, resolution: usize, orientation: Orientation) -> Cell {
    let rec = &A5_ORIENT[orient_index(orientation)];
    let s_axiom = if rec.reverse {
        (1u64 << (2 * resolution)) - 1 - s
    } else {
        s
    };
    let cell = axiom_leaf_cell(&A5, s_axiom, resolution, rec.axiom);
    let base = ab_to_triple(cell.a, cell.b);
    if !rec.is_b {
        return Cell {
            triple: base,
            flavor: cell.flavor,
        };
    }
    let p = POW2[resolution] as i32;
    Cell {
        triple: Triple::new(base.x - p, base.y + p, base.z),
        flavor: cell.flavor,
    }
}

/// The A5 curve position `s` -> triple coordinate. Bijective with `triple_to_s_lattice`.
pub fn s_to_triple(s: u64, resolution: usize, orientation: Orientation) -> Triple {
    s_to_cell(s, resolution, orientation).triple
}

/// Triple coordinate -> the A5 curve position `s`. Inverse of `s_to_triple`.
pub fn triple_to_s_lattice(triple: &Triple, resolution: usize, orientation: Orientation) -> u64 {
    let rec = &A5_ORIENT[orient_index(orientation)];
    let (ab_a, ab_b) = triple_to_ab(triple);
    let tau_sum = if rec.is_b {
        12.0 * POW2[resolution]
    } else {
        0.0
    };
    let s_axiom = axiom_target_to_s(&A5, ab_a - tau_sum, ab_b + tau_sum, resolution, rec.axiom).0;
    if rec.reverse {
        (1u64 << (2 * resolution)) - 1 - s_axiom
    } else {
        s_axiom
    }
}
