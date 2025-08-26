// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

//! Wireframe example that generates A5 cell boundaries at a given resolution
//! 
//! This example replicates the functionality of the Python wireframe example
//! found in `git/a5-py/examples/wireframe/index.py`
//! 
//! Usage: cargo run --example wireframe <resolution> <output.json>

use a5_rs::core::cell::{cell_to_boundary, CellToBoundaryOptions};
use a5_rs::core::hex::u64_to_hex;
use a5_rs::core::serialization::{cell_to_children, WORLD_CELL};
use serde_json::json;
use std::env;
use std::fs;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() != 3 {
        eprintln!("Usage: {} <resolution> <output.json>", args[0]);
        eprintln!("  resolution: A5 cell resolution (integer)");
        process::exit(1);
    }
    
    let resolution = match args[1].parse::<i32>() {
        Ok(r) => r,
        Err(_) => {
            eprintln!("Error: resolution must be an integer");
            process::exit(1);
        }
    };
    
    let output_file = &args[2];
    
    match generate_wireframe(resolution, output_file) {
        Ok(count) => {
            println!("Successfully generated {} A5 cells at resolution {}", count, resolution);
            println!("Output written to {}", output_file);
        }
        Err(error) => {
            eprintln!("Error generating cells: {}", error);
            process::exit(1);
        }
    }
}

fn generate_wireframe(resolution: i32, output_file: &str) -> Result<usize, String> {
    // Calculate total number of cells at this resolution
    let cell_ids = cell_to_children(WORLD_CELL, Some(resolution))?;
    
    let mut features = Vec::new();
    
    // Generate all cells
    for cell_id in &cell_ids {
        let boundary_options = CellToBoundaryOptions {
            closed_ring: true,
            segments: Some(1),
        };
        
        let boundary = cell_to_boundary(*cell_id, Some(boundary_options))?;
        
        // Convert boundary coordinates to [longitude, latitude] format for GeoJSON
        let coordinates: Vec<[f64; 2]> = boundary
            .iter()
            .map(|lonlat| [lonlat.longitude(), lonlat.latitude()])
            .collect();
        
        let feature = json!({
            "type": "Feature",
            "geometry": {
                "type": "Polygon",
                "coordinates": [coordinates]
            },
            "properties": {
                "cellIdHex": u64_to_hex(*cell_id)
            }
        });
        
        features.push(feature);
    }
    
    // Create GeoJSON FeatureCollection
    let geojson = json!({
        "type": "FeatureCollection",
        "features": features
    });
    
    // Write to JSON file
    let json_string = serde_json::to_string_pretty(&geojson)
        .map_err(|e| format!("Failed to serialize JSON: {}", e))?;
    
    fs::write(output_file, json_string)
        .map_err(|e| format!("Failed to write file: {}", e))?;
    
    Ok(features.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use std::path::Path;
    use tempfile::NamedTempFile;
    
    #[test]
    fn test_generate_wireframe_resolution_0() {
        let temp_file = NamedTempFile::new().unwrap();
        let temp_path = temp_file.path().to_str().unwrap();
        
        let result = generate_wireframe(0, temp_path);
        assert!(result.is_ok());
        
        let count = result.unwrap();
        assert_eq!(count, 12); // Resolution 0 should have 12 cells
        
        // Verify file was created and has valid JSON
        assert!(Path::new(temp_path).exists());
        let content = fs::read_to_string(temp_path).unwrap();
        let geojson: Value = serde_json::from_str(&content).unwrap();
        
        assert_eq!(geojson["type"], "FeatureCollection");
        assert_eq!(geojson["features"].as_array().unwrap().len(), 12);
    }
    
    #[test]
    fn test_generate_wireframe_resolution_1() {
        let temp_file = NamedTempFile::new().unwrap();
        let temp_path = temp_file.path().to_str().unwrap();
        
        let result = generate_wireframe(1, temp_path);
        assert!(result.is_ok());
        
        let count = result.unwrap();
        assert_eq!(count, 60); // Resolution 1 should have 60 cells
        
        // Verify file was created and has valid JSON
        assert!(Path::new(temp_path).exists());
        let content = fs::read_to_string(temp_path).unwrap();
        let geojson: Value = serde_json::from_str(&content).unwrap();
        
        assert_eq!(geojson["type"], "FeatureCollection");
        assert_eq!(geojson["features"].as_array().unwrap().len(), 60);
    }
}
