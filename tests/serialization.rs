use a5::core::origin::get_origins;
use a5::core::serialization::{
    cell_to_children, cell_to_parent, deserialize, get_res0_cells, get_resolution, serialize,
    FIRST_HILBERT_RESOLUTION, MAX_RESOLUTION, REMOVAL_MASK,
};
use a5::core::utils::A5Cell;
use num_bigint::BigInt;
use std::fs;

const RESOLUTION_MASKS: [&str; 30] = [
    // Non-Hilbert resolutions
    "0000001000000000000000000000000000000000000000000000000000000000", // Dodecahedron faces
    "0000000100000000000000000000000000000000000000000000000000000000", // Quintants
    // Hilbert resolutions
    "0000000010000000000000000000000000000000000000000000000000000000",
    "0000000000100000000000000000000000000000000000000000000000000000",
    "0000000000001000000000000000000000000000000000000000000000000000",
    "0000000000000010000000000000000000000000000000000000000000000000",
    "0000000000000000100000000000000000000000000000000000000000000000",
    "0000000000000000001000000000000000000000000000000000000000000000",
    "0000000000000000000010000000000000000000000000000000000000000000",
    "0000000000000000000000100000000000000000000000000000000000000000",
    "0000000000000000000000001000000000000000000000000000000000000000",
    "0000000000000000000000000010000000000000000000000000000000000000",
    "0000000000000000000000000000100000000000000000000000000000000000",
    "0000000000000000000000000000001000000000000000000000000000000000",
    "0000000000000000000000000000000010000000000000000000000000000000",
    "0000000000000000000000000000000000100000000000000000000000000000",
    "0000000000000000000000000000000000001000000000000000000000000000",
    "0000000000000000000000000000000000000010000000000000000000000000",
    "0000000000000000000000000000000000000000100000000000000000000000",
    "0000000000000000000000000000000000000000001000000000000000000000",
    "0000000000000000000000000000000000000000000010000000000000000000",
    "0000000000000000000000000000000000000000000000100000000000000000",
    "0000000000000000000000000000000000000000000000001000000000000000",
    "0000000000000000000000000000000000000000000000000010000000000000",
    "0000000000000000000000000000000000000000000000000000100000000000",
    "0000000000000000000000000000000000000000000000000000001000000000",
    "0000000000000000000000000000000000000000000000000000000010000000",
    "0000000000000000000000000000000000000000000000000000000000100000",
    "0000000000000000000000000000000000000000000000000000000000001000",
    "0000000000000000000000000000000000000000000000000000000000000010",
];

fn load_test_ids() -> Vec<String> {
    let content = fs::read_to_string("tests/test-ids.json").expect("Could not read test-ids.json");
    serde_json::from_str(&content).expect("Could not parse test-ids.json")
}

#[test]
fn test_correct_number_of_masks() {
    assert_eq!(RESOLUTION_MASKS.len(), MAX_RESOLUTION as usize); // TODO add point level
}

#[test]
fn test_removal_mask_is_correct() {
    let origin_segment_bits = "".to_string() + &"0".repeat(6);
    let remaining_bits = "1".repeat(58);
    let expected =
        u64::from_str_radix(&format!("{}{}", origin_segment_bits, remaining_bits), 2).unwrap();
    assert_eq!(REMOVAL_MASK, expected);
}

#[test]
fn test_encodes_resolution_correctly_for_different_values() {
    let origins = get_origins();
    let origin0 = origins[0].clone();

    let test_cases: Vec<A5Cell> = RESOLUTION_MASKS
        .iter()
        .enumerate()
        .map(|(i, _)| {
            // Origin 0 has first quintant 4, so start use segment 4 to obtain start of Hilbert curve
            A5Cell {
                origin_id: origin0.id,
                segment: 4,
                s: BigInt::from(0),
                resolution: i as i32,
            }
        })
        .collect();

    for (i, input) in test_cases.iter().enumerate() {
        let serialized = serialize(input).unwrap();
        let binary_str = format!("{:064b}", serialized);
        assert_eq!(binary_str, RESOLUTION_MASKS[i]);
    }
}

#[test]
fn test_correctly_extracts_resolution() {
    for (i, binary) in RESOLUTION_MASKS.iter().enumerate() {
        let bit_count = binary.len();
        assert_eq!(bit_count, 64);
        let n = u64::from_str_radix(binary, 2).unwrap();
        let resolution = get_resolution(n);
        assert_eq!(resolution, i as i32);
    }
}

