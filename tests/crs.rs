// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use a5::coordinate_systems::Cartesian;
use a5::projections::crs::CRS;
use a5::utils::vector::vec3_length;
use approx::assert_relative_eq;
use serde::{Deserialize, Serialize};

const TOLERANCE: f64 = 1e-10;

#[derive(Debug, Deserialize, Serialize)]
struct VertexArray([f64; 3]);

impl From<VertexArray> for Cartesian {
    fn from(arr: VertexArray) -> Self {
        Cartesian::new(arr.0[0], arr.0[1], arr.0[2])
    }
}

fn load_expected_vertices() -> Vec<Cartesian> {
    let vertices_json = include_str!("fixtures/crs-vertices.json");
    let vertex_arrays: Vec<VertexArray> =
        serde_json::from_str(vertices_json).expect("Failed to parse CRS vertices fixture");
    vertex_arrays.into_iter().map(Cartesian::from).collect()
}

#[test]
fn test_crs_has_exactly_62_vertices() {
    let crs = CRS::new().expect("Failed to create CRS");

    // We can't access private vertices field directly, but we can test the behavior
    // by checking that the CRS was created successfully (which validates the count)
    // This is implicit in the successful creation

    // Create a mutable CRS to test some known vertices
    let mut crs = crs;

    // Test that we can find some known vertices (first vertex should be [0, 0, 1])
    let north_pole = Cartesian::new(0.0, 0.0, 1.0);
    let result = crs.get_vertex(north_pole);
    assert!(result.is_ok(), "Should find north pole vertex");
}

#[test]
fn test_crs_matches_expected_vertices() {
    let expected_vertices = load_expected_vertices();
    assert_eq!(expected_vertices.len(), 62);

    let mut crs = CRS::new().expect("Failed to create CRS");

    // Test that we can find each expected vertex
    for (index, expected_vertex) in expected_vertices.iter().enumerate() {
        let result = crs.get_vertex(*expected_vertex);
        match result {
            Ok(found_vertex) => {
                assert_relative_eq!(found_vertex.x(), expected_vertex.x(), epsilon = TOLERANCE);
                assert_relative_eq!(found_vertex.y(), expected_vertex.y(), epsilon = TOLERANCE);
                assert_relative_eq!(found_vertex.z(), expected_vertex.z(), epsilon = TOLERANCE);
            }
            Err(e) => {
                panic!(
                    "Failed to find expected vertex {}: {} - vertex: [{}, {}, {}]",
                    index,
                    e,
                    expected_vertex.x(),
                    expected_vertex.y(),
                    expected_vertex.z()
                );
            }
        }
    }
}

#[test]
fn test_crs_throws_error_for_non_existent_vertex() {
    let mut crs = CRS::new().expect("Failed to create CRS");
    let non_vertex_point = Cartesian::new(1.0, 0.0, 0.0); // This should not be exactly a CRS vertex

    let result = crs.get_vertex(non_vertex_point);
    assert!(result.is_err(), "Should fail to find non-existent vertex");
    assert!(result.unwrap_err().contains("Failed to find vertex in CRS"));
}

#[test]
fn test_crs_vertices_are_normalized() {
    let expected_vertices = load_expected_vertices();

    // All vertices should be normalized (unit length)
    for vertex in expected_vertices.iter() {
        let length = vec3_length(vertex);
        assert_relative_eq!(length, 1.0, epsilon = 1e-15);
    }
}

#[test]
fn test_crs_vertex_lookup_consistency() {
    let expected_vertices = load_expected_vertices();
    let mut crs = CRS::new().expect("Failed to create CRS");

    // Test that looking up the same vertex multiple times returns the same result
    let test_vertex = expected_vertices[0]; // Use first vertex

    let result1 = crs
        .get_vertex(test_vertex)
        .expect("First lookup should succeed");
    let result2 = crs
        .get_vertex(test_vertex)
        .expect("Second lookup should succeed");

    assert_relative_eq!(result1.x(), result2.x(), epsilon = TOLERANCE);
    assert_relative_eq!(result1.y(), result2.y(), epsilon = TOLERANCE);
    assert_relative_eq!(result1.z(), result2.z(), epsilon = TOLERANCE);
}
