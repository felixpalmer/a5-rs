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
use tables::{compile_grammar, CurveTables, POW2, POW4};

/// The compiled A5 grammar.
static A5: LazyLock<CurveTables> = LazyLock::new(|| compile_grammar(&rules(), &draws()));

/// A cell as the descent identifies it: its triple + its pentagon flavor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cell {
    pub triple: Triple,
    pub flavor: u8,
}

// s <-> quaternary digits, via two halves: `lo` holds digits 0-12 (26 bits),
// `hi` the rest (up to 2^34 at resolution 30) — both exact, so the per-level
// work is plain arithmetic.
const LO_DIGITS: usize = 13;
const LO_BITS: u32 = 26;
const LO_MASK: u64 = 0x3ff_ffff;

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
    let lo = (s & LO_MASK) as u32;
    let hi = s >> LO_BITS;
    let mut motif = axiom;
    let mut flip: u8 = 0;
    let mut pos_a = 0.0f64;
    let mut pos_b = 0.0f64;
    let mut level = r;
    while level >= 2 {
        let idx = level - 1;
        let d = if idx < LO_DIGITS {
            ((lo >> (idx * 2)) & 3) as usize
        } else {
            ((hi / 4u64.pow((idx - LO_DIGITS) as u32)) % 4) as usize
        };
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
    let d0 = if r >= 1 { (lo & 3) as usize } else { 0 };
    let base = motif * 2 + flip as usize;
    LeafCell {
        a: 3.0 * pos_a + t.leaf_sum[base * 8 + d0 * 2],
        b: 3.0 * pos_b + t.leaf_sum[base * 8 + d0 * 2 + 1],
        flavor: t.leaf_flavor[base * 4 + d0],
    }
}

// ---------- inverse: descend by which child's convex footprint contains the target ----------
#[allow(clippy::too_many_arguments)]
fn inside_score(
    t: &CurveTables,
    motif: usize,
    flip: u8,
    lvl: usize,
    pos_a: f64,
    pos_b: f64,
    ta: f64,
    tb: f64,
    best: f64,
) -> f64 {
    let scale = POW2[lvl - 1];
    let edges = &t.fp_edges[motif * 2 + flip as usize];
    let mut min_cross = f64::INFINITY;
    let mut e = 0;
    while e < edges.len() {
        let dta = ta - (3.0 * pos_a + edges[e] * scale);
        let dtb = tb - (3.0 * pos_b + edges[e + 1] * scale);
        let cross = edges[e + 2] * dtb - edges[e + 3] * dta;
        if cross < min_cross {
            min_cross = cross;
            if min_cross <= 0.0 && min_cross <= best {
                return min_cross;
            }
        }
        e += 4;
    }
    min_cross
}

// Shared descent for both leaf modes. `exact` targets are corner sums of real
// cells (leaf resolved by exact sum match); fractional targets resolve the leaf
// by point-in-cell over the 4 level-1 triangles. Internal; also used by compat.rs.
pub fn axiom_target_to_s(
    t: &CurveTables,
    ta: f64,
    tb: f64,
    r: usize,
    axiom: usize,
    exact: bool,
) -> u64 {
    let mut motif = axiom;
    let mut flip: u8 = 0;
    let mut pos_a = 0.0f64;
    let mut pos_b = 0.0f64;
    let mut s_lo = 0.0f64;
    let mut s_hi = 0.0f64;
    let mut level = r;
    while level >= 2 {
        let scale = POW2[level - 2];
        let sign = if flip == 1 { -scale } else { scale };
        let mut best_d = 0usize;
        let mut best_score = f64::NEG_INFINITY;
        for d in 0..4 {
            let ci = motif * 4 + d;
            let score = inside_score(
                t,
                t.child_token[ci] as usize,
                flip ^ t.child_flip[ci],
                level - 1,
                pos_a + t.child_off_a[ci] * sign,
                pos_b + t.child_off_b[ci] * sign,
                ta,
                tb,
                best_score,
            );
            if score > best_score {
                best_score = score;
                best_d = d;
                if score > 0.0 {
                    break; // strictly inside: the unique containing child
                }
            }
        }
        let ci = motif * 4 + best_d;
        pos_a += t.child_off_a[ci] * sign;
        pos_b += t.child_off_b[ci] * sign;
        flip ^= t.child_flip[ci];
        motif = t.child_token[ci] as usize;
        let idx = level - 1;
        if idx < LO_DIGITS {
            s_lo += best_d as f64 * POW4[idx];
        } else {
            s_hi += best_d as f64 * POW4[idx - LO_DIGITS];
        }
        level -= 1;
    }
    // level 1: pick the leaf cell, by exact corner-sum match or point-in-cell
    let base = motif * 2 + flip as usize;
    let mut d0 = 0usize;
    if exact {
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
    } else {
        let mut best_score = f64::NEG_INFINITY;
        for d in 0..4 {
            let mut min_cross = f64::INFINITY;
            for e in 0..3 {
                let o = base * 48 + d * 12 + e * 4;
                let dta = ta - (3.0 * pos_a + t.leaf_tri[o]);
                let dtb = tb - (3.0 * pos_b + t.leaf_tri[o + 1]);
                let cross = t.leaf_tri[o + 2] * dtb - t.leaf_tri[o + 3] * dta;
                if cross < min_cross {
                    min_cross = cross;
                }
            }
            if min_cross > best_score {
                best_score = min_cross;
                d0 = d;
                if min_cross > 0.0 {
                    break;
                }
            }
        }
    }
    s_lo += d0 as f64;
    if r > LO_DIGITS {
        ((s_hi as u64) << LO_BITS) | (s_lo as u64)
    } else {
        s_lo as u64
    }
}

