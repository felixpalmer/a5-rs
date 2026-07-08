// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

// Spherical polygon (with holes) prepared for repeated point-containment
// tests: bounding-cap prefilter, then a trig-free crossing-number test with
// the winding-number test as a robust fallback.

use crate::coordinate_systems::Cartesian;
use crate::geometry::spherical_polygon::{point_in_spherical_polygon, ring_segment_normals};

/// Point-in-polygon for a polygon with holes: inside the outer ring and
/// outside every hole ring. Winding-number test — robust but O(atan2) per
/// edge; used as the fallback for the crossing-number fast path below.
fn point_in_polygon_rings(point: Cartesian, ring_vecs_list: &[Vec<Cartesian>]) -> bool {
    if !point_in_spherical_polygon(point, &ring_vecs_list[0]) {
        return false;
    }
    for ring_vecs in &ring_vecs_list[1..] {
        if point_in_spherical_polygon(point, ring_vecs) {
            return false;
        }
    }
    true
}

/// Bounding cap of the polygon: every polygon point is within the cap.
/// The winding-number PIP is blind at the polygon's ANTIPODE (the angle sum is
/// ±2π there too), so distant probes MUST be rejected by the cap first. The cap
/// angle is bounded by the farthest ring vertex plus half the longest edge (any
/// point of an edge arc is within half the edge length of an endpoint).
#[derive(Debug, Clone, Copy)]
pub struct BoundingCap {
    pub center: Cartesian,
    pub min_dot: f64,
}

fn bounding_cap(ring_vecs_list: &[Vec<Cartesian>]) -> BoundingCap {
    let mut cx = 0.0;
    let mut cy = 0.0;
    let mut cz = 0.0;
    for v in &ring_vecs_list[0] {
        cx += v.x();
        cy += v.y();
        cz += v.z();
    }
    let len = (cx * cx + cy * cy + cz * cz).sqrt();
    if len < 1e-12 {
        return BoundingCap {
            center: Cartesian::new(0.0, 0.0, 1.0),
            min_dot: -1.0,
        };
    }
    cx /= len;
    cy /= len;
    cz /= len;
    let center = Cartesian::new(cx, cy, cz);

    let mut max_angle = 0.0_f64;
    let mut max_edge = 0.0_f64;
    for ring_vecs in ring_vecs_list {
        let n = ring_vecs.len();
        for i in 0..n {
            let v = ring_vecs[i];
            let w = ring_vecs[(i + 1) % n];
            let dot_cv = center.x() * v.x() + center.y() * v.y() + center.z() * v.z();
            max_angle = max_angle.max(dot_cv.clamp(-1.0, 1.0).acos());
            let dot_vw = v.x() * w.x() + v.y() * w.y() + v.z() * w.z();
            max_edge = max_edge.max(dot_vw.clamp(-1.0, 1.0).acos());
        }
    }
    let cap_angle = std::f64::consts::PI.min(max_angle + max_edge / 2.0);
    BoundingCap {
        center,
        min_dot: cap_angle.cos(),
    }
}

/// Polygon prepared for repeated containment tests: rings, per-edge great-circle
/// normals, bounding cap, and a reference point for the crossing-number test.
///
/// The reference point sits just OUTSIDE the cap (angle capAngle + 0.2 from its
/// center) rather than at the antipode: probes come from inside the cap, so
/// the probe->ref arc plane stays well conditioned (|p × ref| >= sin 0.2). The
/// fast path is disabled for very large polygons (cap over ~79°), where that
/// construction can't keep the arc short — those fall back to the winding test.
pub struct PreparedPolygon {
    pub ring_vecs_list: Vec<Vec<Cartesian>>,
    pub ring_normals: Vec<Vec<Cartesian>>,
    pub cap: BoundingCap,
    pub reference: Cartesian,
    pub use_fast: bool,
}

