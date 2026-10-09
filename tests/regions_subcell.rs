use a5::{
    cell_to_subcell, cell_to_supercell, count, covering_resolution, get_resolution, hex_to_u64,
    u64_to_hex, uncompact,
};
use serde::Deserialize;
use std::fs;

#[derive(Deserialize)]
struct SubcellFixture {
    cell: String,
    resolution: i32,
    count: u64,
    cells: Vec<String>,
}

#[derive(Deserialize)]
struct SupercellFixture {
    cell: String,
    resolution: i32,
    supercell: String,
}

#[derive(Deserialize)]
struct Fixtures {
    subcell: Vec<SubcellFixture>,
    supercell: Vec<SupercellFixture>,
}

fn load() -> Fixtures {
    let content = fs::read_to_string("tests/fixtures/regions/subcell.json")
        .expect("Could not read subcell fixtures");
    serde_json::from_str(&content).expect("Could not parse subcell fixtures")
}

fn hex(cell: &str) -> u64 {
    hex_to_u64(cell).unwrap()
}

#[test]
fn test_cell_to_subcell() {
    for f in &load().subcell {
        let result = cell_to_subcell(hex(&f.cell), f.resolution).unwrap();
        let cells: Vec<String> = result.iter().map(|&c| u64_to_hex(c)).collect();
        assert_eq!(cells, f.cells, "subcells of {} at {}", f.cell, f.resolution);
        assert_eq!(count(&result).unwrap(), f.count);
    }
}

#[test]
fn test_cell_to_subcell_maps_back() {
    for f in &load().subcell {
        let cell = hex(&f.cell);
        let cell_resolution = get_resolution(cell);
        let subcells = uncompact(&cell_to_subcell(cell, f.resolution).unwrap()).unwrap();
        for subcell in subcells {
            assert_eq!(
                cell_to_supercell(subcell, cell_resolution).unwrap(),
                cell,
                "supercell of {} (a subcell of {})",
                u64_to_hex(subcell),
                f.cell
            );
        }
    }
}

#[test]
fn test_cell_to_subcell_own_resolution() {
    let cell = hex(&load().subcell[5].cell);
    let result = cell_to_subcell(cell, get_resolution(cell)).unwrap();
    assert_eq!(uncompact(&result).unwrap(), vec![cell]);
    assert_eq!(covering_resolution(&result), get_resolution(cell));
}

#[test]
fn test_cell_to_subcell_coarser_resolution_errors() {
    let cell = hex(&load().subcell[5].cell);
    assert!(cell_to_subcell(cell, get_resolution(cell) - 1).is_err());
}

#[test]
fn test_cell_to_supercell() {
    for f in &load().supercell {
        let supercell = cell_to_supercell(hex(&f.cell), f.resolution).unwrap();
        assert_eq!(
            u64_to_hex(supercell),
            f.supercell,
            "supercell of {} at {}",
            f.cell,
            f.resolution
        );
    }
}

#[test]
fn test_cell_to_supercell_finer_resolution_errors() {
    let cell = hex(&load().supercell[0].cell);
    assert!(cell_to_supercell(cell, get_resolution(cell) + 1).is_err());
}
