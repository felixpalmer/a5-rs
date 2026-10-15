use a5::coordinate_systems::LonLat;
use a5::core::hex::u64_to_hex;
use a5::coverings::compact::uncompact;
use a5::coverings::measures::count;
use a5::coverings::resolution::covering_resolution;
use a5::regions::polygon::{polygon_to_cells, Containment, PolygonToCellsOptions};
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
    overlapping: Vec<PolygonFixture>,
    #[serde(default)]
    country: Vec<CountryFixture>,
}

#[derive(Deserialize)]
struct WorldCellPolygonFixture {
    polygon: Vec<Vec<[f64; 2]>>,
    resolution: i32,
    containment: String,
    #[serde(rename = "compactedCells")]
    compacted_cells: Vec<String>,
}

#[derive(Deserialize)]
struct PolygonOutOfRangeFixture {
    polygon: Vec<Vec<[f64; 2]>>,
    resolution: i32,
    containment: String,
}

#[derive(Deserialize)]
struct WorldCellFixtures {
    #[serde(rename = "polygonToCells")]
    polygon_to_cells: Vec<WorldCellPolygonFixture>,
    #[serde(rename = "polygonToCellsOutOfRange")]
    polygon_to_cells_out_of_range: Vec<PolygonOutOfRangeFixture>,
}

fn load_world_cell_fixtures() -> WorldCellFixtures {
    let data = include_str!("fixtures/world-cell.json");
    serde_json::from_str(data).expect("Failed to parse world-cell.json")
}

fn to_containment(name: &str) -> Containment {
    match name {
        "center" => Containment::Center,
        "overlapping" => Containment::Overlapping,
        _ => panic!("unknown containment {name}"),
    }
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
        let result = polygon_to_cells(&rings, f.resolution, None).expect("polygon_to_cells");
        let expanded = uncompact(&result).expect("uncompact");
        let mut sorted = expanded;
        sorted.sort();
        let result_hex: Vec<String> = sorted.into_iter().map(u64_to_hex).collect();
        assert_eq!(result_hex, f.cells, "{}: cells mismatch", f.name);
    }
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
    let open_result = polygon_to_cells(&[ring.clone(), hole.clone()], 6, None).unwrap();
    let closed_result = polygon_to_cells(&[closed(&ring), closed(&hole)], 6, None).unwrap();
    assert_eq!(closed_result, open_result);
}

#[test]
fn test_polygon_to_cells_empty_covering_for_less_than_3_vertices() {
    let p = |lon: f64, lat: f64| LonLat::new(lon, lat);
    let degenerate: Vec<Vec<Vec<LonLat>>> = vec![
        vec![],
        vec![vec![p(0.0, 0.0), p(1.0, 1.0)]],
        // Closed ring with only 2 distinct vertices
        vec![vec![p(0.0, 0.0), p(1.0, 1.0), p(0.0, 0.0)]],
    ];
    for polygon in &degenerate {
        let cells = polygon_to_cells(polygon, 5, None).unwrap();
        assert_eq!(count(&cells).unwrap(), 0);
        // The empty covering still records its resolution
        assert_eq!(covering_resolution(&cells), 5);
    }
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
    let without = polygon_to_cells(&[ring.clone()], 5, None).unwrap();
    let with = polygon_to_cells(&[ring, degenerate_hole], 5, None).unwrap();
    assert_eq!(with, without);
}

#[test]
fn test_polygon_to_cells_overlapping_fixtures() {
    let content = fs::read_to_string("tests/fixtures/regions/polygon.json")
        .expect("Could not read polygon.json");
    let fixtures: Fixtures = serde_json::from_str(&content).expect("Could not parse polygon.json");

    let options = Some(PolygonToCellsOptions {
        containment: Containment::Overlapping,
    });
    for f in &fixtures.overlapping {
        let rings = to_rings(&f.polygon);
        let result = polygon_to_cells(&rings, f.resolution, options).expect("polygon_to_cells");
        let expanded = uncompact(&result).expect("uncompact");
        let mut sorted = expanded;
        sorted.sort();
        let result_hex: Vec<String> = sorted.into_iter().map(u64_to_hex).collect();
        assert_eq!(
            result_hex, f.cells,
            "{}: overlapping cells mismatch",
            f.name
        );
    }
}

#[test]
fn test_polygon_to_cells_overlapping_is_superset_of_center() {
    let ring = vec![
        LonLat::new(-5.0, 54.0),
        LonLat::new(15.0, 54.0),
        LonLat::new(15.0, 44.0),
        LonLat::new(-5.0, 44.0),
    ];
    let center = polygon_to_cells(&[ring.clone()], 6, None).unwrap();
    let overlapping = polygon_to_cells(
        &[ring],
        6,
        Some(PolygonToCellsOptions {
            containment: Containment::Overlapping,
        }),
    )
    .unwrap();
    let center_expanded: HashSet<u64> = uncompact(&center).unwrap().into_iter().collect();
    let overlapping_expanded: HashSet<u64> = uncompact(&overlapping).unwrap().into_iter().collect();
    assert!(center_expanded.is_subset(&overlapping_expanded));
    assert!(overlapping_expanded.len() > center_expanded.len());
}

#[test]
fn test_polygon_to_cells_country_fixtures() {
    let content = fs::read_to_string("tests/fixtures/regions/polygon.json")
        .expect("Could not read polygon.json");
    let fixtures: Fixtures = serde_json::from_str(&content).expect("Could not parse polygon.json");

    for f in &fixtures.country {
        let rings = to_rings(&f.polygon);
        let result = polygon_to_cells(&rings, f.resolution, None).expect("polygon_to_cells");
        let expanded = uncompact(&result).expect("uncompact");
        let unique: HashSet<u64> = expanded.into_iter().collect();
        assert_eq!(
            unique.len(),
            f.cell_count,
            "{}: cell count mismatch",
            f.name
        );
    }
}

// At resolution -1 the world cell overlaps every polygon but has no center, so
// only overlapping containment returns it
#[test]
fn test_polygon_to_cells_world_cell() {
    for f in &load_world_cell_fixtures().polygon_to_cells {
        let options = Some(PolygonToCellsOptions {
            containment: to_containment(&f.containment),
        });
        let result: Vec<String> = polygon_to_cells(&to_rings(&f.polygon), f.resolution, options)
            .unwrap()
            .into_iter()
            .map(u64_to_hex)
            .collect();
        assert_eq!(
            result, f.compacted_cells,
            "resolution={} containment={}",
            f.resolution, f.containment
        );
    }
}

#[test]
fn test_polygon_to_cells_resolution_out_of_range() {
    for f in &load_world_cell_fixtures().polygon_to_cells_out_of_range {
        let options = Some(PolygonToCellsOptions {
            containment: to_containment(&f.containment),
        });
        let err = polygon_to_cells(&to_rings(&f.polygon), f.resolution, options).expect_err(
            &format!("resolution={} containment={}", f.resolution, f.containment),
        );
        assert!(err.contains("is out of range"), "{}", err);
    }
}
