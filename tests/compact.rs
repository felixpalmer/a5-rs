// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use a5::core::compact::{compact, uncompact};
use a5::core::hex::hex_to_u64;
use a5::core::serialization::deserialize;
use serde::Deserialize;
use std::fs;

#[derive(Deserialize)]
struct CompactTestCase {
    name: String,
    #[allow(dead_code)]
    description: String,
    input: Vec<String>,
    #[serde(rename = "expectedOutput")]
    expected_output: Vec<String>,
}

#[derive(Deserialize)]
struct UncompactTestCase {
    name: String,
    #[allow(dead_code)]
    description: String,
    input: Vec<String>,
    #[serde(rename = "targetResolution")]
    target_resolution: i32,
    #[serde(rename = "expectedCount")]
    expected_count: Option<usize>,
    #[serde(rename = "expectedError")]
    expected_error: Option<bool>,
}

#[derive(Deserialize)]
struct RoundTripTestCase {
    name: String,
    #[allow(dead_code)]
    description: String,
    #[serde(rename = "initialCells")]
    initial_cells: Vec<String>,
    #[serde(rename = "afterCompact")]
    after_compact: Vec<String>,
    #[serde(rename = "targetResolution")]
    target_resolution: i32,
    #[serde(rename = "expectedCount")]
    expected_count: Option<usize>,
    #[serde(rename = "expectedFinalCount")]
    expected_final_count: Option<usize>,
}

#[derive(Deserialize)]
struct CompactFixtures {
    compact: Vec<CompactTestCase>,
    uncompact: Vec<UncompactTestCase>,
    #[serde(rename = "roundTrip")]
    round_trip: Vec<RoundTripTestCase>,
}

fn load_fixtures() -> CompactFixtures {
    let fixtures_path = "tests/fixtures/compact.json";
    let fixtures_data = fs::read_to_string(fixtures_path)
        .unwrap_or_else(|_| panic!("Failed to read fixtures file: {}", fixtures_path));
    serde_json::from_str(&fixtures_data).unwrap_or_else(|_| panic!("Failed to parse fixtures JSON"))
}

#[test]
fn test_uncompact_all_fixtures() {
    let fixtures = load_fixtures();

    for test_case in fixtures.uncompact {
        // Skip error test cases - handle separately
        if test_case.expected_error.unwrap_or(false) {
            continue;
        }

        let input_cells: Vec<u64> = test_case
            .input
            .iter()
            .map(|h| hex_to_u64(h).unwrap())
            .collect();

        let result = uncompact(&input_cells, test_case.target_resolution)
            .unwrap_or_else(|_| panic!("Failed test case: {}", test_case.name));

        if let Some(expected_count) = test_case.expected_count {
            assert_eq!(
                result.len(),
                expected_count,
                "Failed test case: {}",
                test_case.name
            );
        }

        // All results should be at target resolution
        for &cell in &result {
            let cell_data = deserialize(cell).unwrap_or_else(|_| {
                panic!("Failed to deserialize cell in test: {}", test_case.name)
            });
            assert_eq!(
                cell_data.resolution, test_case.target_resolution,
                "Failed test case: {}",
                test_case.name
            );
        }
    }
}

#[test]
fn test_uncompact_error_on_lower_resolution() {
    let fixtures = load_fixtures();

    let error_cases: Vec<_> = fixtures
        .uncompact
        .iter()
        .filter(|tc| tc.expected_error.unwrap_or(false))
        .collect();

    if let Some(error_case) = error_cases.first() {
        let input_cells: Vec<u64> = error_case
            .input
            .iter()
            .map(|h| hex_to_u64(h).unwrap())
            .collect();

        let result = uncompact(&input_cells, error_case.target_resolution);
        assert!(
            result.is_err(),
            "Expected error for test case: {}",
            error_case.name
        );
    }
}

#[test]
fn test_compact_all_fixtures() {
    let fixtures = load_fixtures();

    for test_case in fixtures.compact {
        let input_cells: Vec<u64> = test_case
            .input
            .iter()
            .map(|h| hex_to_u64(h).unwrap())
            .collect();

        let mut expected: Vec<u64> = test_case
            .expected_output
            .iter()
            .map(|h| hex_to_u64(h).unwrap())
            .collect();
        expected.sort_unstable();

        let mut result = compact(&input_cells)
            .unwrap_or_else(|_| panic!("Failed test case: {}", test_case.name));
        result.sort_unstable();

        assert_eq!(result, expected, "Failed test case: {}", test_case.name);
    }
}

#[test]
fn test_roundtrip_all_fixtures() {
    let fixtures = load_fixtures();

    for test_case in fixtures.round_trip {
        let initial_cells: Vec<u64> = test_case
            .initial_cells
            .iter()
            .map(|h| hex_to_u64(h).unwrap())
            .collect();

        let mut after_compact_expected: Vec<u64> = test_case
            .after_compact
            .iter()
            .map(|h| hex_to_u64(h).unwrap())
            .collect();
        after_compact_expected.sort_unstable();

        // Verify compact result matches fixture
        let mut compact_result = compact(&initial_cells)
            .unwrap_or_else(|_| panic!("Failed compact in test case: {}", test_case.name));
        compact_result.sort_unstable();

        assert_eq!(
            compact_result, after_compact_expected,
            "Failed compact in test case: {}",
            test_case.name
        );

        // Verify uncompact restores coverage
        let uncompact_result = uncompact(&after_compact_expected, test_case.target_resolution)
            .unwrap_or_else(|_| panic!("Failed uncompact in test case: {}", test_case.name));

        if let Some(expected_count) = test_case.expected_count {
            assert_eq!(
                uncompact_result.len(),
                expected_count,
                "Failed uncompact count in test case: {}",
                test_case.name
            );
        }

        if let Some(expected_final_count) = test_case.expected_final_count {
            assert_eq!(
                uncompact_result.len(),
                expected_final_count,
                "Failed final count in test case: {}",
                test_case.name
            );
        }

        // All results should be at target resolution
        for &cell in &uncompact_result {
            let cell_data = deserialize(cell).unwrap_or_else(|_| {
                panic!(
                    "Failed to deserialize cell in test case: {}",
                    test_case.name
                )
            });
            assert_eq!(
                cell_data.resolution, test_case.target_resolution,
                "Failed resolution check in test case: {}",
                test_case.name
            );
        }
    }
}
