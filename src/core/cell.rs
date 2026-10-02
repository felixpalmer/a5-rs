// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use crate::coordinate_systems::{Face, LonLat, Spherical};
use crate::core::constants::PI_OVER_5;
use crate::core::coordinate_transforms::{
    face_to_ij, from_lon_lat, normalize_longitudes, to_lon_lat, to_polar,
};
use crate::core::origin::{
    find_nearest_origin, find_nearest_origins, quintant_to_segment, segment_to_quintant,
};
use crate::core::serialization::{
    deserialize, serialize, FIRST_HILBERT_RESOLUTION, MAX_RESOLUTION, WORLD_CELL,
};
use crate::core::tiling::{
    cell_margin_scaled, get_face_vertices, get_pentagon_center, get_pentagon_vertices,
    get_quintant_polar, get_quintant_vertices,
};
use crate::core::utils::{A5Cell, Origin, OriginId};
use crate::geometry::pentagon::PentagonShape;
use crate::lattice::curve::round_to_triple;
use crate::lattice::{s_to_cell, triple_flavor, triple_in_bounds, triple_to_s, Triple};
use crate::projections::dodecahedron::DodecahedronProjection;
use crate::traversal::neighbors::NEIGHBOR_DELTAS;
use std::cell::RefCell;

// Single-entry cache of the most recent successful lookup. Speeds up
// dense-sample workloads (polygon boundary tracing, line tracing) where
// consecutive calls often land in the same cell. The cache stores the
// pre-computed pentagon + origin so the hit-test is just one projection
// + one pentagon containment check.
struct LastResult {
    cell_id: u64,
    pentagon: PentagonShape,
    origin_id: OriginId,
    resolution: i32,
}

thread_local! {
    static LAST_RESULT: RefCell<Option<LastResult>> = const { RefCell::new(None) };
}

/// Convert lon/lat coordinates to A5 cell ID
pub fn lonlat_to_cell(lonlat: LonLat, resolution: i32) -> Result<u64, String> {
    spherical_to_cell(from_lon_lat(lonlat), resolution)
}

/// Like `lonlat_to_cell`, but accepts a point already in A5's internal
/// spherical representation (rotated authalic frame, as produced by
/// `from_lon_lat` or `to_spherical(authalic_cartesian)`). Skips the redundant
/// authalic inverse/forward round-trip in dense-sample loops where the input
/// already comes from authalic Cartesian space (e.g. polygon-fill boundary slerp).
pub fn spherical_to_cell(spherical: Spherical, resolution: i32) -> Result<u64, String> {
    // Resolution -1 represents WORLD_CELL, which covers the entire world
    if resolution == -1 {
        return Ok(WORLD_CELL);
    }

    if resolution < FIRST_HILBERT_RESOLUTION {
        // For low resolutions there is no Hilbert curve: the cell is determined
        // by the face (and quintant) alone, so the lookup is exact.
        let origin = find_nearest_origin(spherical);
        let dodecahedron = DodecahedronProjection::get_thread_local();
        let dodec_point = dodecahedron.forward(spherical, origin.id)?;
        let quintant = get_quintant_polar(to_polar(dodec_point));
        let (segment, _) = quintant_to_segment(quintant, origin);
        return serialize(&A5Cell {
            origin_id: origin.id,
            segment,
            s: 0,
            resolution,
        });
    }

    // Try the cached pentagon first — skips the full lookup when consecutive
    // calls land in the same cell (common in dense-sample loops).
    let cached_hit = LAST_RESULT.with(|c| -> Result<Option<u64>, String> {
        let last_ref = c.borrow();
        let last = match last_ref.as_ref() {
            Some(l) if l.resolution == resolution => l,
            _ => return Ok(None),
        };
        let dodecahedron = DodecahedronProjection::get_thread_local();
        let projected = dodecahedron.forward(spherical, last.origin_id)?;
        if last.pentagon.contains_point(projected) > 0.0 {
            Ok(Some(last.cell_id))
        } else {
            Ok(None)
        }
    })?;
    if let Some(cell_id) = cached_hit {
        return Ok(cell_id);
    }

    // Fast path: locate the containing pentagon directly. Round to the leaf
    // triangle, get the closed-form flavor, and test the pentagon geometrically
    // in the scaled quintant frame; the triangular and pentagonal lattices are
    // not congruent, but the containing pentagon is always the triangle's cell
    // or one of its fixed neighbor deltas (verified exhaustively), so at most
    // one 7-candidate walk resolves it — then a single curve encode.
    let origin = find_nearest_origin(spherical);
    let dodecahedron = DodecahedronProjection::get_thread_local();
    let dodec_point = dodecahedron.forward(spherical, origin.id)?;
    let quintant = get_quintant_polar(to_polar(dodec_point));
    let best = lookup_in_quintant(dodec_point, origin, quintant, resolution)?;
    if let Some(candidate) = &best {
        if candidate.margin > 0.0 {
            return Ok(accept_candidate(candidate));
        }
    }
    // No strictly-containing pentagon in the assigned frame: the point sits on
    // a cell boundary or within float noise of a quintant/face seam.
    spherical_to_cell_boundary(spherical, resolution, origin.id, quintant, best)
}

