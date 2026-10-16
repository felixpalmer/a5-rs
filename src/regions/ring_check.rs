// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

//! Whether a polygon ring crosses itself. The fills assume simple rings: a
//! ring whose edges cross, such as a bow tie, has no consistent inside, and
//! the fill floods out of it.

use crate::coordinate_systems::Cartesian;

fn dot(a: Cartesian, b: Cartesian) -> f64 {
    a.x() * b.x() + a.y() * b.y() + a.z() * b.z()
}

fn cross(a: Cartesian, b: Cartesian) -> Cartesian {
    Cartesian::new(
        a.y() * b.z() - a.z() * b.y(),
        a.z() * b.x() - a.x() * b.z(),
        a.x() * b.y() - a.y() * b.x(),
    )
}

// Points closer than this (on the unit sphere, about 6 um on the Earth) are
// one point, and a point this close to a great circle lies on it. Vertices
// that should coincide, as in a zero-width spike, can differ in their last
// bits, and rounding then makes their edges appear to cross.
const EPS: f64 = 1e-12;

fn same(a: Cartesian, b: Cartesian) -> bool {
    (a.x() - b.x()).abs() <= EPS && (a.y() - b.y()).abs() <= EPS && (a.z() - b.z()).abs() <= EPS
}

/// The side of the great circle through `a` and `b` that `c` lies on, or 0
/// when `c` is within `EPS` of it.
fn orient(a: Cartesian, b: Cartesian, c: Cartesian) -> i8 {
    let n = cross(a, b);
    let d = dot(n, c);
    if d.abs() <= EPS * dot(n, n).sqrt() {
        return 0;
    }
    (d > 0.0) as i8 - (d < 0.0) as i8
}

/// Whether arcs `ab` and `cd` cross at a point interior to both, as S2's
/// CrossingSign decides it. Arcs sharing a vertex, touching or collinear do
/// not count, so no simple ring is reported.
fn arcs_cross(a: Cartesian, b: Cartesian, c: Cartesian, d: Cartesian) -> bool {
    if same(a, c) || same(a, d) || same(b, c) || same(b, d) {
        return false;
    }
    let acb = -orient(a, b, c);
    let bda = orient(a, b, d);
    if acb == 0 || acb != bda {
        return false;
    }
    let cbd = -orient(c, d, b);
    let dac = orient(c, d, a);
    cbd == acb && dac == acb
}

/// A box holding the whole arc: its endpoints' box widened by the arc's
/// greatest distance from its chord.
fn arc_box(a: Cartesian, b: Cartesian) -> ([f64; 3], [f64; 3]) {
    let half = dot(a, b).clamp(-1.0, 1.0).acos() / 2.0;
    let sag = 1.0 - half.cos() + 1e-12;
    let (a, b): ([f64; 3], [f64; 3]) = (a.into(), b.into());
    let mut lo = [0.0; 3];
    let mut hi = [0.0; 3];
    for k in 0..3 {
        lo[k] = a[k].min(b[k]) - sag;
        hi[k] = a[k].max(b[k]) + sag;
    }
    (lo, hi)
}

/// Whether two edges of the ring cross. The ring is open (no repeated first
/// vertex). Edges are swept along x by their boxes, so only edges whose boxes
/// overlap are compared.
pub(super) fn ring_crosses_itself(ring: &[Cartesian]) -> bool {
    let mut pts: Vec<Cartesian> = Vec::with_capacity(ring.len());
    for &p in ring {
        if !matches!(pts.last(), Some(&q) if same(q, p)) {
            pts.push(p);
        }
    }
    if pts.len() > 1 && same(pts[0], pts[pts.len() - 1]) {
        pts.pop();
    }
    let n = pts.len();
    if n < 4 {
        return false;
    }
    let boxes: Vec<([f64; 3], [f64; 3])> =
        (0..n).map(|i| arc_box(pts[i], pts[(i + 1) % n])).collect();
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_unstable_by(|&i, &j| boxes[i].0[0].total_cmp(&boxes[j].0[0]));

    let mut active: Vec<usize> = Vec::new();
    for &i in &order {
        let (lo, hi) = boxes[i];
        active.retain(|&j| boxes[j].1[0] >= lo[0]);
        for &j in &active {
            // Neighbouring edges share a vertex
            if (i + 1) % n == j || (j + 1) % n == i {
                continue;
            }
            let (jlo, jhi) = boxes[j];
            if jlo[1] > hi[1] || jhi[1] < lo[1] || jlo[2] > hi[2] || jhi[2] < lo[2] {
                continue;
            }
            if arcs_cross(pts[i], pts[(i + 1) % n], pts[j], pts[(j + 1) % n]) {
                return true;
            }
        }
        active.push(i);
    }
    false
}
