// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use a5::{hex_to_u64, line_string_to_cells, u64_to_hex, LonLat};
use serde::Deserialize;

#[derive(Deserialize)]
struct LineSegmentFixture {
    name: String,
    start: [f64; 2],
    end: [f64; 2],
    resolution: i32,
    cells: Vec<String>,
}

#[derive(Deserialize)]
struct Fixtures {
    #[serde(rename = "lineSegment")]
    line_segment: Vec<LineSegmentFixture>,
}

#[derive(Deserialize)]
struct LineStringToCellsFixture {
    waypoints: Vec<[f64; 2]>,
    resolution: i32,
    cells: Vec<String>,
}

#[derive(Deserialize)]
struct LineStringToCellsOutOfRangeFixture {
    waypoints: Vec<[f64; 2]>,
    resolution: i32,
}

#[derive(Deserialize)]
struct WorldCellFixtures {
    #[serde(rename = "lineStringToCells")]
    line_string_to_cells: Vec<LineStringToCellsFixture>,
    #[serde(rename = "lineStringToCellsOutOfRange")]
    line_string_to_cells_out_of_range: Vec<LineStringToCellsOutOfRangeFixture>,
}

fn to_waypoints(points: &[[f64; 2]]) -> Vec<LonLat> {
    points.iter().map(|p| LonLat::new(p[0], p[1])).collect()
}

fn load_fixtures() -> Fixtures {
    let data = include_str!("fixtures/traversal/line.json");
    serde_json::from_str(data).expect("Failed to parse line.json")
}

fn load_world_cell_fixtures() -> WorldCellFixtures {
    let data = include_str!("fixtures/world-cell.json");
    serde_json::from_str(data).expect("Failed to parse world-cell.json")
}

#[test]
fn test_line_segment_fixtures() {
    let fixtures = load_fixtures();
    for f in &fixtures.line_segment {
        let waypoints = vec![
            LonLat::new(f.start[0], f.start[1]),
            LonLat::new(f.end[0], f.end[1]),
        ];
        let result = line_string_to_cells(&waypoints, f.resolution).unwrap();
        let mut sorted = result.clone();
        sorted.sort();
        let result_hex: Vec<String> = sorted.iter().map(|c| u64_to_hex(*c)).collect();
        let expected_hex: Vec<String> = f
            .cells
            .iter()
            .map(|c| {
                let id = hex_to_u64(c).unwrap();
                u64_to_hex(id)
            })
            .collect();
        assert_eq!(result_hex, expected_hex, "name={}", f.name);
    }
}

#[test]
fn test_empty_waypoints_returns_empty() {
    let waypoints: Vec<LonLat> = Vec::new();
    let result = line_string_to_cells(&waypoints, 5).unwrap();
    assert!(result.is_empty());
}

#[test]
fn test_single_waypoint_returns_single_cell() {
    let waypoints = vec![LonLat::new(10.0, 50.0)];
    let result = line_string_to_cells(&waypoints, 5).unwrap();
    assert_eq!(result.len(), 1);
}

#[test]
fn test_dedup_at_segment_junctions() {
    let waypoints = vec![
        LonLat::new(0.0, 50.0),
        LonLat::new(10.0, 50.0),
        LonLat::new(10.0, 45.0),
    ];
    let result = line_string_to_cells(&waypoints, 3).unwrap();
    let mut unique = result.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(result.len(), unique.len());
}

// At resolution -1 a line with any waypoints is the world cell
#[test]
fn test_line_string_to_cells_world_cell() {
    for f in &load_world_cell_fixtures().line_string_to_cells {
        let result: Vec<String> = line_string_to_cells(&to_waypoints(&f.waypoints), f.resolution)
            .unwrap()
            .into_iter()
            .map(u64_to_hex)
            .collect();
        assert_eq!(
            result, f.cells,
            "waypoints={:?} resolution={}",
            f.waypoints, f.resolution
        );
    }
}

#[test]
fn test_line_string_to_cells_resolution_out_of_range() {
    for f in &load_world_cell_fixtures().line_string_to_cells_out_of_range {
        let err = line_string_to_cells(&to_waypoints(&f.waypoints), f.resolution).expect_err(
            &format!("waypoints={:?} resolution={}", f.waypoints, f.resolution),
        );
        assert!(err.contains("is out of range"), "{}", err);
    }
}
