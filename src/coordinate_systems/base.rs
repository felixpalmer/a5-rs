// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

//! Base types for coordinate systems.
//!
//! This module provides fundamental angle types ([`Degrees`] and [`Radians`])
//! used throughout the coordinate system implementations.

/// Angle measurement in degrees.
///
/// This type provides safe handling of degree values with appropriate
/// normalization for longitude and latitude coordinates.
#[derive(Debug, PartialEq, Copy, Clone, Default)]
pub struct Degrees(pub f64);

impl Degrees {
    pub const fn new_unchecked(value: f64) -> Self {
        Degrees(value)
    }

    /// Create new Degrees with normalization to [-180, 180] range
    /// Allows flexibility for antimeridian-spanning coordinates
    pub fn new(value: f64) -> Self {
        let mut normalized = value % 360.0;
        if normalized > 180.0 {
            normalized -= 360.0;
        } else if normalized <= -180.0 {
            normalized += 360.0;
        }
        Degrees(normalized)
    }

    /// Create new Degrees for longitude with normalization to [-180, 180] range
    pub fn new_longitude(value: f64) -> Self {
        Self::new(value)
    }

    /// Create new Degrees for latitude with clamping to [-90, 90] range
    pub fn new_latitude(value: f64) -> Self {
        Degrees(value.clamp(-90.0, 90.0))
    }

    /// Get the raw value in degrees
    pub const fn get(&self) -> f64 {
        self.0
    }

    /// Convert to radians
    pub fn to_radians(self) -> Radians {
        Radians::new_unchecked(self.0.to_radians())
    }
}

impl From<f64> for Degrees {
    fn from(value: f64) -> Self {
        Self::new(value)
    }
}

impl From<Degrees> for f64 {
    fn from(degrees: Degrees) -> Self {
        degrees.get()
    }
}

/// Angle measurement in radians.
///
/// This type provides safe handling of radian values commonly used
/// in mathematical calculations and coordinate transformations.
#[derive(Debug, PartialEq, Copy, Clone, Default)]
pub struct Radians(pub f64);

impl Radians {
    pub const fn new_unchecked(value: f64) -> Self {
        Radians(value)
    }

    /// Create new Radians with normalization to [0, 2π] range
    pub fn new(value: f64) -> Self {
        use std::f64::consts::TAU; // 2π
        let normalized = value % TAU;
        Radians(if normalized < 0.0 { normalized + TAU } else { normalized })
    }

    /// Get the raw value in radians
    pub const fn get(&self) -> f64 {
        self.0
    }

    /// Convert to degrees
    pub fn to_degrees(self) -> Degrees {
        Degrees::new_unchecked(self.0.to_degrees())
    }
}

impl From<f64> for Radians {
    fn from(value: f64) -> Self {
        Self::new(value)
    }
}

impl From<Radians> for f64 {
    fn from(radians: Radians) -> Self {
        radians.get()
    }
}
