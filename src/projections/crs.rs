// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use crate::coordinate_systems::{Cartesian, Radians, Spherical};
use crate::core::constants::{DISTANCE_TO_EDGE, DISTANCE_TO_VERTEX};
use crate::core::coordinate_transforms::to_cartesian;
use crate::core::origin::get_origins;
use std::f64::consts::PI;

/**
 * The Coordinate Reference System (CRS) of the dodecahedron is a set of 62 vertices:
 * - 12 face centers
 * - 20 vertices
 * - 30 edge midpoints
 *
 * The vertices are used as a rigid frame of reference for the dodecahedron in the
 * dodecahedron projection. By constructing them once, we can avoid recalculating
 * and be sure of their correctness.
 */
pub struct CRS {
    vertices: Vec<Cartesian>,
    invocations: usize,
}

impl CRS {
    pub fn new() -> Result<Self, String> {
        let mut crs = CRS {
            vertices: Vec::new(),
            invocations: 0,
        };

        crs.add_face_centers();
        crs.add_vertices();
        crs.add_midpoints();

        if crs.vertices.len() != 62 {
            return Err(format!(
                "Failed to construct CRS: vertices length is {} instead of 62",
                crs.vertices.len()
            ));
        }

        Ok(crs)
    }

    pub fn get_vertex(&mut self, point: Cartesian) -> Result<Cartesian, String> {
        self.invocations += 1;
        if self.invocations == 10000 {
            eprintln!("Warning: Too many CRS invocations, results should be cached");
        }

        for vertex in &self.vertices {
            if vec3_distance(&point, vertex) < 1e-5 {
                return Ok(*vertex);
            }
        }

        Err("Failed to find vertex in CRS".to_string())
    }

    fn add_face_centers(&mut self) {
        let origins = get_origins();
        for origin in origins {
            let cartesian = to_cartesian(origin.axis);
            self.add(cartesian);
        }
    }

    fn add_vertices(&mut self) {
        let phi_vertex = DISTANCE_TO_VERTEX.atan();

        let origins = get_origins();
        for origin in origins {
            for i in 0..5 {
                let theta_vertex = (2 * i + 1) as f64 * PI / 5.0;
                let spherical = Spherical::new(
                    Radians::new_unchecked(theta_vertex + origin.angle.get()),
                    Radians::new_unchecked(phi_vertex),
                );
                let mut vertex = to_cartesian(spherical);
                vertex = transform_quat(vertex, origin.quat);
                self.add(vertex);
            }
        }
    }

    fn add_midpoints(&mut self) {
        let phi_midpoint = DISTANCE_TO_EDGE.atan();

        let origins = get_origins();
        for origin in origins {
            for i in 0..5 {
                let theta_midpoint = (2 * i) as f64 * PI / 5.0;
                let spherical = Spherical::new(
                    Radians::new_unchecked(theta_midpoint + origin.angle.get()),
                    Radians::new_unchecked(phi_midpoint),
                );
                let mut midpoint = to_cartesian(spherical);
                midpoint = transform_quat(midpoint, origin.quat);
                self.add(midpoint);
            }
        }
    }

    fn add(&mut self, new_vertex: Cartesian) -> bool {
        let normalized = normalize(new_vertex);

        // Check if vertex already exists
        for existing_vertex in &self.vertices {
            if vec3_distance(&normalized, existing_vertex) < 1e-5 {
                return false;
            }
        }

        self.vertices.push(normalized);
        true
    }
}

impl Default for CRS {
    fn default() -> Self {
        Self::new().expect("Failed to create CRS")
    }
}

// Helper functions for vector operations

/// Compute distance between two 3D vectors
fn vec3_distance(a: &Cartesian, b: &Cartesian) -> f64 {
    let dx = a.x() - b.x();
    let dy = a.y() - b.y();
    let dz = a.z() - b.z();
    (dx * dx + dy * dy + dz * dz).sqrt()
}

/// Normalize a vector
fn normalize(v: Cartesian) -> Cartesian {
    let length = (v.x() * v.x() + v.y() * v.y() + v.z() * v.z()).sqrt();
    if length == 0.0 {
        return v;
    }
    Cartesian::new(v.x() / length, v.y() / length, v.z() / length)
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
