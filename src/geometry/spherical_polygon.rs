// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use crate::coordinate_systems::{Cartesian, Radians};
use crate::utils::vector::{slerp, triple_product};

/// Use Cartesian system for all calculations for greater accuracy
/// Using [x, y, z] gives equal precision in all directions, unlike spherical coordinates
pub type SphericalPolygon = Vec<Cartesian>;

pub struct SphericalPolygonShape {
    vertices: SphericalPolygon,
    area: Option<Radians>,
}

impl SphericalPolygonShape {
    pub fn new(vertices: SphericalPolygon) -> Self {
        // Note: TypeScript version has this.isWindingCorrect() commented out
        Self {
            vertices,
            area: None,
        }
    }

    /// Returns a closed boundary of the polygon, with n_segments points per edge
    pub fn get_boundary(&self, n_segments: usize, closed_ring: bool) -> SphericalPolygon {
        let mut points = Vec::new();
        let n = self.vertices.len();

        for s in 0..(n * n_segments) {
            let t = s as f64 / n_segments as f64;
            points.push(self.slerp(t));
        }

        if closed_ring && !points.is_empty() {
            points.push(points[0]);
        }

        points
    }

    /// Interpolates along boundary of polygon. Pass t = 1.5 to get the midpoint between 2nd and 3rd vertices
    pub fn slerp(&self, t: f64) -> Cartesian {
        let n = self.vertices.len();
        let f = t % 1.0;
        let i = (t % n as f64) as usize;
        let j = (i + 1) % n;
        slerp(self.vertices[i], self.vertices[j], f)
    }

    /// Returns the vertex given by index t, along with the vectors:
    /// - VA: Vector from vertex to point A
    /// - VB: Vector from vertex to point B
    pub fn get_transformed_vertices(&self, t: f64) -> (Cartesian, Cartesian, Cartesian) {
        let n = self.vertices.len();
        let i = (t % n as f64) as usize;
        let j = (i + 1) % n;
        let k = (i + n - 1) % n;

        // Points A & B (vertex before and after)
        let v = self.vertices[i];
        let va = subtract(self.vertices[j], v);
        let vb = subtract(self.vertices[k], v);
        (v, va, vb)
    }

    pub fn contains_point(&self, point: Cartesian) -> f64 {
        // Adaption of algorithm from:
        // 'Locating a point on a spherical surface relative to a spherical polygon'
        // Using only the condition of 'necessary strike'
        let n = self.vertices.len();
        let mut theta_delta_min = f64::INFINITY;

        for i in 0..n {
            // Transform point and neighboring vertices into coordinate system centered on vertex
            let (v, va, vb) = self.get_transformed_vertices(i as f64);
            let vp = subtract(point, v);

            // Normalize to obtain unit direction vectors
            let vp = normalize(vp);
            let va = normalize(va);
            let vb = normalize(vb);

            // Cross products will point away from the center of the sphere when
            // point P is within arc formed by VA and VB
            let cross_ap = cross(va, vp);
            let cross_pb = cross(vp, vb);

            // Dot product will be positive when point P is within arc formed by VA and VB
            // The magnitude of the dot product is the sine of the angle between the two vectors
            // which is the same as the angle for small angles.
            let sin_ap = dot(v, cross_ap);
            let sin_pb = dot(v, cross_pb);

            // By returning the minimum value we find the arc where the point is closest to being outside
            theta_delta_min = theta_delta_min.min(sin_ap).min(sin_pb);
        }

        // If point is inside all arcs, will return a position value
        // If point is on edge of arc, will return 0
        // If point is outside all arcs, will return -1, the further away from 0, the further away from the arc
        theta_delta_min
    }

    /// Calculate the area of a spherical triangle given three vertices
    fn get_triangle_area(&self, v1: Cartesian, v2: Cartesian, v3: Cartesian) -> Radians {
        // Calculate midpoints
        let mid_a = normalize(lerp(v2, v3, 0.5));
        let mid_b = normalize(lerp(v3, v1, 0.5));
        let mid_c = normalize(lerp(v1, v2, 0.5));

        // Calculate area using asin of dot product, clamped to valid range
        let s = triple_product(mid_a, mid_b, mid_c);
        let clamped = s.clamp(-1.0, 1.0);

        // sin(x) = x for x < 1e-8
        let area = if clamped.abs() < 1e-8 {
            2.0 * clamped
        } else {
            clamped.asin() * 2.0
        };

        Radians::new_unchecked(area)
    }

    /// Calculate the area of the spherical polygon by decomposing it into a fan of triangles
    pub fn get_area(&mut self) -> Radians {
        // Memoize the result since vertices are immutable
        if let Some(area) = self.area {
            return area;
        }

        let area = self.compute_area();
        self.area = Some(area);
        area
    }

    fn compute_area(&self) -> Radians {
        if self.vertices.len() < 3 {
            return Radians::new_unchecked(0.0);
        }

        if self.vertices.len() == 3 {
            return self.get_triangle_area(self.vertices[0], self.vertices[1], self.vertices[2]);
        }

        // Calculate center of polygon
        let mut center = Cartesian::new(0.0, 0.0, 0.0);
        for vertex in &self.vertices {
            center = add(center, *vertex);
        }
        center = normalize(center);

        // Sum fan of triangles around center
        let mut area = 0.0;
        for i in 0..self.vertices.len() {
            let v1 = self.vertices[i];
            let v2 = self.vertices[(i + 1) % self.vertices.len()];
            let tri_area = self.get_triangle_area(center, v1, v2);
            if !tri_area.get().is_nan() {
                area += tri_area.get();
            }
        }

        Radians::new_unchecked(area)
    }

    /// For debugging purposes, check if the winding order is correct
    /// In production, should always be correct
    #[allow(dead_code)]
    fn is_winding_correct(&mut self) -> bool {
        let area = self.get_area();
        area.get() > 0.0
    }
}

// Helper functions for 3D vector operations

/// Compute dot product of two vectors
fn dot(a: Cartesian, b: Cartesian) -> f64 {
    a.x() * b.x() + a.y() * b.y() + a.z() * b.z()
}

/// Compute cross product of two vectors
fn cross(a: Cartesian, b: Cartesian) -> Cartesian {
    Cartesian::new(
        a.y() * b.z() - a.z() * b.y(),
        a.z() * b.x() - a.x() * b.z(),
        a.x() * b.y() - a.y() * b.x(),
    )
}

/// Compute length of a vector
fn length(v: Cartesian) -> f64 {
    (v.x() * v.x() + v.y() * v.y() + v.z() * v.z()).sqrt()
}

/// Normalize a vector
fn normalize(v: Cartesian) -> Cartesian {
    let len = length(v);
    if len == 0.0 {
        return v;
    }
    Cartesian::new(v.x() / len, v.y() / len, v.z() / len)
}

/// Linear interpolation between two vectors
fn lerp(a: Cartesian, b: Cartesian, t: f64) -> Cartesian {
    Cartesian::new(
        a.x() + t * (b.x() - a.x()),
        a.y() + t * (b.y() - a.y()),
        a.z() + t * (b.z() - a.z()),
    )
}

/// Subtract two vectors
fn subtract(a: Cartesian, b: Cartesian) -> Cartesian {
    Cartesian::new(a.x() - b.x(), a.y() - b.y(), a.z() - b.z())
}

/// Add two vectors
fn add(a: Cartesian, b: Cartesian) -> Cartesian {
    Cartesian::new(a.x() + b.x(), a.y() + b.y(), a.z() + b.z())
}
