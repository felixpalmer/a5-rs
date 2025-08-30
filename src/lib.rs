// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

// Internal modules - public only for testing
#[cfg_attr(not(test), allow(unused))]
pub mod coordinate_systems;
#[cfg_attr(not(test), allow(unused))]
pub mod core;
#[cfg_attr(not(test), allow(unused))]
pub mod geometry;
#[cfg_attr(not(test), allow(unused))]
pub mod projections;
#[cfg_attr(not(test), allow(unused))]
pub mod utils;

// PUBLIC API
// Indexing
pub use core::cell::{cell_to_boundary, cell_to_lonlat, lonlat_to_cell};
pub use core::hex::{u64_to_hex, hex_to_u64};

// Hierarchy
pub use core::cell_info::{cell_area, get_num_cells};
pub use core::serialization::{cell_to_children, cell_to_parent, get_res0_cells, get_resolution};

// Types
pub use coordinate_systems::{Degrees, LonLat, Radians};
pub use core::utils::A5Cell;
