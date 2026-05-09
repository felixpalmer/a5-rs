use a5::coordinate_systems::Cartesian;
use a5::geometry::spherical_polygon::{point_in_spherical_polygon, ring_winding_sign};
use serde::Deserialize;
use std::fs;

#[derive(Deserialize)]
struct PipPoint {
    vec: [f64; 3],
    inside: bool,
}

#[derive(Deserialize)]
struct PipFixture {
    name: String,
    ring: Vec<[f64; 2]>,
    points: Vec<PipPoint>,
}

#[derive(Deserialize)]
struct WindingFixture {
    name: String,
    ring: Vec<[f64; 2]>,
    sign: i32,
}

#[derive(Deserialize)]
struct Fixtures {
    #[serde(rename = "pointInSphericalPolygon")]
    pip: Vec<PipFixture>,
    #[serde(rename = "ringWindingSign")]
    winding: Vec<WindingFixture>,
}

fn ll_to_vec(ll: &[f64; 2]) -> Cartesian {
    let deg_to_rad = std::f64::consts::PI / 180.0;
    let lat = ll[1] * deg_to_rad;
    let lon = ll[0] * deg_to_rad;
    let cos_lat = lat.cos();
    Cartesian::new(cos_lat * lon.cos(), cos_lat * lon.sin(), lat.sin())
}

#[test]
fn test_point_in_spherical_polygon_fixtures() {
    let content = fs::read_to_string("tests/fixtures/geometry/spherical-polygon-primitives.json")
        .expect("Could not read fixture");
    let fixtures: Fixtures = serde_json::from_str(&content).expect("Could not parse fixture");

    for f in &fixtures.pip {
        let ring_vecs: Vec<Cartesian> = f.ring.iter().map(ll_to_vec).collect();
        for p in &f.points {
            let v = Cartesian::new(p.vec[0], p.vec[1], p.vec[2]);
            let result = point_in_spherical_polygon(v, &ring_vecs);
            assert_eq!(
                result, p.inside,
                "{}: point {:?} expected inside={}",
                f.name, p.vec, p.inside
            );
        }
    }
}

#[test]
fn test_ring_winding_sign_fixtures() {
    let content = fs::read_to_string("tests/fixtures/geometry/spherical-polygon-primitives.json")
        .expect("Could not read fixture");
    let fixtures: Fixtures = serde_json::from_str(&content).expect("Could not parse fixture");

    for f in &fixtures.winding {
        let ring_vecs: Vec<Cartesian> = f.ring.iter().map(ll_to_vec).collect();
        let sign = ring_winding_sign(&ring_vecs);
        assert_eq!(sign, f.sign, "{}: winding sign mismatch", f.name);
    }
}
