use super::base::Radians;
use super::polar::Polar;

/// 3D spherical coordinate system centered on unit sphere/dodecahedron
#[derive(Copy, Clone, Default)]
pub struct Spherical {
    pub theta: Radians,
    pub phi: Radians,
}

impl Spherical {
    pub const fn new(theta: Radians, phi: Radians) -> Self {
        Self { theta, phi }
    }

    /// Unproject spherical coordinates to polar coordinates using gnomonic projection.
    ///
    /// # Arguments
    /// * `spherical` - Spherical coordinates (theta, phi) in Radians
    ///
    /// # Returns
    /// * Polar coordinates (rho, gamma)
    pub fn unproject_gnomonic(self) -> Polar {
        let thetha = self.theta;
        let phi = self.phi;
        Polar::new(phi.get().tan(), thetha)
    }
}
