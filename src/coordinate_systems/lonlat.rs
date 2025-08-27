// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

//! Geographic coordinate system using longitude and latitude.

use super::base::Degrees;

/// Geographic coordinates using longitude and latitude in degrees.
///
/// Longitude values are normalized to [-180, 180] range to handle
/// antimeridian-spanning coordinates. Latitude values are clamped
/// to [-90, 90] range.
#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct LonLat {
    /// Longitude, in degrees.
    pub longitude: Degrees,
    /// Latitude, in degrees.
    pub latitude: Degrees,
}

impl LonLat {
    /// Create new longitude/latitude coordinates with validation
    pub fn new(longitude: f64, latitude: f64) -> Self {
        Self {
            longitude: Degrees::new(longitude),
            latitude: Degrees::new(latitude),
        }
    }

    /// Create new longitude/latitude coordinates without validation
    pub const fn new_unchecked(longitude: Degrees, latitude: Degrees) -> Self {
        Self {
            longitude,
            latitude,
        }
    }

    /// Get longitude in degrees
    pub const fn longitude(&self) -> f64 {
        self.longitude.get()
    }

    /// Get latitude in degrees
    pub const fn latitude(&self) -> f64 {
        self.latitude.get()
    }

    /// Create LonLat from degrees with validation
    pub fn from_degrees(longitude: f64, latitude: f64) -> Self {
        Self::new(longitude, latitude)
    }

    /// Convert to tuple of (longitude, latitude) in degrees
    pub const fn to_degrees(&self) -> (f64, f64) {
        (self.longitude.get(), self.latitude.get())
    }
}

impl From<(f64, f64)> for LonLat {
    fn from((longitude, latitude): (f64, f64)) -> Self {
        Self::new(longitude, latitude)
    }
}

impl From<LonLat> for (f64, f64) {
    fn from(lonlat: LonLat) -> Self {
        lonlat.to_degrees()
    }
}
