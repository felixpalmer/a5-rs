// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use a5::lattice::{anchor_to_s, s_to_anchor, Anchor, Orientation};
use serde::Deserialize;

#[derive(Deserialize)]
struct SToAnchorFixture {
    s: u64,
    resolution: usize,
    orientation: String,
    q: u8,
    offset: [f64; 2],
    flips: [i8; 2],
}

#[derive(Deserialize)]
struct Fixtures {
    #[serde(rename = "sToAnchor")]
    s_to_anchor: Vec<SToAnchorFixture>,
}

fn load_fixtures() -> Fixtures {
    let data = include_str!("fixtures/lattice/hilbert.json");
    serde_json::from_str(data).expect("Failed to parse hilbert.json")
}

#[test]
fn test_s_to_anchor() {
    let fixtures = load_fixtures();
    for f in &fixtures.s_to_anchor {
        let orientation = f.orientation.parse::<Orientation>().unwrap();
        let anchor = s_to_anchor(f.s, f.resolution, orientation);
        assert_eq!(
            anchor.q, f.q,
            "q for s={} res={} ori={}",
            f.s, f.resolution, f.orientation
        );
        assert_eq!(
            anchor.offset.x(),
            f.offset[0],
            "offset[0] for s={} res={} ori={}",
            f.s,
            f.resolution,
            f.orientation
        );
        assert_eq!(
            anchor.offset.y(),
            f.offset[1],
            "offset[1] for s={} res={} ori={}",
            f.s,
            f.resolution,
            f.orientation
        );
        assert_eq!(
            anchor.flips[0], f.flips[0],
            "flips[0] for s={} res={} ori={}",
            f.s, f.resolution, f.orientation
        );
        assert_eq!(
            anchor.flips[1], f.flips[1],
            "flips[1] for s={} res={} ori={}",
            f.s, f.resolution, f.orientation
        );
    }
}

#[test]
fn test_anchor_to_s() {
    let fixtures = load_fixtures();
    for f in &fixtures.s_to_anchor {
        let orientation = f.orientation.parse::<Orientation>().unwrap();
        let anchor = Anchor {
            q: f.q,
            offset: a5::coordinate_systems::IJ::new(f.offset[0], f.offset[1]),
            flips: f.flips,
        };
        let s = anchor_to_s(&anchor, f.resolution, orientation);
        assert_eq!(s, f.s, "s for res={} ori={}", f.resolution, f.orientation);
    }
}
