// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

// The compat curve reproduces the ORIGINAL (pre-L-system) A5 curve bit-for-bit;
// these fixtures pin it against regressions.

use a5::coordinate_systems::IJ;
use a5::lattice::{compat_ij_to_s, compat_s_to_cell, compat_s_to_triple, compat_triple_to_s};
use a5::lattice::{Orientation, Triple};
use serde::Deserialize;
use std::fs;
use std::str::FromStr;

#[derive(Deserialize)]
struct Fixtures {
    #[serde(rename = "sToCell")]
    s_to_cell: Vec<CompatFixture>,
    #[serde(rename = "IJToS")]
    ij_to_s: Vec<IJToSFixture>,
}

#[derive(Deserialize)]
struct CompatFixture {
    s: u64,
    resolution: usize,
    orientation: String,
    x: i32,
    y: i32,
    z: i32,
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

fn load() -> Fixtures {
    let content = fs::read_to_string("tests/fixtures/lattice/compat.json")
        .expect("Could not read compat fixtures");
    serde_json::from_str(&content).expect("Could not parse compat fixtures")
}

fn ori(s: &str) -> Orientation {
    Orientation::from_str(s).unwrap()
}

#[test]
fn test_compat_s_to_cell() {
    for f in &load().s_to_cell {
        let cell = compat_s_to_cell(f.s, f.resolution, ori(&f.orientation));
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
fn test_compat_s_to_triple() {
    for f in &load().s_to_cell {
        let triple = compat_s_to_triple(f.s, f.resolution, ori(&f.orientation));
        assert_eq!(triple, Triple::new(f.x, f.y, f.z));
    }
}

#[test]
fn test_compat_triple_to_s() {
    for f in &load().s_to_cell {
        let triple = Triple::new(f.x, f.y, f.z);
        let s = compat_triple_to_s(&triple, f.resolution, ori(&f.orientation));
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
fn test_compat_ij_to_s() {
    for f in &load().ij_to_s {
        let s = compat_ij_to_s(IJ::new(f.i, f.j), f.resolution, ori(&f.orientation));
        assert_eq!(s, f.s, "s for ({},{}) res={}", f.i, f.j, f.resolution);
    }
}
