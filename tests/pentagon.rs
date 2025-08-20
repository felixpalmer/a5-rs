// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use a5_rs::coordinate_systems::Face;
use a5_rs::geometry::{Pentagon, PentagonShape};
use serde_json::Value;

const TOLERANCE: f64 = 1e-6;

fn close_to(a: f64, b: f64, tolerance: f64) -> bool {
    (a - b).abs() < tolerance
}

fn load_fixtures() -> Vec<Value> {
    let fixtures_str = include_str!("geometry/fixtures/pentagon.json");
    serde_json::from_str(fixtures_str).expect("Failed to parse pentagon fixtures")
}

#[test]
fn test_contains_point() {
    let fixtures = load_fixtures();

    for fixture in fixtures.iter() {
        let vertices_array = fixture["vertices"].as_array().unwrap();
        let mut vertices: Pentagon = [Face::new(0.0, 0.0); 5];

        for (i, vertex) in vertices_array.iter().enumerate().take(5) {
            let vertex_array = vertex.as_array().unwrap();
            vertices[i] = Face::new(
                vertex_array[0].as_f64().unwrap(),
                vertex_array[1].as_f64().unwrap(),
            );
        }

        let pentagon = PentagonShape::new(vertices);

        let contains_point_tests = fixture["containsPointTests"].as_array().unwrap();
        for test in contains_point_tests {
            let point_array = test["point"].as_array().unwrap();
            let point = Face::new(
                point_array[0].as_f64().unwrap(),
                point_array[1].as_f64().unwrap(),
            );
            let expected_result = test["result"].as_f64().unwrap();

            let actual = pentagon.contains_point(point);
            assert!(
                close_to(actual, expected_result, TOLERANCE),
                "contains_point mismatch: expected {}, got {}",
                expected_result,
                actual
            );
        }
    }
}

#[test]
fn test_get_area() {
    let fixtures = load_fixtures();

    for fixture in fixtures.iter() {
        let vertices_array = fixture["vertices"].as_array().unwrap();
        let mut vertices: Pentagon = [Face::new(0.0, 0.0); 5];

        for (i, vertex) in vertices_array.iter().enumerate().take(5) {
            let vertex_array = vertex.as_array().unwrap();
            vertices[i] = Face::new(
                vertex_array[0].as_f64().unwrap(),
                vertex_array[1].as_f64().unwrap(),
            );
        }

        let pentagon = PentagonShape::new(vertices);
        let expected_area = fixture["area"].as_f64().unwrap();
        let actual_area = pentagon.get_area();

        assert!(
            close_to(actual_area, expected_area, TOLERANCE),
            "area mismatch: expected {}, got {}",
            expected_area,
            actual_area
        );
    }
}

#[test]
fn test_get_center() {
    let fixtures = load_fixtures();

    for fixture in fixtures.iter() {
        let vertices_array = fixture["vertices"].as_array().unwrap();
        let mut vertices: Pentagon = [Face::new(0.0, 0.0); 5];

        for (i, vertex) in vertices_array.iter().enumerate().take(5) {
            let vertex_array = vertex.as_array().unwrap();
            vertices[i] = Face::new(
                vertex_array[0].as_f64().unwrap(),
                vertex_array[1].as_f64().unwrap(),
            );
        }

        let pentagon = PentagonShape::new(vertices);
        let expected_center_array = fixture["center"].as_array().unwrap();
        let expected_center = Face::new(
            expected_center_array[0].as_f64().unwrap(),
            expected_center_array[1].as_f64().unwrap(),
        );
        let actual_center = pentagon.get_center();

        assert!(
            close_to(actual_center.x(), expected_center.x(), TOLERANCE),
            "center x mismatch: expected {}, got {}",
            expected_center.x(),
            actual_center.x()
        );
        assert!(
            close_to(actual_center.y(), expected_center.y(), TOLERANCE),
            "center y mismatch: expected {}, got {}",
            expected_center.y(),
            actual_center.y()
        );
    }
}

