// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use criterion::{black_box, criterion_group, criterion_main, Criterion};

use a5::lattice::{anchor_to_s, ij_to_s, s_to_anchor, Anchor, Orientation};

mod common;

const N: usize = 256;

fn bench_s_to_anchor(c: &mut Criterion) {
    let mut g = c.benchmark_group("sToAnchor");
    for resolution in [5usize, 15, 28] {
        let values = common::sample_s(resolution, N, 42);
        let mut i = 0usize;
        g.bench_function(format!("sToAnchor res {resolution}"), |b| {
            b.iter(|| {
                let s = values[i & (N - 1)];
                i += 1;
                black_box(s_to_anchor(black_box(s), resolution, Orientation::UV))
            })
        });
    }

    // Orientation with both flip and reversal transforms.
    let values = common::sample_s(15, N, 42);
    let mut i = 0usize;
    g.bench_function("sToAnchor res 15 orientation wu", |b| {
        b.iter(|| {
            let s = values[i & (N - 1)];
            i += 1;
            black_box(s_to_anchor(black_box(s), 15, Orientation::WU))
        })
    });
    g.finish();
}

fn bench_anchor_to_s(c: &mut Criterion) {
    let mut g = c.benchmark_group("anchorToS");
    for resolution in [5usize, 15, 28] {
        let values = common::sample_s(resolution, N, 42);
        let anchors: Vec<Anchor> = values
            .iter()
            .map(|&s| s_to_anchor(s, resolution, Orientation::UV))
            .collect();
        let mut i = 0usize;
        g.bench_function(format!("anchorToS res {resolution}"), |b| {
            b.iter(|| {
                let anchor = &anchors[i & (N - 1)];
                i += 1;
                black_box(anchor_to_s(black_box(anchor), resolution, Orientation::UV))
            })
        });
    }
    g.finish();
}

fn bench_ij_to_s(c: &mut Criterion) {
    let values = common::sample_s(15, N, 42);
    let ijs: Vec<_> = values
        .iter()
        .map(|&s| s_to_anchor(s, 15, Orientation::UV).offset)
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

criterion_group!(benches, bench_s_to_anchor, bench_anchor_to_s, bench_ij_to_s);
criterion_main!(benches);
