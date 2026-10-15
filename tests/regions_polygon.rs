use a5::coordinate_systems::LonLat;
use a5::core::hex::u64_to_hex;
use a5::coverings::compact::uncompact;
use a5::coverings::measures::count;
use a5::coverings::resolution::covering_resolution;
use a5::regions::polygon::{polygon_to_cells, Containment, PolygonToCellsOptions};
use serde::Deserialize;
use std::collections::HashSet;
use std::fs;

#[derive(Deserialize)]
struct PolygonFixture {
    name: String,
    polygon: Vec<Vec<[f64; 2]>>,
    resolution: i32,
    cells: Vec<String>,
}

#[derive(Deserialize)]
struct CountryFixture {
    name: String,
    polygon: Vec<Vec<[f64; 2]>>,
    resolution: i32,
    #[serde(rename = "cellCount")]
    cell_count: usize,
}

#[derive(Deserialize)]
struct Fixtures {
    polygon: Vec<PolygonFixture>,
    #[serde(default)]
    overlapping: Vec<PolygonFixture>,
    #[serde(default)]
    country: Vec<CountryFixture>,
}

fn to_rings(polygon: &[Vec<[f64; 2]>]) -> Vec<Vec<LonLat>> {
    polygon
        .iter()
        .map(|ring| ring.iter().map(|r| LonLat::new(r[0], r[1])).collect())
        .collect()
}

#[test]
fn test_polygon_to_cells_fixtures() {
    let content = fs::read_to_string("tests/fixtures/regions/polygon.json")
        .expect("Could not read polygon.json");
    let fixtures: Fixtures = serde_json::from_str(&content).expect("Could not parse polygon.json");

    for f in &fixtures.polygon {
        let rings = to_rings(&f.polygon);
        let result = polygon_to_cells(&rings, f.resolution, None).expect("polygon_to_cells");
        let expanded = uncompact(&result).expect("uncompact");
        let mut sorted = expanded;
        sorted.sort();
        let result_hex: Vec<String> = sorted.into_iter().map(u64_to_hex).collect();
        assert_eq!(result_hex, f.cells, "{}: cells mismatch", f.name);
    }
}

#[test]
fn test_polygon_to_cells_accepts_closed_rings() {
    let ring = vec![
        LonLat::new(-5.0, 54.0),
        LonLat::new(15.0, 54.0),
        LonLat::new(15.0, 44.0),
        LonLat::new(-5.0, 44.0),
    ];
    let hole = vec![
        LonLat::new(2.0, 51.0),
        LonLat::new(8.0, 51.0),
        LonLat::new(8.0, 47.0),
        LonLat::new(2.0, 47.0),
    ];
    let closed = |r: &[LonLat]| {
        let mut c = r.to_vec();
        c.push(r[0]);
        c
    };
    let open_result = polygon_to_cells(&[ring.clone(), hole.clone()], 6, None).unwrap();
    let closed_result = polygon_to_cells(&[closed(&ring), closed(&hole)], 6, None).unwrap();
    assert_eq!(closed_result, open_result);
}

#[test]
fn test_polygon_to_cells_empty_covering_for_less_than_3_vertices() {
    let p = |lon: f64, lat: f64| LonLat::new(lon, lat);
    let degenerate: Vec<Vec<Vec<LonLat>>> = vec![
        vec![],
        vec![vec![p(0.0, 0.0), p(1.0, 1.0)]],
        // Closed ring with only 2 distinct vertices
        vec![vec![p(0.0, 0.0), p(1.0, 1.0), p(0.0, 0.0)]],
    ];
    for polygon in &degenerate {
        let cells = polygon_to_cells(polygon, 5, None).unwrap();
        assert_eq!(count(&cells).unwrap(), 0);
        // The empty covering still records its resolution
        assert_eq!(covering_resolution(&cells), 5);
    }
}

