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
use crate::geometry::spherical_polygon::spherical_triangle_area;

/// Constants derived from the (single, canonical) face triangle: the forward
/// scalars `volume_abc` / `area_abc`, the inverse coefficient matrix
/// (`alpha_transform`), and `a_dot_b` / `a_dot_c`, which tell a face's B-C
/// orientation apart.
///
/// All dodecahedron face triangles are congruent, so these are computed once and
/// reused. `volume_abc` is a *signed* triple product, so this is only valid while
/// every triangle shares the same winding (chirality) — `DodecahedronProjection`
/// guarantees this. A mirror-image ("odd") face swaps A·B and A·C; `inverse()`
/// normalizes it to the canonical ("even") orientation. Enforced by the
/// constants-agreement test in `dodecahedron.rs`.
#[derive(Debug, Clone, Copy)]
pub struct TriangleConstants {
    /// A · B — the canonical ("even") B-C orientation
    pub a_dot_b: f64,
    /// A · C — the mirror ("odd") orientation, with B and C swapped
    pub a_dot_c: f64,
    /// Affine transform of `[cos(alpha), sin(alpha)]` to the B/C weights of P.
    /// gl-matrix mat2d order `[a, b, c, d, tx, ty] = [c1c, c2c, c1s, c2s, c1o, c2o]`.
    pub alpha_transform: [f64; 6],
    /// Spherical triangle area
    pub area_abc: f64,
    /// A · (B × C) — signed triple product (volume of parallelepiped)
    pub volume_abc: f64,
}

/// Equal area projection originally described by Snyder92 (AN EQUAL-AREA MAP
/// PROJECTION FOR POLYHEDRAL GLOBES), with closed-form equations due to
/// Brenton R. S. Recht.
///
/// The projection maps a point V within a spherical triangle ABC onto a planar
/// point F, in an equal-area-preserving manner. Vertex A is the "radiating
/// vertex": the transformation goes via an intermediate point P, where the
/// great circles through A&V and B&C intersect, then takes the ratio of the
/// areas of triangles ABP & ABC.
pub struct EqualAreaProjection {
    /// Precomputed constants; see [`TriangleConstants`].
    constants: TriangleConstants,
}

impl EqualAreaProjection {
    /// Creates a new equal-area projection with shape constants derived from
    /// the canonical triangle
    pub fn new(canonical_triangle: SphericalTriangle) -> Self {
        Self {
            constants: Self::compute_constants(canonical_triangle),
        }
    }

    /// Computes the constants of a spherical triangle
    pub fn compute_constants(spherical_triangle: SphericalTriangle) -> TriangleConstants {
        let a = spherical_triangle.a;
        let b = spherical_triangle.b;
        let c = spherical_triangle.c;
        let bxc = cross(b, c);
        let a_dot_b = dot(a, b);
        let a_dot_c = dot(a, c);
        let b_dot_c = dot(b, c);

        let v = dot(a, bxc);
        let p = a_dot_c + b_dot_c;
        let q = a_dot_b + 1.0;
        let r = a_dot_b * b_dot_c - a_dot_c;
        let f = p * p - q * q;
        let g = 2.0 * q * r;
        // mat2d order [a, b, c, d, tx, ty] = [c1c, c2c, c1s, c2s, c1o, c2o]
        let alpha_transform = [v * v - f, -g, -2.0 * v * p, 2.0 * v * q, v * v + f, g];

        TriangleConstants {
            a_dot_b,
            a_dot_c,
            alpha_transform,
            area_abc: spherical_triangle_area(a, b, c).get(),
            volume_abc: v,
        }
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
        let area_abc = self.constants.area_abc;
        let volume_abc = self.constants.volume_abc;

        // Compute point P, where great circles through A&V and B&C intersect
        let bxc = cross(b, c);
        let volume_vbc = dot(v, bxc);
        let mut p = scale_and_add(scale(v, volume_abc), a, -volume_vbc);
        let d = length(p);
        let oo_d = if d > 0.0 { 1.0 / d } else { 1.0 };
        p = scale(p, oo_d);

        // Obtain rho & alpha by ratio of areas
        let area_abp = spherical_triangle_area(a, b, p).get().max(0.0);
        let alpha = area_abp / area_abc;
        let rho = (d / volume_abc) * ((1.0 + dot(a, p)) / (1.0 + dot(a, v))).sqrt();

        // Construct barycentric triangle and map to face
        let b_coords = Barycentric::new(1.0 - rho, rho * (1.0 - alpha), rho * alpha);
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

        // Normalize odd (mirror-image) triangles to the canonical even orientation
        // by swapping B↔C and the matching weight b1↔b2, so alpha_transform is correct
        let TriangleConstants {
            a_dot_b,
            a_dot_c,
            alpha_transform: m,
            area_abc,
            ..
        } = self.constants;
        let face_a_dot_b = dot(a, b);
        let odd = (face_a_dot_b - a_dot_b).abs() > (face_a_dot_b - a_dot_c).abs();
        let bn = if odd { c } else { b };
        let cn = if odd { b } else { c };
        let b2 = if odd { b_coords.v } else { b_coords.w };

        // Obtain rho & alpha
        let rho = 1.0 - b_coords.u;
        let alpha = (b2 / rho) * area_abc;

        // Inverse to obtain point P (see forward). weight = alpha_transform * [cos, sin]
        let cos_a = alpha.cos();
        let sin_a = alpha.sin();
        let weight_b = m[0] * cos_a + m[2] * sin_a + m[4];
        let weight_c = m[1] * cos_a + m[3] * sin_a + m[5];
        let p = normalize(scale_and_add(scale(bn, weight_b), cn, weight_c));

        // Compute weights for A & P
        let s = dot(a, p);
        let t = 1.0 + rho * rho * (s - 1.0);
        let weight_p = rho * ((1.0 + t) / (1.0 + s)).sqrt();
        let weight_a = t - s * weight_p;
        scale_and_add(scale(a, weight_a), p, weight_p)
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

/// Scale a vector by a scalar
fn scale(v: Cartesian, s: f64) -> Cartesian {
    Cartesian::new(v.x() * s, v.y() * s, v.z() * s)
}

/// Returns `a + b * s`
fn scale_and_add(a: Cartesian, b: Cartesian, s: f64) -> Cartesian {
    Cartesian::new(a.x() + b.x() * s, a.y() + b.y() * s, a.z() + b.z() * s)
}
