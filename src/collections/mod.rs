// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

//! Sets of cells: compact/uncompact, set operations and measures. Depends only on `core`.

pub mod compact;
pub mod measures;
pub mod resolution;
pub mod set_operations;
pub mod slot_runs;

/// Sorted, disjoint, half-open runs of leaf slots [lo, hi), flattened as
/// [lo0, hi0, lo1, hi1, ...]: the form set operations work on.
pub type SlotRuns = Vec<u64>;
