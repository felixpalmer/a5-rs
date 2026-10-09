// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use criterion::{black_box, criterion_group, criterion_main, Criterion};

mod common;

const N: usize = 64;

fn bench_subcell(c: &mut Criterion) {
    let cells8 = common::sample_cells(8, N, 42);
    let cells15 = common::sample_cells(15, N, 42);

    let mut g = c.benchmark_group("subcell");
    let mut i = 0usize;
    g.bench_function("cellToSupercell res 15 -> 8", |b| {
        b.iter(|| {
            let cell = cells15[i & (N - 1)];
            i += 1;
            black_box(a5::cell_to_supercell(black_box(cell), 8).unwrap())
        })
    });

    let mut j = 0usize;
    g.bench_function("cellToSubcell res 8 -> 11", |b| {
        b.iter(|| {
            let cell = cells8[j & (N - 1)];
            j += 1;
            black_box(a5::cell_to_subcell(black_box(cell), 11).unwrap())
        })
    });

    let mut k = 0usize;
    g.bench_function("cellToSubcell res 8 -> 16", |b| {
        b.iter(|| {
            let cell = cells8[k & (N - 1)];
            k += 1;
            black_box(a5::cell_to_subcell(black_box(cell), 16).unwrap())
        })
    });
    g.finish();
}

criterion_group!(benches, bench_subcell);
criterion_main!(benches);
