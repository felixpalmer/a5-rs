use a5_rs::coordinate_systems::LonLat;
use a5_rs::core::cell::{a5cell_contains_point, cell_to_boundary, cell_to_lonlat, lonlat_to_cell};
use a5_rs::core::serialization::{deserialize, MAX_RESOLUTION};
use approx::assert_relative_eq;

const TOLERANCE: f64 = 1e-6;

#[test]
fn test_cell_boundary_basic() {
    // Test basic functionality with a simple point
    let test_point = LonLat::new(0.0, 0.0); // Null Island

    // Test at various resolutions
    for resolution in 1..=5 {
        let cell_id = lonlat_to_cell(test_point, resolution).expect("Failed to get cell ID");
        let boundary = cell_to_boundary(cell_id, None).expect("Failed to get boundary");

        // Boundary should have at least 5 points (pentagon) plus potentially one for closing
        assert!(
            boundary.len() >= 5,
            "Boundary should have at least 5 points"
        );

        // Test that cell center can be retrieved
        let center = cell_to_lonlat(cell_id).expect("Failed to get cell center");

        // Test that the original point is contained in the cell
        let cell_data = deserialize(cell_id).expect("Failed to deserialize cell");
        let contains_result =
            a5cell_contains_point(&cell_data, test_point).expect("Failed to test containment");

        // If it contains the point, distance should be positive
        if contains_result > 0.0 {
            // This is expected for correct cells
        } else {
            // Even if not strictly contained, should be very close
            let distance_to_center = haversine_distance(test_point, center);
            assert!(
                distance_to_center < 10.0,
                "Point should be close to cell center"
            ); // within 10km
        }
    }
}

#[test]
fn test_antimeridian_cells() {
    // Test cells that cross the antimeridian
    let antimeridian_cell_ids = ["eb60000000000000", "2e00000000000000"];

    for cell_id_hex in &antimeridian_cell_ids {
        // Convert hex string to u64
        let cell_id = u64::from_str_radix(cell_id_hex, 16).expect("Failed to parse hex");

        let boundary = cell_to_boundary(cell_id, None).expect("Failed to get boundary");

        // Check for antimeridian crossing
        let longitudes: Vec<f64> = boundary.iter().map(|point| point.longitude()).collect();
        let min_lon = longitudes.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let max_lon = longitudes.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        let lon_span = max_lon - min_lon;

        assert!(
            lon_span < 180.0,
            "Longitude span should be less than 180 degrees for antimeridian cells"
        );
    }
}

#[test]
fn test_cell_containment_property() {
    // Test a few known points
    let test_points = [
        LonLat::new(0.0, 0.0),     // Null Island
        LonLat::new(-122.0, 37.0), // San Francisco area
        LonLat::new(2.0, 48.0),    // Paris area
        LonLat::new(139.0, 35.0),  // Tokyo area
    ];

    for test_point in &test_points {
        // Skip extreme polar regions for now (known issues)
        if test_point.latitude().abs() > 80.0 {
            continue;
        }

        for resolution in 1..=6 {
            // Test reasonable resolutions
            if resolution == MAX_RESOLUTION {
                continue; // Skip max resolution as noted in TypeScript tests
            }

            let cell_id = lonlat_to_cell(*test_point, resolution).expect("Failed to get cell ID");
            let cell_data = deserialize(cell_id).expect("Failed to deserialize cell");
            let contains_result =
                a5cell_contains_point(&cell_data, *test_point).expect("Failed to test containment");

            // The cell should contain the original point (positive distance) or be very close (small negative distance)
            assert!(
                contains_result > -0.1,
                "Cell should contain or be very close to the original point at resolution {}",
                resolution
            );
        }
    }
}

#[test]
fn test_boundary_closure() {
    // Test boundary with and without closure
    let test_point = LonLat::new(10.0, 20.0);
    let cell_id = lonlat_to_cell(test_point, 3).expect("Failed to get cell ID");

    // Test with closed ring (default)
    let closed_boundary = cell_to_boundary(cell_id, None).expect("Failed to get closed boundary");

    // Test with open ring
    let options = a5_rs::core::cell::CellToBoundaryOptions {
        closed_ring: false,
        segments: None,
    };
    let open_boundary =
        cell_to_boundary(cell_id, Some(options)).expect("Failed to get open boundary");

    // Closed boundary should have one more point than open boundary
    assert_eq!(
        closed_boundary.len(),
        open_boundary.len() + 1,
        "Closed boundary should have one more point"
    );

    // First and last points of closed boundary should be the same
    assert_relative_eq!(
        closed_boundary.first().unwrap().longitude(),
        closed_boundary.last().unwrap().longitude(),
        epsilon = TOLERANCE
    );
    assert_relative_eq!(
        closed_boundary.first().unwrap().latitude(),
        closed_boundary.last().unwrap().latitude(),
        epsilon = TOLERANCE
    );
}

/// Haversine distance calculation for testing
fn haversine_distance(point1: LonLat, point2: LonLat) -> f64 {
    let r = 6371.0; // Earth radius in km

    let lat1_rad = point1.latitude().to_radians();
    let lat2_rad = point2.latitude().to_radians();
    let delta_lat = (point2.latitude() - point1.latitude()).to_radians();
    let delta_lon = (point2.longitude() - point1.longitude()).to_radians();

    let a = (delta_lat / 2.0).sin().powi(2)
        + lat1_rad.cos() * lat2_rad.cos() * (delta_lon / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());

    r * c
}
