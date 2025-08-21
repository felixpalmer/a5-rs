// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use a5_rs::core::dodecahedron_quaternions::QUATERNIONS;
use approx::assert_abs_diff_eq;
use serde_json::Value;

const TOLERANCE: f64 = 1e-10;

fn load_fixture() -> Value {
    let fixture_data = include_str!("fixtures/dodecahedron-quaternions.json");
    serde_json::from_str(fixture_data).expect("Failed to parse dodecahedron-quaternions fixtures")
}

// Helper function to multiply quaternion with vector (quaternion rotation)
fn quat_rotate_vector(quat: &[f64; 4], vec: &[f64; 3]) -> [f64; 3] {
    let [qx, qy, qz, qw] = *quat;
    let [vx, vy, vz] = *vec;
    
    // Quaternion multiplication: q * v * q_conjugate
    // Where v is treated as quaternion [vx, vy, vz, 0]
    
    // First multiply q * v
    let ix = qw * vx + qy * vz - qz * vy;
    let iy = qw * vy + qz * vx - qx * vz;
    let iz = qw * vz + qx * vy - qy * vx;
    let iw = -qx * vx - qy * vy - qz * vz;
    
    // Then multiply result by q_conjugate = [-qx, -qy, -qz, qw]
    let rx = ix * qw + iw * (-qx) + iy * (-qz) - iz * (-qy);
    let ry = iy * qw + iw * (-qy) + iz * (-qx) - ix * (-qz);
    let rz = iz * qw + iw * (-qz) + ix * (-qy) - iy * (-qx);
    
    [rx, ry, rz]
}

fn quat_magnitude(quat: &[f64; 4]) -> f64 {
    (quat[0] * quat[0] + quat[1] * quat[1] + quat[2] * quat[2] + quat[3] * quat[3]).sqrt()
}

fn vector_magnitude(vec: &[f64; 3]) -> f64 {
    (vec[0] * vec[0] + vec[1] * vec[1] + vec[2] * vec[2]).sqrt()
}

fn vector_distance(a: &[f64; 3], b: &[f64; 3]) -> f64 {
    let dx = a[0] - b[0];
    let dy = a[1] - b[1];
    let dz = a[2] - b[2];
    (dx * dx + dy * dy + dz * dz).sqrt()
}

