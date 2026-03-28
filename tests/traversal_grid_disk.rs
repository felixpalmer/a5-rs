// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use a5::{get_resolution, grid_disk, grid_disk_vertex, hex_to_u64, u64_to_hex, uncompact};
use serde::Deserialize;

#[derive(Deserialize)]
struct Fixture {
    #[serde(rename = "cellId")]
    cell_id: String,
    k: usize,
    cells: Vec<String>,
    #[serde(rename = "extraVertexCells")]
    extra_vertex_cells: Vec<String>,
}

fn load_fixtures() -> Vec<Fixture> {
    let data = include_str!("fixtures/traversal/grid-disk.json");
    serde_json::from_str(data).expect("Failed to parse grid-disk.json")
}

/// Sort hex cell IDs by numeric value for stable comparison
fn sort_hex(mut v: Vec<String>) -> Vec<String> {
    v.sort_by(|a, b| {
        let a_padded = format!("{:0>20}", a);
        let b_padded = format!("{:0>20}", b);
        a_padded.cmp(&b_padded)
    });
    v
}

#[test]
fn test_grid_disk() {
    let fixtures = load_fixtures();
    for f in &fixtures {
        let cell_id = hex_to_u64(&f.cell_id).unwrap();
        let target_res = get_resolution(cell_id);
        let disk = grid_disk(cell_id, f.k).unwrap();
        let uncompacted = uncompact(&disk, target_res).unwrap();
        let result: Vec<String> = uncompacted.iter().map(|n| u64_to_hex(*n)).collect();
        assert_eq!(
            sort_hex(result),
            sort_hex(f.cells.clone()),
            "cellId={} k={}",
            f.cell_id,
            f.k
        );
    }
}

#[test]
fn test_grid_disk_vertex() {
    let fixtures = load_fixtures();
    for f in &fixtures {
        let cell_id = hex_to_u64(&f.cell_id).unwrap();
        let target_res = get_resolution(cell_id);
        let mut expected: Vec<String> = f
            .cells
            .iter()
            .chain(f.extra_vertex_cells.iter())
            .cloned()
            .collect();
        expected.sort_by(|a, b| {
            let a_padded = format!("{:0>20}", a);
            let b_padded = format!("{:0>20}", b);
            a_padded.cmp(&b_padded)
        });
        let disk = grid_disk_vertex(cell_id, f.k).unwrap();
        let uncompacted = uncompact(&disk, target_res).unwrap();
        let result: Vec<String> = uncompacted.iter().map(|n| u64_to_hex(*n)).collect();
        assert_eq!(
            sort_hex(result),
            sort_hex(expected),
            "cellId={} k={}",
            f.cell_id,
            f.k
        );
    }
}

#[test]
fn test_grid_disk_k0() {
    let fixtures = load_fixtures();
    let cell_id = hex_to_u64(&fixtures[0].cell_id).unwrap();
    let result: Vec<String> = grid_disk(cell_id, 0)
        .unwrap()
        .iter()
        .map(|n| u64_to_hex(*n))
        .collect();
    assert_eq!(result, vec![fixtures[0].cell_id.clone()]);
}
