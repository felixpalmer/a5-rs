// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use crate::coordinate_systems::Radians;

/// Golden ratio
pub const PHI: f64 = 1.618033988749895; // (1 + sqrt(5)) / 2

/// 2π radians
pub const TWO_PI: Radians = Radians::new_unchecked(std::f64::consts::TAU);

/// 2π/5 radians
pub const TWO_PI_OVER_5: Radians = Radians::new_unchecked(std::f64::consts::TAU / 5.0);

/// π/5 radians
pub const PI_OVER_5: Radians = Radians::new_unchecked(std::f64::consts::PI / 5.0);

/// π/10 radians
pub const PI_OVER_10: Radians = Radians::new_unchecked(std::f64::consts::PI / 10.0);

/// Angle between pentagon faces (radians) = 116.565°
pub const DIHEDRAL_ANGLE: Radians = Radians::new_unchecked(2.0344439357957027); // 2 * atan(φ)

/// Angle between pentagon faces (radians) = 63.435°
pub const INTERHEDRAL_ANGLE: Radians = Radians::new_unchecked(1.1071487177940904); // π - dihedral_angle

/// Face edge angle = 58.28252558853899
pub const FACE_EDGE_ANGLE: Radians = Radians::new_unchecked(1.0172219678978514); // -0.5 * π + acos(-1 / sqrt(3 - φ))

/// Distance from center to edge of pentagon face
pub const DISTANCE_TO_EDGE: f64 = 0.6180339887498949; // (sqrt(5) - 1) / 2, which is φ - 1

/// Distance from center to vertex of pentagon face
pub const DISTANCE_TO_VERTEX: f64 = 0.7639320225002102; // 3 - sqrt(5), which is 2 * (2 - φ)

/// Radius of the inscribed sphere in dodecahedron
pub const R_INSCRIBED: f64 = 1.0;

/// Radius of the sphere that touches the dodecahedron's edge midpoints
pub const R_MIDEDGE: f64 = 1.1755705045849463; // sqrt(3 - φ)

/// Radius of the circumscribed sphere for dodecahedron
pub const R_CIRCUMSCRIBED: f64 = 1.2584085723648188; // sqrt(3) * R_MIDEDGE / φ
