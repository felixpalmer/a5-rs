// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

// Compiles an L-system grammar into flat numeric tables, once at module init,
// so the descents in the lsystem module are pure scalar arithmetic: no string
// expansion and no object allocation per call.
//
// Every grammar compiled here keeps every turn inside a rule at 180°
// (`+++`/`---`), so a child is only ever placed un-flipped or flipped (rotated
// 180°) relative to its parent — never at 60°. compile_grammar enforces that
// invariant and records the orientation as a single `flip` bit (180° = negate
// on this lattice); the whole descent then tracks orientation as one boolean.
// For the A5 grammar the invariant is also what keeps every parallelogram cell
// on-axis; the compat W/Z grammar is the original two-motif curve re-gauged
// into this form (see compat.rs).
//
// Motif tokens are indexed by their position in the motif list (uppercase
// motifs first, then their lowercase reverses); a descent state is
// (motif index, flip bit). All hot-path lookups are flat array reads indexed
// by that state.

use std::collections::HashMap;
use std::sync::LazyLock;

use super::grammar::{expand_once, reverse_motif};
use super::turtle::{host_corners, host_sum, net_of, walk, AB};

/// Flat numeric tables for one grammar, consumed by the descents in the lsystem module.
pub struct CurveTables {
    pub motif_idx: HashMap<char, usize>,
    // children: entry ci = motif * 4 + digit
    pub child_token: Vec<i32>,
    pub child_flip: Vec<u8>,
    pub child_off_a: Vec<f64>,
    pub child_off_b: Vec<f64>,
    // footprint hulls per (motif, flip): edge list [3*c0.a, 3*c0.b, d.a, d.b]*E,
    // all flattened into one contiguous buffer; the edges for state
    // k = motif*2+flip are fp_edges[fp_offset[k]..fp_offset[k + 1]].
    pub fp_edges: Vec<f64>,
    pub fp_offset: Vec<usize>,
    // leaf tables per (motif, flip): 4 host cells as corner sums, point-in-cell
    // triangle edges, and pentagon flavors
    pub leaf_sum: Vec<f64>,
    pub leaf_tri: Vec<f64>,
    pub leaf_flavor: Vec<u8>,
    // Branchless child classifier per state k = motif*2+flip: 3 separating lines
    // (`class_sep[k*9 ..]` = [nx0,ny0,c0, nx1,ny1,c1, nx2,ny2,c2]) evaluated
    // against the normalised target give a 3-bit pattern; `class_lut[k*8 + pat]`
    // is the child digit. Replaces the 4-hull scan with 3 dot products + a LUT
    // read and NO data-dependent branches (unused slots are 0 -> always bit 1).
    pub class_sep: Vec<f64>,
    pub class_lut: Vec<u8>,
}

/// A child-selection BSP built at compile time, later flattened to the
/// branchless `class_sep` / `class_lut` tables.
enum Bsp {
    Leaf(usize),
    Node(f64, f64, f64, Box<Bsp>, Box<Bsp>),
}

// The pentagon FLAVOR (0-3) of the cell a draw symbol hosts: which of the four
// pentagon orientations of the Cairo-like metatile it gets. The pentagon is a
// 1:1 function of the cell's jigsaw piece and reduces to the closed-form rule
//   flavor = BASE[symbol] XOR isLowercase XOR (heading & 1)
// with BASE = {S:0, D:1, E:2, T:3}; bit 0 is a 180° rotation, bit 1 a Y
// reflection of the base pentagon (see core/tiling.rs). Derived and verified
// exhaustively against the pentagon geometry.
fn flavor_base(sym: char) -> Option<u8> {
    match sym {
        'S' => Some(0),
        'D' => Some(1),
        'E' => Some(2),
        'T' => Some(3),
        _ => None,
    }
}

struct Child {
    token: char,
    off_unit: AB, // offset from the parent origin, in net(·,1) units
    flip: bool,
}

// Expand a motif to a pure draw string: `level` rule passes, then one draws
// pass (turning every remaining motif into its leaf terminal).
fn to_draws(
    motif: char,
    level: usize,
    rules: &HashMap<char, String>,
    draws: &HashMap<char, String>,
) -> String {
    let mut s = motif.to_string();
    for _ in 0..level {
        s = expand_once(&s, rules);
    }
    expand_once(&s, draws)
}

fn motif_net(motif: char, rules: &HashMap<char, String>, draws: &HashMap<char, String>) -> AB {
    net_of(&to_draws(motif, 1, rules, draws)).0
}

