//! Coordinate systems used by A5 internally.

mod base;
pub use base::{Degrees, Radians};

mod polar;
pub use polar::Polar;

mod spherical;
pub use spherical::Spherical;

mod lonlat;
pub use lonlat::LonLat;
pub mod vec2;
pub mod vec3;
