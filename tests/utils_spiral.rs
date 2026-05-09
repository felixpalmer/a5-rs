use a5::coordinate_systems::{Radians, Spherical};
use a5::utils::spiral::{Spiral, SPIRAL_SAMPLE_COUNT};
use serde::Deserialize;
use std::fs;

#[derive(Deserialize)]
struct Fixture {
    name: String,
    center: [f64; 2],
    #[serde(rename = "scaleRad")]
    scale_rad: f64,
    #[serde(rename = "sampleCount")]
    sample_count: usize,
    samples: Vec<Vec<f64>>,
}

#[derive(Deserialize)]
struct Fixtures {
    #[serde(rename = "sampleCount")]
    sample_count: usize,
    spiral: Vec<Fixture>,
}

#[test]
fn test_spiral_sample_count_matches_fixture() {
    let content =
        fs::read_to_string("tests/fixtures/utils/spiral.json").expect("Could not read spiral.json");
    let fixtures: Fixtures = serde_json::from_str(&content).expect("Could not parse spiral.json");
    assert_eq!(SPIRAL_SAMPLE_COUNT, fixtures.sample_count);
}

#[test]
fn test_spiral_fixtures() {
    let content =
        fs::read_to_string("tests/fixtures/utils/spiral.json").expect("Could not read spiral.json");
    let fixtures: Fixtures = serde_json::from_str(&content).expect("Could not parse spiral.json");

    for f in &fixtures.spiral {
        let center = Spherical::new(
            Radians::new_unchecked(f.center[0]),
            Radians::new_unchecked(f.center[1]),
        );
        let spiral = Spiral::new(center, f.scale_rad);
        assert_eq!(f.sample_count, SPIRAL_SAMPLE_COUNT, "{}", f.name);
        for i in 0..SPIRAL_SAMPLE_COUNT {
            let s = spiral.sample(i);
            let expected = &f.samples[i];
            for (axis_index, (got, want)) in [s.x(), s.y(), s.z()]
                .iter()
                .zip(expected.iter())
                .enumerate()
            {
                assert!(
                    (got - want).abs() < 1e-6,
                    "{}: sample {} axis {}: {} != {}",
                    f.name,
                    i,
                    axis_index,
                    got,
                    want
                );
            }
        }
    }
}
