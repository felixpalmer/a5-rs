// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use super::base::Radians;
use super::polar::Polar;

/// 3D spherical coordinate system centered on unit sphere/dodecahedron
#[derive(Debug, PartialEq, Copy, Clone, Default)]
pub struct Spherical {
    pub theta: Radians,
    pub phi: Radians,
}

impl Spherical {
    /// Create new spherical coordinates
    pub const fn new(theta: Radians, phi: Radians) -> Self {
        Self { theta, phi }
    }

    /// Get theta (azimuthal angle) in radians
    pub const fn theta(&self) -> Radians {
        self.theta
    }

    /// Get phi (polar angle) in radians
    pub const fn phi(&self) -> Radians {
        self.phi
    }

    /// Unproject spherical coordinates to polar
    /// coordinates using gnomonic projection.
    pub fn unproject_gnomonic(self) -> Polar {
        let theta = self.theta;
        let phi = self.phi;
        Polar::new(phi.get().tan(), theta)
    }
}
