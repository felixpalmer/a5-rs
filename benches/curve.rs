// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

// Benchmarks for the space-filling curve: cell -> s encode (triple_to_s) and
//
// CI runs this same file against both the PR and its merge-base with main, so
// it must compile and run on either side of the L-system migration. It uses
// ONLY the API common to both engines — `triple_to_s` — whose
// signatures and (bit-identical) behavior are unchanged across the swap, so
// both runs measure the equivalent operation on identical inputs. The decode
// primitive changed name across the migration (s_to_anchor -> s_to_cell) with
// no common symbol, so it is not benchmarked here (a static language can't pick
// the function at runtime the way the TypeScript suite does).

use criterion::{black_box, criterion_group, criterion_main, Criterion};

use a5::lattice::{triple_in_bounds, triple_to_s, Orientation, Triple};

mod common;

const N: usize = 256;

/// Deterministic valid triples in the quintant, derived from the shared PRNG
/// sample. Construction guarantees parity in {0,1} and in-bounds coordinates.
fn sample_triples(resolution: usize, n: usize, seed: u32) -> Vec<Triple> {
    let raw = common::sample_s(resolution, n, seed);
    let max_row = (1i64 << resolution) - 1;
    let mut out = Vec::with_capacity(n);
    for &r in &raw {
        let y = (r % (max_row as u64 + 1)) as i64;
        let mut p = ((r >> 20) & 1) as i64;
        if y - p < 0 {
            p = 0;
        }
        let span = y - p;
        let x = -(((r >> 8) % (span as u64 + 1)) as i64);
        let z = p - x - y;
        let t = Triple::new(x as i32, y as i32, z as i32);
        out.push(if triple_in_bounds(&t, max_row as i32) {
            t
        } else {
            Triple::new(0, 0, 0)
        });
    }
    out
}

fn bench_triple_to_s(c: &mut Criterion) {
    let mut g = c.benchmark_group("tripleToS");
    for resolution in [5usize, 15, 28] {
        let triples = sample_triples(resolution, N, 42);
        let mut i = 0usize;
        g.bench_function(format!("tripleToS res {resolution}"), |b| {
            b.iter(|| {
                let t = &triples[i & (N - 1)];
                i += 1;
                black_box(triple_to_s(black_box(t), resolution, Orientation::UV))
            })
        });
    }

    // Orientation with both flip and reversal transforms.
    let triples = sample_triples(15, N, 42);
    let mut i = 0usize;
    g.bench_function("tripleToS res 15 orientation wu", |b| {
        b.iter(|| {
            let t = &triples[i & (N - 1)];
            i += 1;
            black_box(triple_to_s(black_box(t), 15, Orientation::WU))
        })
    });
    g.finish();
}

criterion_group!(benches, bench_triple_to_s);
criterion_main!(benches);
