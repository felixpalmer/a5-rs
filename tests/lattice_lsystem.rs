// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use a5::coordinate_systems::IJ;
use a5::lattice::curve::round_to_triple;
use a5::lattice::lsystem::{
    curve_child, s_to_cell, s_to_triple, triple_to_curve_node, triple_to_s_lattice, CurveNode,
};
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

const ORIENTATIONS: [&str; 6] = ["uv", "vu", "uw", "wu", "vw", "wv"];

fn root(orientation: Orientation) -> CurveNode {
    triple_to_curve_node(&Triple::new(0, 0, 0), 0, orientation).2
}

/// Step to the child with `digit`, checking it against s_to_cell, and the
/// descent state against triple_to_curve_node's from the child's triple.
fn step(
    node: &CurveNode,
    s: u64,
    digit: usize,
    resolution: usize,
    orientation: Orientation,
) -> (CurveNode, u64) {
    let (cell, below) = curve_child(node, digit, resolution, orientation);
    let child_s = s * 4 + digit as u64;
    assert_eq!(cell, s_to_cell(child_s, resolution, orientation));
    assert_eq!(
        triple_to_curve_node(&cell.triple, resolution, orientation),
        (child_s, cell.flavor, below)
    );
    (below, child_s)
}

#[test]
fn test_curve_child_agrees_with_s_to_cell_and_triple_to_curve_node() {
    for name in ORIENTATIONS {
        let orientation = ori(name);
        // Every cell through level 3, then one deep path to level 29 (A5 resolution 30)
        let mut stack: Vec<(CurveNode, u64, usize)> = vec![(root(orientation), 0, 0)];
        while let Some((node, s, resolution)) = stack.pop() {
            if resolution == 3 {
                continue;
            }
            for digit in 0..4 {
                let (child, child_s) = step(&node, s, digit, resolution + 1, orientation);
                stack.push((child, child_s, resolution + 1));
            }
        }
        // Same digit sequence as the TypeScript test (char code of the orientation's first letter)
        let first = name.as_bytes()[0] as usize;
        let (mut node, mut s) = (root(orientation), 0u64);
        for resolution in 1..=29 {
            let digit = (resolution * 7 + first) % 4;
            (node, s) = step(&node, s, digit, resolution, orientation);
        }
    }
}
