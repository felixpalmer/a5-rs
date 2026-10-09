use a5::coordinate_systems::Face;
use a5::core::constants::TWO_PI_OVER_5;
use a5::core::face_adjacency::{seam_transform, seam_triple, FACE_ADJACENCY};
use a5::core::tiling::get_pentagon_center;
use a5::core::utils::OriginId;
use a5::lattice::{triple_flavor, triple_in_bounds, Triple};
use a5::projections::DodecahedronProjection;

const TOLERANCE: f64 = 1e-10;

fn apply(map: &[f64; 6], x: f64, y: f64) -> (f64, f64) {
    (
        map[0] * x + map[2] * y + map[4],
        map[1] * x + map[3] * y + map[5],
    )
}

fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < TOLERANCE,
        "expected {expected}, got {actual}"
    );
}

#[test]
fn seam_transform_agrees_with_the_projection_across_every_base_edge() {
    let mut dodecahedron = DodecahedronProjection::new().unwrap();
    for (o, adjacency) in FACE_ADJACENCY.iter().enumerate() {
        for (q, &(adjacent_id, adjacent_quintant)) in adjacency.iter().enumerate() {
            let map = seam_transform(o as OriginId, q);
            // Points of the neighbor's quintant near the shared edge, projected into this face
            for (r, angle) in [(0.5, 0.0), (0.55, -0.4), (0.55, 0.4)] {
                let gamma: f64 = adjacent_quintant as f64 * TWO_PI_OVER_5.get() + angle;
                let point = Face::new(r * gamma.cos(), r * gamma.sin());
                let spherical = dodecahedron.inverse(point, adjacent_id).unwrap();
                let landed = dodecahedron.forward(spherical, o as OriginId).unwrap();
                let (mx, my) = apply(&map, landed.x(), landed.y());
                assert_close(mx, point.x());
                assert_close(my, point.y());
            }
        }
    }
}

#[test]
fn seam_triple_is_seam_transform_on_cells() {
    for hilbert_res in [1, 2, 5] {
        let max_row = (1 << hilbert_res) - 1;
        for q in 0..5usize {
            let map = seam_transform(0, q);
            let adjacent_quintant = FACE_ADJACENCY[0][q].1;
            // The cells of the two rows along the base edge
            for y in (max_row - 1).max(0)..=max_row {
                for x in (-y - 1)..=0 {
                    for parity in [0, 1] {
                        let triple = Triple::new(x, y, parity - x - y);
                        if !triple_in_bounds(&triple, max_row) {
                            continue;
                        }
                        let flavor = triple_flavor(&triple, max_row);
                        let image = seam_triple(&triple, max_row);
                        assert_eq!(triple_flavor(&image, max_row), flavor ^ 1);
                        let c = get_pentagon_center(hilbert_res, q, &triple, flavor);
                        let (cx, cy) = apply(&map, c.x(), c.y());
                        let image_center =
                            get_pentagon_center(hilbert_res, adjacent_quintant, &image, flavor ^ 1);
                        assert_close(cx, image_center.x());
                        assert_close(cy, image_center.y());
                    }
                }
            }
        }
    }
}