fn child_table(
    rule: &str,
    rules: &HashMap<char, String>,
    draws: &HashMap<char, String>,
) -> Vec<Child> {
    let mut pos = AB::new(0, 0);
    let mut h = 0i32;
    let mut children: Vec<Child> = Vec::new();
    for ch in rule.chars() {
        if ch == '+' {
            h = (h + 1) % 6;
            continue;
        }
        if ch == '-' {
            h = (h + 5) % 6;
            continue;
        }
        if !rules.contains_key(&ch.to_ascii_uppercase()) {
            continue;
        }
        if h != 0 && h != 3 {
            panic!(
                "lsystem: non-180° turn ({}°) before a child in rule \"{}\"",
                60 * h,
                rule
            );
        }
        let flip = h == 3;
        children.push(Child {
            token: ch,
            off_unit: pos,
            flip,
        });
        let n = motif_net(ch, rules, draws);
        pos = if flip {
            AB::new(pos.a - n.a, pos.b - n.b)
        } else {
            AB::new(pos.a + n.a, pos.b + n.b)
        };
    }
    if children.len() != 4 {
        panic!("lsystem: rule \"{}\" must have 4 children", rule);
    }
    children
}

fn convex_hull(pts: &[AB]) -> Vec<AB> {
    let mut p: Vec<AB> = pts.to_vec();
    p.sort_by(|x, y| x.a.cmp(&y.a).then(x.b.cmp(&y.b)));
    p.dedup();
    if p.len() < 3 {
        return p;
    }
    let cross = |o: AB, a: AB, b: AB| -> i64 {
        (a.a - o.a) as i64 * (b.b - o.b) as i64 - (a.b - o.b) as i64 * (b.a - o.a) as i64
    };
    let mut lower: Vec<AB> = Vec::new();
    for &q in &p {
        while lower.len() >= 2 && cross(lower[lower.len() - 2], lower[lower.len() - 1], q) <= 0 {
            lower.pop();
        }
        lower.push(q);
    }
    let mut upper: Vec<AB> = Vec::new();
    for &q in p.iter().rev() {
        while upper.len() >= 2 && cross(upper[upper.len() - 2], upper[upper.len() - 1], q) <= 0 {
            upper.pop();
        }
        upper.push(q);
    }
    lower.truncate(lower.len() - 1);
    upper.truncate(upper.len() - 1);
    lower.extend(upper);
    lower
}