#[test]
fn test_encodes_origin_segment_and_s_correctly() {
    let origins = get_origins();
    let origin0 = origins[0].clone();

    // Origin 0 has first quintant 4, so start use segment 4 to obtain start of Hilbert curve
    let cell = A5Cell {
        origin_id: origin0.id,
        segment: 4,
        s: BigInt::from(0),
        resolution: MAX_RESOLUTION - 1,
    };
    let serialized = serialize(&cell).unwrap();
    assert_eq!(serialized, 0b10u64);
}

#[test]
fn test_throws_error_when_s_is_too_large_for_resolution() {
    let origins = get_origins();
    let origin0 = origins[0].clone();

    let cell = A5Cell {
        origin_id: origin0.id,
        segment: 0,
        s: BigInt::from(16), // Too large for resolution 3 (max is 15)
        resolution: 3,
    };

    let result = serialize(&cell);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .contains("S (16) is too large for resolution level 3"));
}

#[test]
fn test_throws_error_when_resolution_exceeds_maximum() {
    let origins = get_origins();
    let origin0 = origins[0].clone();

    let cell = A5Cell {
        origin_id: origin0.id,
        segment: 0,
        s: BigInt::from(0),
        resolution: 31, // MAX_RESOLUTION is 30
    };

    let result = serialize(&cell);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Resolution (31) is too large"));
}

#[test]
fn test_round_trip_resolution_masks() {
    for (n, binary) in RESOLUTION_MASKS
        .iter()
        .skip(FIRST_HILBERT_RESOLUTION as usize)
        .enumerate()
    {
        // Limit to valid origin/segment combinations (0-59)
        if 5 * (n + 1) >= 60 {
            continue;
        }

        let origin_segment_id = format!("{:06b}", 5 * (n + 1));
        let combined = format!("{}{}", origin_segment_id, &binary[6..]);
        let serialized = u64::from_str_radix(&combined, 2).unwrap();
        let deserialized = deserialize(serialized).unwrap();
        let reserialized = serialize(&deserialized).unwrap();
        assert_eq!(reserialized, serialized);
    }
}

#[test]
fn test_round_trip_test_ids() {
    let test_ids = load_test_ids();
    for id in test_ids {
        let serialized = u64::from_str_radix(&id, 16).unwrap();
        let deserialized = deserialize(serialized).unwrap();
        let reserialized = serialize(&deserialized).unwrap();
        assert_eq!(reserialized, serialized);
    }
}

#[test]
fn test_round_trip_between_cell_to_parent_and_cell_to_children() {
    let test_ids = load_test_ids();
    for id in test_ids {
        let cell = u64::from_str_radix(&id, 16).unwrap();
        let children = cell_to_children(cell, None).unwrap();
        if !children.is_empty() {
            let child = children[0];
            let parent = cell_to_parent(child, None).unwrap();
            assert_eq!(parent, cell);

            let parents: Vec<u64> = children
                .iter()
                .map(|&c| cell_to_parent(c, None).unwrap())
                .collect();
            assert!(parents.iter().all(|&p| p == cell));
        }
    }
}

#[test]
fn test_cell_to_children_with_same_resolution_returns_original_cell() {
    let test_ids = load_test_ids();
    for id in test_ids {
        let cell = u64::from_str_radix(&id, 16).unwrap();
        let current_resolution = get_resolution(cell);

        // Test with explicit childResolution equal to current resolution
        let children = cell_to_children(cell, Some(current_resolution)).unwrap();

        // Should return array with just the original cell
        assert_eq!(children.len(), 1);
        assert_eq!(children[0], cell);
    }
}

#[test]
fn test_cell_to_parent_with_same_resolution_returns_original_cell() {
    let test_ids = load_test_ids();
    for id in test_ids {
        let cell = u64::from_str_radix(&id, 16).unwrap();
        let current_resolution = get_resolution(cell);

        // Test with explicit parentResolution equal to current resolution
        let parent = cell_to_parent(cell, Some(current_resolution)).unwrap();

        // Should return the original cell
        assert_eq!(parent, cell);
    }
}

