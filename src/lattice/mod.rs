// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

pub mod anchor;
pub mod basis;
pub mod hilbert;
pub mod quaternary;
pub mod shift_digits;
pub mod triple;
pub mod types;

// Re-export public items
pub use anchor::{compute_q, offset_flips_to_anchor};
pub use basis::{ij_to_kj, kj_to_ij};
pub use hilbert::{
    anchor_to_s, get_required_digits, ij_to_flips, ij_to_s, ij_to_s_internal, s_to_anchor,
    s_to_anchor_internal,
};
pub use quaternary::{ij_to_quaternary, quaternary_to_flips, quaternary_to_kj};
pub use shift_digits::shift_digits;
pub use triple::{
    anchor_to_triple, triple_in_bounds, triple_parity, triple_to_anchor, triple_to_s, Triple,
};
pub use types::{Anchor, Flip, Orientation, Quaternary, NO, YES};
