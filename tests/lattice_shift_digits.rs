// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use a5::lattice::shift_digits;
use a5::lattice::shift_digits::{
    PATTERN, PATTERN_FLIPPED, PATTERN_FLIPPED_REVERSED, PATTERN_REVERSED,
};
use serde::Deserialize;

#[derive(Deserialize)]
struct ShiftDigitsFixture {
    #[serde(rename = "digitsBefore")]
    digits_before: Vec<u8>,
    i: usize,
    flips: [i8; 2],
    #[serde(rename = "invertJ")]
    invert_j: bool,
    #[serde(rename = "patternName")]
    pattern_name: String,
    #[serde(rename = "digitsAfter")]
    digits_after: Vec<u8>,
}

#[derive(Deserialize)]
struct Fixtures {
    #[serde(rename = "shiftDigits")]
    shift_digits: Vec<ShiftDigitsFixture>,
}

fn load_fixtures() -> Fixtures {
    let data = include_str!("fixtures/lattice/shift-digits.json");
    serde_json::from_str(data).expect("Failed to parse shift-digits.json")
}

#[test]
fn test_shift_digits() {
    let fixtures = load_fixtures();
    for f in &fixtures.shift_digits {
        let pattern: &[usize] = match f.pattern_name.as_str() {
            "PATTERN" => &PATTERN,
            "PATTERN_FLIPPED" => &PATTERN_FLIPPED,
            "PATTERN_REVERSED" => &PATTERN_REVERSED,
            "PATTERN_FLIPPED_REVERSED" => &PATTERN_FLIPPED_REVERSED,
            _ => panic!("Unknown pattern: {}", f.pattern_name),
        };
        let mut digits = f.digits_before.clone();
        shift_digits(&mut digits, f.i, f.flips, f.invert_j, pattern);
        assert_eq!(
            digits, f.digits_after,
            "digits_before={:?} i={} flips={:?} invert_j={} pattern={}",
            f.digits_before, f.i, f.flips, f.invert_j, f.pattern_name
        );
    }
}
