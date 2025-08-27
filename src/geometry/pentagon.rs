// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use crate::coordinate_systems::Face;

pub type Pentagon = [Face; 5];
pub type Triangle = [Face; 3];

#[derive(Debug, Clone)]
pub struct PentagonShape {
    vertices: Vec<Face>,
}

impl PentagonShape {
    pub fn new(vertices: Pentagon) -> Self {
        let mut pentagon = Self {
            vertices: vertices.to_vec(),
        };
        if !pentagon.is_winding_correct() {
            pentagon.vertices.reverse();
        }
        pentagon
    }

    pub fn new_triangle(vertices: Triangle) -> Self {
        let mut pentagon = Self {
            vertices: vertices.to_vec(),
        };
        if !pentagon.is_winding_correct() {
            pentagon.vertices.reverse();
        }
        pentagon
    }

    fn from_vertices(vertices: Vec<Face>) -> Self {
        let mut pentagon = Self { vertices };
        if !pentagon.is_winding_correct() {
            pentagon.vertices.reverse();
        }
        pentagon
    }

    pub fn get_area(&self) -> f64 {
        let mut signed_area = 0.0;
        let n = self.vertices.len();
        for i in 0..n {
            let j = (i + 1) % n;
            signed_area += (self.vertices[j].x() - self.vertices[i].x())
                * (self.vertices[j].y() + self.vertices[i].y());
        }
        signed_area
    }

    fn is_winding_correct(&self) -> bool {
        self.get_area() >= 0.0
    }

    pub fn get_vertices(&self) -> Pentagon {
        let mut pentagon = [Face::new(0.0, 0.0); 5];
        for (i, vertex) in self.vertices.iter().enumerate().take(5) {
            pentagon[i] = *vertex;
        }
        pentagon
    }

    pub fn get_vertices_vec(&self) -> &Vec<Face> {
        &self.vertices
    }

    pub fn scale(&mut self, scale: f64) -> &mut Self {
        for vertex in &mut self.vertices {
            *vertex = Face::new(vertex.x() * scale, vertex.y() * scale);
        }
        self
    }

    /// Rotates the pentagon 180 degrees (equivalent to negating x & y)
    /// Returns the rotated pentagon
    pub fn rotate180(&mut self) -> &mut Self {
        for vertex in &mut self.vertices {
            *vertex = Face::new(-vertex.x(), -vertex.y());
        }
        self
    }

    /// Reflects the pentagon over the x-axis (equivalent to negating y)
    /// and reverses the winding order to maintain consistent orientation
    /// Returns the reflected pentagon
    pub fn reflect_y(&mut self) -> &mut Self {
        // First reflect all vertices
        for vertex in &mut self.vertices {
            *vertex = Face::new(vertex.x(), -vertex.y());
        }

        // Then reverse the winding order to maintain consistent orientation
        self.vertices.reverse();

        self
    }

    pub fn translate(&mut self, translation: Face) -> &mut Self {
        for vertex in &mut self.vertices {
            *vertex = Face::new(vertex.x() + translation.x(), vertex.y() + translation.y());
        }
        self
    }

    pub fn get_center(&self) -> Face {
        let n = self.vertices.len() as f64;
        let (sum_x, sum_y) = self.vertices.iter().fold((0.0, 0.0), |(sum_x, sum_y), v| {
            (sum_x + v.x() / n, sum_y + v.y() / n)
        });
        Face::new(sum_x, sum_y)
    }

    /// Tests if a point is inside the pentagon by checking if it's on the correct side of all edges.
    /// Assumes consistent winding order (counter-clockwise).
    /// Returns 1 if point is inside, otherwise a negative value proportional to the distance from the point to the edge
    pub fn contains_point(&self, point: Face) -> f64 {
        // TODO later we can likely remove this, but for now it's useful for debugging
        if !self.is_winding_correct() {
            panic!("Pentagon is not counter-clockwise");
        }

        let n = self.vertices.len();
        let mut d_max: f64 = 1.0;
        for i in 0..n {
            let v1 = self.vertices[i];
            let v2 = self.vertices[(i + 1) % n];

            // Calculate the cross product to determine which side of the line the point is on
            // (v1 - v2) × (point - v1)
            let dx = v1.x() - v2.x();
            let dy = v1.y() - v2.y();
            let px = point.x() - v1.x();
            let py = point.y() - v1.y();

            // Cross product: dx * py - dy * px
            // If positive, point is on the wrong side
            // If negative, point is on the correct side
            let cross_product = dx * py - dy * px;
            if cross_product < 0.0 {
                // Only normalize by distance of point to edge as we can assume the edges of the
                // pentagon are all the same length
                let p_length = (px * px + py * py).sqrt();
                d_max = d_max.min(cross_product / p_length);
            }
        }

        d_max
    }

    /// Splits each edge of the pentagon into the specified number of segments
    /// Returns a new PentagonShape with more vertices, or the original PentagonShape if segments <= 1
    pub fn split_edges(&self, segments: usize) -> PentagonShape {
        if segments <= 1 {
            return self.clone();
        }

        let mut new_vertices = Vec::new();
        let n = self.vertices.len();

        for i in 0..n {
            let v1 = self.vertices[i];
            let v2 = self.vertices[(i + 1) % n];

            // Add the current vertex
            new_vertices.push(v1);

            // Add interpolated points along the edge (excluding the endpoints)
            for j in 1..segments {
                let t = j as f64 / segments as f64;
                let interpolated = Face::new(
                    v1.x() + t * (v2.x() - v1.x()),
                    v1.y() + t * (v2.y() - v1.y()),
                );
                new_vertices.push(interpolated);
            }
        }

        PentagonShape::from_vertices(new_vertices)
    }
}
