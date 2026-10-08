// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use criterion::{black_box, criterion_group, criterion_main, Criterion};

mod common;

// The resolution argument is for the pre-compaction-marker uncompact(cells, resolution),
// which the baseline run may use; uncompact now reads it from the compaction marker.
// The marker type parameter lets the compiler pick whichever signature exists.
trait UncompactAt<Marker> {
    fn uncompact_at(&self, cells: &[u64], resolution: i32) -> Result<Vec<u64>, String>;
}

impl<F: Fn(&[u64]) -> Result<Vec<u64>, String>> UncompactAt<()> for F {
    fn uncompact_at(&self, cells: &[u64], _resolution: i32) -> Result<Vec<u64>, String> {
        self(cells)
    }
}

impl<F: Fn(&[u64], i32) -> Result<Vec<u64>, String>> UncompactAt<i32> for F {
    fn uncompact_at(&self, cells: &[u64], resolution: i32) -> Result<Vec<u64>, String> {
        self(cells, resolution)
    }
}

fn bench_compact(c: &mut Criterion) {
    let uk = common::load_country("United Kingdom");

    // A realistic mixed-resolution cell set: country fill expanded to a flat list.
    let flat = a5::uncompact
        .uncompact_at(&a5::polygon_to_cells(&uk, 10, None).unwrap(), 10)
        .unwrap();
    let compacted12 = a5::polygon_to_cells(&uk, 12, None).unwrap();

    let mut g = c.benchmark_group("compact");
    g.bench_function(format!("compact UK res 10 ({} cells)", flat.len()), |b| {
        b.iter(|| black_box(a5::compact(black_box(&flat)).unwrap()))
    });
    g.bench_function(
        format!("uncompact UK res 12 ({} cells)", flat.len() * 16),
        |b| {
            b.iter(|| {
                black_box(
                    a5::uncompact
                        .uncompact_at(black_box(&compacted12), 12)
                        .unwrap(),
                )
            })
        },
    );
    g.finish();
}

criterion_group!(benches, bench_compact);
criterion_main!(benches);
