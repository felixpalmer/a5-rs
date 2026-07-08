// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use criterion::{black_box, criterion_group, criterion_main, Criterion};

mod common;

fn bench_compact(c: &mut Criterion) {
    let uk = common::load_country("United Kingdom");

    // A realistic mixed-resolution cell set: country fill expanded to a flat list.
    let compacted = a5::polygon_to_cells(&uk, 10).unwrap();
    let flat = a5::uncompact(&compacted, 10).unwrap();

    let mut g = c.benchmark_group("compact");
    g.bench_function(format!("compact UK res 10 ({} cells)", flat.len()), |b| {
        b.iter(|| black_box(a5::compact(black_box(&flat)).unwrap()))
    });
    g.bench_function(
        format!("uncompact UK res 10 -> 12 ({} cells)", flat.len() * 16),
        |b| b.iter(|| black_box(a5::uncompact(black_box(&flat), 12).unwrap())),
    );
    g.finish();
}

criterion_group!(benches, bench_compact);
criterion_main!(benches);
