// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use crate::coordinate_systems::{Cartesian, Face, LonLat, Spherical};
use crate::core::constants::PI_OVER_5;
use crate::core::coordinate_transforms::{
    face_to_ij, from_lon_lat, normalize_longitudes, to_lon_lat, to_polar,
};
use crate::core::origin::{
    find_nearest_origin, find_nearest_origin_cartesian, quintant_to_segment, segment_to_quintant,
};
use crate::core::serialization::{deserialize, serialize, FIRST_HILBERT_RESOLUTION, WORLD_CELL};
use crate::core::tiling::{
    get_face_vertices, get_pentagon_center, get_pentagon_vertices, get_quintant_polar,
    get_quintant_vertices,
};
use crate::core::utils::{A5Cell, Origin, OriginId};
use crate::geometry::pentagon::PentagonShape;
use crate::lattice::{ij_to_s, s_to_cell};
use crate::projections::dodecahedron::DodecahedronProjection;
use crate::traversal::global_neighbors::get_global_cell_neighbors;
use crate::utils::spiral::{Spiral, SPIRAL_SAMPLE_COUNT};
use std::cell::RefCell;
use std::collections::HashSet;

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

/// Update the single-entry cache with a successful (cell, cell_id) pair.
fn cache_result(cell: &A5Cell, cell_id: u64, resolution: i32) -> Result<u64, String> {
    let pentagon = get_pentagon(cell)?;
    let origin_id = cell.origin_id;
    LAST_RESULT.with(|c| {
        *c.borrow_mut() = Some(LastResult {
            cell_id,
            pentagon,
            origin_id,
            resolution,
        });
    });
    Ok(cell_id)
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
        // For low resolutions there is no Hilbert curve, so we can just return as the result is exact
        let estimate = spherical_to_estimate(spherical, resolution)?;
        return serialize(&estimate);
    }

    // Try the cached pentagon first — skips the full estimate pipeline when
    // consecutive calls land in the same cell (common in dense-sample loops).
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

    // Try the original point's projection-based estimate. Common case for
    // non-boundary points.
    let first_estimate = spherical_to_estimate(spherical, resolution)?;
    let first_key = serialize(&first_estimate)?;
    let first_distance = a5cell_contains_point(&first_estimate, spherical)?;
    if first_distance > 0.0 {
        return cache_result(&first_estimate, first_key, resolution);
    }

    // Spiral search: perturb the point in the tangent plane to find nearby
    // estimate cells (see src/utils/spiral.rs).
    let hilbert_resolution = 1 + resolution - FIRST_HILBERT_RESOLUTION;
    let scale = SPIRAL_SCALE_RAD / 2.0_f64.powi(hilbert_resolution);
    let mut estimate_set: HashSet<u64> = HashSet::new();
    estimate_set.insert(first_key);
    let mut cells: Vec<(u64, f64)> = vec![(first_key, first_distance)];

    let spiral = Spiral::new(spherical, scale);
    for i in 0..SPIRAL_SAMPLE_COUNT {
        let estimate = cartesian_to_estimate(spiral.sample(i), resolution)?;
        let estimate_key = serialize(&estimate)?;
        if estimate_set.contains(&estimate_key) {
            continue;
        }
        estimate_set.insert(estimate_key);
        let distance = a5cell_contains_point(&estimate, spherical)?;
        if distance > 0.0 {
            return cache_result(&estimate, estimate_key, resolution);
        }
        cells.push((estimate_key, distance));
    }

    // Spiral exhausted without finding a strict container. This is reachable
    // for points right at the polar singularity at very high resolutions,
    // where re-projecting any tangent sample snaps back to a small set of
    // cells while the geometrically-containing cell is offset by one
    // adjacency step. Fall back to direct neighbours of the closest spiral
    // candidate, which always finds it.
    cells.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    let k = std::cmp::min(3, cells.len());
    for j in 0..k {
        let neighbors = get_global_cell_neighbors(cells[j].0, false);
        for neighbor_key in neighbors {
            if estimate_set.contains(&neighbor_key) {
                continue;
            }
            estimate_set.insert(neighbor_key);
            let neighbor_cell = deserialize(neighbor_key)?;
            let distance = a5cell_contains_point(&neighbor_cell, spherical)?;
            if distance > 0.0 {
                return cache_result(&neighbor_cell, neighbor_key, resolution);
            }
            cells.push((neighbor_key, distance));
        }
    }

    // True fallback: closest cell wins, even if technically just outside.
    cells.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    let fallback_key = cells[0].0;
    let fallback = deserialize(fallback_key)?;
    cache_result(&fallback, fallback_key, resolution)
}

// Spiral perturbation radius at hilbertResolution=1 (in radians of tangent
// offset). For higher resolutions we scale by 1/2^hilbertResolution.
const SPIRAL_SCALE_RAD: f64 = 70.0 * std::f64::consts::PI / 180.0;

/// The ij_to_s function uses the triangular lattice which only approximates the pentagon lattice
/// Thus these functions only return a cell nearby, and we need to search the neighbourhood to find the correct cell
/// TODO: Implement a more accurate function
fn spherical_to_estimate(spherical: Spherical, resolution: i32) -> Result<A5Cell, String> {
    let origin = find_nearest_origin(spherical);
    let dodecahedron = DodecahedronProjection::get_thread_local();
    let dodec_point = dodecahedron.forward(spherical, origin.id)?;
    face_to_estimate(dodec_point, origin, resolution)
}

fn cartesian_to_estimate(cartesian: Cartesian, resolution: i32) -> Result<A5Cell, String> {
    let origin = find_nearest_origin_cartesian(cartesian);
    let dodecahedron = DodecahedronProjection::get_thread_local();
    let dodec_point = dodecahedron.forward_cartesian(cartesian, origin.id)?;
    face_to_estimate(dodec_point, origin, resolution)
}

fn face_to_estimate(
    mut dodec_point: Face,
    origin: &Origin,
    resolution: i32,
) -> Result<A5Cell, String> {
    let polar = to_polar(dodec_point);
    let quintant = get_quintant_polar(polar);
    let (segment, orientation) = quintant_to_segment(quintant, origin);

    if resolution < FIRST_HILBERT_RESOLUTION {
        // For low resolutions there is no Hilbert curve
        return Ok(A5Cell {
            s: 0,
            segment,
            origin_id: origin.id,
            resolution,
        });
    }

    // Rotate into right fifth
    if quintant != 0 {
        let extra_angle = 2.0 * PI_OVER_5.get() * quintant as f64;
        let cos_angle = (-extra_angle).cos();
        let sin_angle = (-extra_angle).sin();
        let rotated_x = cos_angle * dodec_point.x() - sin_angle * dodec_point.y();
        let rotated_y = sin_angle * dodec_point.x() + cos_angle * dodec_point.y();
        dodec_point = Face::new(rotated_x, rotated_y);
    }

    let hilbert_resolution = 1 + resolution - FIRST_HILBERT_RESOLUTION;
    let scale_factor = 2.0_f64.powi(hilbert_resolution);
    dodec_point = Face::new(
        dodec_point.x() * scale_factor,
        dodec_point.y() * scale_factor,
    );

    let ij = face_to_ij(dodec_point);
    let s = ij_to_s(ij, hilbert_resolution as usize, orientation);

    Ok(A5Cell {
        s,
        segment,
        origin_id: origin.id,
        resolution,
    })
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
