// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use a5::traversal::cap::{estimate_cell_radius, meters_to_h, pick_coarse_resolution};
use a5::{hex_to_u64, spherical_cap, u64_to_hex, uncompact};
use serde::{Deserialize, Deserializer};

/// A JSON number, or one of "NaN", "Infinity" and "-Infinity", which JSON
/// cannot hold as numbers
fn number<'de, D: Deserializer<'de>>(deserializer: D) -> Result<f64, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Number {
        Value(f64),
        Text(String),
    }
    match Number::deserialize(deserializer)? {
        Number::Value(x) => Ok(x),
        Number::Text(s) => s.parse().map_err(serde::de::Error::custom),
    }
}

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
    #[serde(deserialize_with = "number")]
    radius: f64,
    #[serde(rename = "compactedCells")]
    compacted_cells: Vec<String>,
}

#[derive(Deserialize)]
struct MetersToHFixture {
    #[serde(deserialize_with = "number")]
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
struct InvalidRadiusFixture {
    #[serde(rename = "cellId")]
    cell_id: String,
    #[serde(deserialize_with = "number")]
    radius: f64,
}

#[derive(Deserialize)]
struct Fixtures {
    #[serde(rename = "sphericalCap")]
    spherical_cap: Vec<SphericalCapFixture>,
    #[serde(rename = "sphericalCapCompact")]
    spherical_cap_compact: Vec<SphericalCapCompactFixture>,
    helpers: Helpers,
    #[serde(rename = "invalidRadii")]
    invalid_radii: Vec<InvalidRadiusFixture>,
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

#[test]
fn test_spherical_cap_invalid_radius_errors() {
    let fixtures = load_fixtures();
    for f in &fixtures.invalid_radii {
        let cell_id = hex_to_u64(&f.cell_id).unwrap();
        let err = spherical_cap(cell_id, f.radius)
            .expect_err(&format!("cellId={} radius={}", f.cell_id, f.radius));
        assert!(err.contains("must be zero or more"), "{}", err);
    }
}
