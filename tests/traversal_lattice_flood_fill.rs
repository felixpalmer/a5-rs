use a5::core::hex::{hex_to_u64, u64_to_hex};
use a5::core::serialization::FIRST_HILBERT_RESOLUTION;
use a5::traversal::lattice_flood_fill::{triple_space_flood_fill, FloodInput};
use a5::traversal::triple_cells::{cell_ids_to_triples, triple_cells_to_ids};
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
        );

        let hilbert_res = (f.resolution - FIRST_HILBERT_RESOLUTION + 1) as usize;
        let to_hex = |cells: &[[i32; 5]]| {
            let mut ids: Vec<u64> = Vec::new();
            triple_cells_to_ids(cells, hilbert_res, f.resolution, &mut ids).expect("encode");
            let mut hex: Vec<String> = ids.into_iter().map(u64_to_hex).collect();
            hex.sort();
            hex
        };
        let interior = to_hex(&result.interior);
        let frontier = to_hex(&result.frontier);

        assert_eq!(interior, f.interior_cells, "{}: interior mismatch", f.name);
        assert_eq!(frontier, f.frontier_cells, "{}: frontier mismatch", f.name);
    }
}
