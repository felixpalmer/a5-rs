// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use a5::collections::compact::{compact, uncompact};
use a5::collections::resolution::covering_resolution;
use a5::core::hex::hex_to_u64;
use a5::core::serialization::get_resolution;
use serde::Deserialize;
use std::fs;

#[derive(Deserialize)]
struct CompactTestCase {
    name: String,
    input: Vec<String>,
    #[serde(rename = "expectedOutput")]
    expected_output: Vec<String>,
}

#[derive(Deserialize)]
struct UncompactTestCase {
    name: String,
    input: Vec<String>,
    #[serde(rename = "expectedResolution")]
    expected_resolution: i32,
    #[serde(rename = "expectedCount")]
    expected_count: usize,
    #[serde(rename = "expectedCells")]
    expected_cells: Vec<String>,
}

#[derive(Deserialize)]
struct RoundTripTestCase {
    name: String,
    #[serde(rename = "initialCells")]
    initial_cells: Vec<String>,
    #[serde(rename = "afterCompact")]
    after_compact: Vec<String>,
    resolution: i32,
    #[serde(rename = "expectedFinalCount")]
    expected_final_count: usize,
}

#[derive(Deserialize)]
struct CompactFixtures {
    compact: Vec<CompactTestCase>,
    uncompact: Vec<UncompactTestCase>,
    #[serde(rename = "roundTrip")]
    round_trip: Vec<RoundTripTestCase>,
}

fn load_fixtures() -> CompactFixtures {
    let data = fs::read_to_string("tests/fixtures/compact.json").expect("read compact.json");
    serde_json::from_str(&data).expect("parse compact.json")
}

fn to_cells(hex: &[String]) -> Vec<u64> {
    hex.iter().map(|h| hex_to_u64(h).unwrap()).collect()
}

#[test]
fn test_uncompact_all_fixtures() {
    for test_case in load_fixtures().uncompact {
        let input = to_cells(&test_case.input);
        let result = uncompact(&input).unwrap();

        assert_eq!(result.len(), test_case.expected_count, "{}", test_case.name);
        assert_eq!(
            result,
            to_cells(&test_case.expected_cells),
            "{}",
            test_case.name
        );
        assert_eq!(
            covering_resolution(&input),
            test_case.expected_resolution,
            "{}",
            test_case.name
        );

        // All results should be at the covering's resolution
        for &cell in &result {
            assert_eq!(
                get_resolution(cell),
                test_case.expected_resolution,
                "{}",
                test_case.name
            );
        }
    }
}

#[test]
fn test_compact_all_fixtures() {
    for test_case in load_fixtures().compact {
        // Output is canonical: cells in curve order, then the compaction marker
        assert_eq!(
            compact(&to_cells(&test_case.input)).unwrap(),
            to_cells(&test_case.expected_output),
            "{}",
            test_case.name
        );
    }
}

#[test]
fn test_roundtrip_all_fixtures() {
    for test_case in load_fixtures().round_trip {
        let initial_cells = to_cells(&test_case.initial_cells);
        let after_compact = to_cells(&test_case.after_compact);

        // Verify compact result matches fixture
        assert_eq!(
            compact(&initial_cells).unwrap(),
            after_compact,
            "{}",
            test_case.name
        );

        // Verify uncompact restores coverage, at the resolution the compaction marker records
        let result = uncompact(&after_compact).unwrap();
        assert_eq!(
            result.len(),
            test_case.expected_final_count,
            "{}",
            test_case.name
        );
        for &cell in &result {
            assert_eq!(
                get_resolution(cell),
                test_case.resolution,
                "{}",
                test_case.name
            );
        }
    }
}
