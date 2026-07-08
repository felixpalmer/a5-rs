// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use criterion::{black_box, criterion_group, criterion_main, Criterion};

use a5::LonLat;

mod common;

fn bench_line(c: &mut Criterion) {
    let london_paris = [LonLat::new(-0.1276, 51.5072), LonLat::new(2.3522, 48.8566)];

    let round_the_world = [
        LonLat::new(-122.4194, 37.7749), // San Francisco
        LonLat::new(-74.006, 40.7128),   // New York
        LonLat::new(-0.1276, 51.5072),   // London
        LonLat::new(139.6917, 35.6895),  // Tokyo
        LonLat::new(151.2093, -33.8688), // Sydney
    ];

    let mut g = c.benchmark_group("lineStringToCells");
    g.bench_function("lineStringToCells London-Paris res 9", |b| {
        b.iter(|| black_box(a5::line_string_to_cells(black_box(&london_paris), 9).unwrap()))
    });
    g.bench_function("lineStringToCells round-the-world res 6", |b| {
        b.iter(|| black_box(a5::line_string_to_cells(black_box(&round_the_world), 6).unwrap()))
    });
    g.finish();
}

criterion_group!(benches, bench_line);
criterion_main!(benches);
