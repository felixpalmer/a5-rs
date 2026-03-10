use a5::core::origin::get_origins;
use a5::core::serialization::{
    cell_to_children, cell_to_parent, deserialize, get_res0_cells, get_resolution, get_stride,
    is_first_child, serialize, FIRST_HILBERT_RESOLUTION, MAX_RESOLUTION,
};
use a5::core::utils::A5Cell;
use serde_json::Value;
use std::fs;

fn load_fixtures() -> Value {
    let content = fs::read_to_string("tests/fixtures/serialization.json")
        .expect("Could not read serialization.json");
    serde_json::from_str(&content).expect("Could not parse serialization.json")
}

fn get_resolution_masks(fixtures: &Value) -> Vec<String> {
    fixtures["resolutionMasks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect()
}

fn get_test_ids(fixtures: &Value) -> Vec<String> {
    fixtures["testIds"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect()
}

// =============================================================================
// serialize tests
// =============================================================================

#[test]
fn test_correct_number_of_masks() {
    let fixtures = load_fixtures();
    let masks = get_resolution_masks(&fixtures);
    assert_eq!(masks.len(), (MAX_RESOLUTION + 1) as usize);
}

#[test]
fn test_encodes_resolution_correctly_for_different_values() {
    let fixtures = load_fixtures();
    let masks = get_resolution_masks(&fixtures);
    let origins = get_origins();
    let origin0 = origins[0].clone();

    for (i, expected_binary) in masks.iter().enumerate() {
        let cell = A5Cell {
            origin_id: origin0.id,
            segment: 4,
            s: 0,
            resolution: i as i32,
        };
        let serialized = serialize(&cell).unwrap();
        let actual_binary = format!("{:064b}", serialized);
        assert_eq!(
            actual_binary, *expected_binary,
            "Failed at resolution {}",
            i
        );
    }
}

#[test]
fn test_correctly_extracts_resolution() {
    let fixtures = load_fixtures();
    let masks = get_resolution_masks(&fixtures);

    for (i, binary) in masks.iter().enumerate() {
        assert_eq!(binary.len(), 64);
        let n = u64::from_str_radix(binary, 2).unwrap();
        let resolution = get_resolution(n);
        assert_eq!(resolution, i as i32, "Failed at resolution {}", i);
    }
}

#[test]
fn test_encodes_origin_segment_and_s_correctly() {
    let origins = get_origins();
    let origin0 = origins[0].clone();

    let cell = A5Cell {
        origin_id: origin0.id,
        segment: 4,
        s: 0,
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
        s: 16,
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
        s: 0,
        resolution: 31,
    };
    let result = serialize(&cell);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Resolution (31) is too large"));
}

// =============================================================================
// Round trip tests
// =============================================================================

#[test]
fn test_round_trip_test_ids() {
    let fixtures = load_fixtures();
    let test_ids = get_test_ids(&fixtures);
    for id in test_ids {
        let serialized = u64::from_str_radix(&id, 16).unwrap();
        let deserialized = deserialize(serialized).unwrap();
        let reserialized = serialize(&deserialized).unwrap();
        assert_eq!(reserialized, serialized, "Failed for id {}", id);
    }
}

#[test]
fn test_round_trip_resolution_masks_with_origins() {
    let fixtures = load_fixtures();
    let masks = get_resolution_masks(&fixtures);

    // Exclude res 30 (different bit layout)
    for binary in masks
        .iter()
        .skip(FIRST_HILBERT_RESOLUTION as usize)
        .take((MAX_RESOLUTION - FIRST_HILBERT_RESOLUTION) as usize)
    {
        for origin_id in 1..12usize {
            let origin_segment_id = format!("{:06b}", 5 * origin_id);
            let combined = format!("{}{}", origin_segment_id, &binary[6..]);
            let serialized = u64::from_str_radix(&combined, 2).unwrap();
            let deserialized = deserialize(serialized).unwrap();
            let reserialized = serialize(&deserialized).unwrap();
            assert_eq!(reserialized, serialized);
        }
    }
}

#[test]
fn test_serialize_deserialize_round_trip() {
    let origins = get_origins();
    let origin0 = origins[0].clone();

    for resolution in 0..=MAX_RESOLUTION {
        let cell = A5Cell {
            origin_id: origin0.id,
            segment: 4,
            s: 0,
            resolution,
        };
        let serialized = serialize(&cell).unwrap();
        let deserialized = deserialize(serialized).unwrap();

        assert_eq!(deserialized.origin_id, origin0.id);
        if resolution == 0 {
            assert_eq!(deserialized.segment, 0);
        } else {
            assert_eq!(deserialized.segment, 4);
        }
        assert_eq!(deserialized.s, 0);
        assert_eq!(deserialized.resolution, resolution);
        assert_eq!(get_resolution(serialized), resolution);
    }
}

// =============================================================================
// hierarchy tests
// =============================================================================

#[test]
fn test_round_trip_between_cell_to_parent_and_cell_to_children() {
    let fixtures = load_fixtures();
    let test_ids = get_test_ids(&fixtures);
    for id in test_ids {
        let cell = u64::from_str_radix(&id, 16).unwrap();
        let resolution = get_resolution(cell);
        // Skip res 30 (no children) and res 29 with out-of-bounds quintants
        // (res 30 children fall back to res 29)
        if resolution >= MAX_RESOLUTION {
            continue;
        }
        let children = cell_to_children(cell, None).unwrap();
        let child = children[0];
        if get_resolution(child) != resolution + 1 {
            continue;
        }

        let parent = cell_to_parent(child, None).unwrap();
        assert_eq!(parent, cell, "Failed for id {}", id);

        let parents: Vec<u64> = children
            .iter()
            .map(|&c| cell_to_parent(c, None).unwrap())
            .collect();
        assert!(parents.iter().all(|&p| p == cell));
    }
}

#[test]
fn test_cell_to_children_with_same_resolution_returns_original_cell() {
    let fixtures = load_fixtures();
    let test_ids = get_test_ids(&fixtures);
    for id in test_ids {
        let cell = u64::from_str_radix(&id, 16).unwrap();
        let current_resolution = get_resolution(cell);
        let children = cell_to_children(cell, Some(current_resolution)).unwrap();
        assert_eq!(children.len(), 1);
        assert_eq!(children[0], cell);
    }
}

#[test]
fn test_cell_to_parent_with_same_resolution_returns_original_cell() {
    let fixtures = load_fixtures();
    let test_ids = get_test_ids(&fixtures);
    for id in test_ids {
        let cell = u64::from_str_radix(&id, 16).unwrap();
        let current_resolution = get_resolution(cell);
        let parent = cell_to_parent(cell, Some(current_resolution)).unwrap();
        assert_eq!(parent, cell);
    }
}

#[test]
fn test_non_hilbert_to_non_hilbert_hierarchy() {
    let origins = get_origins();
    let origin0 = origins[0].clone();

    let cell = serialize(&A5Cell {
        origin_id: origin0.id,
        segment: 0,
        s: 0,
        resolution: 0,
    })
    .unwrap();
    let children = cell_to_children(cell, None).unwrap();
    assert_eq!(children.len(), 5);
    for child in children {
        assert_eq!(cell_to_parent(child, None).unwrap(), cell);
    }
}

#[test]
fn test_non_hilbert_to_hilbert_hierarchy() {
    let origins = get_origins();
    let origin0 = origins[0].clone();

    let cell = serialize(&A5Cell {
        origin_id: origin0.id,
        segment: 0,
        s: 0,
        resolution: 1,
    })
    .unwrap();
    let children = cell_to_children(cell, None).unwrap();
    assert_eq!(children.len(), 4);
    for child in children {
        assert_eq!(cell_to_parent(child, None).unwrap(), cell);
    }
}

#[test]
fn test_hilbert_to_non_hilbert_hierarchy() {
    let origins = get_origins();
    let origin0 = origins[0].clone();

    let cell = serialize(&A5Cell {
        origin_id: origin0.id,
        segment: 0,
        s: 0,
        resolution: 2,
    })
    .unwrap();
    let parent = cell_to_parent(cell, Some(1)).unwrap();
    let children = cell_to_children(parent, None).unwrap();
    assert_eq!(children.len(), 4);
    assert!(children.contains(&cell));
}

#[test]
fn test_low_resolution_hierarchy_chain() {
    let origins = get_origins();
    let origin0 = origins[0].clone();

    let cells: Vec<u64> = (0..=4)
        .map(|res| {
            serialize(&A5Cell {
                origin_id: origin0.id,
                segment: 0,
                s: 0,
                resolution: res,
            })
            .unwrap()
        })
        .collect();

    for i in 1..cells.len() {
        assert_eq!(cell_to_parent(cells[i], None).unwrap(), cells[i - 1]);
    }
    for i in 0..cells.len() - 1 {
        let children = cell_to_children(cells[i], None).unwrap();
        assert!(children.contains(&cells[i + 1]));
    }
}

#[test]
fn test_base_cell_division_counts() {
    let origins = get_origins();
    let origin0 = origins[0].clone();

    let base_cell = serialize(&A5Cell {
        origin_id: origin0.id,
        segment: 0,
        s: 0,
        resolution: -1,
    })
    .unwrap();
    let mut current_cells = vec![base_cell];
    let expected_counts = [12usize, 60, 240, 960];

    for &expected_count in expected_counts.iter().take(3) {
        let all_children: Vec<u64> = current_cells
            .iter()
            .flat_map(|&cell| cell_to_children(cell, None).unwrap_or_default())
            .collect();
        assert_eq!(all_children.len(), expected_count);
        current_cells = all_children;
    }
}

// =============================================================================
// getRes0Cells tests
// =============================================================================

#[test]
fn test_get_res0_cells() {
    let res0_cells = get_res0_cells().unwrap();
    assert_eq!(res0_cells.len(), 12);
    for cell in &res0_cells {
        assert_eq!(get_resolution(*cell), 0);
    }
}

// =============================================================================
// resolution 30 tests
// =============================================================================

#[test]
fn test_res30_get_resolution_detects_from_lsb() {
    assert_eq!(get_resolution(1), 30);
    assert_eq!(get_resolution(3), 30);
    assert_eq!(get_resolution(0xFFFFFFFFFFFFFFFF), 30);
}

#[test]
fn test_res30_round_trip_valid_quintants() {
    let origins = get_origins();
    for q in 0..42usize {
        let origin_id = q / 5;
        let origin = &origins[origin_id];
        let segment_n = q % 5;
        let segment = (segment_n + origin.first_quintant) % 5;

        let cell = A5Cell {
            origin_id: origin_id as u8,
            segment,
            s: 0,
            resolution: 30,
        };
        let serialized = serialize(&cell).unwrap();
        assert_eq!(get_resolution(serialized), 30, "Failed for quintant {}", q);

        // Verify correct marker pattern
        if q <= 31 {
            assert_eq!(serialized & 1, 1); // ...1 encoding
        } else if q <= 39 {
            assert_eq!(serialized & 0b111, 0b100); // ...100 encoding
        } else {
            assert_eq!(serialized & 0b11111, 0b10000); // ...10000 encoding
        }

        let deserialized = deserialize(serialized).unwrap();
        assert_eq!(deserialized.origin_id, origin_id as u8);
        assert_eq!(deserialized.segment, segment);
        assert_eq!(deserialized.s, 0);
        assert_eq!(deserialized.resolution, 30);

        let reserialized = serialize(&deserialized).unwrap();
        assert_eq!(reserialized, serialized);
    }
}

#[test]
fn test_res30_round_trip_nonzero_s() {
    let origins = get_origins();
    let origin = &origins[0];
    let segment = origin.first_quintant % 5;

    let test_s_values = [0u64, 1, 42, (1u64 << 58) - 1];
    for &s in &test_s_values {
        let cell = A5Cell {
            origin_id: origin.id,
            segment,
            s,
            resolution: 30,
        };
        let serialized = serialize(&cell).unwrap();
        let deserialized = deserialize(serialized).unwrap();
        assert_eq!(deserialized.s, s);
        assert_eq!(deserialized.resolution, 30);
        assert_eq!(serialize(&deserialized).unwrap(), serialized);
    }
}

#[test]
fn test_res30_bit_layout_1_encoding() {
    let origins = get_origins();
    let origin = &origins[0];
    let segment = origin.first_quintant % 5;

    let cell0 = serialize(&A5Cell {
        origin_id: origin.id,
        segment,
        s: 0,
        resolution: 30,
    })
    .unwrap();
    assert_eq!(cell0, 1);

    let cell1 = serialize(&A5Cell {
        origin_id: origin.id,
        segment,
        s: 1,
        resolution: 30,
    })
    .unwrap();
    assert_eq!(cell1, 0b11);
}

#[test]
fn test_res30_bit_layout_10000_encoding() {
    let origins = get_origins();
    let origin = &origins[8];
    let segment = origin.first_quintant % 5;

    let cell0 = serialize(&A5Cell {
        origin_id: origin.id,
        segment,
        s: 0,
        resolution: 30,
    })
    .unwrap();
    assert_eq!(cell0, 0b10000);

    let cell1 = serialize(&A5Cell {
        origin_id: origin.id,
        segment,
        s: 1,
        resolution: 30,
    })
    .unwrap();
    assert_eq!(cell1, 0b110000);
}

#[test]
fn test_res30_bit_layout_100_encoding() {
    let origins = get_origins();
    // Origin 6 has quintants 30-34, segmentN=2 gives quintant 32
    let origin = &origins[6];
    let segment_n = 2;
    let segment = (segment_n + origin.first_quintant) % 5;

    let cell0 = serialize(&A5Cell {
        origin_id: origin.id,
        segment,
        s: 0,
        resolution: 30,
    })
    .unwrap();
    assert_eq!(cell0, 0b100);

    let cell1 = serialize(&A5Cell {
        origin_id: origin.id,
        segment,
        s: 1,
        resolution: 30,
    })
    .unwrap();
    assert_eq!(cell1, 0b1100);
}

#[test]
fn test_res30_round_trip_nonzero_s_100_encoding() {
    let origins = get_origins();
    // Quintant 35 (origin 7, segmentN=0)
    let origin = &origins[7];
    let segment = origin.first_quintant % 5;

    let test_s_values = [0u64, 1, 42, (1u64 << 58) - 1];
    for &s in &test_s_values {
        let cell = A5Cell {
            origin_id: origin.id,
            segment,
            s,
            resolution: 30,
        };
        let serialized = serialize(&cell).unwrap();
        assert_eq!(serialized & 0b111, 0b100);
        let deserialized = deserialize(serialized).unwrap();
        assert_eq!(deserialized.s, s);
        assert_eq!(deserialized.resolution, 30);
        assert_eq!(serialize(&deserialized).unwrap(), serialized);
    }
}

#[test]
fn test_res30_round_trip_nonzero_s_10000_encoding() {
    let origins = get_origins();
    // Quintant 40 (origin 8, segmentN=0)
    let origin = &origins[8];
    let segment = origin.first_quintant % 5;

    let test_s_values = [0u64, 1, 42, (1u64 << 58) - 1];
    for &s in &test_s_values {
        let cell = A5Cell {
            origin_id: origin.id,
            segment,
            s,
            resolution: 30,
        };
        let serialized = serialize(&cell).unwrap();
        assert_eq!(serialized & 0b11111, 0b10000);
        let deserialized = deserialize(serialized).unwrap();
        assert_eq!(deserialized.s, s);
        assert_eq!(deserialized.resolution, 30);
        assert_eq!(serialize(&deserialized).unwrap(), serialized);
    }
}

#[test]
fn test_res30_falls_back_to_res29_for_quintant_gt_41() {
    let origins = get_origins();
    // Origin 9 has quintants 45-49, all > 41
    let origin = &origins[9];
    let segment = origin.first_quintant % 5;

    let cell = serialize(&A5Cell {
        origin_id: origin.id,
        segment,
        s: 0,
        resolution: 30,
    })
    .unwrap();
    assert_eq!(get_resolution(cell), 29);

    let cell2 = serialize(&A5Cell {
        origin_id: origin.id,
        segment,
        s: 7,
        resolution: 30,
    })
    .unwrap();
    assert_eq!(get_resolution(cell2), 29);
    assert_eq!(deserialize(cell2).unwrap().s, 1); // 7 >> 2 = 1
}

#[test]
fn test_res30_falls_back_for_out_of_bounds_quintant_55() {
    let origins = get_origins();
    // Origin 11 has quintants 55-59, all > 41
    let origin = &origins[11];
    let segment_n = 0;
    let segment = (segment_n + origin.first_quintant) % 5;

    let cell = serialize(&A5Cell {
        origin_id: origin.id,
        segment,
        s: 100,
        resolution: 30,
    })
    .unwrap();
    assert_eq!(get_resolution(cell), 29);
    assert_eq!(deserialize(cell).unwrap().s, 25); // 100 >> 2 = 25
    assert_eq!(deserialize(cell).unwrap().origin_id, 11);
}

#[test]
fn test_res30_throws_for_s_too_large() {
    let origins = get_origins();
    let origin = &origins[0];
    let segment = origin.first_quintant % 5;

    let result = serialize(&A5Cell {
        origin_id: origin.id,
        segment,
        s: 1u64 << 58,
        resolution: 30,
    });
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .contains("too large for resolution level 30"));
}

#[test]
fn test_res30_cell_to_parent() {
    let origins = get_origins();
    let origin = &origins[0];
    let segment = origin.first_quintant % 5;

    for i in 0..4u64 {
        let child = serialize(&A5Cell {
            origin_id: origin.id,
            segment,
            s: i,
            resolution: 30,
        })
        .unwrap();
        let parent = cell_to_parent(child, None).unwrap();
        assert_eq!(get_resolution(parent), 29);
        assert_eq!(deserialize(parent).unwrap().s, 0);
    }
}

#[test]
fn test_res30_cell_to_children() {
    let origins = get_origins();
    let origin = &origins[0];
    let segment = origin.first_quintant % 5;

    let parent = serialize(&A5Cell {
        origin_id: origin.id,
        segment,
        s: 0,
        resolution: 29,
    })
    .unwrap();
    let children = cell_to_children(parent, Some(30)).unwrap();
    assert_eq!(children.len(), 4);
    for (i, &child) in children.iter().enumerate() {
        assert_eq!(get_resolution(child), 30);
        assert_eq!(deserialize(child).unwrap().s, i as u64);
    }
}

#[test]
fn test_res30_children_parent_round_trip() {
    let origins = get_origins();
    let origin = &origins[0];
    let segment = origin.first_quintant % 5;

    let parent = serialize(&A5Cell {
        origin_id: origin.id,
        segment,
        s: 42,
        resolution: 29,
    })
    .unwrap();
    let children = cell_to_children(parent, Some(30)).unwrap();
    assert_eq!(children.len(), 4);
    for child in children {
        assert_eq!(cell_to_parent(child, None).unwrap(), parent);
    }
}

#[test]
fn test_res30_get_stride() {
    assert_eq!(get_stride(30), 2);
}

#[test]
fn test_res30_is_first_child_1_encoding() {
    let origins = get_origins();
    let origin = &origins[0];
    let segment = origin.first_quintant % 5;

    assert!(is_first_child(
        serialize(&A5Cell {
            origin_id: origin.id,
            segment,
            s: 0,
            resolution: 30,
        })
        .unwrap(),
        None
    ));
    assert!(!is_first_child(
        serialize(&A5Cell {
            origin_id: origin.id,
            segment,
            s: 1,
            resolution: 30,
        })
        .unwrap(),
        None
    ));
    assert!(is_first_child(
        serialize(&A5Cell {
            origin_id: origin.id,
            segment,
            s: 4,
            resolution: 30,
        })
        .unwrap(),
        None
    ));
}

#[test]
fn test_res30_is_first_child_100_encoding() {
    let origins = get_origins();
    let origin = &origins[7]; // quintant 35, uses ...100
    let segment = origin.first_quintant % 5;

    assert!(is_first_child(
        serialize(&A5Cell {
            origin_id: origin.id,
            segment,
            s: 0,
            resolution: 30,
        })
        .unwrap(),
        None
    ));
    assert!(!is_first_child(
        serialize(&A5Cell {
            origin_id: origin.id,
            segment,
            s: 1,
            resolution: 30,
        })
        .unwrap(),
        None
    ));
    assert!(is_first_child(
        serialize(&A5Cell {
            origin_id: origin.id,
            segment,
            s: 4,
            resolution: 30,
        })
        .unwrap(),
        None
    ));
}

#[test]
fn test_res30_is_first_child_10000_encoding() {
    let origins = get_origins();
    let origin = &origins[8]; // quintant 40, uses ...10000
    let segment = origin.first_quintant % 5;

    assert!(is_first_child(
        serialize(&A5Cell {
            origin_id: origin.id,
            segment,
            s: 0,
            resolution: 30,
        })
        .unwrap(),
        None
    ));
    assert!(!is_first_child(
        serialize(&A5Cell {
            origin_id: origin.id,
            segment,
            s: 1,
            resolution: 30,
        })
        .unwrap(),
        None
    ));
    assert!(is_first_child(
        serialize(&A5Cell {
            origin_id: origin.id,
            segment,
            s: 4,
            resolution: 30,
        })
        .unwrap(),
        None
    ));
}

#[test]
fn test_res30_children_parent_round_trip_10000_encoding() {
    let origins = get_origins();
    let origin = &origins[8];
    let segment = origin.first_quintant % 5;

    let parent = serialize(&A5Cell {
        origin_id: origin.id,
        segment,
        s: 10,
        resolution: 29,
    })
    .unwrap();
    let children = cell_to_children(parent, Some(30)).unwrap();
    assert_eq!(children.len(), 4);
    for child in children {
        assert_eq!(get_resolution(child), 30);
        assert_eq!(child & 0b11111, 0b10000);
        assert_eq!(cell_to_parent(child, None).unwrap(), parent);
    }
}

#[test]
fn test_res30_children_parent_round_trip_100_encoding() {
    let origins = get_origins();
    let origin = &origins[7];
    let segment = origin.first_quintant % 5;

    let parent = serialize(&A5Cell {
        origin_id: origin.id,
        segment,
        s: 10,
        resolution: 29,
    })
    .unwrap();
    let children = cell_to_children(parent, Some(30)).unwrap();
    assert_eq!(children.len(), 4);
    for child in children {
        assert_eq!(get_resolution(child), 30);
        assert_eq!(child & 0b111, 0b100);
        assert_eq!(cell_to_parent(child, None).unwrap(), parent);
    }
}

#[test]
fn test_res30_cell_to_children_throws_at_max() {
    let origins = get_origins();
    let origin = &origins[0];
    let segment = origin.first_quintant % 5;

    let cell = serialize(&A5Cell {
        origin_id: origin.id,
        segment,
        s: 0,
        resolution: 30,
    })
    .unwrap();
    let result = cell_to_children(cell, None);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("exceeds maximum resolution"));
}

