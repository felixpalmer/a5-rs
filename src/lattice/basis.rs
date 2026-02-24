// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use crate::coordinate_systems::{IJ, KJ};

/// Anchor offset is specified in ij units, the eigenbasis of the Hilbert curve
/// Define k as the vector i + j, as it means vectors u & v are of unit length
pub fn ij_to_kj(ij: IJ) -> KJ {
    KJ::new(ij.x() + ij.y(), ij.y())
}

pub fn kj_to_ij(kj: KJ) -> IJ {
    IJ::new(kj.x() - kj.y(), kj.y())
}