// ---------- orientation = which triforce motif is the axiom ----------
// Each orientation is one of the three triforce motifs used as the axiom
// (uv->A, uw->C, wv->B), with the reverse orientations (vu, wu, vw) walking the
// same curve backward (s -> N-1-s).
struct OrientRecipe {
    axiom_char: char,
    reverse: bool,
    is_b: bool,
}

fn orient_recipe(o: Orientation) -> OrientRecipe {
    match o {
        Orientation::UV => OrientRecipe {
            axiom_char: 'A',
            reverse: false,
            is_b: false,
        },
        Orientation::VU => OrientRecipe {
            axiom_char: 'A',
            reverse: true,
            is_b: false,
        },
        Orientation::UW => OrientRecipe {
            axiom_char: 'C',
            reverse: false,
            is_b: false,
        },
        Orientation::WU => OrientRecipe {
            axiom_char: 'C',
            reverse: true,
            is_b: false,
        },
        Orientation::VW => OrientRecipe {
            axiom_char: 'B',
            reverse: true,
            is_b: true,
        },
        Orientation::WV => OrientRecipe {
            axiom_char: 'B',
            reverse: false,
            is_b: true,
        },
    }
}

/// The A5 curve position `s` -> cell (triple coordinate + pentagon flavor), for
/// a given resolution and orientation. The triple is bijective with
/// `triple_to_s_lattice`.
pub fn s_to_cell(s: u64, resolution: usize, orientation: Orientation) -> Cell {
    let n = 1u64 << (2 * resolution);
    let rec = orient_recipe(orientation);
    let axiom = A5.motif_idx[&rec.axiom_char];
    let s_axiom = if rec.reverse { n - 1 - s } else { s };
    let cell = axiom_leaf_cell(&A5, s_axiom, resolution, axiom);
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
    let n = 1u64 << (2 * resolution);
    let rec = orient_recipe(orientation);
    let axiom = A5.motif_idx[&rec.axiom_char];
    let (ab_a, ab_b) = triple_to_ab(triple);
    let tau_sum = if rec.is_b {
        12.0 * POW2[resolution]
    } else {
        0.0
    };
    let s_axiom = axiom_target_to_s(&A5, ab_a - tau_sum, ab_b + tau_sum, resolution, axiom, true);
    if rec.reverse {
        n - 1 - s_axiom
    } else {
        s_axiom
    }
}

/// Fractional point -> the curve position `s` of the containing cell, by direct
/// descent. The target is given in the corner-sum frame (= 3x the L-system (a,b)
/// point frame); callers map their coordinate system into it (for the IJ plane
/// the exact affine map is target = (12*(i+j), -12*j), see curve.rs).
pub fn sum_point_to_s(ta: f64, tb: f64, resolution: usize, orientation: Orientation) -> u64 {
    let n = 1u64 << (2 * resolution);
    let rec = orient_recipe(orientation);
    let axiom = A5.motif_idx[&rec.axiom_char];
    let tau_sum = if rec.is_b {
        12.0 * POW2[resolution]
    } else {
        0.0
    };
    let s_axiom = axiom_target_to_s(&A5, ta - tau_sum, tb + tau_sum, resolution, axiom, false);
    if rec.reverse {
        n - 1 - s_axiom
    } else {
        s_axiom
    }
}
