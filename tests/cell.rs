use a5::coordinate_systems::LonLat;
use a5::core::cell::{
    a5cell_contains_point, cell_to_boundary, lonlat_to_cell, CellToBoundaryOptions,
};
use a5::core::hex::hex_to_big_int;
use a5::core::serialization::{deserialize, MAX_RESOLUTION};
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize)]
struct GeoJSONGeometry {
    coordinates: Vec<f64>,
}

#[derive(Deserialize)]
struct GeoJSONProperties {
    name: Option<String>,
}

#[derive(Deserialize)]
struct GeoJSONFeature {
    properties: GeoJSONProperties,
    geometry: GeoJSONGeometry,
}

#[derive(Deserialize)]
struct GeoJSONFeatureCollection {
    features: Vec<GeoJSONFeature>,
}

fn load_populated_places() -> GeoJSONFeatureCollection {
    let data = include_str!("../tests/data/ne_50m_populated_places_nameonly.json");
    serde_json::from_str(data).expect("Failed to parse populated places data")
}

#[test]
fn test_antimeridian_cell_longitude_span() {
    let antimeridian_cells = ["eb60000000000000", "2e00000000000000"];
    let segments = [1, 10]; // Note: 'auto' is handled as None in Rust

    for cell_id_hex in &antimeridian_cells {
        for &segment in &segments {
            let cell_id = hex_to_big_int(cell_id_hex);
            let cell_id_u64 = cell_id
                .to_string()
                .parse::<u64>()
                .expect("Failed to convert to u64");

            let options = CellToBoundaryOptions {
                closed_ring: true,
                segments: Some(segment),
            };
            let boundary =
                cell_to_boundary(cell_id_u64, Some(options)).expect("Failed to get boundary");

            // Check for antimeridian crossing
            let longitudes: Vec<f64> = boundary.iter().map(|point| point.longitude()).collect();
            let min_lon = longitudes.iter().fold(f64::INFINITY, |a, &b| a.min(b));
            let max_lon = longitudes.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
            let lon_span = max_lon - min_lon;

            assert!(
                lon_span < 180.0,
                "Longitude span should be less than 180 degrees for antimeridian cells, got {} for cell {} with {} segments",
                lon_span, cell_id_hex, segment
            );
        }

        // Test with 'auto' segments (None option)
        let cell_id = hex_to_big_int(cell_id_hex);
        let cell_id_u64 = cell_id
            .to_string()
            .parse::<u64>()
            .expect("Failed to convert to u64");

        let boundary = cell_to_boundary(cell_id_u64, None).expect("Failed to get boundary");

        let longitudes: Vec<f64> = boundary.iter().map(|point| point.longitude()).collect();
        let min_lon = longitudes.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let max_lon = longitudes.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        let lon_span = max_lon - min_lon;

        assert!(
            lon_span < 180.0,
            "Longitude span should be less than 180 degrees for antimeridian cells, got {} for cell {} with auto segments",
            lon_span, cell_id_hex
        );
    }
}

#[test]
fn test_cell_boundary_contains_original_point() {
    let populated_places = load_populated_places();

    // Extract coordinates from GeoJSON features
    let test_points: Vec<LonLat> = populated_places
        .features
        .iter()
        .map(|feature| {
            let coords = &feature.geometry.coordinates;
            LonLat::new(coords[0], coords[1])
        })
        .collect();

    println!(
        "Testing with {} points from GeoJSON file",
        test_points.len()
    );

    // Dictionary to store failures for each resolution and point
    let mut failures: HashMap<String, HashMap<i32, Vec<String>>> = HashMap::new();

    println!(
        "Skipping resolution {} as lonLatToCell is not implemented for this resolution yet",
        MAX_RESOLUTION
    );

    // Test each point from GeoJSON
    for (point_index, test_lonlat) in test_points.iter().enumerate() {
        let feature_name = populated_places.features[point_index]
            .properties
            .name
            .clone()
            .unwrap_or_else(|| format!("Unnamed {}", point_index));
        let point_key = format!(
            "Point {} - {} ({}, {})",
            point_index,
            feature_name,
            test_lonlat.longitude(),
            test_lonlat.latitude()
        );

        // Test resolutions from 1 to MAX_RESOLUTION-1
        for resolution in 1..MAX_RESOLUTION {
            // Issues in polar regions, TODO fix
            if resolution == MAX_RESOLUTION || test_lonlat.latitude().abs() > 80.0 {
                continue;
            }

            let mut resolution_failures: Vec<String> = Vec::new();

            match lonlat_to_cell(*test_lonlat, resolution) {
                Ok(cell_id) => {
                    match cell_to_boundary(cell_id, None) {
                        Ok(_boundary) => {
                            // Verify the original point is contained within the cell
                            match deserialize(cell_id) {
                                Ok(cell) => match a5cell_contains_point(&cell, *test_lonlat) {
                                    Ok(contains_result) => {
                                        if contains_result < 0.0 {
                                            resolution_failures.push(format!(
                                                "Cell {} does not contain the original point {:?}",
                                                cell_id, test_lonlat
                                            ));
                                        }
                                    }
                                    Err(e) => {
                                        resolution_failures
                                            .push(format!("Error checking containment: {}", e));
                                    }
                                },
                                Err(e) => {
                                    resolution_failures
                                        .push(format!("Error deserializing cell: {}", e));
                                }
                            }
                        }
                        Err(e) => {
                            resolution_failures.push(format!("Error getting boundary: {}", e));
                        }
                    }
                }
                Err(e) => {
                    resolution_failures.push(format!("Error getting cell ID: {}", e));
                }
            }

            // Store failures for this resolution if any occurred
            if !resolution_failures.is_empty() {
                failures
                    .entry(point_key.clone())
                    .or_default()
                    .insert(resolution, resolution_failures);
            }
        }
    }

    // Report all failures
    if !failures.is_empty() {
        let mut failure_message = String::from("\nFailures by point and resolution:\n");
        for (point_key, point_failures) in failures {
            if !point_failures.is_empty() {
                failure_message.push_str(&format!("\n{}:\n", point_key));
                for (resolution, resolution_failures) in point_failures {
                    failure_message.push_str(&format!("  Resolution {}:\n", resolution));
                    for failure in resolution_failures {
                        failure_message.push_str(&format!("    - {}\n", failure));
                    }
                }
            }
        }
        panic!("{}", failure_message);
    }
}
