use super::base::Radians;
use super::polar::Polar;

/// 3D spherical coordinate system centered on unit sphere/dodecahedron
#[derive(Copy, Clone, Default)]
pub struct Spherical {
    pub theta: Radians,
    pub phi: Radians,
}

impl Spherical {
    /// Create new spherical coordinates
    pub const fn new(theta: Radians, phi: Radians) -> Self {
        Self { theta, phi }
    }

    /// Unproject spherical coordinates to polar
    /// coordinates using gnomonic projection.
    pub fn unproject_gnomonic(self) -> Polar {
        let thetha = self.theta;
        let phi = self.phi;
        Polar::new(phi.get().tan(), thetha)
    }
}
