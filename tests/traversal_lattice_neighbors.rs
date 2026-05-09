use a5::core::hex::{hex_to_u64, u64_to_hex};
use a5::traversal::lattice_neighbors::get_lattice_neighbors;
use serde::Deserialize;
use std::fs;

#[derive(Deserialize)]
struct Fixture {
    cell: String,
    resolution: i32,
    #[serde(rename = "edgeOnlyNeighbors")]
    edge_only_neighbors: Vec<String>,
    #[serde(rename = "supersetNeighbors")]
    superset_neighbors: Vec<String>,
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

        let mut edge: Vec<String> = get_lattice_neighbors(cell, true)
            .into_iter()
            .map(u64_to_hex)
            .collect();
        edge.sort();
        assert_eq!(
            edge, f.edge_only_neighbors,
            "edgeOnly mismatch for cell {} (res {})",
            f.cell, f.resolution
        );

        let mut superset: Vec<String> = get_lattice_neighbors(cell, false)
            .into_iter()
            .map(u64_to_hex)
            .collect();
        superset.sort();
        assert_eq!(
            superset, f.superset_neighbors,
            "superset mismatch for cell {} (res {})",
            f.cell, f.resolution
        );
    }
}
