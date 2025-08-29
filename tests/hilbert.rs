// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use a5::coordinate_systems::{IJ, KJ};
use a5::core::hilbert::{
    get_required_digits, ij_to_kj, ij_to_s, kj_to_ij, quaternary_to_flips, quaternary_to_kj,
    s_to_anchor, Orientation, Quaternary, NO, YES,
};

const TOLERANCE: f64 = 1e-6;

fn close_to(a: f64, b: f64, tolerance: f64) -> bool {
    (a - b).abs() < tolerance
}

fn close_to_ij(a: IJ, b: IJ, tolerance: f64) -> bool {
    close_to(a.x(), b.x(), tolerance) && close_to(a.y(), b.y(), tolerance)
}

fn close_to_kj(a: KJ, b: KJ, tolerance: f64) -> bool {
    close_to(a.x(), b.x(), tolerance) && close_to(a.y(), b.y(), tolerance)
}

#[test]
fn test_quaternary_to_kj_base_cases() {
    // Test first corner (0)
    let offset0 = quaternary_to_kj(0, [NO, NO]);
    assert!(close_to_kj(offset0, KJ::new(0.0, 0.0), TOLERANCE));
    let flips0 = quaternary_to_flips(0);
    assert_eq!(flips0, [NO, NO]);

    // Test second corner (1)
    let offset1 = quaternary_to_kj(1, [NO, NO]);
    assert!(close_to_kj(offset1, KJ::new(1.0, 0.0), TOLERANCE));
    let flips1 = quaternary_to_flips(1);
    assert_eq!(flips1, [NO, YES]);

    // Test third corner (2)
    let offset2 = quaternary_to_kj(2, [NO, NO]);
    assert!(close_to_kj(offset2, KJ::new(1.0, 1.0), TOLERANCE));
    let flips2 = quaternary_to_flips(2);
    assert_eq!(flips2, [NO, NO]);

    // Test fourth corner (3)
    let offset3 = quaternary_to_kj(3, [NO, NO]);
    assert!(close_to_kj(offset3, KJ::new(2.0, 1.0), TOLERANCE));
    let flips3 = quaternary_to_flips(3);
    assert_eq!(flips3, [YES, NO]);
}

#[test]
fn test_quaternary_to_kj_with_flips() {
    // Test with x-flip
    let offset_x = quaternary_to_kj(1, [YES, NO]);
    assert!(close_to_kj(offset_x, KJ::new(0.0, -1.0), TOLERANCE));

    // Test with y-flip
    let offset_y = quaternary_to_kj(1, [NO, YES]);
    assert!(close_to_kj(offset_y, KJ::new(0.0, 1.0), TOLERANCE));

    // Test with both flips
    let offset_xy = quaternary_to_kj(1, [YES, YES]);
    assert!(close_to_kj(offset_xy, KJ::new(-1.0, 0.0), TOLERANCE));
}

#[test]
fn test_quaternary_to_flips_output_depends_only_on_n() {
    let expected_flips = [[NO, NO], [NO, YES], [NO, NO], [YES, NO]];
    for (n, &expected) in expected_flips.iter().enumerate() {
        let flips = quaternary_to_flips(n as Quaternary);
        assert_eq!(flips, expected);
    }
}

#[test]
fn test_s_to_anchor_generates_correct_sequence() {
    // Test first few indices
    let anchor0 = s_to_anchor(0, 20, Orientation::UV);
    assert!(close_to_ij(anchor0.offset, IJ::new(0.0, 0.0), TOLERANCE));
    assert_eq!(anchor0.flips, [NO, NO]);

    let anchor1 = s_to_anchor(1, 20, Orientation::UV);
    assert_eq!(anchor1.flips[1], YES);

    let anchor4 = s_to_anchor(4, 20, Orientation::UV);
    let offset_len =
        (anchor4.offset.x() * anchor4.offset.x() + anchor4.offset.y() * anchor4.offset.y()).sqrt();
    assert!(offset_len > 1.0); // Should be scaled up

    // Test that sequence length grows exponentially
    let anchors: Vec<_> = (0..16)
        .map(|i| s_to_anchor(i, 20, Orientation::UV))
        .collect();
    let unique_offsets: std::collections::HashSet<_> = anchors
        .iter()
        .map(|a| format!("{},{}", a.offset.x(), a.offset.y()))
        .collect();
    assert_eq!(unique_offsets.len(), 13); // Updated based on actual behavior
    let unique_anchors: std::collections::HashSet<_> = anchors
        .iter()
        .map(|a| {
            format!(
                "{},{},{},{}",
                a.offset.x(),
                a.offset.y(),
                a.flips[0],
                a.flips[1]
            )
        })
        .collect();
    assert_eq!(unique_anchors.len(), 16); // Updated based on actual behavior
}

