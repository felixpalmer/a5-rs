// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use crate::coordinate_systems::{Face, Polar};
use crate::core::constants::TWO_PI_OVER_5;
use crate::core::hilbert::{Anchor, NO, YES};
use crate::core::pentagon::{basis, pentagon, triangle, v, w, Mat2};
use crate::geometry::PentagonShape;

const TRIANGLE_MODE: bool = false;

/// A triangle shape that mimics PentagonShape behavior but with 3 vertices
#[derive(Debug, Clone)]
pub struct TriangleShape {
    vertices: [Face; 3],
}

/// A shape that can be either a pentagon or triangle for tiling operations
#[derive(Debug, Clone)]
pub enum TilingShape {
    Pentagon(PentagonShape),
    Triangle(TriangleShape),
}

impl TilingShape {
    pub fn get_vertices(&self) -> Vec<Face> {
        match self {
            TilingShape::Pentagon(p) => p.get_vertices().to_vec(),
            TilingShape::Triangle(t) => t.get_vertices().to_vec(),
        }
    }

    pub fn get_area(&self) -> f64 {
        match self {
            TilingShape::Pentagon(p) => p.get_area(),
            TilingShape::Triangle(t) => t.get_area(),
        }
    }

    pub fn get_center(&self) -> Face {
        match self {
            TilingShape::Pentagon(p) => p.get_center(),
            TilingShape::Triangle(t) => t.get_center(),
        }
    }
}

impl TriangleShape {
    pub fn new(vertices: [Face; 3]) -> Self {
        let mut triangle = Self { vertices };
        if !triangle.is_winding_correct() {
            triangle.vertices.reverse();
        }
        triangle
    }

    pub fn get_vertices(&self) -> &[Face; 3] {
        &self.vertices
    }

    pub fn get_area(&self) -> f64 {
        let mut signed_area = 0.0;
        for i in 0..3 {
            let j = (i + 1) % 3;
            signed_area += (self.vertices[j].x() - self.vertices[i].x())
                * (self.vertices[j].y() + self.vertices[i].y());
        }
        signed_area
    }

    fn is_winding_correct(&self) -> bool {
        self.get_area() >= 0.0
    }

    pub fn get_center(&self) -> Face {
        let sum_x = (self.vertices[0].x() + self.vertices[1].x() + self.vertices[2].x()) / 3.0;
        let sum_y = (self.vertices[0].y() + self.vertices[1].y() + self.vertices[2].y()) / 3.0;
        Face::new(sum_x, sum_y)
    }
}

/// Shift right vector (clone of w)
fn shift_right() -> Face {
    w()
}

/// Shift left vector (negative w)
fn shift_left() -> Face {
    let w_vec = w();
    Face::new(-w_vec.x(), -w_vec.y())
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
    if transformed_vertices.len() >= 5 {
        let pentagon_vertices: [Face; 5] = [
            transformed_vertices[0], 
            transformed_vertices[1], 
            transformed_vertices[2], 
            transformed_vertices[3], 
            transformed_vertices[4]
        ];
        *pentagon = PentagonShape::new(pentagon_vertices);
    }
}


/// Transform a triangle shape using a 2x2 matrix
fn transform_triangle(triangle: &mut TriangleShape, matrix: &Mat2) {
    let vertices = triangle.get_vertices();
    let mut transformed_vertices = [Face::new(0.0, 0.0); 3];
    
    for i in 0..3 {
        let vertex = &vertices[i];
        let transformed_x = matrix.m00 * vertex.x() + matrix.m01 * vertex.y();
        let transformed_y = matrix.m10 * vertex.x() + matrix.m11 * vertex.y();
        transformed_vertices[i] = Face::new(transformed_x, transformed_y);
    }
    
    // Create new triangle with transformed vertices
    *triangle = TriangleShape::new(transformed_vertices);
}

/// Get pentagon vertices with transformations applied
/// 
/// # Arguments
/// 
/// * `resolution` - The resolution level
/// * `quintant` - The quintant index (0-4)  
/// * `anchor` - The anchor information containing offset and flip data
/// 
/// # Returns
/// 
/// A pentagon shape with transformed vertices
pub fn get_pentagon_vertices(resolution: i32, quintant: usize, anchor: &Anchor) -> TilingShape {
    let mut pentagon_shape = if TRIANGLE_MODE {
        triangle().clone()
    } else {
        pentagon().clone()
    };

    // Transform anchor offset using basis matrix
    let basis_mat = basis();
    let translation_x = basis_mat.m00 * anchor.offset.x() + basis_mat.m01 * anchor.offset.y();
    let translation_y = basis_mat.m10 * anchor.offset.x() + basis_mat.m11 * anchor.offset.y();
    let translation = Face::new(translation_x, translation_y);

    // Apply transformations based on anchor properties
    if anchor.flips[0] == NO && anchor.flips[1] == YES {
        pentagon_shape.rotate180();
    }

    let k = anchor.k;
    let f = anchor.flips[0] + anchor.flips[1];
    
    if 
        // Orient last two pentagons when both or neither flips are YES
        ((f == -2 || f == 2) && k > 1) ||
        // Orient first & last pentagons when only one of flips is YES  
        (f == 0 && (k == 0 || k == 3))
    {
        pentagon_shape.reflect_y();
    }

    if anchor.flips[0] == YES && anchor.flips[1] == YES {
        pentagon_shape.rotate180();
    } else if anchor.flips[0] == YES {
        pentagon_shape.translate(shift_left());
    } else if anchor.flips[1] == YES {
        pentagon_shape.translate(shift_right());
    }

    // Position within quintant
    pentagon_shape.translate(translation);
    pentagon_shape.scale(1.0 / (2.0_f64.powi(resolution)));
    
    let rotations = quintant_rotations();
    transform_pentagon(&mut pentagon_shape, &rotations[quintant]);

    TilingShape::Pentagon(pentagon_shape)
}

/// Get quintant vertices
/// 
/// # Arguments
/// 
/// * `quintant` - The quintant index (0-4)
/// 
/// # Returns
/// 
/// Triangle vertices for the specified quintant wrapped as TilingShape
pub fn get_quintant_vertices(quintant: usize) -> TilingShape {
    // Create proper 3-vertex triangle from the triangle vertices
    let triangle_verts = triangle().get_vertices();
    let triangle_3_verts = [triangle_verts[0], triangle_verts[1], triangle_verts[2]];
    
    let mut triangle_shape = TriangleShape::new(triangle_3_verts);
    let rotations = quintant_rotations();
    transform_triangle(&mut triangle_shape, &rotations[quintant]);
    TilingShape::Triangle(triangle_shape)
}

/// Get face vertices with correct winding order
/// 
/// # Returns
/// 
/// Pentagon shape representing the face vertices
pub fn get_face_vertices() -> TilingShape {
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
        vertices[0], vertices[1], vertices[2], vertices[3], vertices[4]
    ];
    TilingShape::Pentagon(PentagonShape::new(pentagon_vertices))
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