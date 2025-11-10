// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use crate::coordinate_systems::{
    Cartesian, Face, FaceTriangle, Polar, Radians, Spherical, SphericalTriangle,
};
use crate::core::constants::{DISTANCE_TO_EDGE, INTERHEDRAL_ANGLE, PI_OVER_5, TWO_PI_OVER_5};
use crate::core::coordinate_transforms::{to_cartesian, to_face, to_polar, to_spherical};
use crate::core::origin::get_origins;
use crate::core::tiling::get_quintant_vertices;
use crate::core::utils::OriginId;
use crate::projections::crs::CRS;
use crate::projections::gnomonic::GnomonicProjection;
use crate::projections::polyhedral::PolyhedralProjection;
use std::thread_local;

type FaceTriangleIndex = usize; // 0-9

thread_local! {
    // Each thread gets its own heap-allocated DodecahedronProjection
    static THREAD_DODECA: *mut DodecahedronProjection = {
        let b = Box::new(DodecahedronProjection::new().unwrap());
        Box::into_raw(b) // raw pointer, lifetime is tied to thread
    };
}

pub struct DodecahedronProjection {
    face_triangles: Vec<Option<FaceTriangle>>,
    spherical_triangles: Vec<Option<SphericalTriangle>>,
    polyhedral: PolyhedralProjection,
    gnomonic: GnomonicProjection,
    crs: CRS,
}

impl DodecahedronProjection {
    pub fn new() -> Result<Self, String> {
        Ok(DodecahedronProjection {
            face_triangles: vec![None; 30], // 10 base + 10 reflected + 10 squashed
            spherical_triangles: vec![None; 240], // 120 base + 120 reflected
            polyhedral: PolyhedralProjection::new(),
            gnomonic: GnomonicProjection,
            crs: CRS::new()?,
        })
    }

