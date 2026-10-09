// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

// Benchmarks for the space-filling curve: s -> cell decode and cell -> s encode.

use criterion::{black_box, criterion_group, criterion_main, Criterion};

use a5::lattice::{s_to_cell, triple_to_s, Orientation, Triple};

mod common;
use common::BATCH;

/// The triples of the cells at `values`.
fn triples_of(values: &[u64], resolution: usize, orientation: Orientation) -> Vec<Triple> {
    values
        .iter()
        .map(|&s| s_to_cell(s, resolution, orientation).triple)
        .collect()
}

fn bench_s_to_cell(c: &mut Criterion) {
    let mut g = c.benchmark_group("sToCell");
    for resolution in [5usize, 15, 28] {
        let values = common::sample_s(resolution, BATCH, 42);
        g.bench_function(format!("sToCell res {resolution} ×100"), |b| {
            b.iter(|| {
                for &s in &values {
                    black_box(s_to_cell(
                        black_box(s),
                        black_box(resolution),
                        Orientation::UV,
                    ));
                }
            })
        });
    }

    // Orientation with both flip and reversal transforms.
    let values = common::sample_s(15, BATCH, 42);
    g.bench_function("sToCell res 15 orientation wu ×100", |b| {
        b.iter(|| {
            for &s in &values {
                black_box(s_to_cell(black_box(s), black_box(15), Orientation::WU));
            }
        })
    });
    g.finish();
}

fn bench_triple_to_s(c: &mut Criterion) {
    let mut g = c.benchmark_group("tripleToS");
    for resolution in [5usize, 15, 28] {
        let triples = triples_of(
            &common::sample_s(resolution, BATCH, 42),
            resolution,
            Orientation::UV,
        );
        g.bench_function(format!("tripleToS res {resolution} ×100"), |b| {
            b.iter(|| {
                for t in &triples {
                    black_box(triple_to_s(
                        black_box(t),
                        black_box(resolution),
                        Orientation::UV,
                    ));
                }
            })
        });
    }

    // Orientation with both flip and reversal transforms.
    let triples = triples_of(&common::sample_s(15, BATCH, 42), 15, Orientation::WU);
    g.bench_function("tripleToS res 15 orientation wu ×100", |b| {
        b.iter(|| {
            for t in &triples {
                black_box(triple_to_s(black_box(t), black_box(15), Orientation::WU));
            }
        })
    });
    g.finish();
}

criterion_group!(benches, bench_s_to_cell, bench_triple_to_s);
criterion_main!(benches);
