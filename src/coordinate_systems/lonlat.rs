// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use super::base::Degrees;

/// Latitude/longitude.
#[derive(Clone, Copy, Default)]
pub struct LonLat {
    /// Longitude, in degrees.
    pub longitude: Degrees,
    /// Latitude, in degrees.
    pub latitude: Degrees,
}
