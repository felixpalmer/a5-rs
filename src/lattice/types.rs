// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use crate::coordinate_systems::IJ;

pub type Quaternary = u8; // 0, 1, 2, 3

pub const YES: i8 = -1;
pub const NO: i8 = 1;
pub type Flip = i8;

#[derive(Debug, Clone, PartialEq)]
pub struct Anchor {
    pub q: Quaternary,
    pub offset: IJ,
    pub flips: [Flip; 2],
}

/// Orientation of the Hilbert curve. The curve fills a space defined by the triangle with vertices
/// u, v & w. The orientation describes which corner the curve starts and ends at, e.g. wv is a
/// curve that starts at w and ends at v.
#[derive(Debug, Clone, Copy, PartialEq)]
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
