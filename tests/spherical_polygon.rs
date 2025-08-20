// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use a5_rs::coordinate_systems::Cartesian;
use a5_rs::geometry::SphericalPolygonShape;
use approx::assert_abs_diff_eq;
use serde_json::Value;

const TOLERANCE: f64 = 1e-6;

fn close_to_array(actual: &[f64], expected: &[f64], tolerance: f64) -> bool {
    if actual.len() != expected.len() {
        return false;
    }
    actual
        .iter()
        .zip(expected.iter())
        .all(|(a, e)| (a - e).abs() < tolerance)
}

fn load_fixtures() -> Vec<Value> {
    let fixture_data = include_str!("fixtures/spherical-polygon.json");
    serde_json::from_str(fixture_data).expect("Failed to parse spherical-polygon fixtures")
}

#[test]
fn test_get_boundary() {
    let fixtures = load_fixtures();
    for (i, fixture) in fixtures.iter().enumerate() {
        let vertices: Vec<Cartesian> = fixture["vertices"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| {
                let coords = v.as_array().unwrap();
                Cartesian::new(
                    coords[0].as_f64().unwrap(),
                    coords[1].as_f64().unwrap(),
                    coords[2].as_f64().unwrap(),
                )
            })
            .collect();

        let polygon = SphericalPolygonShape::new(vertices);

        // Test boundaries with 1-3 segments
        for n_segments in 1..=3 {
            let boundary = polygon.get_boundary(n_segments, true);
            let expected_boundary = &fixture[&format!("boundary{}", n_segments)];
            let expected: Vec<Vec<f64>> = expected_boundary
                .as_array()
                .unwrap()
                .iter()
                .map(|v| {
                    v.as_array()
                        .unwrap()
                        .iter()
                        .map(|x| x.as_f64().unwrap())
                        .collect()
                })
                .collect();

            assert_eq!(
                boundary.len(),
                expected.len(),
                "Fixture {}, segments {}: boundary length mismatch",
                i,
                n_segments
            );

            for (j, (point, expected_point)) in boundary.iter().zip(expected.iter()).enumerate() {
                let actual = [point.x(), point.y(), point.z()];
                assert!(
                    close_to_array(&actual, expected_point, TOLERANCE),
                    "Fixture {}, segments {}, point {}: expected {:?}, got {:?}",
                    i,
                    n_segments,
                    j,
                    expected_point,
                    actual
                );
            }
        }
    }
}

#[test]
fn test_slerp() {
    let fixtures = load_fixtures();
    for (i, fixture) in fixtures.iter().enumerate() {
        let vertices: Vec<Cartesian> = fixture["vertices"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| {
                let coords = v.as_array().unwrap();
                Cartesian::new(
                    coords[0].as_f64().unwrap(),
                    coords[1].as_f64().unwrap(),
                    coords[2].as_f64().unwrap(),
                )
            })
            .collect();

        let polygon = SphericalPolygonShape::new(vertices);

        for test in fixture["slerpTests"].as_array().unwrap() {
            let t = test["t"].as_f64().unwrap();
            let expected_result = test["result"].as_array().unwrap();
            let expected = [
                expected_result[0].as_f64().unwrap(),
                expected_result[1].as_f64().unwrap(),
                expected_result[2].as_f64().unwrap(),
            ];

            let actual = polygon.slerp(t);
            let actual_array = [actual.x(), actual.y(), actual.z()];

            assert!(
                close_to_array(&actual_array, &expected, TOLERANCE),
                "Fixture {}, t={}: expected {:?}, got {:?}",
                i,
                t,
                expected,
                actual_array
            );

            // Should be normalized
            let length =
                (actual.x() * actual.x() + actual.y() * actual.y() + actual.z() * actual.z())
                    .sqrt();
            assert_abs_diff_eq!(length, 1.0, epsilon = 1e-10);
        }
    }
}

#[test]
fn test_contains_point() {
    let fixtures = load_fixtures();
    for (i, fixture) in fixtures.iter().enumerate() {
        let vertices: Vec<Cartesian> = fixture["vertices"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| {
                let coords = v.as_array().unwrap();
                Cartesian::new(
                    coords[0].as_f64().unwrap(),
                    coords[1].as_f64().unwrap(),
                    coords[2].as_f64().unwrap(),
                )
            })
            .collect();

        let polygon = SphericalPolygonShape::new(vertices);

        for test in fixture["containsPointTests"].as_array().unwrap() {
            let point_coords = test["point"].as_array().unwrap();
            let point = Cartesian::new(
                point_coords[0].as_f64().unwrap(),
                point_coords[1].as_f64().unwrap(),
                point_coords[2].as_f64().unwrap(),
            );
            let expected_result = test["result"].as_f64().unwrap();

            let actual = polygon.contains_point(point);
            assert!(
                (actual - expected_result).abs() < TOLERANCE,
                "Fixture {}, point {:?}: expected {}, got {}",
                i,
                [point.x(), point.y(), point.z()],
                expected_result,
                actual
            );
        }
    }
}

#[test]
fn test_get_area() {
    let fixtures = load_fixtures();
    for (i, fixture) in fixtures.iter().enumerate() {
        let vertices: Vec<Cartesian> = fixture["vertices"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| {
                let coords = v.as_array().unwrap();
                Cartesian::new(
                    coords[0].as_f64().unwrap(),
                    coords[1].as_f64().unwrap(),
                    coords[2].as_f64().unwrap(),
                )
            })
            .collect();

        let mut polygon = SphericalPolygonShape::new(vertices);
        let area = polygon.get_area();
        let expected_area = fixture["area"].as_f64().unwrap();

        assert!(
            (area.get() - expected_area).abs() < TOLERANCE,
            "Fixture {}: expected area {}, got {}",
            i,
            expected_area,
            area.get()
        );

        // Area can be negative for some winding orders, so check absolute value
        assert!(
            area.get().abs() > 0.0,
            "Fixture {}: area should be non-zero",
            i
        );
        assert!(
            area.get().abs() <= 2.0 * std::f64::consts::PI,
            "Fixture {}: area should not exceed 2π",
            i
        );
    }
}

#[test]
fn test_get_area_degenerate_polygons() {
    // Test empty polygon
    let mut empty_polygon = SphericalPolygonShape::new(vec![]);
    assert_eq!(empty_polygon.get_area().get(), 0.0);

    // Test single vertex
    let mut single_vertex = SphericalPolygonShape::new(vec![Cartesian::new(1.0, 0.0, 0.0)]);
    assert_eq!(single_vertex.get_area().get(), 0.0);

    // Test two vertices
    let mut two_vertices = SphericalPolygonShape::new(vec![
        Cartesian::new(1.0, 0.0, 0.0),
        Cartesian::new(0.0, 1.0, 0.0),
    ]);
    assert_eq!(two_vertices.get_area().get(), 0.0);
}
