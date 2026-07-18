// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

// The canonical A5 curve is the non-self-intersecting L-system curve
// (lsystem/ + curve.rs): point location via round_to_triple, s <-> cell mappings via
// s_to_cell / s_to_triple / triple_to_s. This is a breaking change from previous
// releases — cell IDs differ from the original construction. The original curve
// remains available bit-for-bit via the compat_* exports below for migration.

pub mod compat;
pub mod curve;
pub mod lsystem;
pub mod triple;
pub mod types;

pub use curve::round_to_triple;
pub use lsystem::{s_to_cell, s_to_triple, Cell};

pub use triple::{triple_flavor, triple_in_bounds, triple_parity, triple_to_s};
pub use types::{Orientation, Triple};

// The ORIGINAL (pre-L-system) curve, bit-for-bit, for the migration path —
// same cells, same pentagon flavors, old visiting order (tests/lattice_compat.rs).
pub use compat::{compat_ij_to_s, compat_s_to_cell, compat_s_to_triple, compat_triple_to_s};
