// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use a5::lattice::{
    anchor_to_triple, s_to_anchor, triple_in_bounds, triple_parity, triple_to_s, Orientation,
    Triple,
};
use serde::Deserialize;

#[derive(Deserialize)]
struct AnchorToTripleFixture {
    s: u64,
    resolution: usize,
    orientation: String,
    x: i32,
    y: i32,
    z: i32,
    parity: i32,
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

#[derive(Deserialize)]
struct Fixtures {
    #[serde(rename = "anchorToTriple")]
    anchor_to_triple: Vec<AnchorToTripleFixture>,
    #[serde(rename = "tripleInBounds")]
    triple_in_bounds: Vec<TripleInBoundsFixture>,
}

fn load_fixtures() -> Fixtures {
    let data = include_str!("fixtures/lattice/triple.json");
    serde_json::from_str(data).expect("Failed to parse triple.json")
}

#[test]
fn test_anchor_to_triple() {
    let fixtures = load_fixtures();
    for f in &fixtures.anchor_to_triple {
        let orientation = f.orientation.parse::<Orientation>().unwrap();
        let anchor = s_to_anchor(f.s, f.resolution, orientation);
        let triple = anchor_to_triple(&anchor);
        assert_eq!(
            triple.x, f.x,
            "x for s={} res={} ori={}",
            f.s, f.resolution, f.orientation
        );
        assert_eq!(
            triple.y, f.y,
            "y for s={} res={} ori={}",
            f.s, f.resolution, f.orientation
        );
        assert_eq!(
            triple.z, f.z,
            "z for s={} res={} ori={}",
            f.s, f.resolution, f.orientation
        );
        assert_eq!(
            triple_parity(&triple),
            f.parity,
            "parity for s={} res={} ori={}",
            f.s,
            f.resolution,
            f.orientation
        );
    }
}

#[test]
fn test_triple_to_s_roundtrip() {
    let fixtures = load_fixtures();
    for f in &fixtures.anchor_to_triple {
        let orientation = f.orientation.parse::<Orientation>().unwrap();
        let triple = Triple::new(f.x, f.y, f.z);
        let s = triple_to_s(&triple, f.resolution, orientation);
        assert_eq!(
            s,
            Some(f.s),
            "triple_to_s for ({},{},{}) res={} ori={}",
            f.x,
            f.y,
            f.z,
            f.resolution,
            f.orientation
        );
    }
}

#[test]
fn test_triple_in_bounds() {
    let fixtures = load_fixtures();
    for f in &fixtures.triple_in_bounds {
        let triple = Triple::new(f.x, f.y, f.z);
        let result = triple_in_bounds(&triple, f.max_row);
        assert_eq!(
            result, f.expected,
            "triple_in_bounds({},{},{}) maxRow={}",
            f.x, f.y, f.z, f.max_row
        );
    }
}