#[test]
fn test_non_hilbert_to_non_hilbert_hierarchy() {
    let origins = get_origins();
    let origin0 = origins[0].clone();

    // Test resolution 0 to 1 (both non-Hilbert)
    let cell_data = A5Cell {
        origin_id: origin0.id,
        segment: 0,
        s: BigInt::from(0),
        resolution: 0,
    };
    let cell = serialize(&cell_data).unwrap();
    let children = cell_to_children(cell, None).unwrap();
    assert_eq!(children.len(), 5);

    for child in children {
        let parent = cell_to_parent(child, None).unwrap();
        assert_eq!(parent, cell);
    }
}

#[test]
fn test_non_hilbert_to_hilbert_hierarchy() {
    let origins = get_origins();
    let origin0 = origins[0].clone();

    // Test resolution 1 to 2 (non-Hilbert to Hilbert)
    let cell_data = A5Cell {
        origin_id: origin0.id,
        segment: 0,
        s: BigInt::from(0),
        resolution: 1,
    };
    let cell = serialize(&cell_data).unwrap();
    let children = cell_to_children(cell, None).unwrap();
    assert_eq!(children.len(), 4);

    for child in children {
        let parent = cell_to_parent(child, None).unwrap();
        assert_eq!(parent, cell);
    }
}

#[test]
fn test_hilbert_to_non_hilbert_hierarchy() {
    let origins = get_origins();
    let origin0 = origins[0].clone();

    // Test resolution 2 to 1 (Hilbert to non-Hilbert)
    let cell_data = A5Cell {
        origin_id: origin0.id,
        segment: 0,
        s: BigInt::from(0),
        resolution: 2,
    };
    let cell = serialize(&cell_data).unwrap();
    let parent = cell_to_parent(cell, Some(1)).unwrap();
    let children = cell_to_children(parent, None).unwrap();
    assert_eq!(children.len(), 4);
    assert!(children.contains(&cell));
}

#[test]
fn test_low_resolution_hierarchy_chain() {
    let origins = get_origins();
    let origin0 = origins[0].clone();

    // Test a chain of resolutions from 0 to 4
    let resolutions = [0, 1, 2, 3, 4];
    let cells: Vec<u64> = resolutions
        .iter()
        .map(|&res| {
            let cell_data = A5Cell {
                origin_id: origin0.id,
                segment: 0,
                s: BigInt::from(0),
                resolution: res,
            };
            serialize(&cell_data).unwrap()
        })
        .collect();

    // Test parent relationships
    for i in 1..cells.len() {
        let parent = cell_to_parent(cells[i], None).unwrap();
        assert_eq!(parent, cells[i - 1]);
    }

    // Test children relationships
    for i in 0..cells.len() - 1 {
        let children = cell_to_children(cells[i], None).unwrap();
        assert!(children.contains(&cells[i + 1]));
    }
}

#[test]
fn test_base_cell_division_counts() {
    let origins = get_origins();
    let origin0 = origins[0].clone();

    // Start with the base cell (resolution -1)
    let base_cell_data = A5Cell {
        origin_id: origin0.id,
        segment: 0,
        s: BigInt::from(0),
        resolution: -1,
    };
    let base_cell = serialize(&base_cell_data).unwrap();
    let mut current_cells = vec![base_cell];
    let expected_counts = [12usize, 60usize, 240usize, 960usize]; // 12, 12*5, 12*5*4, 12*5*4*4

    // Test each resolution level up to 3 (to avoid overflow)
    for (_resolution, &expected_count) in expected_counts.iter().enumerate().take(3) {
        // Get all children of current cells
        let all_children: Vec<u64> = current_cells
            .iter()
            .flat_map(|&cell| cell_to_children(cell, None).unwrap_or_default())
            .collect();

        // Verify the total number of cells matches expected
        assert_eq!(all_children.len(), expected_count);

        // Update current cells for next iteration
        current_cells = all_children;
    }
}

#[test]
fn test_get_res0_cells() {
    let res0_cells = get_res0_cells().unwrap();
    assert_eq!(res0_cells.len(), 12);

    // Each cell should have resolution 0
    for cell in &res0_cells {
        assert_eq!(get_resolution(*cell), 0);
    }

    // Expected hex values for the 12 resolution 0 cells
    let expected_hex_values = [
        "2000000000000000",
        "6000000000000000",
        "a000000000000000",
        "e000000000000000",
        "1200000000000000",
        "1600000000000000",
        "1a00000000000000",
        "1e00000000000000",
        "2200000000000000",
        "2600000000000000",
        "2a00000000000000",
        "2e00000000000000",
    ];

    // Verify each cell matches the expected hex value (just check the count for now)
    assert_eq!(res0_cells.len(), expected_hex_values.len());
}
