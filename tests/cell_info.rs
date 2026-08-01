use a5::coordinate_systems::LonLat;
use a5::core::cell::{cell_to_boundary, CellToBoundaryOptions};
use a5::core::cell_info::{cell_area, cell_edge_length_avg, get_num_cells};
use a5::core::constants::AUTHALIC_RADIUS_EARTH;
use a5::core::hex::hex_to_u64;
use a5::core::serialization::get_resolution;
use approx::assert_relative_eq;
use serde::Deserialize;

#[derive(Deserialize)]
struct NumCellsFixture {
    resolution: i32,
    count: u64,
}

#[derive(Deserialize)]
struct CellAreaFixture {
    resolution: i32,
    #[serde(rename = "areaM2")]
    area_m2: f64,
}

#[derive(Deserialize)]
struct CellEdgeLengthFixture {
    resolution: i32,
    #[serde(rename = "lengthM")]
    length_m: f64,
}

#[derive(Deserialize)]
struct CellInfoFixtures {
    #[serde(rename = "numCells")]
    num_cells: Vec<NumCellsFixture>,
    #[serde(rename = "cellArea")]
    cell_area: Vec<CellAreaFixture>,
    #[serde(rename = "cellEdgeLengthAvg")]
    cell_edge_length_avg: Vec<CellEdgeLengthFixture>,
}

fn load_cell_info_fixtures() -> CellInfoFixtures {
    let fixture_data = include_str!("../tests/fixtures/cell-info.json");
    serde_json::from_str(fixture_data).expect("Failed to parse cell-info fixtures")
}

#[test]
fn test_get_num_cells() {
    let fixtures = load_cell_info_fixtures();

    for fixture in fixtures.num_cells {
        // Test u64 version
        assert_eq!(
            get_num_cells(fixture.resolution),
            fixture.count,
            "get_num_cells failed for resolution {}",
            fixture.resolution
        );
    }
}

#[test]
fn test_cell_area() {
    let fixtures = load_cell_info_fixtures();

    // Use relative-epsilon equality: f64 arithmetic order can drift the result
    // by 1 ULP between Rust and JS, which would fail assert_eq even when the
    // computation is correct. Epsilon still catches genuine formula changes.
    for fixture in fixtures.cell_area {
        assert_relative_eq!(
            cell_area(fixture.resolution),
            fixture.area_m2,
            max_relative = 1e-12
        );
    }
}

#[test]
fn test_cell_edge_length_avg() {
    let fixtures = load_cell_info_fixtures();

    for fixture in fixtures.cell_edge_length_avg {
        assert_relative_eq!(
            cell_edge_length_avg(fixture.resolution),
            fixture.length_m,
            max_relative = 1e-12
        );
    }
}

/// Geodesic distance between two points on the authalic sphere, in meters
fn geodesic(a: &LonLat, b: &LonLat) -> f64 {
    let lat1 = a.latitude().to_radians();
    let lat2 = b.latitude().to_radians();
    let d_lon = (b.longitude() - a.longitude()).to_radians();
    let h =
        ((lat2 - lat1) / 2.0).sin().powi(2) + lat1.cos() * lat2.cos() * (d_lon / 2.0).sin().powi(2);
    2.0 * AUTHALIC_RADIUS_EARTH * h.sqrt().asin()
}

#[test]
fn test_every_boundary_edge_of_test_cells_is_within_10_percent_of_average() {
    let fixture_data = include_str!("../tests/fixtures/serialization.json");
    let fixtures: serde_json::Value =
        serde_json::from_str(fixture_data).expect("Failed to parse serialization fixtures");
    let test_ids: Vec<String> = fixtures["testIds"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();

    // Sample each edge with multiple segments to measure its true curved length
    const SEGMENTS: usize = 10;
    for hex in test_ids {
        let cell = hex_to_u64(&hex).unwrap();
        let resolution = get_resolution(cell);
        let avg = cell_edge_length_avg(resolution);
        let boundary = cell_to_boundary(
            cell,
            Some(CellToBoundaryOptions {
                closed_ring: true,
                segments: Some(SEGMENTS as i32),
            }),
        )
        .unwrap();
        let num_edges = (boundary.len() - 1) / SEGMENTS;
        for e in 0..num_edges {
            let mut length = 0.0;
            for i in 0..SEGMENTS {
                let idx = e * SEGMENTS + i;
                length += geodesic(&boundary[idx], &boundary[idx + 1]);
            }
            let ratio = length / avg;
            assert!(
                ratio > 0.9 && ratio < 1.1,
                "cell {hex} edge {e}: ratio {ratio}"
            );
        }
    }
}
