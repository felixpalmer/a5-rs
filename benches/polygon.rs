// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use criterion::{black_box, criterion_group, criterion_main, Criterion};

mod common;

// Country outlines cover the interesting cases: many vertices (line tracing),
// large interiors (flood fill), multi-ring coastlines and high latitudes.
// All five names are present in tests/fixtures/regions/polygon.json.
const CASES: [(&str, i32); 5] = [
    ("United Kingdom", 7),
    ("France", 7),
    ("Brazil", 6),
    ("United States of America", 5),
    ("Fiji", 8), // antimeridian
];

fn bench_polygon(c: &mut Criterion) {
    let mut g = c.benchmark_group("polygonToCells");
    for (name, resolution) in CASES {
        let polygon = common::load_country(name);
        g.bench_function(format!("polygonToCells {name} res {resolution}"), |b| {
            b.iter(|| black_box(a5::polygon_to_cells(black_box(&polygon), resolution).unwrap()))
        });
    }
    g.finish();
}

criterion_group!(benches, bench_polygon);
criterion_main!(benches);
