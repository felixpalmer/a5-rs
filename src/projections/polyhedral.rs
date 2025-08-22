// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

// IVEA (Icosahedral Vertex Equal Area) projection implementation
// Adaptation of icoVertexGreatCircle.ec from DGGAL project
// BSD 3-Clause License
// 
// Copyright (c) 2014-2025, Ecere Corporation
// 
// Redistribution and use in source and binary forms, with or without
// modification, are permitted provided that the following conditions are met:
// 
// 1. Redistributions of source code must retain the above copyright notice, this
//    list of conditions and the following disclaimer.
// 
// 2. Redistributions in binary form must reproduce the above copyright notice,
//    this list of conditions and the following disclaimer in the documentation
//    and/or other materials provided with the distribution.
// 
// 3. Neither the name of the copyright holder nor the names of its
//    contributors may be used to endorse or promote products derived from
//    this software without specific prior written permission.
// 
// THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
// AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
// IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
// DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE
// FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
// DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
// SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER
// CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY,
// OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
// OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

use crate::coordinate_systems::{Barycentric, Cartesian, Face, FaceTriangle, SphericalTriangle};
use crate::core::coordinate_transforms::{barycentric_to_face, face_to_barycentric};
use crate::geometry::spherical_triangle::SphericalTriangleShape;
use crate::utils::vector::{quadruple_product, slerp, vector_difference};

/// Polyhedral projection implementing IVEA (Icosahedral Vertex Equal Area) projection
pub struct PolyhedralProjection;

impl PolyhedralProjection {
    /// Creates a new polyhedral projection instance
    pub fn new() -> Self {
        Self
    }

    /// Forward projection: converts a spherical point to face coordinates
    ///
    /// # Arguments
    ///
    /// * `v` - The spherical point to project
    /// * `spherical_triangle` - The spherical triangle vertices
    /// * `face_triangle` - The face triangle vertices
    ///
    /// # Returns
    ///
    /// The face coordinates
    pub fn forward(
        &self,
        v: Cartesian,
        spherical_triangle: SphericalTriangle,
        face_triangle: FaceTriangle,
    ) -> Face {
        let a = spherical_triangle.a;
        let b = spherical_triangle.b;
        let c = spherical_triangle.c;
        let mut triangle_shape = SphericalTriangleShape::new(vec![a, b, c])
            .expect("Failed to create spherical triangle");

        // When v is close to A, the quadruple product is unstable.
        // As we just need the intersection of two great circles we can use difference
        // between A and v, as it lies in the same plane of the great circle containing A & v
        let z = normalize(subtract(v, a));
        let p = normalize(quadruple_product(a, z, b, c));

        let h = vector_difference(a, v) / vector_difference(a, p);
        let area_abc = triangle_shape.get_area().get();
        let scaled_area = h / area_abc;
        let b_coords = Barycentric::new(
            1.0 - h,
            scaled_area * SphericalTriangleShape::new(vec![a, p, c])
                .expect("Failed to create spherical triangle")
                .get_area()
                .get(),
            scaled_area * SphericalTriangleShape::new(vec![a, b, p])
                .expect("Failed to create spherical triangle")
                .get_area()
                .get(),
        );
        barycentric_to_face(b_coords, face_triangle)
    }

    /// Inverse projection: converts face coordinates back to spherical coordinates
    ///
    /// # Arguments
    ///
    /// * `face_point` - The face coordinates
    /// * `face_triangle` - The face triangle vertices
    /// * `spherical_triangle` - The spherical triangle vertices
    ///
    /// # Returns
    ///
    /// The spherical coordinates
    pub fn inverse(
        &self,
        face_point: Face,
        face_triangle: FaceTriangle,
        spherical_triangle: SphericalTriangle,
    ) -> Cartesian {
        let a = spherical_triangle.a;
        let b = spherical_triangle.b;
        let c = spherical_triangle.c;
        let mut triangle_shape = SphericalTriangleShape::new(vec![a, b, c])
            .expect("Failed to create spherical triangle");
        let b_coords = face_to_barycentric(face_point, face_triangle);

        let threshold = 1.0 - 1e-14;
        if b_coords.u > threshold {
            return a;
        }
        if b_coords.v > threshold {
            return b;
        }
        if b_coords.w > threshold {
            return c;
        }

        let c1 = cross(b, c);
        let area_abc = triangle_shape.get_area().get();
        let h = 1.0 - b_coords.u;
        let r = b_coords.w / h;
        let alpha = r * area_abc;
        let s = alpha.sin();
        let half_c = (alpha / 2.0).sin();
        let cc = 2.0 * half_c * half_c; // Half angle formula

        let c01 = dot(a, b);
        let c12 = dot(b, c);
        let c20 = dot(c, a);
        let s12 = length(c1);

        let v = dot(a, c1); // Triple product of A, B, C. Constant??
        let f = s * v + cc * (c01 * c12 - c20);
        let g = cc * s12 * (1.0 + c01);
        let q = (2.0 / c12.acos()) * g.atan2(f);
        let p = slerp(b, c, q);
        let k = vector_difference(a, p);
        let t = self.safe_acos(h * k) / self.safe_acos(k);
        slerp(a, p, t)
    }

    /// Computes acos(1 - 2 * x * x) without loss of precision for small x
    ///
    /// # Arguments
    ///
    /// * `x` - Input value
    ///
    /// # Returns
    ///
    /// acos(1 - x)
    fn safe_acos(&self, x: f64) -> f64 {
        if x < 1e-3 {
            2.0 * x + x * x * x / 3.0
        } else {
            (1.0 - 2.0 * x * x).acos()
        }
    }
}

impl Default for PolyhedralProjection {
    fn default() -> Self {
        Self::new()
    }
}

// Helper functions for vector operations

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

/// Subtract two vectors
fn subtract(a: Cartesian, b: Cartesian) -> Cartesian {
    Cartesian::new(a.x() - b.x(), a.y() - b.y(), a.z() - b.z())
}