// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use crate::coordinate_systems::{Face, LonLat};
use crate::core::constants::PI_OVER_5;
use crate::core::coordinate_transforms::{
    face_to_ij, from_lon_lat, normalize_longitudes, to_lon_lat, to_polar,
};
use crate::core::hilbert::{ij_to_s, s_to_anchor};
use crate::core::origin::{find_nearest_origin, quintant_to_segment, segment_to_quintant};
use crate::core::serialization::{deserialize, serialize, FIRST_HILBERT_RESOLUTION, WORLD_CELL};
use crate::core::tiling::{
    get_face_vertices, get_pentagon_vertices, get_quintant_polar, get_quintant_vertices,
};
use crate::core::utils::{A5Cell, OriginId};
use crate::geometry::pentagon::PentagonShape;
use crate::projections::dodecahedron::DodecahedronProjection;
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
    // Resolution -1 represents WORLD_CELL, which covers the entire world
    if resolution == -1 {
        return Ok(WORLD_CELL);
    }

    if resolution < FIRST_HILBERT_RESOLUTION {
        // For low resolutions there is no Hilbert curve, so we can just return as the result is exact
        let estimate = lonlat_to_estimate(lonlat, resolution)?;
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
        let spherical = from_lon_lat(lonlat);
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
    let first_estimate = lonlat_to_estimate(lonlat, resolution)?;
    let first_key = serialize(&first_estimate)?;
    let first_distance = a5cell_contains_point(&first_estimate, lonlat)?;
    if first_distance > 0.0 {
        return cache_result(&first_estimate, first_key, resolution);
    }

    // Spiral search: perturb lonLat to find nearby estimate cells (the projection
    // approximation can land in a neighbor at pentagon boundaries). Samples are
    // generated lazily — if the first sample hits we skip 25 trig+alloc ops.
    let hilbert_resolution = 1 + resolution - FIRST_HILBERT_RESOLUTION;
    let n = 25;
    let scale = 50.0 / 2.0_f64.powi(hilbert_resolution);
    let mut estimate_set: HashSet<u64> = HashSet::new();
    estimate_set.insert(first_key);
    let mut cells: Vec<(A5Cell, f64)> = vec![(first_estimate, first_distance)];

    // i=0 yields R=0 → same as the original sample, so start at i=1.
    for i in 1..n {
        let r = (i as f64 / n as f64) * scale;
        let sample = LonLat::new(
            lonlat.longitude() + (i as f64).cos() * r,
            lonlat.latitude() + (i as f64).sin() * r,
        );
        let estimate = lonlat_to_estimate(sample, resolution)?;
        let estimate_key = serialize(&estimate)?;
        if estimate_set.contains(&estimate_key) {
            continue;
        }
        estimate_set.insert(estimate_key);
        let distance = a5cell_contains_point(&estimate, lonlat)?;
        if distance > 0.0 {
            return cache_result(&estimate, estimate_key, resolution);
        }
        cells.push((estimate, distance));
    }

    // Fallback: pick the closest estimate. Cache it so subsequent dense-sample
    // calls still benefit even though this lookup was approximate.
    cells.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    let (fallback, _) = &cells[0];
    let fallback_key = serialize(fallback)?;
    cache_result(fallback, fallback_key, resolution)
}

/// The ij_to_s function uses the triangular lattice which only approximates the pentagon lattice
/// Thus this function only returns a cell nearby, and we need to search the neighbourhood to find the correct cell
/// TODO: Implement a more accurate function
fn lonlat_to_estimate(lonlat: LonLat, resolution: i32) -> Result<A5Cell, String> {
    let spherical = from_lon_lat(lonlat);
    let origin = find_nearest_origin(spherical);

    let dodecahedron = DodecahedronProjection::get_thread_local();
    let mut dodec_point = dodecahedron.forward(spherical, origin.id)?;
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
    let s_u64 = cell
        .s
        .to_string()
        .parse::<u64>()
        .map_err(|_| "Failed to convert BigInt to u64")?;
    let anchor = s_to_anchor(s_u64, hilbert_resolution as usize, orientation);
    let pentagon_shape = get_pentagon_vertices(hilbert_resolution, quintant, &anchor);
    Ok(pentagon_shape)
}

/// Convert A5 cell ID to spherical coordinates of cell center
pub fn cell_to_spherical(cell: u64) -> Result<crate::coordinate_systems::Spherical, String> {
    let cell_data = deserialize(cell)?;
    let pentagon = get_pentagon(&cell_data)?;
    let dodecahedron = DodecahedronProjection::get_thread_local();
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

/// Test if an A5 cell contains a given point
pub fn a5cell_contains_point(cell: &A5Cell, point: LonLat) -> Result<f64, String> {
    use crate::core::tiling::{get_face_vertices, get_quintant_vertices};

    let spherical = from_lon_lat(point);
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
