// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

/// Latitude/longitude.
#[derive(Clone, Copy, Default)]
pub struct LonLat {
    /// Longitude, in degrees.
    pub lng: f64,
    /// Latitude, in degrees.
    pub lat: f64,
}
