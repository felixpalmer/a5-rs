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

fn load_fixtures() -> Fixtures {
    let data = include_str!("fixtures/traversal/line.json");
    serde_json::from_str(data).expect("Failed to parse line.json")
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
