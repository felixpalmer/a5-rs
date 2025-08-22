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
use crate::core::serialization::{deserialize, serialize, FIRST_HILBERT_RESOLUTION};
use crate::core::tiling::{
    get_face_vertices, get_pentagon_vertices, get_quintant_polar, get_quintant_vertices,
};
use crate::core::utils::A5Cell;
use crate::geometry::pentagon::PentagonShape;
use crate::projections::dodecahedron::DodecahedronProjection;
use num_bigint::BigInt;
use std::collections::HashSet;

/// Convert lon/lat coordinates to A5 cell ID
pub fn lonlat_to_cell(lonlat: LonLat, resolution: i32) -> Result<u64, String> {
    if resolution < FIRST_HILBERT_RESOLUTION {
        // For low resolutions there is no Hilbert curve, so we can just return as the result is exact
        let estimate = lonlat_to_estimate(lonlat, resolution)?;
        return serialize(&estimate);
    }

    let hilbert_resolution = 1 + resolution - FIRST_HILBERT_RESOLUTION;
    let mut samples = vec![lonlat];
    let n = 25;
    let scale = 50.0 / 2.0_f64.powi(hilbert_resolution);

    for i in 0..n {
        let r = (i as f64 / n as f64) * scale;
        let coordinate = LonLat::new(
            lonlat.longitude() + (i as f64).cos() * r,
            lonlat.latitude() + (i as f64).sin() * r,
        );
        samples.push(coordinate);
    }

    // Deduplicate estimates
    let mut estimate_set = HashSet::new();
    let mut unique_estimates = Vec::new();
    let mut cells = Vec::new();

    for sample in samples {
        let estimate = lonlat_to_estimate(sample, resolution)?;
        let estimate_key = serialize(&estimate)?.to_string();
        if !estimate_set.contains(&estimate_key) {
            estimate_set.insert(estimate_key.clone());
            unique_estimates.push(estimate.clone());

            // Check if we have a hit, storing distance if not
            let distance = a5cell_contains_point(&estimate, sample)?;
            if distance > 0.0 {
                return serialize(&estimate);
            } else {
                cells.push((estimate, distance));
            }
        }
    }

    // As fallback, sort cells by distance and use the closest one
    cells.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    serialize(&cells[0].0)
}

/// The ij_to_s function uses the triangular lattice which only approximates the pentagon lattice
/// Thus this function only returns a cell nearby, and we need to search the neighbourhood to find the correct cell
/// TODO: Implement a more accurate function
fn lonlat_to_estimate(lonlat: LonLat, resolution: i32) -> Result<A5Cell, String> {
    let spherical = from_lon_lat(lonlat);
    let origin = find_nearest_origin(spherical);

    let mut dodecahedron = DodecahedronProjection::new()?;
    let mut dodec_point = dodecahedron.forward(spherical, origin.id)?;
    let polar = to_polar(dodec_point);
    let quintant = get_quintant_polar(polar);
    let (segment, orientation) = quintant_to_segment(quintant, origin);

    if resolution < FIRST_HILBERT_RESOLUTION {
        // For low resolutions there is no Hilbert curve
        return Ok(A5Cell {
            s: BigInt::from(0),
            segment,
            origin: (*origin).clone(),
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
        s: BigInt::from(s),
        segment,
        origin: (*origin).clone(),
        resolution,
    })
}

/// Get the pentagon shape for a given A5 cell
pub fn get_pentagon(cell: &A5Cell) -> Result<PentagonShape, String> {
    let (quintant, orientation) = segment_to_quintant(cell.segment, &cell.origin);

    if cell.resolution == FIRST_HILBERT_RESOLUTION - 1 {
        let tiling_shape = get_quintant_vertices(quintant);
        return match tiling_shape {
            crate::core::tiling::TilingShape::Pentagon(p) => Ok(p),
            crate::core::tiling::TilingShape::Triangle(t) => {
                // Convert triangle to pentagon shape by using the triangle vertices
                // This is a fallback - the triangle represents a triangular region that approximates a pentagon
                let vertices = t.get_vertices();
                let pentagon_vertices = [
                    vertices[0],
                    vertices[1],
                    vertices[2],
                    vertices[0],
                    vertices[1], // Repeat first two to make 5 vertices
                ];
                Ok(crate::geometry::pentagon::PentagonShape::new(
                    pentagon_vertices,
                ))
            }
        };
    } else if cell.resolution == FIRST_HILBERT_RESOLUTION - 2 {
        let tiling_shape = get_face_vertices();
        return match tiling_shape {
            crate::core::tiling::TilingShape::Pentagon(p) => Ok(p),
            crate::core::tiling::TilingShape::Triangle(t) => {
                let vertices = t.get_vertices();
                let pentagon_vertices = [
                    vertices[0],
                    vertices[1],
                    vertices[2],
                    vertices[0],
                    vertices[1], // Repeat first two to make 5 vertices
                ];
                Ok(crate::geometry::pentagon::PentagonShape::new(
                    pentagon_vertices,
                ))
            }
        };
    }

    let hilbert_resolution = cell.resolution - FIRST_HILBERT_RESOLUTION + 1;
    let s_u64 = cell
        .s
        .to_string()
        .parse::<u64>()
        .map_err(|_| "Failed to convert BigInt to u64")?;
    let anchor = s_to_anchor(s_u64, hilbert_resolution as usize, orientation);
    let tiling_shape = get_pentagon_vertices(hilbert_resolution, quintant, &anchor);
    match tiling_shape {
        crate::core::tiling::TilingShape::Pentagon(p) => Ok(p),
        crate::core::tiling::TilingShape::Triangle(t) => {
            let vertices = t.get_vertices();
            let pentagon_vertices = [
                vertices[0],
                vertices[1],
                vertices[2],
                vertices[0],
                vertices[1], // Repeat first two to make 5 vertices
            ];
            Ok(crate::geometry::pentagon::PentagonShape::new(
                pentagon_vertices,
            ))
        }
    }
}

/// Convert A5 cell ID to lon/lat coordinates of cell center
pub fn cell_to_lonlat(cell: u64) -> Result<LonLat, String> {
    let cell_data = deserialize(cell)?;
    let pentagon = get_pentagon(&cell_data)?;
    let mut dodecahedron = DodecahedronProjection::new()?;
    let point = dodecahedron.inverse(pentagon.get_center(), cell_data.origin.id)?;
    Ok(to_lon_lat(point))
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
    let opts = options.unwrap_or_default();
    let cell_data = deserialize(cell_id)?;

    let segments = opts
        .segments
        .unwrap_or_else(|| std::cmp::max(1, 2_i32.pow((6 - cell_data.resolution).max(0) as u32)));

    let pentagon = get_pentagon(&cell_data)?;

    // Split each edge into segments before projection
    // Important to do before projection to obtain equal area cells
    let split_pentagon = pentagon.split_edges(segments as usize);
    let vertices = split_pentagon.get_vertices();

    // Unproject to obtain lon/lat coordinates
    let mut dodecahedron = DodecahedronProjection::new()?;
    let mut unprojected_vertices = Vec::new();
    for vertex in vertices {
        let unprojected = dodecahedron.inverse(vertex, cell_data.origin.id)?;
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
    let pentagon = get_pentagon(cell)?;
    let spherical = from_lon_lat(point);
    let mut dodecahedron = DodecahedronProjection::new()?;
    let projected_point = dodecahedron.forward(spherical, cell.origin.id)?;
    Ok(pentagon.contains_point(projected_point))
}
