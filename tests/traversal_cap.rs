// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use a5::traversal::cap::{estimate_cell_radius, meters_to_h, pick_coarse_resolution};
use a5::{hex_to_u64, spherical_cap, u64_to_hex, uncompact};
use serde::Deserialize;

#[derive(Deserialize)]
struct SphericalCapFixture {
    #[serde(rename = "cellId")]
    cell_id: String,
    radius: f64,
    cells: Vec<String>,
}

#[derive(Deserialize)]
struct SphericalCapCompactFixture {
    #[serde(rename = "cellId")]
    cell_id: String,
    radius: f64,
    #[serde(rename = "compactedCells")]
    compacted_cells: Vec<String>,
}

#[derive(Deserialize)]
struct MetersToHFixture {
    meters: f64,
    #[serde(rename = "expectedH")]
    expected_h: f64,
}

#[derive(Deserialize)]
struct EstimateCellRadiusFixture {
    resolution: i32,
    #[serde(rename = "expectedMeters")]
    expected_meters: f64,
}

#[derive(Deserialize)]
struct PickCoarseResolutionFixture {
    radius: f64,
    #[serde(rename = "targetRes")]
    target_res: i32,
    #[serde(rename = "expectedCoarseRes")]
    expected_coarse_res: i32,
}

#[derive(Deserialize)]
struct Helpers {
    #[serde(rename = "metersToH")]
    meters_to_h: Vec<MetersToHFixture>,
    #[serde(rename = "estimateCellRadius")]
    estimate_cell_radius: Vec<EstimateCellRadiusFixture>,
    #[serde(rename = "pickCoarseResolution")]
    pick_coarse_resolution: Vec<PickCoarseResolutionFixture>,
}

#[derive(Deserialize)]
struct Fixtures {
    #[serde(rename = "sphericalCap")]
    spherical_cap: Vec<SphericalCapFixture>,
    #[serde(rename = "sphericalCapCompact")]
    spherical_cap_compact: Vec<SphericalCapCompactFixture>,
    helpers: Helpers,
}

fn load_fixtures() -> Fixtures {
    let data = include_str!("fixtures/traversal/cap.json");
    serde_json::from_str(data).expect("Failed to parse cap.json")
}

#[test]
fn test_spherical_cap() {
    let fixtures = load_fixtures();
    for f in &fixtures.spherical_cap {
        let cell_id = hex_to_u64(&f.cell_id).unwrap();
        let cap = spherical_cap(cell_id, f.radius).unwrap();
        let uncompacted = uncompact(&cap).unwrap();
        let mut result_hex: Vec<String> = uncompacted.iter().map(|n| u64_to_hex(*n)).collect();
        result_hex.sort();
        let mut expected = f.cells.clone();
        expected.sort();
        assert_eq!(
            result_hex, expected,
            "cellId={} radius={}",
            f.cell_id, f.radius
        );
    }
}

#[test]
fn test_spherical_cap_compact() {
    let fixtures = load_fixtures();
    for f in &fixtures.spherical_cap_compact {
        let cell_id = hex_to_u64(&f.cell_id).unwrap();
        let result = spherical_cap(cell_id, f.radius).unwrap();
        let result_hex: Vec<String> = result.iter().map(|n| u64_to_hex(*n)).collect();
        assert_eq!(
            result_hex, f.compacted_cells,
            "cellId={} radius={}",
            f.cell_id, f.radius
        );
    }
}

#[test]
fn test_meters_to_h() {
    let fixtures = load_fixtures();
    for f in &fixtures.helpers.meters_to_h {
        let result = meters_to_h(f.meters);
        assert!(
            (result - f.expected_h).abs() < 1e-15,
            "meters_to_h({}) = {} expected {}",
            f.meters,
            result,
            f.expected_h
        );
    }
}

#[test]
fn test_estimate_cell_radius() {
    let fixtures = load_fixtures();
    for f in &fixtures.helpers.estimate_cell_radius {
        let result = estimate_cell_radius(f.resolution);
        // Allow tiny tolerance: Rust's f64 math may round differently from JS
        assert!(
            (result - f.expected_meters).abs() < 1e-6,
            "estimate_cell_radius({}): {} != {}",
            f.resolution,
            result,
            f.expected_meters
        );
    }
}

#[test]
fn test_pick_coarse_resolution() {
    let fixtures = load_fixtures();
    for f in &fixtures.helpers.pick_coarse_resolution {
        let result = pick_coarse_resolution(f.radius, f.target_res);
        assert_eq!(
            result, f.expected_coarse_res,
            "pick_coarse_resolution({}, {})",
            f.radius, f.target_res
        );
    }
}

/// Half the circumference of the authalic sphere: the farthest any point can be.
const HALF_CIRCUMFERENCE: f64 = std::f64::consts::PI * a5::core::constants::AUTHALIC_RADIUS_EARTH;

/// The cells at the center's resolution whose centers lie within `radius`, by
/// testing every cell.
fn brute_force_cap(cell_id: u64, radius: f64) -> Vec<u64> {
    use a5::core::cell::cell_to_spherical;
    use a5::core::origin::haversine;
    let center = cell_to_spherical(cell_id).unwrap();
    let h = meters_to_h(radius);
    let resolution = a5::get_resolution(cell_id);
    let mut cells: Vec<u64> = a5::cell_to_children(a5::WORLD_CELL, Some(resolution))
        .unwrap()
        .into_iter()
        .filter(|&c| haversine(center, cell_to_spherical(c).unwrap()) <= h)
        .collect();
    cells.sort_unstable();
    cells
}

#[test]
fn test_spherical_cap_near_half_circumference() {
    // The expanded search radius passes piR here, which used to shrink it
    let cell = a5::lonlat_to_cell(a5::LonLat::new(-3.19, 55.95), 3).unwrap();
    for radius in [1.5e7, 1.9e7, 1.99e7, 2.0e7] {
        let mut cells = uncompact(&spherical_cap(cell, radius).unwrap()).unwrap();
        cells.sort_unstable();
        assert_eq!(cells, brute_force_cap(cell, radius), "radius {}", radius);
    }
}

#[test]
fn test_spherical_cap_whole_globe() {
    let cell = a5::lonlat_to_cell(a5::LonLat::new(-3.19, 55.95), 5).unwrap();
    for radius in [HALF_CIRCUMFERENCE, 2.5e7, 4e7, f64::INFINITY] {
        let cap = spherical_cap(cell, radius).unwrap();
        assert_eq!(cap[0], a5::WORLD_CELL, "radius {}", radius);
        assert_eq!(
            a5::count(&cap).unwrap(),
            a5::get_num_cells(5),
            "radius {}",
            radius
        );
    }
}

#[test]
fn test_spherical_cap_invalid_radius() {
    let cell = a5::lonlat_to_cell(a5::LonLat::new(-3.19, 55.95), 5).unwrap();
    for radius in [f64::NAN, -1.0, f64::NEG_INFINITY] {
        assert!(spherical_cap(cell, radius).is_err(), "radius {}", radius);
    }
}
