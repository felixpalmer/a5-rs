// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use crate::coordinate_systems::{Cartesian, LonLat};
use crate::core::cell::cell_to_spherical;
use crate::core::compact::compact;
use crate::core::coordinate_transforms::{from_lon_lat, to_cartesian};
use crate::core::serialization::{
    cell_to_children, get_resolution, FIRST_HILBERT_RESOLUTION, MAX_RESOLUTION, WORLD_CELL,
};
use crate::geometry::prepared_polygon::{point_in_prepared_polygon, prepare_polygon};
use crate::traversal::triple_cells::cell_ids_to_triples;

use super::curve_runs::fill_by_curve_runs;
use super::interior_flood::{fill_by_flood, prefers_flood};
use super::polygon_boundary::{classify_boundary, sample_boundary};

/// How a cell is judged to belong to the polygon.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Containment {
    /// Include a cell iff its center lies inside the polygon.
    #[default]
    Center,
    /// Additionally include every cell that overlaps the polygon boundary,
    /// giving gap-free coverage (a superset of [`Containment::Center`]).
    Overlapping,
}

/// Options for [`polygon_to_cells`].
#[derive(Debug, Clone, Copy, Default)]
pub struct PolygonToCellsOptions {
    /// Which cells to include relative to the polygon. Defaults to
    /// [`Containment::Center`].
    pub containment: Containment,
}

/// Find all cells within a polygon. The result is compacted — use `uncompact`
/// to expand to the input resolution.
///
/// `polygon` is GeoJSON-style rings `[outer, ...holes]` of `[longitude, latitude]`
/// vertices; cells inside a hole are excluded. Rings may be open or closed
/// (GeoJSON-style, first vertex repeated at the end) — closure is automatic
/// either way. Holes with fewer than 3 distinct vertices are ignored.
///
/// Pass `None` for `options` to use the defaults. `options.containment` selects
/// [`Containment::Center`] (default, cell center inside the polygon) or
/// [`Containment::Overlapping`] (any cell touching the polygon, for gap-free
/// coverage). Returns sorted, compacted cell IDs.
pub fn polygon_to_cells(
    polygon: &[Vec<LonLat>],
    resolution: i32,
    options: Option<PolygonToCellsOptions>,
) -> Result<Vec<u64>, String> {
    let options = options.unwrap_or_default();
    // GeoJSON rings repeat the first vertex at the end — drop the duplicate.
    fn strip_closing(ring: &[LonLat]) -> &[LonLat] {
        if ring.len() > 1 && ring[0] == ring[ring.len() - 1] {
            &ring[..ring.len() - 1]
        } else {
            ring
        }
    }

    if polygon.is_empty() {
        return Ok(Vec::new());
    }
    let outer = strip_closing(&polygon[0]);
    if outer.len() < 3 {
        return Ok(Vec::new());
    }
    let mut rings: Vec<&[LonLat]> = vec![outer];
    for hole in &polygon[1..] {
        let hole = strip_closing(hole);
        if hole.len() >= 3 {
            rings.push(hole);
        }
    }

    // Authalic-sphere ring vectors — A5's internal sphere, so cell centers
    // compare directly with no geodetic↔authalic round-trip.
    let mut ring_vecs_list: Vec<Vec<Cartesian>> = Vec::with_capacity(rings.len());
    for ring in &rings {
        let mut ring_vecs: Vec<Cartesian> = Vec::with_capacity(ring.len());
        for v in *ring {
            ring_vecs.push(to_cartesian(from_lon_lat(*v)));
        }
        ring_vecs_list.push(ring_vecs);
    }

    let prep = prepare_polygon(ring_vecs_list.clone());
    let sampled = sample_boundary(&rings, &ring_vecs_list, resolution)?;

    // Res 30 covers only quintants 0-41 (elsewhere A5 answers at res 29, see
    // serialize), so a polygon reaching past them is filled at res 29: mixing the
    // two lattices would leave the fill without a consistent grid.
    if resolution == MAX_RESOLUTION
        && sampled
            .cells
            .iter()
            .any(|&cell| get_resolution(cell) != resolution)
    {
        return polygon_to_cells(polygon, resolution - 1, Some(options));
    }

    let boundary = classify_boundary(sampled, &ring_vecs_list, &prep)?;
    let overlapping = options.containment == Containment::Overlapping;

    // Resolutions 0 and 1 have no lattice (a quintant is a single cell): every
    // cell off the boundary is in or out by its center, and there are at most 60
    // of them.
    if resolution < FIRST_HILBERT_RESOLUTION {
        let mut out = boundary.output(overlapping);
        for cell in cell_to_children(WORLD_CELL, Some(resolution))? {
            if !boundary.set.contains(&cell)
                && point_in_prepared_polygon(to_cartesian(cell_to_spherical(cell)?), &prep)
            {
                out.push(cell);
            }
        }
        return compact(&out);
    }

    // A quintant holding no boundary cells is wholly inside or outside; it can
    // only be inside when the polygon's bounding cap holds a quintant's area (4π/60)
    let pi = std::f64::consts::PI;
    let cap_holds_quintant = 2.0 * pi * (1.0 - prep.cap.min_dot) >= (4.0 * pi) / 60.0;
    let mut triples: Vec<[i32; 5]> = Vec::with_capacity(boundary.cells.len());
    cell_ids_to_triples(boundary.cells.iter().copied(), &mut triples)?;
    if prefers_flood(
        &ring_vecs_list,
        boundary.cells.len(),
        resolution,
        cap_holds_quintant,
    ) {
        fill_by_flood(&boundary, &triples, resolution, overlapping)
    } else {
        fill_by_curve_runs(
            &boundary,
            &triples,
            resolution,
            overlapping,
            cap_holds_quintant,
        )
    }
}
