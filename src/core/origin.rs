// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use crate::coordinate_systems::{Radians, Spherical};
use crate::core::constants::{INTERHEDRAL_ANGLE, PI_OVER_5, TWO_PI_OVER_5};
use crate::core::dodecahedron_quaternions::QUATERNIONS;
use crate::core::hilbert::Orientation;
use crate::core::utils::{Origin, OriginId, Quat};

// Quintant layouts (clockwise & counterclockwise)
pub const CLOCKWISE_FAN: [Orientation; 5] = [
    Orientation::VU,
    Orientation::UW,
    Orientation::VW,
    Orientation::VW,
    Orientation::VW,
];

pub const CLOCKWISE_STEP: [Orientation; 5] = [
    Orientation::WU,
    Orientation::UW,
    Orientation::VW,
    Orientation::VU,
    Orientation::UW,
];

pub const COUNTER_STEP: [Orientation; 5] = [
    Orientation::WU,
    Orientation::UV,
    Orientation::WV,
    Orientation::WU,
    Orientation::UW,
];

pub const COUNTER_JUMP: [Orientation; 5] = [
    Orientation::VU,
    Orientation::UV,
    Orientation::WV,
    Orientation::WU,
    Orientation::UW,
];

const QUINTANT_ORIENTATIONS_ARRAYS: [[Orientation; 5]; 12] = [
    CLOCKWISE_FAN,  // 0 Arctic
    COUNTER_JUMP,   // 1 North America
    COUNTER_STEP,   // 2 South America
    CLOCKWISE_STEP, // 3 North Atlantic & Western Europe & Africa
    COUNTER_STEP,   // 4 South Atlantic & Africa
    COUNTER_JUMP,   // 5 Europe, Middle East & CentralAfrica
    COUNTER_STEP,   // 6 Indian Ocean
    CLOCKWISE_STEP, // 7 Asia
    CLOCKWISE_STEP, // 8 Australia
    CLOCKWISE_STEP, // 9 North Pacific
    COUNTER_JUMP,   // 10 South Pacific
    COUNTER_JUMP,   // 11 Antarctic
];

// Within each face, these are the indices of the first quintant
const QUINTANT_FIRST: [usize; 12] = [4, 2, 3, 2, 0, 4, 3, 2, 2, 0, 3, 0];

// Placements of dodecahedron faces along the Hilbert curve
const ORIGIN_ORDER: [usize; 12] = [0, 1, 2, 4, 3, 5, 7, 8, 6, 11, 10, 9];

use std::sync::OnceLock;

static ORIGINS: OnceLock<Vec<Origin>> = OnceLock::new();

fn quat_conjugate(q: Quat) -> Quat {
    [-q[0], -q[1], -q[2], q[3]]
}

fn generate_origins() -> Vec<Origin> {
    let mut origins = Vec::with_capacity(12);
    let mut origin_id: OriginId = 0;

    // Helper function to add origins
    let mut add_origin = |axis: Spherical, angle: Radians, quaternion: Quat| {
        if origin_id > 11 {
            panic!("Too many origins: {}", origin_id);
        }
        let inverse_quat = quat_conjugate(quaternion);
        let orientation = QUINTANT_ORIENTATIONS_ARRAYS[origin_id as usize].to_vec();
        let first_quintant = QUINTANT_FIRST[origin_id as usize];

        let origin = Origin {
            id: origin_id,
            axis,
            quat: quaternion,
            inverse_quat,
            angle,
            orientation,
            first_quintant,
        };
        origins.push(origin);
        origin_id += 1;
    };

    // North pole
    add_origin(
        Spherical::new(Radians::new_unchecked(0.0), Radians::new_unchecked(0.0)),
        Radians::new_unchecked(0.0),
        QUATERNIONS[0],
    );

    // Middle band
    for i in 0..5 {
        let alpha = (i as f64) * TWO_PI_OVER_5.get();
        let alpha2 = alpha + PI_OVER_5.get();
        add_origin(
            Spherical::new(Radians::new_unchecked(alpha), INTERHEDRAL_ANGLE),
            Radians::new_unchecked(PI_OVER_5.get()),
            QUATERNIONS[i + 1],
        );
        add_origin(
            Spherical::new(
                Radians::new_unchecked(alpha2),
                Radians::new_unchecked(std::f64::consts::PI - INTERHEDRAL_ANGLE.get()),
            ),
            Radians::new_unchecked(PI_OVER_5.get()),
            QUATERNIONS[(i + 3) % 5 + 6],
        );
    }

    // South pole
    add_origin(
        Spherical::new(
            Radians::new_unchecked(0.0),
            Radians::new_unchecked(std::f64::consts::PI),
        ),
        Radians::new_unchecked(0.0),
        QUATERNIONS[11],
    );

    // Reorder origins to match the order of the hilbert curve
    let mut reordered = Vec::with_capacity(12);
    for (new_id, &original_id) in ORIGIN_ORDER.iter().enumerate() {
        let mut origin = origins[original_id].clone();
        origin.id = new_id as OriginId;
        reordered.push(origin);
    }

    reordered
}

