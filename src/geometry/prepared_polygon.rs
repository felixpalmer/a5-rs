// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

// Spherical polygon (with holes) prepared for repeated point-containment
// tests: bounding-cap prefilter, then a trig-free crossing-number test with
// the winding-number test as a robust fallback.

use crate::coordinate_systems::Cartesian;
use crate::geometry::spherical_polygon::{
    point_in_spherical_polygon, ring_segment_normals, ring_winding_sign,
};
use crate::utils::vector::angle;

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
    /// Cap half-angle in radians; kept alongside min_dot so no lossy
    /// acos(min_dot) round-trip is needed
    pub angle: f64,
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
            angle: std::f64::consts::PI,
            min_dot: -1.0,
        };
    }
    cx /= len;
    cy /= len;
    cz /= len;
    let center = Cartesian::new(cx, cy, cz);

    // angle (2·atan2 form) keeps full precision for tiny polygons, where
    // acos(dot) would lose half the digits carried on near-parallel vectors
    let mut max_angle = 0.0_f64;
    let mut max_edge = 0.0_f64;
    for ring_vecs in ring_vecs_list {
        let n = ring_vecs.len();
        for i in 0..n {
            let v = ring_vecs[i];
            let w = ring_vecs[(i + 1) % n];
            max_angle = max_angle.max(angle(center, v));
            max_edge = max_edge.max(angle(v, w));
        }
    }
    let cap_angle = std::f64::consts::PI.min(max_angle + max_edge / 2.0);
    BoundingCap {
        center,
        angle: cap_angle,
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
    /// Reference points known to be INSIDE the polygon, for polygons whose
    /// bounding cap reaches a hemisphere; empty otherwise (see `interior_refs`).
    pub inside_refs: Vec<Cartesian>,
}

pub fn prepare_polygon(ring_vecs_list: Vec<Vec<Cartesian>>) -> PreparedPolygon {
    let cap = bounding_cap(&ring_vecs_list);
    let ring_normals: Vec<Vec<Cartesian>> = ring_vecs_list
        .iter()
        .map(|ring| ring_segment_normals(ring))
        .collect();
    let cap_angle = cap.angle;
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
    let inside_refs = if cap_angle >= std::f64::consts::FRAC_PI_2 {
        interior_refs(&ring_vecs_list[0], &ring_normals[0])
    } else {
        Vec::new()
    };
    PreparedPolygon {
        ring_vecs_list,
        ring_normals,
        cap,
        reference,
        use_fast,
        inside_refs,
    }
}

// How far the interior reference points sit from the ring edge, in radians.
// Far above CROSSING_EPS so crossing tests against the edge stay well
// conditioned, far below any cell size so they can't clip another edge.
const INTERIOR_REF_OFFSET: f64 = 1e-7;
const INTERIOR_REF_COUNT: usize = 3;

/// Unit vector, scaled by the reciprocal length (same rounding as the
/// TypeScript vec3.normalize).
fn normalize(x: f64, y: f64, z: f64) -> Cartesian {
    let mut len = x * x + y * y + z * z;
    if len > 0.0 {
        len = 1.0 / len.sqrt();
    }
    Cartesian::new(x * len, y * len, z * len)
}

/// Points just inside the midpoints of the outer ring's longest edges.
///
/// The winding-number test answers "is the point in the region that does not
/// contain the point's own antipode", which is only containment when the
/// polygon lies within a hemisphere. For larger polygons a crossing test is
/// needed, and that needs a reference point whose containment is known. The
/// interior side of each edge follows the ring's winding (`ring_winding_sign`),
/// the same convention the boundary-cell filter uses. Several are kept in case
/// a probe falls near-degenerately against one.
fn interior_refs(ring: &[Cartesian], normals: &[Cartesian]) -> Vec<Cartesian> {
    let side = ring_winding_sign(ring) as f64;
    let n = ring.len();
    let lengths: Vec<f64> = (0..n).map(|i| angle(ring[i], ring[(i + 1) % n])).collect();
    // Longest first; sort_by is stable, so ties keep ring order
    let mut edges: Vec<usize> = (0..n).collect();
    edges.sort_by(|&a, &b| lengths[b].partial_cmp(&lengths[a]).unwrap());
    let mut refs = Vec::with_capacity(INTERIOR_REF_COUNT);
    for &i in edges.iter().take(INTERIOR_REF_COUNT) {
        let (a, b) = (ring[i], ring[(i + 1) % n]);
        let mid = normalize(a.x() + b.x(), a.y() + b.y(), a.z() + b.z());
        // For a counter-clockwise ring, each edge's normal points to its interior side
        let inward = normalize(normals[i].x(), normals[i].y(), normals[i].z());
        let s = side * INTERIOR_REF_OFFSET;
        refs.push(normalize(
            mid.x() + inward.x() * s,
            mid.y() + inward.y() * s,
            mid.z() + inward.z() * s,
        ));
    }
    refs
}

const CROSSING_EPS: f64 = 1e-14;

/// Crossing-number containment: count proper crossings of the arc probe->ref
/// with every ring edge (just sign tests — no trig); odd parity = inside
/// (`ref` is outside the polygon, and the even-odd rule handles holes for
/// free). Returns None on any near-degenerate sign (probe or a vertex on
/// an arc plane) — the caller falls back to the winding test, which also keeps
/// on-edge tie-breaking identical to the previous implementation.
fn crossing_parity(p: Cartesian, prep: &PreparedPolygon, r: Cartesian) -> Option<bool> {
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
    // Polygons reaching a hemisphere: crossing parity against a point known to
    // be inside (even parity = same side = inside)
    for inside_ref in &prep.inside_refs {
        if let Some(result) = crossing_parity(p, prep, *inside_ref) {
            return !result;
        }
    }
    if prep.use_fast {
        if let Some(result) = crossing_parity(p, prep, prep.reference) {
            return result;
        }
    }
    point_in_polygon_rings(p, &prep.ring_vecs_list)
}