fn vector_dot(a: &[f64; 3], b: &[f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

#[test]
fn test_quaternion_array() {
    let fixture = load_fixture();
    
    // Has 12 quaternions for 12 dodecahedron faces
    assert_eq!(QUATERNIONS.len(), fixture["metadata"]["totalQuaternions"].as_u64().unwrap() as usize);
    
    // First quaternion is identity (north pole)
    assert_eq!(QUATERNIONS[0], [0.0, 0.0, 0.0, 1.0]);
    
    // Last quaternion represents south pole rotation
    assert_eq!(QUATERNIONS[11], [0.0, -1.0, 0.0, 0.0]);
}

#[test]
fn test_all_quaternions_normalized() {
    let fixture = load_fixture();
    
    for (i, quat) in QUATERNIONS.iter().enumerate() {
        let magnitude = quat_magnitude(quat);
        let expected_magnitude = fixture["quaternions"][i]["magnitude"].as_f64().unwrap();
        
        assert_abs_diff_eq!(magnitude, expected_magnitude, epsilon = TOLERANCE);
        assert_abs_diff_eq!(magnitude, 1.0, epsilon = TOLERANCE);
    }
}

#[test]
fn test_quaternions_are_valid() {
    for quat in &QUATERNIONS {
        assert_eq!(quat.len(), 4);
        for &component in quat {
            assert!(component.is_finite());
            assert!(!component.is_nan());
        }
    }
}

#[test]
fn test_first_ring_quaternions() {
    let cos_alpha = (1.0 + (0.2_f64).sqrt()) / 2.0;
    let cos_alpha = cos_alpha.sqrt();
    
    for i in 1..=5 {
        let q = QUATERNIONS[i];
        // Third component should be 0 for first ring
        assert_abs_diff_eq!(q[2], 0.0, epsilon = 1e-15);
        // Fourth component should be cosAlpha for first ring
        assert_abs_diff_eq!(q[3], cos_alpha, epsilon = TOLERANCE);
    }
}

#[test]
fn test_second_ring_quaternions() {
    let sin_alpha = (1.0 - (0.2_f64).sqrt()) / 2.0;
    let sin_alpha = sin_alpha.sqrt();
    
    for i in 6..=10 {
        let q = QUATERNIONS[i];
        // Third component should be 0 for second ring
        assert_abs_diff_eq!(q[2], 0.0, epsilon = 1e-15);
        // Fourth component should be sinAlpha for second ring
        assert_abs_diff_eq!(q[3], sin_alpha, epsilon = TOLERANCE);
    }
}

#[test]
fn test_quaternions_represent_rotations_around_axes() {
    let north_pole = [0.0, 0.0, 1.0];
    
    for (i, quat) in QUATERNIONS.iter().enumerate() {
        let rotated = quat_rotate_vector(quat, &north_pole);
        
        // Should be on unit sphere
        let magnitude = vector_magnitude(&rotated);
        assert_abs_diff_eq!(magnitude, 1.0, epsilon = TOLERANCE);
        
        // For non-identity quaternions, should be different from north pole
        if i != 0 {
            let distance = vector_distance(&rotated, &north_pole);
            assert!(distance > 0.1);
        }
    }
}

#[test]
fn test_quaternions_produce_distinct_face_centers() {
    let north_pole = [0.0, 0.0, 1.0];
    let mut face_centers = Vec::new();
    
    for quat in &QUATERNIONS {
        let rotated = quat_rotate_vector(quat, &north_pole);
        face_centers.push(rotated);
    }
    
    // All face centers should be distinct
    for i in 0..face_centers.len() {
        for j in (i + 1)..face_centers.len() {
            let distance = vector_distance(&face_centers[i], &face_centers[j]);
            assert!(distance > 0.1, "Face centers {} and {} are too close: {}", i, j, distance);
        }
    }
}

#[test]
fn test_quaternion_conjugates_reverse_rotations() {
    let test_vector = [1.0, 0.0, 0.0];
    
    for quat in &QUATERNIONS {
        // Quaternion conjugate
        let conjugate = [-quat[0], -quat[1], -quat[2], quat[3]];
        
        let rotated = quat_rotate_vector(quat, &test_vector);
        let back_rotated = quat_rotate_vector(&conjugate, &rotated);
        
        let distance = vector_distance(&test_vector, &back_rotated);
        assert_abs_diff_eq!(distance, 0.0, epsilon = TOLERANCE);
    }
}

#[test]
fn test_quaternions_maintain_orthogonality() {
    let v1 = [1.0, 0.0, 0.0];
    let v2 = [0.0, 1.0, 0.0];
    
    for quat in &QUATERNIONS {
        let rotated1 = quat_rotate_vector(quat, &v1);
        let rotated2 = quat_rotate_vector(quat, &v2);
        
        let dot_product = vector_dot(&rotated1, &rotated2);
        assert_abs_diff_eq!(dot_product, 0.0, epsilon = TOLERANCE);
    }
}

#[test]
fn test_quaternions_preserve_vector_magnitudes() {
    let test_vectors = [
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 0.0, 1.0],
        [1.0, 1.0, 1.0],
    ];
    
    for quat in &QUATERNIONS {
        for test_vector in &test_vectors {
            let original_length = vector_magnitude(test_vector);
            let rotated = quat_rotate_vector(quat, test_vector);
            let new_length = vector_magnitude(&rotated);
            
            assert_abs_diff_eq!(new_length, original_length, epsilon = TOLERANCE);
        }
    }
}

#[test]
fn test_face_centers_have_correct_distribution() {
    let north_pole = [0.0, 0.0, 1.0];
    let mut face_centers = Vec::new();
    
    for quat in &QUATERNIONS {
        let rotated = quat_rotate_vector(quat, &north_pole);
        face_centers.push(rotated);
    }
    
    // Check that we have correct z-distribution:
    // 1 at z=1 (north), 1 at z=-1 (south), 5 at each intermediate level
    let mut z_values: Vec<f64> = face_centers.iter().map(|fc| fc[2]).collect();
    z_values.sort_by(|a, b| b.partial_cmp(a).unwrap());
    
    assert_abs_diff_eq!(z_values[0], 1.0, epsilon = TOLERANCE); // North pole
    assert_abs_diff_eq!(z_values[11], -1.0, epsilon = TOLERANCE); // South pole
    
    // Two rings of 5 each at intermediate z values
    let inv_sqrt5 = (0.2_f64).sqrt();
    let first_ring_z = &z_values[1..6];
    let second_ring_z = &z_values[6..11];
    
    for &z in first_ring_z {
        assert_abs_diff_eq!(z, inv_sqrt5, epsilon = 1e-5);
    }
    for &z in second_ring_z {
        assert_abs_diff_eq!(z, -inv_sqrt5, epsilon = 1e-5);
    }
}

#[test]
fn test_face_centers_form_regular_pentagonal_arrangements() {
    let north_pole = [0.0, 0.0, 1.0];
    let mut face_centers = Vec::new();
    
    for quat in &QUATERNIONS {
        let rotated = quat_rotate_vector(quat, &north_pole);
        face_centers.push(rotated);
    }
    
    // Check angular distribution for first ring (indices 1-5)
    let first_ring = &face_centers[1..6];
    for i in 0..5 {
        let next = (i + 1) % 5;
        let angle1 = first_ring[i][1].atan2(first_ring[i][0]);
        let angle2 = first_ring[next][1].atan2(first_ring[next][0]);
        let mut angle_diff = angle2 - angle1;
        if angle_diff < 0.0 {
            angle_diff += 2.0 * std::f64::consts::PI;
        }
        if angle_diff > std::f64::consts::PI {
            angle_diff = 2.0 * std::f64::consts::PI - angle_diff;
        }
        
        // Should be approximately 2π/5 = 72 degrees
        assert_abs_diff_eq!(angle_diff, 2.0 * std::f64::consts::PI / 5.0, epsilon = 0.1);
    }
}

#[test]
fn test_fixture_metadata() {
    let fixture = load_fixture();
    
    assert_eq!(fixture["metadata"]["totalQuaternions"].as_u64().unwrap(), 12);
    assert_abs_diff_eq!(
        fixture["constants"]["INV_SQRT5"].as_f64().unwrap(),
        (0.2_f64).sqrt(),
        epsilon = 1e-15
    );
    assert_abs_diff_eq!(
        fixture["constants"]["expectedPentagonAngle"].as_f64().unwrap(),
        2.0 * std::f64::consts::PI / 5.0,
        epsilon = 1e-15
    );
}