pub fn get_origins() -> &'static Vec<Origin> {
    ORIGINS.get_or_init(generate_origins)
}

pub fn quintant_to_segment(quintant: usize, origin: &Origin) -> (usize, Orientation) {
    // Lookup winding direction of this face
    let layout = &origin.orientation;
    let is_clockwise = is_layout_clockwise(layout);
    let step = if is_clockwise { -1i32 } else { 1i32 };

    // Find (CCW) delta from first quintant of this face
    let delta = (quintant + 5 - origin.first_quintant) % 5;

    // To look up the orientation, we need to use clockwise/counterclockwise counting
    let face_relative_quintant = ((step * delta as i32) + 5) % 5;
    let orientation = layout[face_relative_quintant as usize];
    let segment = (origin.first_quintant + face_relative_quintant as usize) % 5;

    (segment, orientation)
}

pub fn segment_to_quintant(segment: usize, origin: &Origin) -> (usize, Orientation) {
    // Lookup winding direction of this face
    let layout = &origin.orientation;
    let is_clockwise = is_layout_clockwise(layout);
    let step = if is_clockwise { -1i32 } else { 1i32 };

    let face_relative_quintant = (segment + 5 - origin.first_quintant) % 5;
    let orientation = layout[face_relative_quintant];

    // Handle the arithmetic more carefully to avoid overflow
    let step_offset = (step * face_relative_quintant as i32) % 5;
    let quintant = if step_offset >= 0 {
        (origin.first_quintant + step_offset as usize) % 5
    } else {
        (origin.first_quintant + 5 - ((-step_offset) as usize)) % 5
    };

    (quintant, orientation)
}

fn is_layout_clockwise(layout: &[Orientation]) -> bool {
    // Check if layout matches clockwise patterns
    layout == CLOCKWISE_FAN.as_slice() || layout == CLOCKWISE_STEP.as_slice()
}

/// Find the nearest origin to a point on the sphere
/// Uses haversine formula to calculate great-circle distance
pub fn find_nearest_origin(point: Spherical) -> &'static Origin {
    let origins = get_origins();
    let mut min_distance = f64::INFINITY;
    let mut nearest = &origins[0];

    for origin in origins {
        let distance = haversine(point, origin.axis);
        if distance < min_distance {
            min_distance = distance;
            nearest = origin;
        }
    }

    nearest
}

pub fn is_nearest_origin(point: Spherical, origin: &Origin) -> bool {
    haversine(point, origin.axis) > 0.49999999
}

/// Modified haversine formula to calculate great-circle distance.
/// Returns the "angle" between the two points. We need to minimize this to find the nearest origin
/// TODO figure out derivation!
pub fn haversine(point: Spherical, axis: Spherical) -> f64 {
    let theta = point.theta().get();
    let phi = point.phi().get();
    let theta2 = axis.theta().get();
    let phi2 = axis.phi().get();
    let dtheta = theta2 - theta;
    let dphi = phi2 - phi;
    let a1 = (dphi / 2.0).sin();
    let a2 = (dtheta / 2.0).sin();
    a1 * a1 + a2 * a2 * phi.sin() * phi2.sin()
}
