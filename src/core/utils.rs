// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use crate::coordinate_systems::{Radians, Spherical};
use crate::core::hilbert::Orientation;
use num_bigint::BigInt;

/// Origin identifier type (0-11)
pub type OriginId = u8;

/// Quaternion type - 4-element array [x, y, z, w]
pub type Quat = [f64; 4];

/// Origin represents one pentagon face of the dodecahedron
#[derive(Debug, Clone, PartialEq)]
pub struct Origin {
    /// Origin identifier (0-11)
    pub id: OriginId,
    /// Axis in spherical coordinates
    pub axis: Spherical,
    /// Quaternion for rotation
    pub quat: Quat,
    /// Inverse quaternion for reverse rotation
    pub inverse_quat: Quat,
    /// Angle in radians
    pub angle: Radians,
    /// Orientation array for Hilbert curve
    pub orientation: Vec<Orientation>,
    /// First quintant index
    pub first_quintant: usize,
}

/// A5 Cell represents a position in the A5 hierarchical indexing system
#[derive(Debug, Clone, PartialEq)]
pub struct A5Cell {
    /// Origin representing one of pentagon face of the dodecahedron
    pub origin: Origin,
    /// Index (0-4) of triangular segment within pentagonal dodecahedron face
    pub segment: usize,
    /// Position along Hilbert curve within triangular segment
    pub s: BigInt,
    /// Resolution of the cell
    pub resolution: i32,
}
