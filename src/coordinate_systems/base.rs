// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

// Base types

/// Degrees
#[derive(Debug, PartialEq, Copy, Clone, Default)]
pub struct Degrees(pub f64);

#[allow(dead_code)]
impl Degrees {
    pub const fn new_unchecked(value: f64) -> Self {
        Degrees(value)
    }

    /// Create new Degrees with normalization to [-180, 180] range
    /// Allows flexibility for antimeridian-spanning coordinates
    pub fn new(value: f64) -> Self {
        let normalized = ((value + 180.0) % 360.0) - 180.0;
        Degrees(if normalized == -180.0 { 180.0 } else { normalized })
    }

    /// Create new Degrees for longitude with normalization to [-180, 180] range
    pub fn new_longitude(value: f64) -> Self {
        Self::new(value)
    }

    /// Create new Degrees for latitude with clamping to [-90, 90] range
    pub fn new_latitude(value: f64) -> Self {
        Degrees(value.clamp(-90.0, 90.0))
    }

    pub const fn get(&self) -> f64 {
        self.0
    }
}

/// Radians
#[derive(Debug, PartialEq, Copy, Clone, Default)]
pub struct Radians(pub f64);

impl Radians {
    pub const fn new_unchecked(value: f64) -> Self {
        Radians(value)
    }

    pub const fn get(&self) -> f64 {
        self.0
    }
}
