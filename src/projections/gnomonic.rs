// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use crate::coordinate_systems::{Polar, Radians, Spherical};

/// Gnomonic projection implementation that converts between spherical and polar coordinates.
pub struct GnomonicProjection;

impl GnomonicProjection {
    /// Projects spherical coordinates to polar coordinates using gnomonic projection
    ///
    /// # Arguments
    ///
    /// * `spherical` - Spherical coordinates [theta, phi]
    ///
    /// # Returns
    ///
    /// Polar coordinates [rho, gamma]
    pub fn forward(&self, spherical: Spherical) -> Polar {
        let theta = spherical.theta();
        let phi = spherical.phi();
        Polar::new(phi.get().tan(), theta)
    }

    /// Unprojects polar coordinates to spherical coordinates using gnomonic projection
    ///
    /// # Arguments
    ///
    /// * `polar` - Polar coordinates [rho, gamma]
    ///
    /// # Returns
    ///
    /// Spherical coordinates [theta, phi]
    pub fn inverse(&self, polar: Polar) -> Spherical {
        let rho = polar.rho();
        let gamma = polar.gamma();
        Spherical::new(gamma, Radians::new_unchecked(rho.atan()))
    }
}
