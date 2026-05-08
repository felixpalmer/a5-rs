use a5::coordinate_systems::LonLat;
use a5::core::compact::uncompact;
use a5::core::hex::u64_to_hex;
use a5::regions::polygon::polygon_to_cells;
use serde::Deserialize;
use std::collections::HashSet;
use std::fs;

#[derive(Deserialize)]
struct PolygonFixture {
    name: String,
    ring: Vec<[f64; 2]>,
    resolution: i32,
    cells: Vec<String>,
}

#[derive(Deserialize)]
struct CountryFixture {
    name: String,
    ring: Vec<[f64; 2]>,
    resolution: i32,
    #[serde(rename = "cellCount")]
    cell_count: usize,
}

#[derive(Deserialize)]
struct Fixtures {
    polygon: Vec<PolygonFixture>,
    #[serde(default)]
    country: Vec<CountryFixture>,
}

fn ring_to_lonlat(ring: &[[f64; 2]]) -> Vec<LonLat> {
    ring.iter().map(|r| LonLat::new(r[0], r[1])).collect()
}

#[test]
fn test_polygon_to_cells_fixtures() {
    let content = fs::read_to_string("tests/fixtures/regions/polygon.json")
        .expect("Could not read polygon.json");
    let fixtures: Fixtures = serde_json::from_str(&content).expect("Could not parse polygon.json");

    for f in &fixtures.polygon {
        let ring = ring_to_lonlat(&f.ring);
        let result = polygon_to_cells(&ring, f.resolution).expect("polygon_to_cells");
        let expanded = uncompact(&result, f.resolution).expect("uncompact");
        let mut sorted = expanded;
        sorted.sort();
        let result_hex: Vec<String> = sorted.into_iter().map(u64_to_hex).collect();
        assert_eq!(result_hex, f.cells, "{}: cells mismatch", f.name);
    }
}

#[test]
fn test_polygon_to_cells_empty_for_too_few_vertices() {
    assert_eq!(polygon_to_cells(&[], 5).unwrap().len(), 0);
    assert_eq!(
        polygon_to_cells(&[LonLat::new(0.0, 0.0), LonLat::new(1.0, 1.0)], 5)
            .unwrap()
            .len(),
        0
    );
}

#[test]
fn test_polygon_to_cells_country_fixtures() {
    let content = fs::read_to_string("tests/fixtures/regions/polygon.json")
        .expect("Could not read polygon.json");
    let fixtures: Fixtures = serde_json::from_str(&content).expect("Could not parse polygon.json");

    for f in &fixtures.country {
        let ring = ring_to_lonlat(&f.ring);
        let result = polygon_to_cells(&ring, f.resolution).expect("polygon_to_cells");
        let expanded = uncompact(&result, f.resolution).expect("uncompact");
        let unique: HashSet<u64> = expanded.into_iter().collect();
        assert_eq!(
            unique.len(),
            f.cell_count,
            "{}: cell count mismatch",
            f.name
        );
    }
}
