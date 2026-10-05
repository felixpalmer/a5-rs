use a5::core::hex::{hex_to_u64, u64_to_hex};
use a5::traversal::lattice_flood_fill::{triple_space_flood_fill, FloodInput};
use a5::traversal::triple_cells::cell_ids_to_triples;
use serde::Deserialize;
use std::fs;

#[derive(Deserialize)]
struct Fixture {
    name: String,
    resolution: i32,
    #[serde(rename = "seedCells")]
    seed_cells: Vec<String>,
    #[serde(rename = "firewallCells")]
    firewall_cells: Vec<String>,
    #[serde(rename = "maxLayers", default)]
    max_layers: Option<usize>,
    #[serde(rename = "interiorCells")]
    interior_cells: Vec<String>,
    #[serde(rename = "frontierCells")]
    frontier_cells: Vec<String>,
}

#[derive(Deserialize)]
struct Fixtures {
    cases: Vec<Fixture>,
}

#[test]
fn test_triple_space_flood_fill_fixtures() {
    let content = fs::read_to_string("tests/fixtures/traversal/lattice-flood-fill.json")
        .expect("Could not read lattice-flood-fill.json");
    let fixtures: Fixtures =
        serde_json::from_str(&content).expect("Could not parse lattice-flood-fill.json");

    for f in &fixtures.cases {
        let to_triples = |hexes: &[String]| {
            let mut cells: Vec<[i32; 5]> = Vec::new();
            cell_ids_to_triples(
                hexes.iter().map(|h| hex_to_u64(h).expect("hex_to_u64")),
                &mut cells,
            )
            .expect("cell_ids_to_triples");
            cells
        };
        let seeds = to_triples(&f.seed_cells);
        let firewall = to_triples(&f.firewall_cells);

        let result = triple_space_flood_fill(
            FloodInput::Firewall(&firewall),
            &seeds,
            f.resolution,
            f.max_layers,
        )
        .expect("flood fill");

        let mut interior: Vec<String> = result
            .interior_cells
            .iter()
            .copied()
            .map(u64_to_hex)
            .collect();
        interior.sort();
        let mut frontier: Vec<String> = result
            .frontier_cell_ids
            .iter()
            .copied()
            .map(u64_to_hex)
            .collect();
        frontier.sort();

        assert_eq!(interior, f.interior_cells, "{}: interior mismatch", f.name);
        assert_eq!(frontier, f.frontier_cells, "{}: frontier mismatch", f.name);
    }
}
