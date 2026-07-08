// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use std::f64::consts::PI;

use criterion::{black_box, criterion_group, criterion_main, Criterion};

use a5::coordinate_systems::{Face, Polar, Radians, Spherical};
use a5::core::cell::cell_to_spherical;
use a5::core::serialization::deserialize;
use a5::projections::{AuthalicProjection, DodecahedronProjection, GnomonicProjection};

mod common;

const N: usize = 256;

fn bench_projections(c: &mut Criterion) {
    // Spherical points paired with the origin of the face they fall on.
    let cells = common::sample_cells(10, N, 42);
    let sphericals: Vec<Spherical> = cells
        .iter()
        .map(|&cell| cell_to_spherical(cell).unwrap())
        .collect();
    let origin_ids: Vec<u8> = cells
        .iter()
        .map(|&cell| deserialize(cell).unwrap().origin_id)
        .collect();

    let mut dodec = DodecahedronProjection::new().unwrap();
    let faces: Vec<Face> = (0..N)
        .map(|n| dodec.forward(sphericals[n], origin_ids[n]).unwrap())
        .collect();

    let mut g = c.benchmark_group("dodecahedron projection");
    let mut i = 0usize;
    g.bench_function("forward", |b| {
        b.iter(|| {
            let n = i & (N - 1);
            i += 1;
            black_box(dodec.forward(sphericals[n], origin_ids[n]).unwrap())
        })
    });
    let mut j = 0usize;
    g.bench_function("inverse", |b| {
        b.iter(|| {
            let n = j & (N - 1);
            j += 1;
            black_box(dodec.inverse(faces[n], origin_ids[n]).unwrap())
        })
    });
    g.finish();

    let authalic = AuthalicProjection;
    let gnomonic = GnomonicProjection;
    let mut rng = common::Rng::new(7);
    let phis: Vec<Radians> = (0..N)
        .map(|_| Radians::new_unchecked(PI * (rng.next() - 0.5)))
        .collect();

    let mut g = c.benchmark_group("authalic projection");
    let mut i = 0usize;
    g.bench_function("forward", |b| {
        b.iter(|| {
            let phi = phis[i & (N - 1)];
            i += 1;
            black_box(authalic.forward(black_box(phi)))
        })
    });
    let mut j = 0usize;
    g.bench_function("inverse", |b| {
        b.iter(|| {
            let phi = phis[j & (N - 1)];
            j += 1;
            black_box(authalic.inverse(black_box(phi)))
        })
    });
    g.finish();

    let polars: Vec<Polar> = sphericals.iter().map(|&s| gnomonic.forward(s)).collect();

    let mut g = c.benchmark_group("gnomonic projection");
    let mut i = 0usize;
    g.bench_function("forward", |b| {
        b.iter(|| {
            let s = sphericals[i & (N - 1)];
            i += 1;
            black_box(gnomonic.forward(black_box(s)))
        })
    });
    let mut j = 0usize;
    g.bench_function("inverse", |b| {
        b.iter(|| {
            let p = polars[j & (N - 1)];
            j += 1;
            black_box(gnomonic.inverse(black_box(p)))
        })
    });
    g.finish();
}

criterion_group!(benches, bench_projections);
criterion_main!(benches);
