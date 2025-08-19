// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use super::base::Radians;
use super::spherical::Spherical;

/// 2D polar coordinate system with origin at the center of
/// a dodecahedron face
#[derive(Debug, PartialEq, Copy, Clone)]
pub struct Polar {
    pub rho: f64,
    pub gamma: Radians,
}

impl Polar {
    /// Create new polar coordinates
    ///
    /// where
    ///   - rho: radial distance from face center
    ///   - gamma: azimuthal angle
    pub const fn new(rho: f64, gamma: Radians) -> Self {
        Self { rho, gamma }
    }

    /// Get rho (radial distance from face center)
    pub const fn rho(&self) -> f64 {
        self.rho
    }

    /// Get gamma (azimuthal angle) in radians
    pub const fn gamma(&self) -> Radians {
        self.gamma
    }

    /// Project polar coordinates to spherical coordinates
    /// using gnomonic projection.
    pub fn project_gnomonic(&self) -> Spherical {
        let gamma = self.gamma;
        let rho = self.rho;
        Spherical::new(gamma, Radians::new_unchecked(rho.atan()))
    }
}
