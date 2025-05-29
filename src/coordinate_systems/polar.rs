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
    pub const fn new(rho: f64, gamma: Radians) -> Self {
        Self { rho, gamma }
    }

    /// Project polar coordinates to spherical coordinates
    /// using gnomonic projection.
    ///
    /// # Arguments
    /// * `polar` - Polar coordinates (rho, gamma), where:
    ///     - rho: radial distance from face center
    ///     - gamma: azimuthal angle (Radians)
    ///
    /// # Returns
    /// * Spherical coordinates (theta, phi) in Radians
    pub fn project_gnomonic(&self) -> Spherical {
        let gamma = self.gamma;
        let rho = self.rho;
        Spherical::new(gamma, Radians::new_unchecked(rho.atan()))
    }
}
