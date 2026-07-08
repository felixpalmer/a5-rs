// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

// Benchmarks for the space-filling curve: s -> cell decode, cell -> s encode,
// and fractional-point location (IJToS).

use criterion::{black_box, criterion_group, criterion_main, Criterion};

use a5::coordinate_systems::IJ;
use a5::lattice::{ij_to_s, s_to_cell, triple_to_s, Orientation, Triple};

mod common;

const N: usize = 256;

/// The cell's centroid in IJ coordinates
/// (parity 0: (x+y+1/3, -x+1/3), parity 1: (x+y-1/3, -x+2/3)).
fn centroid_ij(t: &Triple) -> IJ {
    let parity = t.x + t.y + t.z;
    if parity == 0 {
        IJ::new(t.x as f64 + t.y as f64 + 1.0 / 3.0, -t.x as f64 + 1.0 / 3.0)
    } else {
        IJ::new(t.x as f64 + t.y as f64 - 1.0 / 3.0, -t.x as f64 + 2.0 / 3.0)
    }
}

fn bench_s_to_cell(c: &mut Criterion) {
    let mut g = c.benchmark_group("sToCell");
    for resolution in [5usize, 15, 28] {
        let values = common::sample_s(resolution, N, 42);
        let mut i = 0usize;
        g.bench_function(format!("sToCell res {resolution}"), |b| {
            b.iter(|| {
                let s = values[i & (N - 1)];
                i += 1;
                black_box(s_to_cell(black_box(s), resolution, Orientation::UV))
            })
        });
    }

    // Orientation with both flip and reversal transforms.
    let values = common::sample_s(15, N, 42);
    let mut i = 0usize;
    g.bench_function("sToCell res 15 orientation wu", |b| {
        b.iter(|| {
            let s = values[i & (N - 1)];
            i += 1;
            black_box(s_to_cell(black_box(s), 15, Orientation::WU))
        })
    });
    g.finish();
}

fn bench_triple_to_s(c: &mut Criterion) {
    let mut g = c.benchmark_group("tripleToS");
    for resolution in [5usize, 15, 28] {
        let values = common::sample_s(resolution, N, 42);
        let triples: Vec<Triple> = values
            .iter()
            .map(|&s| s_to_cell(s, resolution, Orientation::UV).triple)
            .collect();
        let mut i = 0usize;
        g.bench_function(format!("tripleToS res {resolution}"), |b| {
            b.iter(|| {
                let t = &triples[i & (N - 1)];
                i += 1;
                black_box(triple_to_s(black_box(t), resolution, Orientation::UV))
            })
        });
    }
    g.finish();
}

fn bench_ij_to_s(c: &mut Criterion) {
    let values = common::sample_s(15, N, 42);
    let ijs: Vec<IJ> = values
        .iter()
        .map(|&s| centroid_ij(&s_to_cell(s, 15, Orientation::UV).triple))
        .collect();
    let mut g = c.benchmark_group("IJToS");
    let mut i = 0usize;
    g.bench_function("IJToS res 15", |b| {
        b.iter(|| {
            let ij = ijs[i & (N - 1)];
            i += 1;
            black_box(ij_to_s(black_box(ij), 15, Orientation::UV))
        })
    });
    g.finish();
}

criterion_group!(benches, bench_s_to_cell, bench_triple_to_s, bench_ij_to_s);
criterion_main!(benches);
