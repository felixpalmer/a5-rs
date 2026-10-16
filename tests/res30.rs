// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

//! Resolution 30 IDs exist only in quintants 0-41. In quintants 42-59 a
//! request for resolution 30 gives res-29 cells, which stand in once each.

use a5::core::serialization::slot_to_cell;
use a5::{cell_to_children, compact, count, hex_to_u64, spherical_cap, u64_to_hex, uncompact};
use serde::Deserialize;

#[derive(Deserialize)]
struct CellToChildrenCase {
    name: String,
    cell: String,
    #[serde(rename = "childResolution")]
    child_resolution: i32,
    children: Vec<String>,
}

#[derive(Deserialize)]
struct SlotToCellCase {
    slot: String,
    resolution: i32,
    cell: String,
}

#[derive(Deserialize)]
struct CountCase {
    name: String,
    cells: Vec<String>,
    count: String,
}

#[derive(Deserialize)]
struct UncompactCase {
    name: String,
    input: Vec<String>,
    #[serde(rename = "expectedCells")]
    expected_cells: Vec<String>,
}

#[derive(Deserialize)]
struct CompactCase {
    name: String,
    input: Vec<String>,
    #[serde(rename = "expectedOutput")]
    expected_output: Vec<String>,
}

#[derive(Deserialize)]
struct SphericalCapCase {
    #[serde(rename = "cellId")]
    cell_id: String,
    radius: f64,
    #[serde(rename = "compactedCells")]
    compacted_cells: Vec<String>,
    count: String,
}

#[derive(Deserialize)]
struct Fixtures {
    #[serde(rename = "cellToChildren")]
    cell_to_children: Vec<CellToChildrenCase>,
    #[serde(rename = "slotToCell")]
    slot_to_cell: Vec<SlotToCellCase>,
    count: Vec<CountCase>,
    uncompact: Vec<UncompactCase>,
    compact: Vec<CompactCase>,
    #[serde(rename = "sphericalCap")]
    spherical_cap: Vec<SphericalCapCase>,
}

fn load_fixtures() -> Fixtures {
    let data = include_str!("fixtures/res30.json");
    serde_json::from_str(data).expect("Failed to parse res30.json")
}

fn to_ids(hexes: &[String]) -> Vec<u64> {
    hexes.iter().map(|h| hex_to_u64(h).unwrap()).collect()
}

fn to_hexes(ids: &[u64]) -> Vec<String> {
    ids.iter().map(|&id| u64_to_hex(id)).collect()
}

#[test]
fn test_cell_to_children() {
    for f in &load_fixtures().cell_to_children {
        let cell = hex_to_u64(&f.cell).unwrap();
        let children = cell_to_children(cell, Some(f.child_resolution)).unwrap();
        assert_eq!(to_hexes(&children), f.children, "{}", f.name);
    }
}

#[test]
fn test_slot_to_cell() {
    for f in &load_fixtures().slot_to_cell {
        let slot = hex_to_u64(&f.slot).unwrap();
        assert_eq!(
            u64_to_hex(slot_to_cell(slot, f.resolution)),
            f.cell,
            "slot {}",
            f.slot
        );
    }
}

#[test]
fn test_count() {
    for f in &load_fixtures().count {
        let expected = f.count.parse::<u64>().unwrap();
        assert_eq!(count(&to_ids(&f.cells)).unwrap(), expected, "{}", f.name);
    }
}

#[test]
fn test_uncompact() {
    for f in &load_fixtures().uncompact {
        let cells = uncompact(&to_ids(&f.input)).unwrap();
        assert_eq!(to_hexes(&cells), f.expected_cells, "{}", f.name);
    }
}

#[test]
fn test_compact() {
    for f in &load_fixtures().compact {
        let covering = compact(&to_ids(&f.input)).unwrap();
        assert_eq!(to_hexes(&covering), f.expected_output, "{}", f.name);
    }
}

#[test]
fn test_spherical_cap() {
    for f in &load_fixtures().spherical_cap {
        let cell_id = hex_to_u64(&f.cell_id).unwrap();
        let cap = spherical_cap(cell_id, f.radius).unwrap();
        let context = format!("cellId={} radius={}", f.cell_id, f.radius);
        assert_eq!(to_hexes(&cap), f.compacted_cells, "{}", context);
        let expected = f.count.parse::<u64>().unwrap();
        assert_eq!(count(&cap).unwrap(), expected, "{}", context);
        assert_eq!(
            uncompact(&cap).unwrap().len() as u64,
            expected,
            "{}",
            context
        );
    }
}