#[test]
fn test_polygon_to_cells_ignores_degenerate_holes() {
    let ring = vec![
        LonLat::new(-5.0, 54.0),
        LonLat::new(15.0, 54.0),
        LonLat::new(15.0, 44.0),
        LonLat::new(-5.0, 44.0),
    ];
    let degenerate_hole = vec![LonLat::new(2.0, 50.0), LonLat::new(3.0, 49.0)];
    let without = polygon_to_cells(&[ring.clone()], 5, None).unwrap();
    let with = polygon_to_cells(&[ring, degenerate_hole], 5, None).unwrap();
    assert_eq!(with, without);
}

#[test]
fn test_polygon_to_cells_overlapping_fixtures() {
    let content = fs::read_to_string("tests/fixtures/regions/polygon.json")
        .expect("Could not read polygon.json");
    let fixtures: Fixtures = serde_json::from_str(&content).expect("Could not parse polygon.json");

    let options = Some(PolygonToCellsOptions {
        containment: Containment::Overlapping,
    });
    for f in &fixtures.overlapping {
        let rings = to_rings(&f.polygon);
        let result = polygon_to_cells(&rings, f.resolution, options).expect("polygon_to_cells");
        let expanded = uncompact(&result).expect("uncompact");
        let mut sorted = expanded;
        sorted.sort();
        let result_hex: Vec<String> = sorted.into_iter().map(u64_to_hex).collect();
        assert_eq!(
            result_hex, f.cells,
            "{}: overlapping cells mismatch",
            f.name
        );
    }
}

#[test]
fn test_polygon_to_cells_overlapping_is_superset_of_center() {
    let ring = vec![
        LonLat::new(-5.0, 54.0),
        LonLat::new(15.0, 54.0),
        LonLat::new(15.0, 44.0),
        LonLat::new(-5.0, 44.0),
    ];
    let center = polygon_to_cells(&[ring.clone()], 6, None).unwrap();
    let overlapping = polygon_to_cells(
        &[ring],
        6,
        Some(PolygonToCellsOptions {
            containment: Containment::Overlapping,
        }),
    )
    .unwrap();
    let center_expanded: HashSet<u64> = uncompact(&center).unwrap().into_iter().collect();
    let overlapping_expanded: HashSet<u64> = uncompact(&overlapping).unwrap().into_iter().collect();
    assert!(center_expanded.is_subset(&overlapping_expanded));
    assert!(overlapping_expanded.len() > center_expanded.len());
}

// A real-world sliver in Brazil: 7.6 ha with an 18 km perimeter, about 8 m wide
// on average. Its segments clip boundary cells between samples, so sampling the
// boundary non-exactly left segments out of those cells' records.
#[rustfmt::skip]
const SLIVER: [[f64; 2]; 61] = [
    [-53.5026091, -18.9003394], [-53.5044539, -18.9005638], [-53.5044547, -18.9005639],
    [-53.5091255, -18.9012084], [-53.5145501, -18.9019568], [-53.5145559, -18.9019576],
    [-53.5151615, -18.9020412], [-53.5250784, -18.9034088], [-53.525177, -18.9034224],
    [-53.5334107, -18.9045575], [-53.5351489, -18.8956725], [-53.5350746, -18.8956625],
    [-53.5350737, -18.8956623], [-53.5344119, -18.8990449], [-53.5333495, -18.904476],
    [-53.5333494, -18.9044761], [-53.5333493, -18.9044761], [-53.5134871, -18.9017372],
    [-53.513487, -18.9017372], [-53.501915, -18.9001404], [-53.5008177, -18.899989],
    [-53.5002414, -18.8999094], [-53.4569422, -18.8939276], [-53.4569396, -18.8939622],
    [-53.4620193, -18.8946555], [-53.4629496, -18.8947722], [-53.4629506, -18.8947723],
    [-53.463011, -18.8947811], [-53.4631319, -18.8947987], [-53.4631319, -18.8947986],
    [-53.4631337, -18.894799], [-53.4631354, -18.8947993], [-53.4633537, -18.8948482],
    [-53.4637524, -18.8948808], [-53.4637538, -18.894881], [-53.4637559, -18.8948812],
    [-53.4637582, -18.8948816], [-53.4643553, -18.8950023], [-53.464856, -18.8950575],
    [-53.4648567, -18.8950576], [-53.4648581, -18.8950578], [-53.4648598, -18.8950581],
    [-53.4648606, -18.8950582], [-53.4653928, -18.8951689], [-53.4703933, -18.89586],
    [-53.4739361, -18.8963496], [-53.4786342, -18.8969987], [-53.4818114, -18.8974731],
    [-53.4827538, -18.8976001], [-53.4895212, -18.8985121], [-53.4996221, -18.8999761],
    [-53.4997698, -18.8999941], [-53.5002517, -18.9000527], [-53.5003447, -18.900064],
    [-53.5003487, -18.9000481], [-53.5002757, -18.8999142], [-53.5004292, -18.8999354],
    [-53.5010136, -18.9000161], [-53.5026958, -18.9002482], [-53.5026834, -18.9003207],
    [-53.5026091, -18.9003394],
];