    /// Get a reference to the thread local dodecahedron projection instance
    pub fn get_thread_local() -> &'static mut DodecahedronProjection {
        THREAD_DODECA.with(|ptr| unsafe { &mut **ptr })
    }

    /// Projects spherical coordinates to face coordinates using dodecahedron projection
    pub fn forward(&mut self, spherical: Spherical, origin_id: OriginId) -> Result<Face, String> {
        let origins = get_origins();
        if (origin_id as usize) >= origins.len() {
            return Err("Invalid origin ID".to_string());
        }
        let origin = &origins[origin_id as usize];

        // Transform back origin space
        let unprojected = to_cartesian(spherical);
        let out = transform_quat(unprojected, origin.inverse_quat);

        // Unproject gnomonically to polar coordinates in origin space
        let projected_spherical = to_spherical(out);
        let polar = self.gnomonic.forward(projected_spherical);

        // Rotate around face axis to remove origin rotation
        let rotated_polar = Polar::new(
            polar.rho(),
            Radians::new_unchecked(polar.gamma().get() - origin.angle.get()),
        );

        let face_triangle_index = self.get_face_triangle_index(rotated_polar)?;
        let reflect = self.should_reflect(rotated_polar);
        let face_triangle = self.get_face_triangle(face_triangle_index, reflect, false)?;
        let spherical_triangle =
            self.get_spherical_triangle(face_triangle_index, origin_id, reflect)?;

        Ok(self
            .polyhedral
            .forward(unprojected, spherical_triangle, face_triangle))
    }

    /// Unprojects face coordinates to spherical coordinates using dodecahedron projection
    pub fn inverse(&mut self, face: Face, origin_id: OriginId) -> Result<Spherical, String> {
        let polar = to_polar(face);
        let face_triangle_index = self.get_face_triangle_index(polar)?;

        let reflect = self.should_reflect(polar);
        let face_triangle = self.get_face_triangle(face_triangle_index, reflect, false)?;
        let spherical_triangle =
            self.get_spherical_triangle(face_triangle_index, origin_id, reflect)?;
        let unprojected = self
            .polyhedral
            .inverse(face, face_triangle, spherical_triangle);
        Ok(to_spherical(unprojected))
    }

    /// Detects when point is beyond the edge of the dodecahedron face
    fn should_reflect(&self, polar: Polar) -> bool {
        let normalized_gamma = self.normalize_gamma(polar.gamma());
        let test_polar = Polar::new(polar.rho(), normalized_gamma);
        let d = to_face(test_polar).x();
        d > DISTANCE_TO_EDGE
    }

    /// Given a polar coordinate, returns the index of the face triangle it belongs to
    fn get_face_triangle_index(&self, polar: Polar) -> Result<FaceTriangleIndex, String> {
        let gamma = polar.gamma().get();
        let index = ((gamma / PI_OVER_5.get()).floor() as i32 + 10) % 10;
        if index < 0 {
            Ok((index + 10) as usize)
        } else {
            Ok(index as usize)
        }
    }

    /// Gets the face triangle for a given polar coordinate
    fn get_face_triangle(
        &mut self,
        face_triangle_index: FaceTriangleIndex,
        reflected: bool,
        squashed: bool,
    ) -> Result<FaceTriangle, String> {
        if face_triangle_index > 9 {
            return Err("Face triangle index must be 0-9".to_string());
        }
        let mut index = face_triangle_index;
        if reflected {
            index += if squashed { 20 } else { 10 };
        }

        if index >= self.face_triangles.len() {
            return Err("Face triangle index out of bounds".to_string());
        }

        if let Some(cached) = &self.face_triangles[index] {
            return Ok(*cached);
        }

        let face_triangle = if reflected {
            self.get_reflected_face_triangle(face_triangle_index, squashed)?
        } else {
            self.get_base_face_triangle(face_triangle_index)?
        };

        self.face_triangles[index] = Some(face_triangle);
        Ok(face_triangle)
    }

    fn get_base_face_triangle(
        &self,
        face_triangle_index: FaceTriangleIndex,
    ) -> Result<FaceTriangle, String> {
        let quintant = face_triangle_index.div_ceil(2) % 5;
        let vertices = get_quintant_vertices(quintant);
        let verts = vertices.get_vertices();
        if verts.len() < 3 {
            return Err("Triangle vertices not available".to_string());
        }
        let (v_center, v_corner1, v_corner2) = (verts[0], verts[1], verts[2]);

        let v_edge_midpoint = Face::new(
            (v_corner1.x() + v_corner2.x()) / 2.0,
            (v_corner1.y() + v_corner2.y()) / 2.0,
        );

        let even = face_triangle_index % 2 == 0;

        // Note: center & midpoint compared to DGGAL implementation are swapped
        // as we are using a dodecahedron, rather than an icosahedron.
        Ok(if even {
            FaceTriangle::new(v_center, v_edge_midpoint, v_corner1)
        } else {
            FaceTriangle::new(v_center, v_corner2, v_edge_midpoint)
        })
    }

    fn get_reflected_face_triangle(
        &self,
        face_triangle_index: FaceTriangleIndex,
        squashed: bool,
    ) -> Result<FaceTriangle, String> {
        // First obtain ordinary unreflected triangle
        let base = self.get_base_face_triangle(face_triangle_index)?;
        let (mut a, b, c) = (base.a, base.b, base.c);

        // Reflect dodecahedron center (A) across edge (BC)
        let even = face_triangle_index % 2 == 0;
        a = Face::new(-a.x(), -a.y());
        let midpoint = if even { b } else { c };

        // Squashing is important. A squashed triangle when unprojected will yield the correct spherical triangle.
        let scale = if squashed {
            1.0 + 1.0 / INTERHEDRAL_ANGLE.get().cos()
        } else {
            2.0
        };
        a = Face::new(a.x() + midpoint.x() * scale, a.y() + midpoint.y() * scale);

        // Swap midpoint and corner to maintain correct vertex order
        Ok(FaceTriangle::new(a, c, b))
    }

    /// Gets the spherical triangle for a given face triangle index and origin
    fn get_spherical_triangle(
        &mut self,
        face_triangle_index: FaceTriangleIndex,
        origin_id: OriginId,
        reflected: bool,
    ) -> Result<SphericalTriangle, String> {
        let mut index = 10 * (origin_id as usize) + face_triangle_index; // 0-119
        if reflected {
            index += 120;
        }

        if index >= self.spherical_triangles.len() {
            return Err("Spherical triangle index out of bounds".to_string());
        }

        if let Some(cached) = &self.spherical_triangles[index] {
            return Ok(*cached);
        }

        let spherical_triangle =
            self.compute_spherical_triangle(face_triangle_index, origin_id, reflected)?;
        self.spherical_triangles[index] = Some(spherical_triangle);
        Ok(spherical_triangle)
    }

    fn compute_spherical_triangle(
        &mut self,
        face_triangle_index: FaceTriangleIndex,
        origin_id: OriginId,
        reflected: bool,
    ) -> Result<SphericalTriangle, String> {
        let origins = get_origins();
        if (origin_id as usize) >= origins.len() {
            return Err("Invalid origin ID".to_string());
        }
        let origin = &origins[origin_id as usize];

        let face_triangle = self.get_face_triangle(face_triangle_index, reflected, true)?;

        let mut spherical_vertices = Vec::new();
        for face in [face_triangle.a, face_triangle.b, face_triangle.c] {
            let polar = to_polar(face);
            let rotated_polar = Polar::new(
                polar.rho(),
                Radians::new_unchecked(polar.gamma().get() + origin.angle.get()),
            );
            let rotated = to_cartesian(self.gnomonic.inverse(rotated_polar));
            let transformed = transform_quat(rotated, origin.quat);
            let vertex = self.crs.get_vertex(transformed)?;
            spherical_vertices.push(vertex);
        }

        Ok(SphericalTriangle::new(
            spherical_vertices[0],
            spherical_vertices[1],
            spherical_vertices[2],
        ))
    }

    /// Normalizes gamma to the range [-PI_OVER_5, PI_OVER_5]
    fn normalize_gamma(&self, gamma: Radians) -> Radians {
        let segment = gamma.get() / TWO_PI_OVER_5.get();
        let s_center = segment.round();
        let s_offset = segment - s_center;

        // Azimuthal angle from triangle bisector
        let beta = s_offset * TWO_PI_OVER_5.get();
        Radians::new_unchecked(beta)
    }
}

