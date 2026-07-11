// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

/// Orientation of the space-filling curve. The curve fills a space defined by the triangle with
/// vertices u, v & w. The orientation describes which corner the curve starts and ends at, e.g. wv
/// is a curve that starts at w and ends at v.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Orientation {
    UV,
    VU,
    UW,
    WU,
    VW,
    WV,
}

impl std::str::FromStr for Orientation {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "uv" => Ok(Self::UV),
            "vu" => Ok(Self::VU),
            "uw" => Ok(Self::UW),
            "wu" => Ok(Self::WU),
            "vw" => Ok(Self::VW),
            "wv" => Ok(Self::WV),
            _ => Err(format!("Unknown orientation: {}", s)),
        }
    }
}

/// Triple coordinates for the triangular grid underlying the pentagonal A5 grid.
///
/// Neighbors differ by ±1 in exactly one coordinate while the other two stay constant.
/// Triple coordinates are orientation-independent — the same geometric cell always has
/// the same triple coords regardless of curve orientation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Triple {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl Triple {
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }
}
