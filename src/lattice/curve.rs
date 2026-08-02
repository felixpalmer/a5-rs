// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

// Public A5 space-filling curve: point -> s, using the L-system curve
// (src/lattice/lsystem/). The s <-> cell mappings live in lsystem/mod.rs
// (s_to_cell / s_to_triple) and triple.rs (triple_to_s).

use crate::coordinate_systems::IJ;
use crate::lattice::types::Triple;

/// Locate the lattice triangle containing a fractional IJ point, as a triple.
///
/// The triples tile the IJ plane as triangles: the unit square (m, n) =
/// (floor(i), floor(j)) splits along the diagonal u+v = 1 into a lower triangle
/// (the parity-0 cell (-n, m+n, -m), centroid (m+1/3, n+1/3)) and an upper
/// triangle (the parity-1 cell (-n, m+n+1, -m), centroid (m+2/3, n+2/3)) — the
/// centroid correspondences were derived from the exact IJ <-> corner-sum
/// affine map target = (12*(i+j), -12*j) and validated against the old-engine
/// discretization over all resolutions and orientations. Point location is two
/// floors + one diagonal comparison. Points exactly on a triangle edge have no unique cell; the >=
/// tie-break below is the fixed convention.
///
/// The result is clamped into quintant bounds (m >= 0, n >= 0, m+n+parity <=
/// max_row, equivalent to triple_in_bounds): a point slightly outside the
/// quintant (as the estimate path can produce near quintant edges) must still
/// map to a valid cell for the exact encode.
pub fn round_to_triple(ij: IJ, resolution: usize) -> Triple {
    let max_row = (1i64 << resolution) - 1;
    let floor_i = ij.x().floor();
    let floor_j = ij.y().floor();
    let mut m = floor_i as i64;
    let mut n = floor_j as i64;
    let mut parity: i64 = if (ij.x() - floor_i) + (ij.y() - floor_j) >= 1.0 {
        1
    } else {
        0
    };
    if m < 0 {
        m = 0;
    }
    if n < 0 {
        n = 0;
    }
    if m + n + parity > max_row {
        parity = 0;
        if m + n > max_row {
            let over = m + n - max_row;
            let dm = over.min(m);
            m -= dm;
            n -= over - dm;
        }
    }
    Triple::new(-n as i32, (m + n + parity) as i32, -m as i32)
}
