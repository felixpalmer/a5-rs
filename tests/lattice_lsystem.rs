// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use a5::coordinate_systems::IJ;
use a5::lattice::curve::round_to_triple;
use a5::lattice::lsystem::{s_to_cell, s_to_triple, triple_to_s_lattice};
use a5::lattice::{Orientation, Triple};
use serde::Deserialize;
use std::fs;
use std::str::FromStr;

#[derive(Deserialize)]
struct Fixtures {
    #[serde(rename = "sToCell")]
    s_to_cell: Vec<SToCellFixture>,
    #[serde(rename = "pointToS")]
    point_to_s: Vec<PointToSFixture>,
}

#[derive(Deserialize)]
struct SToCellFixture {
    s: u64,
    resolution: usize,
    orientation: String,
    x: i32,
    y: i32,
    z: i32,
    #[allow(dead_code)]
    parity: i32,
    flavor: u8,
}

#[derive(Deserialize)]
struct PointToSFixture {
    i: f64,
    j: f64,
    resolution: usize,
    orientation: String,
    s: u64,
}

fn load() -> Fixtures {
    let content = fs::read_to_string("tests/fixtures/lattice/lsystem.json")
        .expect("Could not read lsystem fixtures");
    serde_json::from_str(&content).expect("Could not parse lsystem fixtures")
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
fn test_triple_to_s_lattice() {
    for f in &load().s_to_cell {
        let triple = Triple::new(f.x, f.y, f.z);
        let s = triple_to_s_lattice(&triple, f.resolution, ori(&f.orientation));
        assert_eq!(
            s, f.s,
            "s for ({},{},{}) res={}",
            f.x, f.y, f.z, f.resolution
        );
    }
}

#[test]
fn test_point_to_s() {
    for f in &load().point_to_s {
        let s = triple_to_s_lattice(
            &round_to_triple(IJ::new(f.i, f.j), f.resolution),
            f.resolution,
            ori(&f.orientation),
        );
        assert_eq!(s, f.s, "s for ({},{}) res={}", f.i, f.j, f.resolution);
    }
}
