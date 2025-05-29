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
        // TODO:
        // Add limitations on degrees, i.e. 0 < 360 or -180-180
        Degrees(value)
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
