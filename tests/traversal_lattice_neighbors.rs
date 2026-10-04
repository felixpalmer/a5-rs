use a5::core::hex::{hex_to_u64, u64_to_hex};
use a5::traversal::lattice_neighbors::get_lattice_neighbors;
use serde::Deserialize;
use std::fs;

#[derive(Deserialize)]
struct Fixture {
    cell: String,
    resolution: i32,
    neighbors: Vec<String>,
}

#[derive(Deserialize)]
struct Fixtures {
    cases: Vec<Fixture>,
}

#[test]
fn test_get_lattice_neighbors_fixtures() {
    let content = fs::read_to_string("tests/fixtures/traversal/lattice-neighbors.json")
        .expect("Could not read lattice-neighbors.json");
    let fixtures: Fixtures =
        serde_json::from_str(&content).expect("Could not parse lattice-neighbors.json");

    for f in &fixtures.cases {
        let cell = hex_to_u64(&f.cell).expect("hex_to_u64");

        let mut neighbors: Vec<String> = get_lattice_neighbors(cell)
            .into_iter()
            .map(u64_to_hex)
            .collect();
        neighbors.sort();
        assert_eq!(
            neighbors, f.neighbors,
            "mismatch for cell {} (res {})",
            f.cell, f.resolution
        );
    }
}
