// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use crate::coordinate_systems::{Degrees, Face, Radians};
use crate::core::constants::{DISTANCE_TO_EDGE, PI_OVER_10, PI_OVER_5};
use crate::geometry::PentagonShape;

// Pentagon vertex angles
pub const A: Degrees = Degrees::new_unchecked(72.0);
pub const B: Degrees = Degrees::new_unchecked(127.94543761193603);
pub const C: Degrees = Degrees::new_unchecked(108.0);
pub const D: Degrees = Degrees::new_unchecked(82.29202980963508);
pub const E: Degrees = Degrees::new_unchecked(149.7625318412527);

/// Pentagon vertices (a, b, c, d, e)
pub struct PentagonVertices {
    pub a: Face,
    pub b: Face,
    pub c: Face,
    pub d: Face,
    pub e: Face,
}

/// Triangle vertices (u, v, w) and angle V
pub struct TriangleVertices {
    pub u: Face,
    pub v: Face,
    pub w: Face,
    pub v_angle: Radians,
}

/// 2x2 Matrix for linear transformations
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mat2 {
    pub m00: f64,
    pub m01: f64,
    pub m10: f64,
    pub m11: f64,
}

impl Mat2 {
    pub fn new(m00: f64, m01: f64, m10: f64, m11: f64) -> Self {
        Self { m00, m01, m10, m11 }
    }

    pub fn from_cols(col0: Face, col1: Face) -> Self {
        Self {
            m00: col0.x(),
            m01: col1.x(),
            m10: col0.y(),
            m11: col1.y(),
        }
    }

    pub fn determinant(&self) -> f64 {
        self.m00 * self.m11 - self.m01 * self.m10
    }

    pub fn inverse(&self) -> Option<Mat2> {
        let det = self.determinant();
        if det.abs() < f64::EPSILON {
            return None;
        }

        let inv_det = 1.0 / det;
        Some(Mat2 {
            m00: self.m11 * inv_det,
            m01: -self.m01 * inv_det,
            m10: -self.m10 * inv_det,
            m11: self.m00 * inv_det,
        })
    }

    pub fn transform(&self, v: Face) -> Face {
        Face::new(
            self.m00 * v.x() + self.m01 * v.y(),
            self.m10 * v.x() + self.m11 * v.y(),
        )
    }
}

/// Lazy static values for pentagon definition
pub struct PentagonConstants {
    pub vertices: PentagonVertices,
    pub pentagon: PentagonShape,
    pub triangle_vertices: TriangleVertices,
    pub triangle: PentagonShape,
    pub basis: Mat2,
    pub basis_inverse: Mat2,
}

impl PentagonConstants {
    fn compute() -> Self {
        // Initial vertex definitions
        let mut a = Face::new(0.0, 0.0);
        let mut b = Face::new(0.0, 1.0);
        // c & d calculated by circle intersections. Perhaps can obtain geometrically.
        let mut c = Face::new(0.7885966681787006, 1.6149108024237764);
        let mut d = Face::new(1.6171013659387945, 1.054928690397459);
        let mut e = Face::new(PI_OVER_10.get().cos(), PI_OVER_10.get().sin());

        // Distance to edge midpoint
        let c_length = (c.x() * c.x() + c.y() * c.y()).sqrt();
        let edge_midpoint_d = 2.0 * c_length * PI_OVER_5.get().cos();

        // Lattice growth direction is AC, want to rotate it so that it is parallel to x-axis
        let basis_rotation = PI_OVER_5.get() - c.y().atan2(c.x()); // -27.97 degrees

        // Scale to match unit sphere
        let scale = 2.0 * DISTANCE_TO_EDGE / edge_midpoint_d;

        // Apply scaling and rotation to all vertices
        for vertex in [&mut a, &mut b, &mut c, &mut d, &mut e].iter_mut() {
            // Scale
            let scaled_x = vertex.x() * scale;
            let scaled_y = vertex.y() * scale;

            // Rotate
            let cos_angle = basis_rotation.cos();
            let sin_angle = basis_rotation.sin();
            let rotated_x = scaled_x * cos_angle - scaled_y * sin_angle;
            let rotated_y = scaled_x * sin_angle + scaled_y * cos_angle;
            **vertex = Face::new(rotated_x, rotated_y);
        }

        let pentagon = PentagonShape::new([a, b, c, d, e]);

        let bisector_angle = c.y().atan2(c.x()) - PI_OVER_5.get();

        // Define triangle also, as UVW
        let u = Face::new(0.0, 0.0);
        let l = DISTANCE_TO_EDGE / PI_OVER_5.get().cos();

        let v_angle_value = bisector_angle + PI_OVER_5.get();
        let v = Face::new(l * v_angle_value.cos(), l * v_angle_value.sin());

        let w_angle = bisector_angle - PI_OVER_5.get();
        let w = Face::new(l * w_angle.cos(), l * w_angle.sin());

        let triangle = PentagonShape::new([u, v, w, Face::new(0.0, 0.0), Face::new(0.0, 0.0)]);

        // Basis vectors used to layout primitive unit
        let basis = Mat2::from_cols(v, w);
        let basis_inverse = basis.inverse().expect("Basis matrix should be invertible");

        Self {
            vertices: PentagonVertices { a, b, c, d, e },
            pentagon,
            triangle_vertices: TriangleVertices {
                u,
                v,
                w,
                v_angle: Radians::new_unchecked(v_angle_value),
            },
            triangle,
            basis,
            basis_inverse,
        }
    }
}

/// Global pentagon constants
static PENTAGON_CONSTANTS: std::sync::LazyLock<PentagonConstants> =
    std::sync::LazyLock::new(PentagonConstants::compute);

/// Pentagon vertex a
pub fn a() -> Face {
    PENTAGON_CONSTANTS.vertices.a
}

/// Pentagon vertex b
pub fn b() -> Face {
    PENTAGON_CONSTANTS.vertices.b
}

/// Pentagon vertex c
pub fn c() -> Face {
    PENTAGON_CONSTANTS.vertices.c
}

/// Pentagon vertex d
pub fn d() -> Face {
    PENTAGON_CONSTANTS.vertices.d
}

/// Pentagon vertex e
pub fn e() -> Face {
    PENTAGON_CONSTANTS.vertices.e
}

/// Pentagon shape definition
pub fn pentagon() -> &'static PentagonShape {
    &PENTAGON_CONSTANTS.pentagon
}

/// Triangle vertex u
pub fn u() -> Face {
    PENTAGON_CONSTANTS.triangle_vertices.u
}

/// Triangle vertex v
pub fn v() -> Face {
    PENTAGON_CONSTANTS.triangle_vertices.v
}

/// Triangle vertex w
pub fn w() -> Face {
    PENTAGON_CONSTANTS.triangle_vertices.w
}

/// Triangle angle V
pub fn v_angle() -> Radians {
    PENTAGON_CONSTANTS.triangle_vertices.v_angle
}

/// Triangle shape definition
pub fn triangle() -> &'static PentagonShape {
    &PENTAGON_CONSTANTS.triangle
}

/// Basis matrix for coordinate transformations
pub fn basis() -> Mat2 {
    PENTAGON_CONSTANTS.basis
}

/// Inverse basis matrix
pub fn basis_inverse() -> Mat2 {
    PENTAGON_CONSTANTS.basis_inverse
}