// =============================================================================
// resolution 30 location tests
// =============================================================================

#[test]
fn test_res30_locations_round_trip() {
    let fixtures = load_fixtures();
    let locations = fixtures["res30Locations"].as_array().unwrap();
    for loc in locations {
        let hex = loc["hex"].as_str().unwrap();
        let cell = u64::from_str_radix(hex, 16).unwrap();
        let deserialized = deserialize(cell).unwrap();
        let reserialized = serialize(&deserialized).unwrap();
        assert_eq!(reserialized, cell, "Failed for {}", loc["name"]);
    }
}

#[test]
fn test_res30_locations_out_of_bounds_fall_back() {
    let fixtures = load_fixtures();
    let locations = fixtures["res30Locations"].as_array().unwrap();
    let out_of_bounds: Vec<&Value> = locations
        .iter()
        .filter(|l| l["resolution"].as_i64().unwrap() == 29)
        .collect();
    assert!(!out_of_bounds.is_empty());
    for loc in out_of_bounds {
        let hex = loc["hex"].as_str().unwrap();
        let cell = u64::from_str_radix(hex, 16).unwrap();
        assert_eq!(get_resolution(cell), 29);
    }
}

#[test]
fn test_res30_locations_in_bounds_encode_at_res30() {
    let fixtures = load_fixtures();
    let locations = fixtures["res30Locations"].as_array().unwrap();
    let in_bounds: Vec<&Value> = locations
        .iter()
        .filter(|l| l["resolution"].as_i64().unwrap() == 30)
        .collect();
    assert!(!in_bounds.is_empty());
    for loc in in_bounds {
        let hex = loc["hex"].as_str().unwrap();
        let cell = u64::from_str_radix(hex, 16).unwrap();
        assert_eq!(get_resolution(cell), 30);
    }
}
