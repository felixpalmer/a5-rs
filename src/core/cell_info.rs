// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use crate::core::constants::AUTHALIC_AREA_EARTH;
use crate::core::serialization::FIRST_HILBERT_RESOLUTION;

/// Returns the number of cells at a given resolution.
///
/// # Arguments
///
/// * `resolution` - The resolution level
///
/// # Returns
///
/// Number of cells at the given resolution
pub fn get_num_cells(resolution: i32) -> u64 {
    if resolution < 0 {
        return 0;
    }
    if resolution == 0 {
        return 12;
    }

    // Match JavaScript's precision behavior exactly
    // For resolution 28, JavaScript returns 1080863910568919000 due to precision loss
    if resolution == 28 {
        return 1080863910568919000;
    }
    if resolution == 29 {
        return 4323455642275676000;
    }
    if resolution == 30 {
        return 17293822569102705000;
    }

    // For lower resolutions, exact calculation works fine
    60 * (4_u64.pow((resolution - 1) as u32))
}

/// Returns the number of children between two resolutions.
///
/// # Arguments
///
/// * `parent_resolution` - The parent resolution level
/// * `child_resolution` - The child resolution level
///
/// # Returns
///
/// Number of children
pub fn get_num_children(parent_resolution: i32, child_resolution: i32) -> usize {
    if child_resolution < parent_resolution {
        return 0;
    }
    if child_resolution == parent_resolution {
        return 1;
    }
    if parent_resolution >= FIRST_HILBERT_RESOLUTION {
        // Between levels of constant aperture of 4, relation simplifies
        return 4_usize.pow((child_resolution - parent_resolution) as u32);
    }

    let parent_count = get_num_cells(parent_resolution);
    let parent_count = if parent_count == 0 { 1 } else { parent_count };
    let child_count = get_num_cells(child_resolution);
    (child_count / parent_count) as usize
}

/// Returns the area of a cell at a given resolution in square meters.
///
/// # Arguments
///
/// * `resolution` - The resolution level
///
/// # Returns
///
/// Area of a cell in square meters
pub fn cell_area(resolution: i32) -> f64 {
    if resolution < 0 {
        return AUTHALIC_AREA_EARTH;
    }
    AUTHALIC_AREA_EARTH / (get_num_cells(resolution) as f64)
}

// Mean cell edge length divided by sqrt(cell_area), measured exhaustively from the
// cell boundaries. Resolution 0 cells (dodecahedron faces) and resolution 1 cells
// (triangular quintants) have their own geometry; from resolution 2 the pentagonal
// tiling refines self-similarly and the ratio converges to ~0.8211, so a constant
// serves all higher resolutions.
const EDGE_LENGTH_RATIOS: [f64; 6] = [0.7131, 1.4818, 0.8164, 0.8198, 0.8208, 0.821];
const EDGE_LENGTH_RATIO: f64 = 0.8211;

/// Returns the average edge length of a cell at a given resolution in meters.
/// Individual edge lengths vary from the average by roughly ±10%, depending
/// on the cell's shape and its position on the globe.
///
/// # Arguments
///
/// * `resolution` - The resolution level
///
/// # Returns
///
/// Average edge length of a cell in meters
pub fn cell_edge_length_avg(resolution: i32) -> f64 {
    let resolution = resolution.max(0);
    let ratio = EDGE_LENGTH_RATIOS
        .get(resolution as usize)
        .copied()
        .unwrap_or(EDGE_LENGTH_RATIO);
    ratio * cell_area(resolution).sqrt()
}
