// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

/// 2D floating-point vector.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vec2 {
    pub x: f64,
    pub y: f64,
}

impl Eq for Vec2 {}

impl Vec2 {
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}
