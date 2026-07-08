// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use criterion::{black_box, criterion_group, criterion_main, Criterion};

mod common;

fn bench_grid_disk(c: &mut Criterion) {
    let london = a5::lonlat_to_cell(a5::LonLat::new(-0.1276, 51.5072), 9).unwrap();

    let mut g = c.benchmark_group("gridDisk");
    for k in [1usize, 5, 20] {
        g.bench_function(format!("gridDisk k={k}"), |b| {
            b.iter(|| black_box(a5::grid_disk(black_box(london), k).unwrap()))
        });
    }
    g.bench_function("gridDiskVertex k=5", |b| {
        b.iter(|| black_box(a5::grid_disk_vertex(black_box(london), 5).unwrap()))
    });
    g.finish();
}

criterion_group!(benches, bench_grid_disk);
criterion_main!(benches);