// The best cell for `dodec_point` (face frame of `origin`) within one
// quintant: round to the leaf triangle, closed-form flavor, geometric margin,
// and — when the triangle's cell doesn't strictly contain the point — the
// best of its fixed neighbor deltas. margin > 0 ⇔ the unique strictly-
// containing pentagon.
struct CellCandidate {
    margin: f64,
    cell_id: u64,
    triple: Triple,
    flavor: u8,
    quintant: usize,
    hilbert_resolution: usize,
    origin_id: OriginId,
    resolution: i32,
}

#[inline]
fn lookup_in_quintant(
    dodec_point: Face,
    origin: &Origin,
    quintant: usize,
    resolution: i32,
) -> Result<Option<CellCandidate>, String> {
    let (segment, orientation) = quintant_to_segment(quintant, origin);

    // Res-30 ids can only encode quintants 0-41 (by design: 64 bits cannot fit
    // res 30 globally, so A5 covers the populous region). In the unsupported
    // quintants, answer at the finest representable resolution instead — the
    // res-29 cell CONTAINING the point. (Previously the cap lived only in
    // serialize, which swapped in the res-29 parent of a res-30 search result —
    // a cell that fails to contain the query point ~44% of the time there.)
    let segment_n = (segment + 5 - origin.first_quintant) % 5;
    let resolution = if resolution == MAX_RESOLUTION && 5 * origin.id as usize + segment_n > 41 {
        MAX_RESOLUTION - 1
    } else {
        resolution
    };

    let (mut px, mut py) = (dodec_point.x(), dodec_point.y());
    if quintant != 0 {
        let extra_angle = 2.0 * PI_OVER_5.get() * quintant as f64;
        let cos_angle = (-extra_angle).cos();
        let sin_angle = (-extra_angle).sin();
        let rotated_x = cos_angle * px - sin_angle * py;
        let rotated_y = sin_angle * px + cos_angle * py;
        px = rotated_x;
        py = rotated_y;
    }
    let hilbert_resolution = (1 + resolution - FIRST_HILBERT_RESOLUTION) as usize;
    let scale = (1u64 << hilbert_resolution) as f64;
    px *= scale;
    py *= scale;
    let ij = face_to_ij(Face::new(px, py));

    let base = round_to_triple(ij, hilbert_resolution);
    let mut triple = base;
    let mut flavor = triple_flavor(&base);
    let mut margin = cell_margin_scaled(px, py, base.x, base.y, flavor);
    if margin <= 0.0 {
        // All deltas are relative to the ROUNDED triple (the containing
        // pentagon is always among its fixed neighbors), not to intermediate
        // best cells.
        let max_row = (1i64 << hilbert_resolution) as i32 - 1;
        for d in &NEIGHBOR_DELTAS[flavor as usize].all {
            let neighbor = Triple::new(base.x + d.x, base.y + d.y, base.z + d.z);
            if !triple_in_bounds(&neighbor, max_row) {
                continue;
            }
            let neighbor_flavor = triple_flavor(&neighbor);
            let neighbor_margin =
                cell_margin_scaled(px, py, neighbor.x, neighbor.y, neighbor_flavor);
            if neighbor_margin > margin {
                triple = neighbor;
                flavor = neighbor_flavor;
                margin = neighbor_margin;
                if margin > 0.0 {
                    break;
                }
            }
        }
    }
    let s = match triple_to_s(&triple, hilbert_resolution, orientation) {
        Some(s) => s,
        None => return Ok(None),
    };
    let cell_id = serialize(&A5Cell {
        origin_id: origin.id,
        segment,
        s,
        resolution,
    })?;
    Ok(Some(CellCandidate {
        margin,
        cell_id,
        triple,
        flavor,
        quintant,
        hilbert_resolution,
        origin_id: origin.id,
        resolution,
    }))
}

