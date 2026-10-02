// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

// The canonical (engine) curve, via the top-level lattice API.

use a5::coordinate_systems::IJ;
use a5::lattice::{
    ij_to_s, s_to_cell, s_to_triple, triple_in_bounds, triple_parity, triple_to_s, Orientation,
    Triple,
};
use serde::Deserialize;
use std::fs;
use std::str::FromStr;

#[derive(Deserialize)]
struct Fixtures {
    #[serde(rename = "sToCell")]
    s_to_cell: Vec<SToCellFixture>,
    #[serde(rename = "IJToS")]
    ij_to_s: Vec<IJToSFixture>,
    #[serde(rename = "tripleInBounds")]
    triple_in_bounds: Vec<TripleInBoundsFixture>,
}

#[derive(Deserialize)]
struct SToCellFixture {
    s: u64,
    resolution: usize,
    orientation: String,
    x: i32,
    y: i32,
    z: i32,
    parity: i32,
    flavor: u8,
}

#[derive(Deserialize)]
struct IJToSFixture {
    i: f64,
    j: f64,
    resolution: usize,
    orientation: String,
    s: u64,
}

#[derive(Deserialize)]
struct TripleInBoundsFixture {
    x: i32,
    y: i32,
    z: i32,
    #[serde(rename = "maxRow")]
    max_row: i32,
    expected: bool,
}

fn load() -> Fixtures {
    let content = fs::read_to_string("tests/fixtures/lattice/curve.json")
        .expect("Could not read curve fixtures");
    serde_json::from_str(&content).expect("Could not parse curve fixtures")
}

fn ori(s: &str) -> Orientation {
    Orientation::from_str(s).unwrap()
}

#[test]
fn test_s_to_cell() {
    for f in &load().s_to_cell {
        let cell = s_to_cell(f.s, f.resolution, ori(&f.orientation));
        assert_eq!(cell.triple.x, f.x, "x for s={} res={}", f.s, f.resolution);
        assert_eq!(cell.triple.y, f.y, "y for s={} res={}", f.s, f.resolution);
        assert_eq!(cell.triple.z, f.z, "z for s={} res={}", f.s, f.resolution);
        assert_eq!(
            cell.flavor, f.flavor,
            "flavor for s={} res={}",
            f.s, f.resolution
        );
    }
}

#[test]
fn test_s_to_triple() {
    for f in &load().s_to_cell {
        let triple = s_to_triple(f.s, f.resolution, ori(&f.orientation));
        assert_eq!(triple, Triple::new(f.x, f.y, f.z));
    }
}

#[test]
fn test_triple_parity() {
    for f in &load().s_to_cell {
        let triple = Triple::new(f.x, f.y, f.z);
        assert_eq!(
            triple_parity(&triple),
            f.parity,
            "parity for ({},{},{})",
            f.x,
            f.y,
            f.z
        );
    }
}

#[test]
fn test_triple_to_s() {
    for f in &load().s_to_cell {
        let triple = Triple::new(f.x, f.y, f.z);
        let s = triple_to_s(&triple, f.resolution, ori(&f.orientation));
        assert_eq!(
            s,
            Some(f.s),
            "s for ({},{},{}) res={}",
            f.x,
            f.y,
            f.z,
            f.resolution
        );
    }
}

#[test]
fn test_ij_to_s() {
    for f in &load().ij_to_s {
        let s = ij_to_s(IJ::new(f.i, f.j), f.resolution, ori(&f.orientation));
        assert_eq!(s, f.s, "s for ({},{}) res={}", f.i, f.j, f.resolution);
    }
}

#[test]
fn test_triple_in_bounds() {
    for f in &load().triple_in_bounds {
        let triple = Triple::new(f.x, f.y, f.z);
        assert_eq!(
            triple_in_bounds(&triple, f.max_row),
            f.expected,
            "({},{},{}) maxRow={}",
            f.x,
            f.y,
            f.z,
            f.max_row
        );
    }
}
