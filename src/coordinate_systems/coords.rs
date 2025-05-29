// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use super::{base::Radians, vec2::Vec2, vec3::Vec3};

// 2D coordinate systems

/// 2D cartesian coordinate system with origin at the center of
/// a dodecahedron face
pub struct Face(pub Vec2);

/// 2D planar coordinate system defined by the eigenvectors of
/// the lattice tiling
#[derive(Debug, PartialEq, Copy, Clone)]
pub struct IJ(pub Vec2);

/// 2D planar coordinate system formed by the transformation K -> I + J
#[derive(Debug, PartialEq, Copy, Clone)]
pub struct KJ(pub Vec2);

// 3D coordinate systems

/// 3D cartesian system centered on unit sphere/dodecahedron
#[derive(Debug, PartialEq, Copy, Clone)]
pub struct Cartesian(pub Vec3);
