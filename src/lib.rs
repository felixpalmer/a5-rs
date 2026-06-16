// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

// Internal modules. These are `pub` only so the fixture-driven integration tests in
// `tests/` (separate crates) can reach internal functions. They are NOT part of the
// supported public API — `#[doc(hidden)]` keeps them off docs.rs so downstream consumers
// see only the curated re-exports below. Use those, not `a5::core::*` paths.
#[doc(hidden)]
#[cfg_attr(not(test), allow(unused))]
pub mod coordinate_systems;
#[doc(hidden)]
#[cfg_attr(not(test), allow(unused))]
pub mod core;
#[doc(hidden)]
#[cfg_attr(not(test), allow(unused))]
pub mod geometry;
#[doc(hidden)]
#[cfg_attr(not(test), allow(unused))]
pub mod lattice;
#[doc(hidden)]
#[cfg_attr(not(test), allow(unused))]
pub mod projections;
#[doc(hidden)]
#[cfg_attr(not(test), allow(unused))]
pub mod regions;
#[doc(hidden)]
#[cfg_attr(not(test), allow(unused))]
pub mod traversal;
#[doc(hidden)]
#[cfg_attr(not(test), allow(unused))]
pub mod utils;

// PUBLIC API
// Indexing
pub use core::cell::{cell_to_boundary, cell_to_lonlat, lonlat_to_cell};
pub use core::hex::{hex_to_u64, u64_to_hex};

// Hierarchy
pub use core::cell_info::{cell_area, get_num_cells, get_num_children};
pub use core::serialization::{
    cell_to_children, cell_to_parent, get_res0_cells, get_resolution, MAX_RESOLUTION, WORLD_CELL,
};

// Compaction
pub use core::compact::{compact, uncompact};

// Traversal
pub use traversal::cap::spherical_cap;
pub use traversal::grid_disk::{grid_disk, grid_disk_vertex};
pub use traversal::line::line_string_to_cells;

// Regions
pub use regions::polygon::polygon_to_cells;

// Types
pub use coordinate_systems::{Degrees, LonLat, Radians};
pub use core::utils::A5Cell;
