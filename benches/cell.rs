// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use criterion::{black_box, criterion_group, criterion_main, Criterion};

mod common;

const N: usize = 256;

fn bench_lonlat_to_cell(c: &mut Criterion) {
    let points = common::sample_points(N, 42);
    let mut g = c.benchmark_group("lonLatToCell");
    for resolution in [5, 15, 30] {
        let mut i = 0usize;
        g.bench_function(format!("lonLatToCell res {resolution}"), |b| {
            b.iter(|| {
                let p = points[i & (N - 1)];
                i += 1;
                black_box(a5::lonlat_to_cell(black_box(p), resolution).unwrap())
            })
        });
    }
    g.finish();
}

fn bench_cell_to_lonlat(c: &mut Criterion) {
    let mut g = c.benchmark_group("cellToLonLat");
    for resolution in [5, 15, 30] {
        let cells = common::sample_cells(resolution, N, 42);
        let mut i = 0usize;
        g.bench_function(format!("cellToLonLat res {resolution}"), |b| {
            b.iter(|| {
                let cell = cells[i & (N - 1)];
                i += 1;
                black_box(a5::cell_to_lonlat(black_box(cell)).unwrap())
            })
        });
    }
    g.finish();
}

fn bench_cell_to_boundary(c: &mut Criterion) {
    let mut g = c.benchmark_group("cellToBoundary");
    for resolution in [5, 15, 30] {
        let cells = common::sample_cells(resolution, N, 42);
        let mut i = 0usize;
        g.bench_function(format!("cellToBoundary res {resolution}"), |b| {
            b.iter(|| {
                let cell = cells[i & (N - 1)];
                i += 1;
                black_box(a5::cell_to_boundary(black_box(cell), None).unwrap())
            })
        });
    }

    let cells = common::sample_cells(15, N, 42);
    let mut i = 0usize;
    g.bench_function("cellToBoundary res 15 segments 10", |b| {
        b.iter(|| {
            let cell = cells[i & (N - 1)];
            i += 1;
            let opts = a5::core::cell::CellToBoundaryOptions {
                closed_ring: true,
                segments: Some(10),
            };
            black_box(a5::cell_to_boundary(black_box(cell), Some(opts)).unwrap())
        })
    });
    g.finish();
}

criterion_group!(
    benches,
    bench_lonlat_to_cell,
    bench_cell_to_lonlat,
    bench_cell_to_boundary
);
criterion_main!(benches);