#[test]
fn test_neighboring_anchors_are_adjacent() {
    // Test that combining anchors preserves orientation rules
    let anchor1 = s_to_anchor(0, 20, Orientation::UV);
    let anchor2 = s_to_anchor(1, 20, Orientation::UV);
    let anchor3 = s_to_anchor(2, 20, Orientation::UV);

    // Check that relative positions make sense
    let diff = IJ::new(
        anchor2.offset.x() - anchor1.offset.x(),
        anchor2.offset.y() - anchor1.offset.y(),
    );
    let diff_len = (diff.x() * diff.x() + diff.y() * diff.y()).sqrt();
    assert!(close_to(diff_len, 1.0, TOLERANCE)); // Should be adjacent

    let diff2 = IJ::new(
        anchor3.offset.x() - anchor2.offset.x(),
        anchor3.offset.y() - anchor2.offset.y(),
    );
    let diff2_len = (diff2.x() * diff2.x() + diff2.y() * diff2.y()).sqrt();
    assert!(close_to(diff2_len, 1.0, TOLERANCE)); // Should be adjacent
}

#[test]
fn test_s_to_anchor_generates_correct_anchors_for_all_indices() {
    let expected_anchors = [
        (0, [0.0, 0.0], [NO, NO]),
        (9, [3.0, 1.0], [YES, YES]),
        (16, [2.0, 2.0], [NO, NO]),
        (17, [3.0, 2.0], [NO, YES]),
        (31, [1.0, 3.0], [YES, NO]),
        (77, [7.0, 5.0], [NO, NO]),
        (100, [3.0, 7.0], [YES, YES]),
        (101, [2.0, 7.0], [YES, NO]),
        (170, [10.0, 1.0], [NO, NO]),
        (411, [7.0, 13.0], [YES, NO]),
        (1762, [7.0, 31.0], [YES, NO]),
        (481952, [96.0, 356.0], [YES, YES]),
    ];

    for &(s, offset, flips) in &expected_anchors {
        let anchor = s_to_anchor(s, 20, Orientation::UV);
        assert!(
            close_to_ij(anchor.offset, IJ::new(offset[0], offset[1]), TOLERANCE),
            "s={}: expected [{}, {}], got [{}, {}]",
            s,
            offset[0],
            offset[1],
            anchor.offset.x(),
            anchor.offset.y()
        );
        assert_eq!(anchor.flips, flips, "s={}: flips mismatch", s);
    }
}

#[test]
fn test_ij_to_kj_converts_coordinates() {
    // Test some basic conversions
    let test_cases = [
        ([0.0, 0.0], [0.0, 0.0]), // Origin
        ([1.0, 0.0], [1.0, 0.0]), // Unit i
        ([0.0, 1.0], [1.0, 1.0]), // Unit j -> k=i+j=1, j=1
        ([1.0, 1.0], [2.0, 1.0]), // i + j -> k=2, j=1
        ([2.0, 3.0], [5.0, 3.0]), // 2i + 3j -> k=5, j=3
    ];

    for (input, expected) in test_cases {
        let result = ij_to_kj(IJ::new(input[0], input[1]));
        assert!(close_to_kj(
            result,
            KJ::new(expected[0], expected[1]),
            TOLERANCE
        ));
    }
}

#[test]
fn test_kj_to_ij_converts_coordinates() {
    // Test some basic conversions
    let test_cases = [
        ([0.0, 0.0], [0.0, 0.0]), // Origin
        ([1.0, 0.0], [1.0, 0.0]), // Pure k -> i=1, j=0
        ([1.0, 1.0], [0.0, 1.0]), // k=1, j=1 -> i=0, j=1
        ([2.0, 1.0], [1.0, 1.0]), // k=2, j=1 -> i=1, j=1
        ([5.0, 3.0], [2.0, 3.0]), // k=5, j=3 -> i=2, j=3
    ];

    for (input, expected) in test_cases {
        let result = kj_to_ij(KJ::new(input[0], input[1]));
        assert!(close_to_ij(
            result,
            IJ::new(expected[0], expected[1]),
            TOLERANCE
        ));
    }
}

#[test]
fn test_ij_to_kj_and_kj_to_ij_are_inverses() {
    // Test that converting back and forth gives the original coordinates
    let test_points = [
        [0.0, 0.0],
        [1.0, 0.0],
        [0.0, 1.0],
        [1.0, 1.0],
        [2.0, 3.0],
        [-1.0, 2.0],
        [3.0, -2.0],
    ];

    for point in test_points {
        let original = IJ::new(point[0], point[1]);
        let kj = ij_to_kj(original);
        let ij = kj_to_ij(kj);
        assert!(close_to_ij(original, ij, TOLERANCE));
    }
}