/// Compile a grammar (motif rules + leaf draws) into flat descent tables.
/// Lowercase motifs are the uppercase rules reversed, generated automatically.
pub fn compile_grammar(
    rules: &HashMap<char, String>,
    draws: &HashMap<char, String>,
) -> CurveTables {
    // Deterministic motif order (indices are internal, so a stable sort suffices).
    let mut motifs: Vec<char> = rules.keys().copied().collect();
    motifs.sort_unstable();
    let all_motifs: Vec<char> = motifs
        .iter()
        .copied()
        .chain(motifs.iter().map(|m| m.to_ascii_lowercase()))
        .collect();
    let motif_count = all_motifs.len();
    let mut motif_idx: HashMap<char, usize> = HashMap::new();
    for (i, &m) in all_motifs.iter().enumerate() {
        motif_idx.insert(m, i);
    }

    // ---------- child tables: 4 children per motif ----------
    let mut children_of: HashMap<char, Vec<Child>> = HashMap::new();
    for &m in &motifs {
        children_of.insert(m, child_table(&rules[&m], rules, draws));
    }
    for &m in &motifs {
        children_of.insert(
            m.to_ascii_lowercase(),
            child_table(&reverse_motif(&rules[&m]), rules, draws),
        );
    }

    let mut child_token = vec![0i32; motif_count * 4];
    let mut child_flip = vec![0u8; motif_count * 4];
    let mut child_off_a = vec![0.0f64; motif_count * 4];
    let mut child_off_b = vec![0.0f64; motif_count * 4];
    for &m in &all_motifs {
        let cs = &children_of[&m];
        for d in 0..4 {
            let ci = motif_idx[&m] * 4 + d;
            child_token[ci] = motif_idx[&cs[d].token] as i32;
            child_flip[ci] = if cs[d].flip { 1 } else { 0 };
            child_off_a[ci] = cs[d].off_unit.a as f64;
            child_off_b[ci] = cs[d].off_unit.b as f64;
        }
    }

    // ---------- footprint hulls (convex hull of leaf host corners) ----------
    // per (motif, flip): edge list [3*c0.a, 3*c0.b, d.a, d.b]*E.
    // The corner is pre-tripled (the descent works in the corner-sum frame, = 3x
    // the (a,b) point frame); the edge direction stays UNIT so the containment
    // cross products stay ~O(2^R) instead of O(2^2R) — exact integer at every
    // resolution. The flipped variant is the hull negated (180° = negate,
    // winding-preserving).
    let mut by_state: Vec<Vec<f64>> = vec![Vec::new(); motif_count * 2];
    for &m in &all_motifs {
        let mut corners: Vec<AB> = Vec::new();
        walk(
            &to_draws(m, 1, rules, draws),
            AB::new(0, 0),
            0,
            |sym, from, h| corners.extend_from_slice(&host_corners(sym, from, h)),
        );
        let hull = convex_hull(&corners);
        for flip in 0..2 {
            let sign: f64 = if flip == 1 { -1.0 } else { 1.0 };
            let mut edges = vec![0.0f64; hull.len() * 4];
            for i in 0..hull.len() {
                let c0 = hull[i];
                let c1 = hull[(i + 1) % hull.len()];
                edges[i * 4] = 3.0 * sign * c0.a as f64;
                edges[i * 4 + 1] = 3.0 * sign * c0.b as f64;
                edges[i * 4 + 2] = sign * (c1.a - c0.a) as f64;
                edges[i * 4 + 3] = sign * (c1.b - c0.b) as f64;
            }
            by_state[motif_idx[&m] * 2 + flip] = edges;
        }
    }
    // Flatten the per-state hulls into one contiguous buffer + offset index, so
    // the containment descent (inside_score) hits a single cache-friendly slice.
    let mut fp_edges: Vec<f64> = Vec::new();
    let mut fp_offset = vec![0usize; motif_count * 2 + 1];
    for (k, edges) in by_state.iter().enumerate() {
        fp_edges.extend_from_slice(edges);
        fp_offset[k + 1] = fp_edges.len();
    }

    // ---------- leaf tables: per (motif, flip = heading 0|3) the 4 level-1 host cells ----------
    let mut leaf_sum = vec![0.0f64; motif_count * 2 * 8];
    let mut leaf_tri = vec![0.0f64; motif_count * 2 * 48];
    let mut leaf_flavor = vec![0u8; motif_count * 2 * 4];
    for &m in &all_motifs {
        let draw_str = to_draws(m, 1, rules, draws);
        for flip in 0..2 {
            let base = motif_idx[&m] * 2 + flip;
            let mut d = 0usize;
            walk(
                &draw_str,
                AB::new(0, 0),
                if flip == 1 { 3 } else { 0 },
                |sym, from, hh| {
                    let sum = host_sum(sym, from, hh);
                    leaf_sum[base * 8 + d * 2] = sum.a as f64;
                    leaf_sum[base * 8 + d * 2 + 1] = sum.b as f64;
                    let upper = sym.to_ascii_uppercase();
                    let fbase = flavor_base(upper).unwrap_or_else(|| {
                        panic!("lsystem: no pentagon flavor for draw symbol {}", sym)
                    });
                    let is_lower = if sym == upper { 0 } else { 1 };
                    leaf_flavor[base * 4 + d] = fbase ^ is_lower ^ ((hh & 1) as u8);
                    let mut c = host_corners(sym, from, hh);
                    let area = (c[1].a - c[0].a) as i64 * (c[2].b - c[0].b) as i64
                        - (c[1].b - c[0].b) as i64 * (c[2].a - c[0].a) as i64;
                    if area < 0 {
                        c = [c[0], c[2], c[1]];
                    }
                    for e in 0..3 {
                        let c0 = c[e];
                        let c1 = c[(e + 1) % 3];
                        let o = base * 48 + d * 12 + e * 4;
                        leaf_tri[o] = 3.0 * c0.a as f64;
                        leaf_tri[o + 1] = 3.0 * c0.b as f64;
                        leaf_tri[o + 2] = (c1.a - c0.a) as f64;
                        leaf_tri[o + 3] = (c1.b - c0.b) as f64;
                    }
                    d += 1;
                },
            );
        }
    }

    // ---------- branchless child classifier (3 line tests + LUT per state) ----------
    let mut class_sep = vec![0.0f64; motif_count * 2 * 9];
    let mut class_lut = vec![0u8; motif_count * 2 * 8];
    for m in 0..motif_count {
        for f in 0..2 {
            let k = m * 2 + f;
            let children = child_polys(
                m,
                f,
                &child_token,
                &child_flip,
                &child_off_a,
                &child_off_b,
                &fp_edges,
                &fp_offset,
            );
            let tree = build_bsp_rec(&children);
            // distinct separators in DFS order (<= 3 for a 4-way split)
            let mut seps: Vec<(f64, f64, f64)> = Vec::new();
            collect_seps(&tree, &mut seps);
            for (i, s) in seps.iter().enumerate() {
                class_sep[k * 9 + i * 3] = s.0;
                class_sep[k * 9 + i * 3 + 1] = s.1;
                class_sep[k * 9 + i * 3 + 2] = s.2;
            }
            for p in 0..8u8 {
                class_lut[k * 8 + p as usize] = walk_bsp(&tree, p, &seps) as u8;
            }
        }
    }

    CurveTables {
        motif_idx,
        child_token,
        child_flip,
        child_off_a,
        child_off_b,
        fp_edges,
        fp_offset,
        leaf_sum,
        leaf_tri,
        leaf_flavor,
        class_sep,
        class_lut,
    }
}

