// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors
//
// Shared helpers for the criterion benchmark suite. Each bench file compiles as
// its own binary, so this module is included via `mod common;` in every bench.
// Not every bench uses every helper, hence the crate-wide dead_code allow.
#![allow(dead_code)]

use a5::LonLat;
use std::fs;

/// Deterministic PRNG (mulberry32) so every run benchmarks identical inputs.
/// Mirrors the TypeScript `createRandom` (see tests/zz_bench_ea.rs).
pub struct Rng {
    s: u32,
}

impl Rng {
    pub fn new(seed: u32) -> Self {
        Rng { s: seed }
    }

    pub fn next(&mut self) -> f64 {
        self.s = self.s.wrapping_add(0x6d2b_79f5);
        let mut t = self.s;
        t = (t ^ (t >> 15)).wrapping_mul(t | 1);
        t ^= t.wrapping_add((t ^ (t >> 7)).wrapping_mul(t | 61));
        ((t ^ (t >> 14)) as f64) / 4_294_967_296.0
    }
}

/// Points distributed uniformly over the sphere (area-uniform in latitude).
pub fn sample_points(n: usize, seed: u32) -> Vec<LonLat> {
    let mut rng = Rng::new(seed);
    let mut points = Vec::with_capacity(n);
    for _ in 0..n {
        let lon = 360.0 * rng.next() - 180.0;
        let lat = (2.0 * rng.next() - 1.0).asin().to_degrees();
        points.push(LonLat::new(lon, lat));
    }
    points
}

/// Cell IDs of uniformly distributed points at the given resolution.
pub fn sample_cells(resolution: i32, n: usize, seed: u32) -> Vec<u64> {
    sample_points(n, seed)
        .into_iter()
        .map(|p| a5::lonlat_to_cell(p, resolution).unwrap())
        .collect()
}

/// Deterministic S values in [0, 4^resolution).
pub fn sample_s(resolution: usize, n: usize, seed: u32) -> Vec<u64> {
    let mut rng = Rng::new(seed);
    let max = 1u64 << (2 * resolution);
    let mut values = Vec::with_capacity(n);
    for _ in 0..n {
        let hi = (rng.next() * 4_294_967_296.0).floor() as u64;
        let lo = (rng.next() * 4_294_967_296.0).floor() as u64;
        values.push(((hi << 32) | lo) % max);
    }
    values
}

/// Load a country's polygon (outer ring + holes) from the shared fixture,
/// returning it as `Vec<Vec<LonLat>>` for `polygon_to_cells`.
pub fn load_country(name: &str) -> Vec<Vec<LonLat>> {
    let path = format!(
        "{}/tests/fixtures/regions/polygon.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let content = fs::read_to_string(&path).expect("Could not read polygon.json");
    let json: serde_json::Value =
        serde_json::from_str(&content).expect("Could not parse polygon.json");
    let countries = json["country"]
        .as_array()
        .expect("`country` array missing from fixture");
    let country = countries
        .iter()
        .find(|c| c["name"].as_str() == Some(name))
        .unwrap_or_else(|| panic!("country `{}` not found in fixture", name));
    country["polygon"]
        .as_array()
        .expect("`polygon` must be an array of rings")
        .iter()
        .map(|ring| {
            ring.as_array()
                .expect("ring must be an array")
                .iter()
                .map(|pair| {
                    let p = pair.as_array().expect("pair must be an array");
                    LonLat::new(p[0].as_f64().unwrap(), p[1].as_f64().unwrap())
                })
                .collect()
        })
        .collect()
}