/// Cache the winning pentagon for the dense-sample fast accept and return its id.
#[inline]
fn accept_candidate(c: &CellCandidate) -> u64 {
    let pentagon =
        get_pentagon_vertices(c.hilbert_resolution as i32, c.quintant, &c.triple, c.flavor);
    let cell_id = c.cell_id;
    let origin_id = c.origin_id;
    let resolution = c.resolution;
    LAST_RESULT.with(|cache| {
        *cache.borrow_mut() = Some(LastResult {
            cell_id,
            pentagon,
            origin_id,
            resolution,
        });
    });
    cell_id
}

// Tie margin tolerance: containment margins are cross products of unit-scale
// pentagon edges against coordinates of magnitude up to 2^hilbert_resolution,
// so their float noise is ~2^(hilbert_resolution - 52); 2^-44 gives a wide
// safety factor while staying geometrically negligible (cells are unit-size
// in the scaled frame).
const TIE_EPS: f64 = 5.684341886080802e-14; // 2^-44

/// Boundary resolution: the point has no strictly-containing pentagon in its
/// assigned frame — it lies on a cell edge, or within float noise of a
/// quintant or face seam (where the containing cell belongs to a neighboring
/// frame). Deterministically rerun the same lookup in every frame that could
/// own the point — all 5 quintants of the 3 nearest faces (a dodecahedron
/// vertex joins 3 faces; a face center joins 5 quintants). A strictly-
/// containing pentagon is unique, so the first strict hit wins; if none
/// exists the point is exactly on a boundary shared by the near-best
/// candidates, and the tie-break is the cell that comes FIRST ALONG THE CURVE
/// — the lowest cell id (origin/segment occupy the top id bits in curve
/// order, so numeric order is curve order globally).
fn spherical_to_cell_boundary(
    spherical: Spherical,
    resolution: i32,
    first_origin_id: OriginId,
    first_quintant: usize,
    first: Option<CellCandidate>,
) -> Result<u64, String> {
    let mut candidates: Vec<CellCandidate> = Vec::with_capacity(16);
    if let Some(c) = first {
        candidates.push(c);
    }
    let dodecahedron = DodecahedronProjection::get_thread_local();
    for origin in find_nearest_origins(spherical, 3) {
        let dodec_point = dodecahedron.forward(spherical, origin.id)?;
        // Try this origin's assigned quintant first, then its gamma-adjacent
        // neighbors: seam points resolve in the adjacent frame, so this order
        // finds the strict container in 1-2 lookups instead of scanning all 5.
        let q0 = get_quintant_polar(to_polar(dodec_point));
        for dq in [0usize, 1, 4, 2, 3] {
            let quintant = (q0 + dq) % 5;
            if origin.id == first_origin_id && quintant == first_quintant {
                continue;
            }
            let Some(c) = lookup_in_quintant(dodec_point, origin, quintant, resolution)? else {
                continue;
            };
            if c.margin > 0.0 {
                return Ok(accept_candidate(&c));
            }
            candidates.push(c);
        }
    }
    let mut best = f64::NEG_INFINITY;
    for c in &candidates {
        if c.margin > best {
            best = c.margin;
        }
    }
    let eps = TIE_EPS * (1u64 << (1 + resolution - FIRST_HILBERT_RESOLUTION)) as f64;
    let mut winner: Option<&CellCandidate> = None;
    for c in &candidates {
        if c.margin >= best - eps && winner.is_none_or(|w| c.cell_id < w.cell_id) {
            winner = Some(c);
        }
    }
    match winner {
        Some(w) => Ok(accept_candidate(w)),
        None => Err("spherical_to_cell: no candidate cell found".to_string()),
    }
}

