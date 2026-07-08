// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

// Public A5 space-filling curve: point -> s, using the L-system curve
// (src/lattice/lsystem/). The s <-> cell mappings live in lsystem/mod.rs
// (s_to_cell / s_to_triple) and triple.rs (triple_to_s).

use crate::coordinate_systems::IJ;
use crate::lattice::lsystem::sum_point_to_s;
use crate::lattice::types::Orientation;

/// Fractional IJ point -> curve position `s` of the containing cell, by direct
/// L-system descent. The IJ plane maps onto the L-system's corner-sum frame by
/// the exact affine map target = (12*(i+j), -12*j).
pub fn ij_to_s(ij: IJ, resolution: usize, orientation: Orientation) -> u64 {
    sum_point_to_s(
        12.0 * (ij.x() + ij.y()),
        -12.0 * ij.y(),
        resolution,
        orientation,
    )
}