impl Default for DodecahedronProjection {
    fn default() -> Self {
        Self::new().expect("Failed to create DodecahedronProjection")
    }
}

/// Transform a vector by a quaternion
fn transform_quat(v: Cartesian, q: [f64; 4]) -> Cartesian {
    let [qx, qy, qz, qw] = q;

    // First, convert vector to quaternion (w=0)
    let vx = v.x();
    let vy = v.y();
    let vz = v.z();

    // Compute q * v * q^(-1)
    // q^(-1) = conjugate(q) / |q|^2, but since q is unit quaternion, q^(-1) = conjugate(q)
    let qconj_x = -qx;
    let qconj_y = -qy;
    let qconj_z = -qz;
    let qconj_w = qw;

    // First multiplication: q * v
    let t1_x = qw * vx + qy * vz - qz * vy;
    let t1_y = qw * vy + qz * vx - qx * vz;
    let t1_z = qw * vz + qx * vy - qy * vx;
    let t1_w = -qx * vx - qy * vy - qz * vz;

    // Second multiplication: (q * v) * q^(-1)
    let result_x = t1_w * qconj_x + t1_x * qconj_w + t1_y * qconj_z - t1_z * qconj_y;
    let result_y = t1_w * qconj_y + t1_y * qconj_w + t1_z * qconj_x - t1_x * qconj_z;
    let result_z = t1_w * qconj_z + t1_z * qconj_w + t1_x * qconj_y - t1_y * qconj_x;

    Cartesian::new(result_x, result_y, result_z)
}
