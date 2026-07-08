// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use crate::coordinate_systems::{Face, Polar};
use crate::core::constants::TWO_PI_OVER_5;
use crate::core::pentagon::{basis, pentagon, triangle, v, Mat2};
use crate::geometry::PentagonShape;
use crate::lattice::Triple;

const TRIANGLE_MODE: bool = false;

/// Center of the base PENTAGON under each flavor's orientation ops. The vertex
/// mean is linear, so an oriented pentagon's center is the transformed base
/// center — no need to construct the five vertices when only the center is
/// wanted (see `get_pentagon_center`).
fn flavor_centers() -> [Face; 4] {
    let mut centers = [Face::new(0.0, 0.0); 4];
    for (flavor, center) in centers.iter_mut().enumerate() {
        let mut p = pentagon().clone();
        if flavor & 1 == 1 {
            p.rotate180();
        }
        if flavor & 2 == 2 {
            p.reflect_y();
        }
        *center = p.get_center();
    }
    centers
}

/// Generate quintant rotation matrices
fn quintant_rotations() -> [Mat2; 5] {
    let mut rotations = [Mat2::new(1.0, 0.0, 0.0, 1.0); 5];

    for (quintant, rotation) in rotations.iter_mut().enumerate() {
        let angle = (TWO_PI_OVER_5).0 * quintant as f64;
        let cos_angle = angle.cos();
        let sin_angle = angle.sin();
        *rotation = Mat2::new(cos_angle, -sin_angle, sin_angle, cos_angle);
    }

    rotations
}

/// Transform a pentagon shape using a 2x2 matrix
fn transform_pentagon(pentagon: &mut PentagonShape, matrix: &Mat2) {
    let vertices = pentagon.get_vertices_vec();
    let mut transformed_vertices = Vec::new();

    for vertex in vertices {
        let transformed_x = matrix.m00 * vertex.x() + matrix.m01 * vertex.y();
        let transformed_y = matrix.m10 * vertex.x() + matrix.m11 * vertex.y();
        transformed_vertices.push(Face::new(transformed_x, transformed_y));
    }

    // Create new pentagon with transformed vertices - need 5 for Pentagon type
    if transformed_vertices.len() == 5 {
        let pentagon_vertices: [Face; 5] = [
            transformed_vertices[0],
            transformed_vertices[1],
            transformed_vertices[2],
            transformed_vertices[3],
            transformed_vertices[4],
        ];
        *pentagon = PentagonShape::new(pentagon_vertices);
    } else if transformed_vertices.len() == 3 {
        let pentagon_vertices: [Face; 3] = [
            transformed_vertices[0],
            transformed_vertices[1],
            transformed_vertices[2],
        ];
        *pentagon = PentagonShape::new_triangle(pentagon_vertices);
    }
}

/// Get pentagon vertices for a cell.
///
/// A cell's pentagon is one of exactly FOUR orientations of the base PENTAGON
/// (the Cairo-like metatile): flavor bit 0 is a 180° rotation, bit 1 a Y
/// reflection. The oriented pentagon sits at the triple-derived lattice point
/// ref = (x+y, -x) in IJ, shifted by one j unit for the rotated flavors.
/// The flavor is a 1:1 function of the cell's L-system jigsaw piece and is
/// produced by the descent (`s_to_cell`); the placement was derived and verified
/// exhaustively against the pentagon geometry.
///
/// # Arguments
///
/// * `resolution` - The resolution level
/// * `quintant` - The quintant index (0-4)
/// * `triple` - The cell's triple coordinates
/// * `flavor` - The cell's pentagon flavor (0-3)
///
/// # Returns
///
/// A pentagon shape with transformed vertices
pub fn get_pentagon_vertices(
    resolution: i32,
    quintant: usize,
    triple: &Triple,
    flavor: u8,
) -> PentagonShape {
    let mut pentagon_shape = if TRIANGLE_MODE {
        triangle().clone()
    } else {
        pentagon().clone()
    };

    if flavor & 1 == 1 {
        pentagon_shape.rotate180();
    }
    if flavor & 2 == 2 {
        pentagon_shape.reflect_y();
    }

    // Position within quintant: ref(triple), plus (0, 1) for the rotated flavors
    let ref_ij = Face::new(
        (triple.x + triple.y) as f64,
        (-triple.x + (flavor & 1) as i32) as f64,
    );
    let translation = basis().transform(ref_ij);
    pentagon_shape.translate(translation);
    pentagon_shape.scale(1.0 / (2.0_f64.powi(resolution)));

    let rotations = quintant_rotations();
    transform_pentagon(&mut pentagon_shape, &rotations[quintant]);

    pentagon_shape
}

/// The center of a cell's pentagon, without constructing the pentagon —
/// O(1) via the precomputed flavor centers. Equivalent to
/// `get_pentagon_vertices(...).get_center()` (up to float associativity).
pub fn get_pentagon_center(resolution: i32, quintant: usize, triple: &Triple, flavor: u8) -> Face {
    let c = flavor_centers()[flavor as usize];
    let ref_ij = Face::new(
        (triple.x + triple.y) as f64,
        (-triple.x + (flavor & 1) as i32) as f64,
    );
    let translation = basis().transform(ref_ij);
    let scale = 2.0_f64.powi(resolution);
    let out = Face::new(
        (c.x() + translation.x()) / scale,
        (c.y() + translation.y()) / scale,
    );
    quintant_rotations()[quintant].transform(out)
}

/// Get quintant vertices
///
/// # Arguments
///
/// * `quintant` - The quintant index (0-4)
///
/// # Returns
///
/// Triangle vertices for the specified quintant as PentagonShape
pub fn get_quintant_vertices(quintant: usize) -> crate::geometry::pentagon::PentagonShape {
    // Create proper 3-vertex triangle from the triangle vertices
    let triangle_verts = triangle().get_vertices();
    let triangle_3_verts = [triangle_verts[0], triangle_verts[1], triangle_verts[2]];

    let mut pentagon_shape =
        crate::geometry::pentagon::PentagonShape::new_triangle(triangle_3_verts);
    let rotations = quintant_rotations();
    transform_pentagon(&mut pentagon_shape, &rotations[quintant]);
    pentagon_shape
}

/// Get face vertices with correct winding order
///
/// # Returns
///
/// Pentagon shape representing the face vertices
pub fn get_face_vertices() -> PentagonShape {
    let mut vertices = Vec::new();
    let v_vertex = v();
    let rotations = quintant_rotations();

    for rotation in &rotations {
        // Transform v vertex by rotation matrix
        let transformed_x = rotation.m00 * v_vertex.x() + rotation.m01 * v_vertex.y();
        let transformed_y = rotation.m10 * v_vertex.x() + rotation.m11 * v_vertex.y();
        vertices.push(Face::new(transformed_x, transformed_y));
    }

    // Need to reverse to obtain correct winding order
    vertices.reverse();

    // Convert Vec to array for PentagonShape::new
    let pentagon_vertices: [Face; 5] = [
        vertices[0],
        vertices[1],
        vertices[2],
        vertices[3],
        vertices[4],
    ];
    PentagonShape::new(pentagon_vertices)
}

/// Get quintant from polar coordinates
///
/// # Arguments
///
/// * `polar` - Polar coordinates [rho, gamma]
///
/// # Returns  
///
/// The quintant index (0-4)
pub fn get_quintant_polar(polar: Polar) -> usize {
    let gamma = polar.gamma().0; // Extract f64 from Radians
    ((gamma / (TWO_PI_OVER_5).0).round() as i32 + 5) as usize % 5
}
