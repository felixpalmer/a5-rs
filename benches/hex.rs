// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use criterion::{black_box, criterion_group, criterion_main, Criterion};

mod common;

const N: usize = 256;

fn bench_hex(c: &mut Criterion) {
    let cells = common::sample_cells(20, N, 42);
    let hexes: Vec<String> = cells.iter().map(|&cell| a5::u64_to_hex(cell)).collect();

    let mut g = c.benchmark_group("hex");
    let mut i = 0usize;
    g.bench_function("u64ToHex", |b| {
        b.iter(|| {
            let cell = cells[i & (N - 1)];
            i += 1;
            black_box(a5::u64_to_hex(black_box(cell)))
        })
    });

    let mut j = 0usize;
    g.bench_function("hexToU64", |b| {
        b.iter(|| {
            let hex = &hexes[j & (N - 1)];
            j += 1;
            black_box(a5::hex_to_u64(black_box(hex)).unwrap())
        })
    });
    g.finish();
}

criterion_group!(benches, bench_hex);
criterion_main!(benches);
