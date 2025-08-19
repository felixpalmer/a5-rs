// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use super::{vec2::Vec2, vec3::Vec3};

// 2D coordinate systems

/// 2D cartesian coordinate system with origin at the center of
/// a dodecahedron face
#[derive(Debug, PartialEq, Copy, Clone)]
pub struct Face(pub Vec2);

impl Face {
    pub fn new(x: f64, y: f64) -> Self {
        Face(Vec2::new(x, y))
    }

    pub fn x(&self) -> f64 {
        self.0.x
    }

    pub fn y(&self) -> f64 {
        self.0.y
    }
}

impl From<[f64; 2]> for Face {
    fn from(arr: [f64; 2]) -> Self {
        Face::new(arr[0], arr[1])
    }
}

impl From<Face> for [f64; 2] {
    fn from(face: Face) -> Self {
        [face.x(), face.y()]
    }
}

/// 2D planar coordinate system defined by the eigenvectors of
/// the lattice tiling
#[derive(Debug, PartialEq, Copy, Clone)]
pub struct IJ(pub Vec2);

impl IJ {
    pub fn new(x: f64, y: f64) -> Self {
        IJ(Vec2::new(x, y))
    }

    pub fn x(&self) -> f64 {
        self.0.x
    }

    pub fn y(&self) -> f64 {
        self.0.y
    }
}

/// 2D planar coordinate system formed by the transformation K -> I + J
#[derive(Debug, PartialEq, Copy, Clone)]
pub struct KJ(pub Vec2);

impl KJ {
    pub fn new(x: f64, y: f64) -> Self {
        KJ(Vec2::new(x, y))
    }

    pub fn x(&self) -> f64 {
        self.0.x
    }

    pub fn y(&self) -> f64 {
        self.0.y
    }
}

// 3D coordinate systems

/// 3D cartesian system centered on unit sphere/dodecahedron
#[derive(Debug, PartialEq, Copy, Clone)]
pub struct Cartesian(pub Vec3);

impl Cartesian {
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Cartesian(Vec3::new(x, y, z))
    }

    pub fn x(&self) -> f64 {
        self.0.x
    }

    pub fn y(&self) -> f64 {
        self.0.y
    }

    pub fn z(&self) -> f64 {
        self.0.z
    }
}

impl From<[f64; 3]> for Cartesian {
    fn from(arr: [f64; 3]) -> Self {
        Cartesian::new(arr[0], arr[1], arr[2])
    }
}

impl From<Cartesian> for [f64; 3] {
    fn from(cart: Cartesian) -> Self {
        [cart.x(), cart.y(), cart.z()]
    }
}

// Barycentric coordinates and triangle types

/// Barycentric coordinates for a triangle (sum to 1)
#[derive(Debug, PartialEq, Copy, Clone)]
pub struct Barycentric {
    pub u: f64,
    pub v: f64,
    pub w: f64,
}

impl Barycentric {
    pub fn new(u: f64, v: f64, w: f64) -> Self {
        Self { u, v, w }
    }

    /// Check if barycentric coordinates are valid (sum to 1)
    pub fn is_valid(&self) -> bool {
        (self.u + self.v + self.w - 1.0).abs() < f64::EPSILON
    }

    /// Check if point is inside triangle (all coordinates non-negative)
    pub fn is_inside_triangle(&self) -> bool {
        self.u >= 0.0 && self.v >= 0.0 && self.w >= 0.0
    }
}

impl From<[f64; 3]> for Barycentric {
    fn from(arr: [f64; 3]) -> Self {
        Barycentric::new(arr[0], arr[1], arr[2])
    }
}

impl From<Barycentric> for [f64; 3] {
    fn from(bary: Barycentric) -> Self {
        [bary.u, bary.v, bary.w]
    }
}

/// Triangle defined by three face coordinates
#[derive(Debug, PartialEq, Copy, Clone)]
pub struct FaceTriangle {
    pub a: Face,
    pub b: Face,
    pub c: Face,
}

impl FaceTriangle {
    pub fn new(a: Face, b: Face, c: Face) -> Self {
        Self { a, b, c }
    }
}

impl From<[Face; 3]> for FaceTriangle {
    fn from(arr: [Face; 3]) -> Self {
        FaceTriangle::new(arr[0], arr[1], arr[2])
    }
}

impl From<[[f64; 2]; 3]> for FaceTriangle {
    fn from(arr: [[f64; 2]; 3]) -> Self {
        FaceTriangle::new(
            Face::from(arr[0]),
            Face::from(arr[1]),
            Face::from(arr[2])
        )
    }
}

/// Triangle defined by three cartesian coordinates
#[derive(Debug, PartialEq, Copy, Clone)]
pub struct SphericalTriangle {
    pub a: Cartesian,
    pub b: Cartesian,
    pub c: Cartesian,
}

impl SphericalTriangle {
    pub fn new(a: Cartesian, b: Cartesian, c: Cartesian) -> Self {
        Self { a, b, c }
    }
}

impl From<[Cartesian; 3]> for SphericalTriangle {
    fn from(arr: [Cartesian; 3]) -> Self {
        SphericalTriangle::new(arr[0], arr[1], arr[2])
    }
}
