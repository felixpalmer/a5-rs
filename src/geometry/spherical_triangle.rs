// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use crate::coordinate_systems::{Cartesian, Radians};
use crate::geometry::{SphericalPolygon, SphericalPolygonShape};

#[derive(Debug)]
pub struct SphericalTriangleShape {
    inner: SphericalPolygonShape,
}

impl SphericalTriangleShape {
    pub fn new(vertices: SphericalPolygon) -> Result<Self, String> {
        if vertices.len() != 3 {
            return Err("SphericalTriangleShape requires exactly 3 vertices".to_string());
        }
        Ok(Self {
            inner: SphericalPolygonShape::new(vertices),
        })
    }

    /// Returns a closed boundary of the triangle, with n_segments points per edge
    pub fn get_boundary(&self, n_segments: usize, closed_ring: bool) -> SphericalPolygon {
        self.inner.get_boundary(n_segments, closed_ring)
    }

    /// Interpolates along boundary of triangle. Pass t = 1.5 to get the midpoint between 2nd and 3rd vertices
    pub fn slerp(&self, t: f64) -> Cartesian {
        self.inner.slerp(t)
    }

    /// Returns the vertex given by index t, along with the vectors:
    /// - VA: Vector from vertex to point A
    /// - VB: Vector from vertex to point B
    pub fn get_transformed_vertices(&self, t: f64) -> (Cartesian, Cartesian, Cartesian) {
        self.inner.get_transformed_vertices(t)
    }

    pub fn contains_point(&self, point: Cartesian) -> f64 {
        self.inner.contains_point(point)
    }

    /// Calculate the area of the spherical triangle
    pub fn get_area(&mut self) -> Radians {
        self.inner.get_area()
    }
}
