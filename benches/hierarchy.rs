// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use criterion::{black_box, criterion_group, criterion_main, Criterion};

mod common;
use common::BATCH;

const N: usize = 256;

fn bench_hierarchy(c: &mut Criterion) {
    let cells15 = common::sample_cells(15, BATCH, 42);
    let cells10 = common::sample_cells(10, N, 42);

    let mut g = c.benchmark_group("hierarchy");
    g.bench_function("getResolution res 15 ×100", |b| {
        b.iter(|| {
            for &cell in &cells15 {
                black_box(a5::get_resolution(black_box(cell)));
            }
        })
    });

    g.bench_function("cellToParent res 15 -> 14 ×100", |b| {
        b.iter(|| {
            for &cell in &cells15 {
                black_box(a5::cell_to_parent(black_box(cell), None).unwrap());
            }
        })
    });

    g.bench_function("cellToParent res 15 -> 5 ×100", |b| {
        b.iter(|| {
            for &cell in &cells15 {
                black_box(a5::cell_to_parent(black_box(cell), black_box(Some(5))).unwrap());
            }
        })
    });

    g.bench_function("cellToChildren res 15 -> 16 ×100", |b| {
        b.iter(|| {
            for &cell in &cells15 {
                black_box(a5::cell_to_children(black_box(cell), None).unwrap());
            }
        })
    });

    let mut m = 0usize;
    g.bench_function("cellToChildren res 10 -> 13", |b| {
        b.iter(|| {
            let cell = cells10[m & (N - 1)];
            m += 1;
            black_box(a5::cell_to_children(black_box(cell), Some(13)).unwrap())
        })
    });

    g.bench_function("getRes0Cells ×100", |b| {
        b.iter(|| {
            for _ in 0..BATCH {
                black_box(a5::get_res0_cells().unwrap());
            }
        })
    });
    g.finish();

    let mut g = c.benchmark_group("cell-info");
    g.bench_function("getNumCells res 15 ×100", |b| {
        b.iter(|| {
            for _ in 0..BATCH {
                black_box(a5::get_num_cells(black_box(15)));
            }
        })
    });
    g.bench_function("getNumChildren res 0 -> 15 ×100", |b| {
        b.iter(|| {
            for _ in 0..BATCH {
                black_box(a5::get_num_children(black_box(0), black_box(15)));
            }
        })
    });
    g.bench_function("cellArea res 15 ×100", |b| {
        b.iter(|| {
            for _ in 0..BATCH {
                black_box(a5::cell_area(black_box(15)));
            }
        })
    });
    g.finish();
}

criterion_group!(benches, bench_hierarchy);
criterion_main!(benches);
