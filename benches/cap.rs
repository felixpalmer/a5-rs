// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use criterion::{black_box, criterion_group, criterion_main, Criterion};

mod common;

fn bench_cap(c: &mut Criterion) {
    let london9 = a5::lonlat_to_cell(a5::LonLat::new(-0.1276, 51.5072), 9).unwrap();
    let london12 = a5::lonlat_to_cell(a5::LonLat::new(-0.1276, 51.5072), 12).unwrap();

    let mut g = c.benchmark_group("sphericalCap");
    g.bench_function("sphericalCap res 9 radius 10km", |b| {
        b.iter(|| black_box(a5::spherical_cap(black_box(london9), 10_000.0).unwrap()))
    });
    g.bench_function("sphericalCap res 9 radius 100km", |b| {
        b.iter(|| black_box(a5::spherical_cap(black_box(london9), 100_000.0).unwrap()))
    });
    g.bench_function("sphericalCap res 12 radius 5km", |b| {
        b.iter(|| black_box(a5::spherical_cap(black_box(london12), 5_000.0).unwrap()))
    });
    g.finish();
}

criterion_group!(benches, bench_cap);
criterion_main!(benches);
