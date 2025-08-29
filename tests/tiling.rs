// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use a5::coordinate_systems::{Polar, Radians, IJ};
use a5::core::hilbert::{Anchor, Flip};
use a5::core::tiling::{
    get_face_vertices, get_pentagon_vertices, get_quintant_polar, get_quintant_vertices,
};
use serde::Deserialize;
use std::fs;

#[derive(Deserialize)]
struct TestFixtures {
    #[serde(rename = "getPentagonVertices")]
    get_pentagon_vertices: Vec<PentagonVerticesTestCase>,
    #[serde(rename = "getQuintantVertices")]
    get_quintant_vertices: Vec<QuintantVerticesTestCase>,
    #[serde(rename = "getFaceVertices")]
    get_face_vertices: FaceVerticesTestCase,
    #[serde(rename = "getQuintantPolar")]
    get_quintant_polar: Vec<QuintantPolarTestCase>,
}

#[derive(Deserialize)]
struct PentagonVerticesTestCase {
    input: PentagonVerticesInput,
    output: GeometryOutput,
}

#[derive(Deserialize)]
struct PentagonVerticesInput {
    resolution: i32,
    quintant: usize,
    anchor: AnchorData,
}

#[derive(Deserialize)]
struct AnchorData {
    offset: [f64; 2],
    flips: [i8; 2],
    k: u8,
}

#[derive(Deserialize)]
struct GeometryOutput {
    vertices: Vec<[f64; 2]>,
    area: f64,
    center: [f64; 2],
}

#[derive(Deserialize)]
struct QuintantVerticesTestCase {
    input: QuintantVerticesInput,
    output: GeometryOutput,
}

#[derive(Deserialize)]
struct QuintantVerticesInput {
    quintant: usize,
}

#[derive(Deserialize)]
struct FaceVerticesTestCase {
    vertices: Vec<[f64; 2]>,
    area: f64,
    center: [f64; 2],
}

#[derive(Deserialize)]
struct QuintantPolarTestCase {
    input: QuintantPolarInput,
    output: QuintantPolarOutput,
}

#[derive(Deserialize)]
struct QuintantPolarInput {
    polar: [f64; 2],
}

#[derive(Deserialize)]
struct QuintantPolarOutput {
    quintant: usize,
}

fn load_test_fixtures() -> TestFixtures {
    let content = fs::read_to_string("tests/fixtures/tiling.json")
        .expect("Could not read tiling test fixtures");
    serde_json::from_str(&content).expect("Could not parse tiling test fixtures")
}

fn assert_close(a: f64, b: f64, tolerance: f64) {
    assert!(
        (a - b).abs() < tolerance,
        "Expected {}, got {} (difference: {})",
        a,
        b,
        (a - b).abs()
    );
}

#[test]
fn test_get_pentagon_vertices() {
    let fixtures = load_test_fixtures();

    for (index, test_case) in fixtures.get_pentagon_vertices.iter().enumerate() {
        let input = &test_case.input;
        let output = &test_case.output;

        let anchor = Anchor {
            k: input.anchor.k,
            offset: IJ::new(input.anchor.offset[0], input.anchor.offset[1]),
            flips: [input.anchor.flips[0] as Flip, input.anchor.flips[1] as Flip],
        };

        let shape = get_pentagon_vertices(input.resolution, input.quintant, &anchor);
        let vertices = shape.get_vertices();
        let area = shape.get_area();
        let center = shape.get_center();

        // Check vertices match
        assert_eq!(
            vertices.len(),
            output.vertices.len(),
            "Test case {}: vertex count mismatch",
            index
        );

        for (vertex, expected) in vertices.iter().zip(output.vertices.iter()) {
            assert_close(vertex.x(), expected[0], 1e-15);
            assert_close(vertex.y(), expected[1], 1e-15);
        }

        // Check area matches
        assert_close(area, output.area, 1e-15);

        // Check center matches
        assert_close(center.x(), output.center[0], 1e-15);
        assert_close(center.y(), output.center[1], 1e-15);
    }
}

#[test]
fn test_get_quintant_vertices() {
    let fixtures = load_test_fixtures();

    for test_case in &fixtures.get_quintant_vertices {
        let input = &test_case.input;
        let output = &test_case.output;

        let shape = get_quintant_vertices(input.quintant);
        let vertices = shape.get_vertices_vec();
        let area = shape.get_area();
        let center = shape.get_center();

        // Check vertices match
        assert_eq!(vertices.len(), output.vertices.len());

        for (vertex, expected) in vertices.iter().zip(output.vertices.iter()) {
            assert_close(vertex.x(), expected[0], 1e-15);
            assert_close(vertex.y(), expected[1], 1e-15);
        }

        // Check area matches
        assert_close(area, output.area, 1e-15);

        // Check center matches
        assert_close(center.x(), output.center[0], 1e-15);
        assert_close(center.y(), output.center[1], 1e-15);
    }
}

#[test]
fn test_get_face_vertices() {
    let fixtures = load_test_fixtures();
    let expected = &fixtures.get_face_vertices;

    let shape = get_face_vertices();
    let vertices = shape.get_vertices();
    let area = shape.get_area();
    let center = shape.get_center();

    // Check vertices match
    assert_eq!(vertices.len(), expected.vertices.len());

    for (vertex, expected_vertex) in vertices.iter().zip(expected.vertices.iter()) {
        assert_close(vertex.x(), expected_vertex[0], 1e-15);
        assert_close(vertex.y(), expected_vertex[1], 1e-15);
    }

    // Check area matches
    assert_close(area, expected.area, 1e-15);

    // Check center matches
    assert_close(center.x(), expected.center[0], 1e-15);
    assert_close(center.y(), expected.center[1], 1e-15);
}

#[test]
fn test_get_quintant_polar() {
    let fixtures = load_test_fixtures();

    for test_case in &fixtures.get_quintant_polar {
        let input = &test_case.input;
        let output = &test_case.output;

        let polar = Polar::new(input.polar[0], Radians::new(input.polar[1]));
        let result = get_quintant_polar(polar);

        assert_eq!(result, output.quintant);
    }
}