#[test]
fn test_scale_transformation() {
    let fixtures = load_fixtures();

    for fixture in fixtures.iter() {
        let vertices_array = fixture["vertices"].as_array().unwrap();
        let mut vertices: Pentagon = [Face::new(0.0, 0.0); 5];

        for (i, vertex) in vertices_array.iter().enumerate().take(5) {
            let vertex_array = vertex.as_array().unwrap();
            vertices[i] = Face::new(
                vertex_array[0].as_f64().unwrap(),
                vertex_array[1].as_f64().unwrap(),
            );
        }

        let pentagon = PentagonShape::new(vertices);
        let mut scaled = pentagon.clone();
        scaled.scale(2.0);
        let scaled_vertices = scaled.get_vertices();

        let expected_scale_array = fixture["transformTests"]["scale"].as_array().unwrap();
        for (i, expected) in expected_scale_array.iter().enumerate().take(5) {
            let expected_array = expected.as_array().unwrap();
            let expected_vertex = Face::new(
                expected_array[0].as_f64().unwrap(),
                expected_array[1].as_f64().unwrap(),
            );

            assert!(
                close_to(scaled_vertices[i].x(), expected_vertex.x(), TOLERANCE),
                "scale x[{}] mismatch: expected {}, got {}",
                i,
                expected_vertex.x(),
                scaled_vertices[i].x()
            );
            assert!(
                close_to(scaled_vertices[i].y(), expected_vertex.y(), TOLERANCE),
                "scale y[{}] mismatch: expected {}, got {}",
                i,
                expected_vertex.y(),
                scaled_vertices[i].y()
            );
        }
    }
}

#[test]
fn test_rotate180_transformation() {
    let fixtures = load_fixtures();

    for fixture in fixtures.iter() {
        let vertices_array = fixture["vertices"].as_array().unwrap();
        let mut vertices: Pentagon = [Face::new(0.0, 0.0); 5];

        for (i, vertex) in vertices_array.iter().enumerate().take(5) {
            let vertex_array = vertex.as_array().unwrap();
            vertices[i] = Face::new(
                vertex_array[0].as_f64().unwrap(),
                vertex_array[1].as_f64().unwrap(),
            );
        }

        let pentagon = PentagonShape::new(vertices);
        let mut rotated = pentagon.clone();
        rotated.rotate180();
        let rotated_vertices = rotated.get_vertices();

        let expected_rotate_array = fixture["transformTests"]["rotate180"].as_array().unwrap();
        for (i, expected) in expected_rotate_array.iter().enumerate().take(5) {
            let expected_array = expected.as_array().unwrap();
            let expected_vertex = Face::new(
                expected_array[0].as_f64().unwrap(),
                expected_array[1].as_f64().unwrap(),
            );

            assert!(
                close_to(rotated_vertices[i].x(), expected_vertex.x(), TOLERANCE),
                "rotate180 x[{}] mismatch: expected {}, got {}",
                i,
                expected_vertex.x(),
                rotated_vertices[i].x()
            );
            assert!(
                close_to(rotated_vertices[i].y(), expected_vertex.y(), TOLERANCE),
                "rotate180 y[{}] mismatch: expected {}, got {}",
                i,
                expected_vertex.y(),
                rotated_vertices[i].y()
            );
        }
    }
}

#[test]
fn test_reflect_y_transformation() {
    let fixtures = load_fixtures();

    for fixture in fixtures.iter() {
        let vertices_array = fixture["vertices"].as_array().unwrap();
        let mut vertices: Pentagon = [Face::new(0.0, 0.0); 5];

        for (i, vertex) in vertices_array.iter().enumerate().take(5) {
            let vertex_array = vertex.as_array().unwrap();
            vertices[i] = Face::new(
                vertex_array[0].as_f64().unwrap(),
                vertex_array[1].as_f64().unwrap(),
            );
        }

        let pentagon = PentagonShape::new(vertices);
        let mut reflected = pentagon.clone();
        reflected.reflect_y();
        let reflected_vertices = reflected.get_vertices();

        let expected_reflect_array = fixture["transformTests"]["reflectY"].as_array().unwrap();
        for (i, expected) in expected_reflect_array.iter().enumerate().take(5) {
            let expected_array = expected.as_array().unwrap();
            let expected_vertex = Face::new(
                expected_array[0].as_f64().unwrap(),
                expected_array[1].as_f64().unwrap(),
            );

            assert!(
                close_to(reflected_vertices[i].x(), expected_vertex.x(), TOLERANCE),
                "reflectY x[{}] mismatch: expected {}, got {}",
                i,
                expected_vertex.x(),
                reflected_vertices[i].x()
            );
            assert!(
                close_to(reflected_vertices[i].y(), expected_vertex.y(), TOLERANCE),
                "reflectY y[{}] mismatch: expected {}, got {}",
                i,
                expected_vertex.y(),
                reflected_vertices[i].y()
            );
        }
    }
}

