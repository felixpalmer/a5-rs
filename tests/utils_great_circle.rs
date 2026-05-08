use a5::coordinate_systems::Cartesian;
use a5::utils::great_circle::{great_circle_distance, sample_great_circle_arc};
use serde::Deserialize;
use std::fs;

#[derive(Deserialize)]
struct Fixture {
    name: String,
    #[serde(rename = "aVec")]
    a_vec: [f64; 3],
    #[serde(rename = "bVec")]
    b_vec: [f64; 3],
    #[serde(rename = "sampleInterval")]
    sample_interval: f64,
    distance: f64,
    #[serde(rename = "sampleCount")]
    sample_count: usize,
    samples: Vec<Vec<f64>>,
}

#[derive(Deserialize)]
struct Fixtures {
    #[serde(rename = "sampleGreatCircleArc")]
    sample_great_circle_arc: Vec<Fixture>,
}

#[test]
fn test_sample_great_circle_arc_fixtures() {
    let content = fs::read_to_string("tests/fixtures/utils/great-circle.json")
        .expect("Could not read great-circle.json");
    let fixtures: Fixtures =
        serde_json::from_str(&content).expect("Could not parse great-circle.json");

    for f in &fixtures.sample_great_circle_arc {
        let a = Cartesian::new(f.a_vec[0], f.a_vec[1], f.a_vec[2]);
        let b = Cartesian::new(f.b_vec[0], f.b_vec[1], f.b_vec[2]);

        let dist = great_circle_distance(a, b);
        assert!(
            (dist - f.distance).abs() < 1e-6,
            "{}: distance {} != {}",
            f.name,
            dist,
            f.distance
        );

        let samples = sample_great_circle_arc(a, b, f.sample_interval);
        assert_eq!(
            samples.len(),
            f.sample_count,
            "{}: sample count mismatch",
            f.name
        );
        for (i, s) in samples.iter().enumerate() {
            let expected = &f.samples[i];
            assert!(
                (s.x() - expected[0]).abs() < 1e-6
                    && (s.y() - expected[1]).abs() < 1e-6
                    && (s.z() - expected[2]).abs() < 1e-6,
                "{}: sample {} mismatch — got [{}, {}, {}], expected {:?}",
                f.name,
                i,
                s.x(),
                s.y(),
                s.z(),
                expected
            );
        }
    }
}
