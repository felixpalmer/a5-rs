// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use a5::traversal::global_neighbors::get_global_cell_neighbors;
use a5::{hex_to_u64, u64_to_hex};
use serde::Deserialize;

#[derive(Deserialize)]
struct Input {
    #[serde(rename = "cellId")]
    cell_id: String,
}

#[derive(Deserialize)]
struct Output {
    neighbors: Vec<String>,
    #[serde(rename = "edgeNeighbors")]
    edge_neighbors: Vec<String>,
}

#[derive(Deserialize)]
struct Fixture {
    input: Input,
    output: Output,
}

fn load_fixtures() -> Vec<Fixture> {
    let data = include_str!("fixtures/traversal/global-neighbors.json");
    serde_json::from_str(data).expect("Failed to parse global-neighbors.json")
}

#[test]
fn test_all_vertex_sharing_neighbors() {
    let fixtures = load_fixtures();
    for f in &fixtures {
        let cell_id = hex_to_u64(&f.input.cell_id).unwrap();
        let result: Vec<String> = get_global_cell_neighbors(cell_id, false)
            .iter()
            .map(|n| u64_to_hex(*n))
            .collect();
        assert_eq!(result, f.output.neighbors, "cellId={}", f.input.cell_id);
    }
}

#[test]
fn test_edge_only_neighbors() {
    let fixtures = load_fixtures();
    for f in &fixtures {
        let cell_id = hex_to_u64(&f.input.cell_id).unwrap();
        let result: Vec<String> = get_global_cell_neighbors(cell_id, true)
            .iter()
            .map(|n| u64_to_hex(*n))
            .collect();
        assert_eq!(
            result, f.output.edge_neighbors,
            "cellId={}",
            f.input.cell_id
        );
    }
}

#[test]
fn test_always_5_edge_neighbors() {
    let fixtures = load_fixtures();
    for f in &fixtures {
        assert_eq!(
            f.output.edge_neighbors.len(),
            5,
            "cellId={} should have 5 edge neighbors",
            f.input.cell_id
        );
    }
}