pub const BSP_EPS: f64 = 1e-6;

/// The 4 child footprint polygons for state (motif, pflip), in the NORMALISED
/// target-relative-to-cursor frame (scale-invariant), as (digit, verts).
#[allow(clippy::too_many_arguments)]
fn child_polys(
    motif: usize,
    pflip: usize,
    child_token: &[i32],
    child_flip: &[u8],
    child_off_a: &[f64],
    child_off_b: &[f64],
    fp_edges: &[f64],
    fp_offset: &[usize],
) -> Vec<(usize, Vec<(f64, f64)>)> {
    let psign = if pflip == 1 { -1.0 } else { 1.0 };
    let mut out = Vec::with_capacity(4);
    for d in 0..4 {
        let ci = motif * 4 + d;
        let tok = child_token[ci] as usize;
        let cfl = child_flip[ci] as usize;
        let oa = child_off_a[ci];
        let ob = child_off_b[ci];
        let k = tok * 2 + (pflip ^ cfl);
        let mut verts = Vec::new();
        let mut e = fp_offset[k];
        while e < fp_offset[k + 1] {
            verts.push((
                3.0 * oa * psign + fp_edges[e],
                3.0 * ob * psign + fp_edges[e + 1],
            ));
            e += 4;
        }
        out.push((d, verts));
    }
    out
}

/// Build a child-selection BSP: at each step some polygon edge separates the
/// (convex, tiling) children cleanly into two groups.
fn build_bsp_rec(children: &[(usize, Vec<(f64, f64)>)]) -> Bsp {
    if children.len() == 1 {
        return Bsp::Leaf(children[0].0);
    }
    for (_, poly) in children {
        let n = poly.len();
        for i in 0..n {
            let (x1, y1) = poly[i];
            let (x2, y2) = poly[(i + 1) % n];
            let nx = y2 - y1;
            let ny = -(x2 - x1);
            let c = -(nx * x1 + ny * y1);
            let mut pos: Vec<(usize, Vec<(f64, f64)>)> = Vec::new();
            let mut neg: Vec<(usize, Vec<(f64, f64)>)> = Vec::new();
            let mut ok = true;
            for (d, cp) in children {
                let mut mn = f64::INFINITY;
                let mut mx = f64::NEG_INFINITY;
                for &(x, y) in cp {
                    let val = nx * x + ny * y + c;
                    mn = mn.min(val);
                    mx = mx.max(val);
                }
                if mn >= -BSP_EPS {
                    pos.push((*d, cp.clone()));
                } else if mx <= BSP_EPS {
                    neg.push((*d, cp.clone()));
                } else {
                    ok = false;
                    break;
                }
            }
            if ok && !pos.is_empty() && !neg.is_empty() {
                return Bsp::Node(
                    nx,
                    ny,
                    c,
                    Box::new(build_bsp_rec(&pos)),
                    Box::new(build_bsp_rec(&neg)),
                );
            }
        }
    }
    panic!("lsystem: no clean BSP split for child set");
}

fn collect_seps(tree: &Bsp, seps: &mut Vec<(f64, f64, f64)>) {
    if let Bsp::Node(nx, ny, c, pos, neg) = tree {
        let key = (*nx, *ny, *c);
        if !seps.contains(&key) {
            seps.push(key);
        }
        collect_seps(pos, seps);
        collect_seps(neg, seps);
    }
}

/// Walk the tree for a fixed sign pattern `p` (bit i = which side of separator i).
fn walk_bsp(tree: &Bsp, p: u8, seps: &[(f64, f64, f64)]) -> usize {
    match tree {
        Bsp::Leaf(d) => *d,
        Bsp::Node(nx, ny, c, pos, neg) => {
            let idx = seps.iter().position(|s| *s == (*nx, *ny, *c)).unwrap();
            if (p >> idx) & 1 == 1 {
                walk_bsp(pos, p, seps)
            } else {
                walk_bsp(neg, p, seps)
            }
        }
    }
}

// powers of 2 used by the descents (index by level), the child-offset scale
pub static POW2: LazyLock<[f64; 32]> = LazyLock::new(|| {
    let mut a = [0.0f64; 32];
    for (i, slot) in a.iter_mut().enumerate() {
        *slot = 2f64.powi(i as i32);
    }
    a
});
