// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use a5::collections::compact::{compact, uncompact};
use a5::collections::measures::{area, count};
use a5::collections::resolution::covering_resolution;
use a5::collections::set_operations::{contains, difference, intersect, overlaps, union};
use a5::core::cell::cell_to_boundary;
use a5::core::compaction_marker::is_compaction_marker;
use a5::core::hex::hex_to_u64;
use serde::Deserialize;
use std::fs;

#[derive(Deserialize)]
struct SetOperationCase {
    name: String,
    a: Vec<String>,
    b: Vec<String>,
    resolution: i32,
    union: Vec<String>,
    intersect: Vec<String>,
    difference: Vec<String>,
    overlaps: bool,
}

#[derive(Deserialize)]
struct PairCase {
    name: String,
    a: Vec<String>,
    b: Vec<String>,
}

#[derive(Deserialize)]
struct MeasureCase {
    name: String,
    cells: Vec<String>,
    resolution: i32,
    count: String,
    area: f64,
}

#[derive(Deserialize)]
struct Probe {
    cell: String,
    expected: bool,
}

#[derive(Deserialize)]
struct ContainsCase {
    name: String,
    cells: Vec<String>,
    probes: Vec<Probe>,
}

#[derive(Deserialize)]
struct MismatchedProbeCase {
    name: String,
    cells: Vec<String>,
    probes: Vec<String>,
}

#[derive(Deserialize)]
struct MarkerCase {
    value: String,
    expected: bool,
    resolution: Option<i32>,
}

#[derive(Deserialize)]
struct Fixtures {
    #[serde(rename = "setOperations")]
    set_operations: Vec<SetOperationCase>,
    #[serde(rename = "mismatchedResolutions")]
    mismatched_resolutions: Vec<PairCase>,
    measures: Vec<MeasureCase>,
    contains: Vec<ContainsCase>,
    #[serde(rename = "mismatchedProbes")]
    mismatched_probes: Vec<MismatchedProbeCase>,
    #[serde(rename = "isCompactionMarker")]
    is_compaction_marker: Vec<MarkerCase>,
}

fn load_fixtures() -> Fixtures {
    let data = fs::read_to_string("tests/fixtures/collection.json").expect("read collection.json");
    serde_json::from_str(&data).expect("parse collection.json")
}

fn to_cells(hex: &[String]) -> Vec<u64> {
    hex.iter().map(|h| hex_to_u64(h).unwrap()).collect()
}

#[test]
fn test_set_operations() {
    for f in load_fixtures().set_operations {
        let a = to_cells(&f.a);
        let b = to_cells(&f.b);
        assert_eq!(union(&a, &b).unwrap(), to_cells(&f.union), "{}", f.name);
        assert_eq!(
            intersect(&a, &b).unwrap(),
            to_cells(&f.intersect),
            "{}",
            f.name
        );
        assert_eq!(
            difference(&a, &b).unwrap(),
            to_cells(&f.difference),
            "{}",
            f.name
        );
        assert_eq!(overlaps(&a, &b).unwrap(), f.overlaps, "{}", f.name);
        assert_eq!(overlaps(&b, &a).unwrap(), f.overlaps, "{}", f.name);
        assert_eq!(
            covering_resolution(&union(&a, &b).unwrap()),
            f.resolution,
            "{}",
            f.name
        );
    }
}

#[test]
fn test_set_operations_refuse_mismatched_resolutions() {
    for f in load_fixtures().mismatched_resolutions {
        let a = to_cells(&f.a);
        let b = to_cells(&f.b);
        assert!(union(&a, &b).is_err(), "{}", f.name);
        assert!(intersect(&a, &b).is_err(), "{}", f.name);
        assert!(difference(&a, &b).is_err(), "{}", f.name);
        assert!(overlaps(&a, &b).is_err(), "{}", f.name);
    }
}

#[test]
fn test_set_operations_out_of_curve_order() {
    // Sorted input is merged as given; anything else is detected and sorted first
    for f in load_fixtures().set_operations {
        let mut a = to_cells(&f.a);
        let mut b = to_cells(&f.b);
        a.reverse();
        b.reverse();
        assert_eq!(union(&a, &b).unwrap(), to_cells(&f.union), "{}", f.name);
        assert_eq!(
            intersect(&a, &b).unwrap(),
            to_cells(&f.intersect),
            "{}",
            f.name
        );
        assert_eq!(
            difference(&a, &b).unwrap(),
            to_cells(&f.difference),
            "{}",
            f.name
        );
    }
}

