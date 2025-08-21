// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use crate::core::utils::Quat;

// The quaternions for a regular dodecahedron are computed from exact trigonometric values.
// 
// Dodecahedron face centers (origins) can be defined exactly using trigonometry:
// - The north and south poles are at z=1 and z=-1
// - Two rings at z = ±sqrt(0.2), with radius 2 * sqrt(0.2)
//
// The computation involves:
// - cos36° = (√5 + 1) / 4, cos72° = (√5 - 1) / 4
// - sin36° = √(10 - 2√5) / 4, sin72° = √(10 + 2√5) / 4
// - Half-angle rotations: sinAlpha = √((1 - √0.2) / 2), cosAlpha = √((1 + √0.2) / 2)

/// Array of 12 quaternions representing rotations for each face of a regular dodecahedron
/// 
/// The quaternions are arranged as follows:
/// - Index 0: North pole (identity quaternion)
/// - Indices 1-5: First ring around the north pole
/// - Indices 6-10: Second ring around the south pole  
/// - Index 11: South pole
/// 
/// Each quaternion represents the rotation needed to transform the north pole (0,0,1)
/// to the center of the corresponding dodecahedron face.
pub const QUATERNIONS: [Quat; 12] = [
    [0.0, 0.0, 0.0, 1.0], // 0: North pole (identity)
    
    // First ring (indices 1-5): z component = 0, w component = cosAlpha
    [0.0, 0.5257311121191336, 0.0, 0.8506508083520399], // 1
    [-0.5, 0.16245984811645314, 0.0, 0.8506508083520399], // 2
    [-0.30901699437494745, -0.42532540417602, 0.0, 0.8506508083520399], // 3
    [0.30901699437494745, -0.42532540417602, 0.0, 0.8506508083520399], // 4
    [0.5, 0.16245984811645314, 0.0, 0.8506508083520399], // 5
    
    // Second ring (indices 6-10): z component = 0, w component = sinAlpha
    [0.0, -0.8506508083520399, 0.0, 0.5257311121191336], // 6
    [0.8090169943749475, -0.2628655560595668, 0.0, 0.5257311121191336], // 7
    [0.5, 0.6881909602355868, 0.0, 0.5257311121191336], // 8
    [-0.5, 0.6881909602355868, 0.0, 0.5257311121191336], // 9
    [-0.8090169943749475, -0.2628655560595668, 0.0, 0.5257311121191336], // 10
    
    [0.0, -1.0, 0.0, 0.0], // 11: South pole
];

#[cfg(test)]
mod tests {
    use super::*;

    const COS_ALPHA: f64 = 0.8506508083520399;
    const SIN_ALPHA: f64 = 0.5257311121191336;

    #[test]
    fn test_quaternions_length() {
        assert_eq!(QUATERNIONS.len(), 12);
    }

    #[test]
    fn test_quaternions_normalized() {
        for (i, q) in QUATERNIONS.iter().enumerate() {
            let magnitude = (q[0] * q[0] + q[1] * q[1] + q[2] * q[2] + q[3] * q[3]).sqrt();
            assert!(
                (magnitude - 1.0).abs() < 1e-10,
                "Quaternion {} is not normalized: magnitude = {}",
                i,
                magnitude
            );
        }
    }

    #[test]
    fn test_north_pole_identity() {
        assert_eq!(QUATERNIONS[0], [0.0, 0.0, 0.0, 1.0]);
    }

    #[test]
    fn test_south_pole() {
        assert_eq!(QUATERNIONS[11], [0.0, -1.0, 0.0, 0.0]);
    }

    #[test]
    fn test_first_ring_structure() {
        for i in 1..=5 {
            let q = QUATERNIONS[i];
            // Third component should be 0 for first ring
            assert!((q[2] - 0.0).abs() < 1e-15);
            // Fourth component should be cosAlpha for first ring
            assert!((q[3] - COS_ALPHA).abs() < 1e-10);
        }
    }

    #[test]
    fn test_second_ring_structure() {
        for i in 6..=10 {
            let q = QUATERNIONS[i];
            // Third component should be 0 for second ring
            assert!((q[2] - 0.0).abs() < 1e-15);
            // Fourth component should be sinAlpha for second ring
            assert!((q[3] - SIN_ALPHA).abs() < 1e-10);
        }
    }
}