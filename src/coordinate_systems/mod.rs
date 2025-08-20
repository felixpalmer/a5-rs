//! Coordinate systems used by A5 internally.

mod base;
pub use base::{Degrees, Radians};

mod polar;
pub use polar::Polar;

mod spherical;
pub use spherical::Spherical;

mod lonlat;
pub use lonlat::LonLat;

mod coords;
pub use coords::{Barycentric, Cartesian, Face, FaceTriangle, SphericalTriangle, IJ, KJ};

pub mod vec2;
pub mod vec3;
