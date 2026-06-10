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
    polygon: Vec<Vec<[f64; 2]>>,
    resolution: i32,
    cells: Vec<String>,
}

#[derive(Deserialize)]
struct CountryFixture {
    name: String,
    polygon: Vec<Vec<[f64; 2]>>,
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

fn to_rings(polygon: &[Vec<[f64; 2]>]) -> Vec<Vec<LonLat>> {
    polygon
        .iter()
        .map(|ring| ring.iter().map(|r| LonLat::new(r[0], r[1])).collect())
        .collect()
}

#[test]
fn test_polygon_to_cells_fixtures() {
    let content = fs::read_to_string("tests/fixtures/regions/polygon.json")
        .expect("Could not read polygon.json");
    let fixtures: Fixtures = serde_json::from_str(&content).expect("Could not parse polygon.json");

    for f in &fixtures.polygon {
        let rings = to_rings(&f.polygon);
        let result = polygon_to_cells(&rings, f.resolution).expect("polygon_to_cells");
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
        polygon_to_cells(&[vec![LonLat::new(0.0, 0.0), LonLat::new(1.0, 1.0)]], 5)
            .unwrap()
            .len(),
        0
    );
    // Closed ring with only 2 distinct vertices
    assert_eq!(
        polygon_to_cells(
            &[vec![
                LonLat::new(0.0, 0.0),
                LonLat::new(1.0, 1.0),
                LonLat::new(0.0, 0.0)
            ]],
            5
        )
        .unwrap()
        .len(),
        0
    );
}

#[test]
fn test_polygon_to_cells_accepts_closed_rings() {
    let ring = vec![
        LonLat::new(-5.0, 54.0),
        LonLat::new(15.0, 54.0),
        LonLat::new(15.0, 44.0),
        LonLat::new(-5.0, 44.0),
    ];
    let hole = vec![
        LonLat::new(2.0, 51.0),
        LonLat::new(8.0, 51.0),
        LonLat::new(8.0, 47.0),
        LonLat::new(2.0, 47.0),
    ];
    let closed = |r: &[LonLat]| {
        let mut c = r.to_vec();
        c.push(r[0]);
        c
    };
    let open_result = polygon_to_cells(&[ring.clone(), hole.clone()], 6).unwrap();
    let closed_result = polygon_to_cells(&[closed(&ring), closed(&hole)], 6).unwrap();
    assert_eq!(closed_result, open_result);
}

#[test]
fn test_polygon_to_cells_ignores_degenerate_holes() {
    let ring = vec![
        LonLat::new(-5.0, 54.0),
        LonLat::new(15.0, 54.0),
        LonLat::new(15.0, 44.0),
        LonLat::new(-5.0, 44.0),
    ];
    let degenerate_hole = vec![LonLat::new(2.0, 50.0), LonLat::new(3.0, 49.0)];
    let without = polygon_to_cells(&[ring.clone()], 5).unwrap();
    let with = polygon_to_cells(&[ring, degenerate_hole], 5).unwrap();
    assert_eq!(with, without);
}

#[test]
fn test_polygon_to_cells_country_fixtures() {
    let content = fs::read_to_string("tests/fixtures/regions/polygon.json")
        .expect("Could not read polygon.json");
    let fixtures: Fixtures = serde_json::from_str(&content).expect("Could not parse polygon.json");

    for f in &fixtures.country {
        let rings = to_rings(&f.polygon);
        let result = polygon_to_cells(&rings, f.resolution).expect("polygon_to_cells");
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
