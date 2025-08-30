// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

pub mod coordinate_systems;
pub mod core;
pub mod geometry;
pub mod projections;
pub mod utils;

#[cfg(test)]
mod test;

// PUBLIC API
// Indexing
pub use core::cell::{cell_to_boundary, cell_to_lonlat, lonlat_to_cell};
pub use core::hex::{hex_to_u64, u64_to_hex};

// Hierarchy
pub use core::serialization::{cell_to_parent, cell_to_children, get_resolution, get_res0_cells};
pub use core::cell_info::{get_num_cells, cell_area};

// Types
pub use coordinate_systems::{Degrees, Radians, LonLat};
pub use core::utils::A5Cell;
