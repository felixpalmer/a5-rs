// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use a5::lattice::Orientation;
use a5::traversal::quintant_neighbors::get_cell_neighbors;
use serde::Deserialize;

#[derive(Deserialize)]
struct Input {
    s: u64,
    resolution: usize,
    orientation: String,
}

#[derive(Deserialize)]
struct Output {
    neighbors: Vec<u64>,
}

#[derive(Deserialize)]
struct Fixture {
    input: Input,
    output: Output,
}

fn load_fixtures() -> Vec<Fixture> {
    let data = include_str!("fixtures/traversal/quintant-neighbors.json");
    serde_json::from_str(data).expect("Failed to parse quintant-neighbors.json")
}

#[test]
fn test_get_cell_neighbors() {
    let fixtures = load_fixtures();
    for f in &fixtures {
        let orientation = f.input.orientation.parse::<Orientation>().unwrap();
        let result = get_cell_neighbors(f.input.s, f.input.resolution, orientation, false);
        assert_eq!(
            result, f.output.neighbors,
            "s={} res={} ori={}",
            f.input.s, f.input.resolution, f.input.orientation
        );
    }
}
