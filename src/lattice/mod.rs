// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

// The canonical A5 curve is currently the ORIGINAL construction (compat.rs):
// the two-motif quaternary L-system with the shift_digits recode on top, so
// cell IDs remain bit-identical to previous releases. The non-self-intersecting
// L-system curve (lsystem/ + curve.rs) powers the machinery underneath and is
// fully implemented and pinned by fixtures (tests/lattice_lsystem.rs); making
// it canonical is a planned follow-up — a breaking change of all cell IDs that
// swaps the exports below to the lsystem versions and regenerates the fixtures.

pub mod compat;
pub mod curve;
pub mod lsystem;
pub mod triple;
pub mod types;

// The engine uses the compat (original) curve, exported under the plain names.
pub use compat::{
    compat_ij_to_s as ij_to_s, compat_s_to_cell as s_to_cell, compat_s_to_triple as s_to_triple,
    compat_triple_to_s as triple_to_s,
};

// Also exported under their own names, so the old-curve behavior stays pinned
// explicitly (tests/lattice_compat.rs) across the future canonical swap.
pub use compat::{compat_ij_to_s, compat_s_to_cell, compat_s_to_triple, compat_triple_to_s};

pub use lsystem::Cell;
pub use triple::{triple_in_bounds, triple_parity};
pub use types::{Orientation, Triple};
