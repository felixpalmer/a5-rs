// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use a5::coordinate_systems::IJ;
use a5::lattice::{ij_to_quaternary, quaternary_to_flips, quaternary_to_kj};
use serde::Deserialize;

#[derive(Deserialize)]
struct IJToQuaternaryFixture {
    ij: [f64; 2],
    flips: [i8; 2],
    digit: u8,
}

#[derive(Deserialize)]
struct QuaternaryToKJFixture {
    q: u8,
    flips: [i8; 2],
    kj: [f64; 2],
}

#[derive(Deserialize)]
struct QuaternaryToFlipsFixture {
    q: u8,
    flips: [i8; 2],
}

#[derive(Deserialize)]
struct Fixtures {
    #[serde(rename = "IJToQuaternary")]
    ij_to_quaternary: Vec<IJToQuaternaryFixture>,
    #[serde(rename = "quaternaryToKJ")]
    quaternary_to_kj: Vec<QuaternaryToKJFixture>,
    #[serde(rename = "quaternaryToFlips")]
    quaternary_to_flips: Vec<QuaternaryToFlipsFixture>,
}

fn load_fixtures() -> Fixtures {
    let data = include_str!("fixtures/lattice/quaternary.json");
    serde_json::from_str(data).expect("Failed to parse quaternary.json")
}

#[test]
fn test_ij_to_quaternary() {
    let fixtures = load_fixtures();
    for f in &fixtures.ij_to_quaternary {
        let ij = IJ::new(f.ij[0], f.ij[1]);
        let result = ij_to_quaternary(ij, f.flips);
        assert_eq!(result, f.digit, "ij={:?} flips={:?}", f.ij, f.flips);
    }
}

#[test]
fn test_quaternary_to_kj() {
    let fixtures = load_fixtures();
    for f in &fixtures.quaternary_to_kj {
        let result = quaternary_to_kj(f.q, f.flips);
        assert_eq!(result.x(), f.kj[0], "n={} flips={:?} kj[0]", f.q, f.flips);
        assert_eq!(result.y(), f.kj[1], "n={} flips={:?} kj[1]", f.q, f.flips);
    }
}

#[test]
fn test_quaternary_to_flips() {
    let fixtures = load_fixtures();
    for f in &fixtures.quaternary_to_flips {
        let result = quaternary_to_flips(f.q);
        assert_eq!(result, f.flips, "n={}", f.q);
    }
}