/// Get the pentagon shape for a given A5 cell
pub fn get_pentagon(cell: &A5Cell) -> Result<PentagonShape, String> {
    let (quintant, orientation) = segment_to_quintant(cell.segment, cell.origin());

    if cell.resolution == FIRST_HILBERT_RESOLUTION - 1 {
        let pentagon_shape = get_quintant_vertices(quintant);
        return Ok(pentagon_shape);
    } else if cell.resolution == FIRST_HILBERT_RESOLUTION - 2 {
        let pentagon_shape = get_face_vertices();
        return Ok(pentagon_shape);
    }

    let hilbert_resolution = cell.resolution - FIRST_HILBERT_RESOLUTION + 1;
    let cell_geom = s_to_cell(cell.s, hilbert_resolution as usize, orientation);
    let pentagon_shape = get_pentagon_vertices(
        hilbert_resolution,
        quintant,
        &cell_geom.triple,
        cell_geom.flavor,
    );
    Ok(pentagon_shape)
}

/// Convert A5 cell ID to spherical coordinates of cell center
pub fn cell_to_spherical(cell: u64) -> Result<crate::coordinate_systems::Spherical, String> {
    let cell_data = deserialize(cell)?;
    let dodecahedron = DodecahedronProjection::get_thread_local();
    if cell_data.resolution >= FIRST_HILBERT_RESOLUTION {
        // Fast path: the pentagon center is O(1) from (triple, flavor) — no need
        // to construct the pentagon itself.
        let (quintant, orientation) = segment_to_quintant(cell_data.segment, cell_data.origin());
        let hilbert_resolution = cell_data.resolution - FIRST_HILBERT_RESOLUTION + 1;
        let cell_geom = s_to_cell(cell_data.s, hilbert_resolution as usize, orientation);
        let center = get_pentagon_center(
            hilbert_resolution,
            quintant,
            &cell_geom.triple,
            cell_geom.flavor,
        );
        return dodecahedron.inverse(center, cell_data.origin_id);
    }
    let pentagon = get_pentagon(&cell_data)?;
    dodecahedron.inverse(pentagon.get_center(), cell_data.origin_id)
}

/// Convert A5 cell ID to lon/lat coordinates of cell center
pub fn cell_to_lonlat(cell: u64) -> Result<LonLat, String> {
    // WORLD_CELL represents the entire world, return (0, 0) as a reasonable default
    if cell == WORLD_CELL {
        return Ok(LonLat::new(0.0, 0.0));
    }

    Ok(to_lon_lat(cell_to_spherical(cell)?))
}

/// Options for cell boundary generation
pub struct CellToBoundaryOptions {
    /// Pass true to close the ring with the first point (default: true)
    pub closed_ring: bool,
    /// Number of segments to use for each edge. Pass None to use the resolution of the cell (default: None)
    pub segments: Option<i32>,
}

impl Default for CellToBoundaryOptions {
    fn default() -> Self {
        Self {
            closed_ring: true,
            segments: None,
        }
    }
}