#[test]
fn test_measures() {
    for f in load_fixtures().measures {
        let cells = to_cells(&f.cells);
        assert_eq!(covering_resolution(&cells), f.resolution, "{}", f.name);
        assert_eq!(
            count(&cells).unwrap(),
            f.count.parse::<u64>().unwrap(),
            "{}",
            f.name
        );
        assert!(
            (area(&cells).unwrap() - f.area).abs() <= 1e-10 * f.area,
            "{}",
            f.name
        );
    }
}

#[test]
fn test_measures_of_overlapping_input() {
    // Every cell given is counted, including duplicates
    for f in load_fixtures().measures {
        let cells = to_cells(&f.cells);
        let doubled: Vec<u64> = cells.iter().chain(cells.iter()).copied().collect();
        let expected = f.count.parse::<u64>().unwrap();
        assert_eq!(count(&doubled).unwrap(), 2 * expected, "{}", f.name);
        assert!(
            (area(&doubled).unwrap() - 2.0 * f.area).abs() <= 1e-10 * f.area,
            "{}",
            f.name
        );
        // union merges them
        assert_eq!(
            count(&union(&cells, &cells).unwrap()).unwrap(),
            expected,
            "{}",
            f.name
        );
    }
}

#[test]
fn test_contains() {
    for f in load_fixtures().contains {
        let cells = to_cells(&f.cells);
        for probe in &f.probes {
            assert_eq!(
                contains(&cells, hex_to_u64(&probe.cell).unwrap()).unwrap(),
                probe.expected,
                "{} {}",
                f.name,
                probe.cell
            );
        }
    }
}

#[test]
fn test_contains_refuses_other_resolutions() {
    for f in load_fixtures().mismatched_probes {
        let cells = to_cells(&f.cells);
        for probe in &f.probes {
            assert!(
                contains(&cells, hex_to_u64(probe).unwrap()).is_err(),
                "{} {}",
                f.name,
                probe
            );
        }
    }
}

#[test]
fn test_is_compaction_marker() {
    for f in load_fixtures().is_compaction_marker {
        let value = hex_to_u64(&f.value).unwrap();
        assert_eq!(is_compaction_marker(value), f.expected, "{}", f.value);
        if f.expected {
            // Records its resolution, and has an empty boundary
            assert_eq!(
                covering_resolution(&[value]),
                f.resolution.unwrap(),
                "{}",
                f.value
            );
            assert!(
                cell_to_boundary(value, None).unwrap().is_empty(),
                "{}",
                f.value
            );
        }
    }
}

#[derive(Deserialize)]
struct EdgeFixtures {
    #[serde(rename = "invalidCells")]
    invalid_cells: Vec<String>,
    #[serde(rename = "validEdgeCells")]
    valid_edge_cells: Vec<String>,
}

type Operation = fn(&[u64]) -> Result<(), String>;

/// Each operation applied to a single-value set, as in the TS 'invalid cells' tests
const OPERATIONS: [(&str, Operation); 6] = [
    ("compact", |cells| compact(cells).map(|_| ())),
    ("uncompact", |cells| uncompact(cells).map(|_| ())),
    ("count", |cells| count(cells).map(|_| ())),
    ("area", |cells| area(cells).map(|_| ())),
    ("union", |cells| union(cells, cells).map(|_| ())),
    ("contains", |cells| contains(cells, cells[0]).map(|_| ())),
];

fn load_edge_fixtures() -> EdgeFixtures {
    let data = fs::read_to_string("tests/fixtures/collection.json").expect("read collection.json");
    serde_json::from_str(&data).expect("parse collection.json")
}

#[test]
fn test_invalid_cells_are_refused() {
    let f = load_edge_fixtures();
    for (name, operation) in OPERATIONS {
        for value in &f.invalid_cells {
            let cell = hex_to_u64(value).unwrap();
            assert!(
                operation(&[cell]).is_err(),
                "{} should refuse {}",
                name,
                value
            );
        }
    }
}

#[test]
fn test_valid_edge_cells_are_accepted() {
    let f = load_edge_fixtures();
    for (name, operation) in OPERATIONS {
        for value in &f.valid_edge_cells {
            let cell = hex_to_u64(value).unwrap();
            let result = operation(&[cell]);
            assert!(
                result.is_ok(),
                "{} should accept {}: {:?}",
                name,
                value,
                result
            );
        }
    }
}