#[test]
fn test_polygon_to_cells_center_sliver_does_not_leak() {
    use a5::core::cell_info::get_num_cells;
    use a5::core::coordinate_transforms::{from_lon_lat, to_cartesian};
    use a5::geometry::spherical_polygon::spherical_triangle_area;

    let ring: Vec<LonLat> = SLIVER.iter().map(|p| LonLat::new(p[0], p[1])).collect();
    let vecs: Vec<_> = ring
        .iter()
        .map(|&p| to_cartesian(from_lon_lat(p)))
        .collect();
    let area: f64 = (1..vecs.len() - 1)
        .map(|i| spherical_triangle_area(vecs[0], vecs[i], vecs[i + 1]).get())
        .sum::<f64>()
        .abs();
    let polygon = vec![ring];
    let overlapping_options = Some(PolygonToCellsOptions {
        containment: Containment::Overlapping,
    });
    // Without exact boundary sampling the flood leaked out of the polygon at
    // 12, 14-17 and 20-22, up to a whole quintant (4,194,285 cells at 12), and
    // the curve runs at 23 emitted cells outside the overlapping cells
    for resolution in 12..=23 {
        let expected = area / (4.0 * std::f64::consts::PI) * get_num_cells(resolution) as f64;
        let center = polygon_to_cells(&polygon, resolution, None).unwrap();
        let n = count(&center).unwrap() as f64;
        assert!(
            (n - expected).abs() <= 0.1 * expected + 2.0,
            "resolution {resolution}: {n} cells, area holds {expected:.1}"
        );
        let overlapping = polygon_to_cells(&polygon, resolution, overlapping_options).unwrap();
        let center: HashSet<u64> = uncompact(&center).unwrap().into_iter().collect();
        let overlapping: HashSet<u64> = uncompact(&overlapping).unwrap().into_iter().collect();
        assert!(
            center.is_subset(&overlapping),
            "resolution {resolution}: center cells outside the overlapping cells"
        );
    }
}

#[test]
fn test_polygon_to_cells_country_fixtures() {
    let content = fs::read_to_string("tests/fixtures/regions/polygon.json")
        .expect("Could not read polygon.json");
    let fixtures: Fixtures = serde_json::from_str(&content).expect("Could not parse polygon.json");

    for f in &fixtures.country {
        let rings = to_rings(&f.polygon);
        let result = polygon_to_cells(&rings, f.resolution, None).expect("polygon_to_cells");
        let expanded = uncompact(&result).expect("uncompact");
        let unique: HashSet<u64> = expanded.into_iter().collect();
        assert_eq!(
            unique.len(),
            f.cell_count,
            "{}: cell count mismatch",
            f.name
        );
    }
}