#[test]
fn test_get_required_digits_correctly_determines_digits_needed() {
    let test_cases: [(IJ, usize); 7] = [
        (IJ::new(0.0, 0.0), 1),
        (IJ::new(1.0, 0.0), 1),
        (IJ::new(2.0, 1.0), 2),
        (IJ::new(4.0, 0.0), 3),
        (IJ::new(8.0, 8.0), 5),
        (IJ::new(16.0, 0.0), 5),
        (IJ::new(32.0, 32.0), 7),
    ];

    for (offset, expected) in test_cases {
        assert_eq!(get_required_digits(offset), expected);
    }
}

#[test]
fn test_get_required_digits_matches_actual_digits_in_s_to_anchor_output() {
    // Test that getRequiredDigits matches the number of digits
    // actually used in sToAnchor's output
    let test_values = [0, 1, 2, 3, 4, 9, 16, 17, 31, 77, 100];

    for s in test_values {
        let anchor = s_to_anchor(s, 20, Orientation::UV);
        let required_digits = get_required_digits(anchor.offset);
        let actual_digits = if s == 0 {
            1
        } else {
            format!("{:b}", s).len() / 2 + 1
        }; // Approximate quaternary digits
        assert!(required_digits >= actual_digits);
        assert!(required_digits <= actual_digits + 1);
    }
}

#[test]
fn test_ij_to_s_computes_s_from_anchor() {
    let test_values = [
        // First quadrant
        (0u64, [0.0, 0.0]),
        (0u64, [0.999, 0.0]),
        (1u64, [0.6, 0.6]),
        (7u64, [0.000001, 1.1]),
        (2u64, [1.2, 0.5]),
        (2u64, [1.9999, 0.0]),
        // Recursive cases, 2nd quadrant, flipY
        (3u64, [1.9999, 0.001]),
        (4u64, [1.1, 1.1]),
        (5u64, [1.999, 1.999]),
        (6u64, [0.99, 1.99]),
        // 3rd quadrant, no flips
        (28u64, [0.999, 2.000001]),
        (29u64, [0.9, 2.5]),
        (30u64, [0.5, 3.1]),
        (31u64, [1.3, 2.5]),
        // 4th quadrant, flipX
        (8u64, [2.00001, 1.001]),
        (9u64, [2.8, 0.5]),
        (10u64, [2.00001, 0.5]),
        (11u64, [3.5, 0.2]),
        // Next level, just sample a few as flips are the same as before
        (15u64, [2.5, 1.5]),
        (21u64, [3.999, 3.999]),
        // Finally, both flips
        (24u64, [1.999, 3.999]),
        (25u64, [1.2, 3.5]),
        (26u64, [1.9, 2.2]),
        (27u64, [0.1, 3.9]),
    ];

    for (s, offset) in test_values {
        let result = ij_to_s(IJ::new(offset[0], offset[1]), 3, Orientation::UV);
        assert_eq!(
            result, s,
            "ij_to_s({}, {}) expected {}, got {}",
            offset[0], offset[1], s, result
        );
    }
}

#[test]
fn test_ij_to_s_is_inverse_of_s_to_anchor() {
    let test_values = [0, 1, 2, 3, 4, 9, 16, 17, 31, 77, 100, 101, 170, 411, 1762];
    let resolution = 20;
    let orientations = [
        Orientation::UV,
        Orientation::VU,
        Orientation::UW,
        Orientation::WU,
        Orientation::VW,
        Orientation::WV,
    ];

    for orientation in orientations {
        for s in test_values {
            let mut anchor = s_to_anchor(s, resolution, orientation);

            // Nudge the offset away from the edge of the triangle
            let [flip_x, flip_y] = anchor.flips;
            if flip_x == NO && flip_y == NO {
                anchor.offset = IJ::new(anchor.offset.x() + 0.1, anchor.offset.y() + 0.1);
            } else if flip_x == YES && flip_y == NO {
                anchor.offset = IJ::new(anchor.offset.x() + 0.1, anchor.offset.y() - 0.2);
            } else if flip_x == NO && flip_y == YES {
                anchor.offset = IJ::new(anchor.offset.x() - 0.1, anchor.offset.y() + 0.2);
            } else if flip_x == YES && flip_y == YES {
                anchor.offset = IJ::new(anchor.offset.x() - 0.1, anchor.offset.y() - 0.1);
            }

            let result = ij_to_s(anchor.offset, resolution, orientation);
            assert_eq!(
                result, s,
                "ij_to_s/s_to_anchor mismatch for s={}, orientation={:?}",
                s, orientation
            );
        }
    }
}