/// Convert A5 cell ID to boundary coordinates
pub fn cell_to_boundary(
    cell_id: u64,
    options: Option<CellToBoundaryOptions>,
) -> Result<Vec<LonLat>, String> {
    // WORLD_CELL represents the entire world and is unbounded
    if cell_id == WORLD_CELL {
        return Ok(Vec::new());
    }

    let opts = options.unwrap_or_default();
    let cell_data = deserialize(cell_id)?;

    let segments = opts
        .segments
        .unwrap_or_else(|| std::cmp::max(1, 2_i32.pow((6 - cell_data.resolution).max(0) as u32)));

    let pentagon = get_pentagon(&cell_data)?;

    // Split each edge into segments before projection
    // Important to do before projection to obtain equal area cells
    let split_pentagon = pentagon.split_edges(segments as usize);
    let vertices = split_pentagon.get_vertices_vec();

    // Unproject to obtain lon/lat coordinates
    let dodecahedron = DodecahedronProjection::get_thread_local();
    let mut unprojected_vertices = Vec::new();
    for vertex in vertices {
        let unprojected = dodecahedron.inverse(*vertex, cell_data.origin_id)?;
        unprojected_vertices.push(unprojected);
    }

    let mut boundary = Vec::new();
    for vertex in unprojected_vertices {
        boundary.push(to_lon_lat(vertex));
    }

    // Normalize longitudes to handle antimeridian crossing
    let mut normalized_boundary = normalize_longitudes(boundary);

    if opts.closed_ring {
        let first_point = normalized_boundary[0];
        normalized_boundary.push(first_point);
    }

    // TODO: This is a patch to make the boundary CCW, but we should fix the winding order of the pentagon
    // throughout the whole codebase
    normalized_boundary.reverse();
    Ok(normalized_boundary)
}

/// Test if an A5 cell contains a given point (in A5's internal spherical frame).
pub fn a5cell_contains_point(cell: &A5Cell, spherical: Spherical) -> Result<f64, String> {
    use crate::core::tiling::{get_face_vertices, get_quintant_vertices};

    let dodecahedron = DodecahedronProjection::get_thread_local();
    let projected_point = dodecahedron.forward(spherical, cell.origin_id)?;

    let (quintant, _orientation) = segment_to_quintant(cell.segment, cell.origin());

    let containment_result = if cell.resolution == FIRST_HILBERT_RESOLUTION - 1 {
        // Use quintant vertices (triangle as PentagonShape)
        let pentagon_shape = get_quintant_vertices(quintant);
        pentagon_shape.contains_point(projected_point)
    } else if cell.resolution == FIRST_HILBERT_RESOLUTION - 2 {
        // Use face vertices (pentagon)
        let pentagon_shape = get_face_vertices();
        pentagon_shape.contains_point(projected_point)
    } else {
        // Use pentagon for higher resolutions
        let pentagon = get_pentagon(cell)?;
        pentagon.contains_point(projected_point)
    };

    Ok(containment_result)
}

/// Tests whether the segment between two LonLat points intersects a cell.
///
/// The test runs entirely in the cell's Face coordinate system: both endpoints
/// are projected via the dodecahedron projection, then checked against the
/// pentagon's straight 2D edges. The segment is treated as a 2D straight line
/// in Face coords — accurate when the segment is short relative to the face
/// (DSEA distortion is negligible at sub-cell scales).
pub fn cell_intersects_segment(cell_id: u64, a: LonLat, b: LonLat) -> Result<bool, String> {
    if cell_id == WORLD_CELL {
        return Ok(true);
    }
    let cell = deserialize(cell_id)?;
    let pentagon = get_pentagon(&cell)?;
    let dodecahedron = DodecahedronProjection::get_thread_local();
    let a_face = dodecahedron.forward(from_lon_lat(a), cell.origin_id)?;
    let b_face = dodecahedron.forward(from_lon_lat(b), cell.origin_id)?;
    Ok(pentagon.intersects_segment(a_face, b_face))
}
