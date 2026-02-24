// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

//! Backward-compatible re-exports from the lattice module.

pub use crate::lattice::{
    // Hilbert
    anchor_to_s,
    get_required_digits,
    ij_to_flips,
    // Basis
    ij_to_kj,
    // Quaternary
    ij_to_quaternary,
    ij_to_s,
    ij_to_s_internal,
    kj_to_ij,
    quaternary_to_flips,
    quaternary_to_kj,
    s_to_anchor,
    s_to_anchor_internal,
    // Shift digits
    shift_digits,
    // Types
    Anchor,
    Flip,
    Orientation,
    Quaternary,
    NO,
    YES,
};