#[test]
fn test_translate_transformation() {
    let fixtures = load_fixtures();

    for fixture in fixtures.iter() {
        let vertices_array = fixture["vertices"].as_array().unwrap();
        let mut vertices: Pentagon = [Face::new(0.0, 0.0); 5];

        for (i, vertex) in vertices_array.iter().enumerate().take(5) {
            let vertex_array = vertex.as_array().unwrap();
            vertices[i] = Face::new(
                vertex_array[0].as_f64().unwrap(),
                vertex_array[1].as_f64().unwrap(),
            );
        }

        let pentagon = PentagonShape::new(vertices);
        let mut translated = pentagon.clone();
        translated.translate(Face::new(1.0, 1.0));
        let translated_vertices = translated.get_vertices();

        let expected_translate_array = fixture["transformTests"]["translate"].as_array().unwrap();
        for (i, expected) in expected_translate_array.iter().enumerate().take(5) {
            let expected_array = expected.as_array().unwrap();
            let expected_vertex = Face::new(
                expected_array[0].as_f64().unwrap(),
                expected_array[1].as_f64().unwrap(),
            );

            assert!(
                close_to(translated_vertices[i].x(), expected_vertex.x(), TOLERANCE),
                "translate x[{}] mismatch: expected {}, got {}",
                i,
                expected_vertex.x(),
                translated_vertices[i].x()
            );
            assert!(
                close_to(translated_vertices[i].y(), expected_vertex.y(), TOLERANCE),
                "translate y[{}] mismatch: expected {}, got {}",
                i,
                expected_vertex.y(),
                translated_vertices[i].y()
            );
        }
    }
}

#[test]
fn test_split_edges() {
    let fixtures = load_fixtures();

    for fixture in fixtures.iter() {
        let vertices_array = fixture["vertices"].as_array().unwrap();
        let mut vertices: Pentagon = [Face::new(0.0, 0.0); 5];

        for (i, vertex) in vertices_array.iter().enumerate().take(5) {
            let vertex_array = vertex.as_array().unwrap();
            vertices[i] = Face::new(
                vertex_array[0].as_f64().unwrap(),
                vertex_array[1].as_f64().unwrap(),
            );
        }

        let pentagon = PentagonShape::new(vertices);

        // Test with 2 and 3 segments
        for n_segments in [2, 3] {
            let split = pentagon.split_edges(n_segments);
            let split_vertices = split.get_vertices_vec();
            let field_name = format!("segments{}", n_segments);
            let expected_vertices_array =
                fixture["splitEdgesTests"][&field_name].as_array().unwrap();

            assert_eq!(
                split_vertices.len(),
                expected_vertices_array.len(),
                "splitEdges({}) length mismatch: expected {}, got {}",
                n_segments,
                expected_vertices_array.len(),
                split_vertices.len()
            );

            for (i, expected) in expected_vertices_array.iter().enumerate() {
                let expected_array = expected.as_array().unwrap();
                let expected_vertex = Face::new(
                    expected_array[0].as_f64().unwrap(),
                    expected_array[1].as_f64().unwrap(),
                );

                assert!(
                    close_to(split_vertices[i].x(), expected_vertex.x(), TOLERANCE),
                    "splitEdges({}) x[{}] mismatch: expected {}, got {}",
                    n_segments,
                    i,
                    expected_vertex.x(),
                    split_vertices[i].x()
                );
                assert!(
                    close_to(split_vertices[i].y(), expected_vertex.y(), TOLERANCE),
                    "splitEdges({}) y[{}] mismatch: expected {}, got {}",
                    n_segments,
                    i,
                    expected_vertex.y(),
                    split_vertices[i].y()
                );
            }
        }
    }
}