pub fn prepare_polygon(ring_vecs_list: Vec<Vec<Cartesian>>) -> PreparedPolygon {
    let cap = bounding_cap(&ring_vecs_list);
    let ring_normals: Vec<Vec<Cartesian>> = ring_vecs_list
        .iter()
        .map(|ring| ring_segment_normals(ring))
        .collect();
    let cap_angle = cap.min_dot.clamp(-1.0, 1.0).acos();
    let use_fast = cap.min_dot > -1.0 && cap_angle < 1.37;
    let c = cap.center;

    // perp = c × (Z_AXIS or X_AXIS), unit vector perpendicular to the cap center
    let axis = if c.z().abs() < 0.9 {
        Cartesian::new(0.0, 0.0, 1.0)
    } else {
        Cartesian::new(1.0, 0.0, 0.0)
    };
    let perp = Cartesian::new(
        c.y() * axis.z() - c.z() * axis.y(),
        c.z() * axis.x() - c.x() * axis.z(),
        c.x() * axis.y() - c.y() * axis.x(),
    );
    let d_len = {
        let l = (perp.x() * perp.x() + perp.y() * perp.y() + perp.z() * perp.z()).sqrt();
        if l == 0.0 {
            1.0
        } else {
            l
        }
    };
    let theta = cap_angle + 0.2;
    let cos_t = theta.cos();
    let sin_t = theta.sin() / d_len;
    let reference = Cartesian::new(
        c.x() * cos_t + perp.x() * sin_t,
        c.y() * cos_t + perp.y() * sin_t,
        c.z() * cos_t + perp.z() * sin_t,
    );
    PreparedPolygon {
        ring_vecs_list,
        ring_normals,
        cap,
        reference,
        use_fast,
    }
}

const CROSSING_EPS: f64 = 1e-14;

/// Crossing-number containment: count proper crossings of the arc probe->ref
/// with every ring edge (just sign tests — no trig); odd parity = inside
/// (`ref` is outside the polygon, and the even-odd rule handles holes for
/// free). Returns None on any near-degenerate sign (probe or a vertex on
/// an arc plane) — the caller falls back to the winding test, which also keeps
/// on-edge tie-breaking identical to the previous implementation.
fn crossing_parity(p: Cartesian, prep: &PreparedPolygon) -> Option<bool> {
    let r = prep.reference;
    // normal of the probe->ref arc plane
    let abx = p.y() * r.z() - p.z() * r.y();
    let aby = p.z() * r.x() - p.x() * r.z();
    let abz = p.x() * r.y() - p.y() * r.x();
    let mut crossings = 0u32;
    for ri in 0..prep.ring_vecs_list.len() {
        let verts = &prep.ring_vecs_list[ri];
        let norms = &prep.ring_normals[ri];
        let n = verts.len();
        let s_first = abx * verts[0].x() + aby * verts[0].y() + abz * verts[0].z();
        if s_first.abs() < CROSSING_EPS {
            return None;
        }
        let mut s_prev = s_first;
        for i in 0..n {
            let s_next = if i + 1 == n {
                s_first
            } else {
                let v = verts[i + 1];
                let s = abx * v.x() + aby * v.y() + abz * v.z();
                if s.abs() < CROSSING_EPS {
                    return None;
                }
                s
            };
            if s_prev * s_next < 0.0 {
                // edge endpoints straddle the probe arc's plane: test whether the
                // probe arc straddles the edge's plane on the matching side
                let cd = norms[i];
                let cbd = -(cd.x() * r.x() + cd.y() * r.y() + cd.z() * r.z());
                let dac = cd.x() * p.x() + cd.y() * p.y() + cd.z() * p.z();
                if cbd.abs() < CROSSING_EPS || dac.abs() < CROSSING_EPS {
                    return None;
                }
                let acb = -s_prev;
                if acb * cbd > 0.0 && acb * dac > 0.0 {
                    crossings += 1;
                }
            }
            s_prev = s_next;
        }
    }
    Some((crossings & 1) == 1)
}

/// Full containment test of a point: cap prefilter, then crossing test with winding fallback.
pub fn point_in_prepared_polygon(p: Cartesian, prep: &PreparedPolygon) -> bool {
    let cap = &prep.cap;
    if p.x() * cap.center.x() + p.y() * cap.center.y() + p.z() * cap.center.z() < cap.min_dot {
        return false;
    }
    if prep.use_fast {
        if let Some(result) = crossing_parity(p, prep) {
            return result;
        }
    }
    point_in_polygon_rings(p, &prep.ring_vecs_list)
}
