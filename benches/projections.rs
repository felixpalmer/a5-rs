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
use common::BATCH;

fn bench_projections(c: &mut Criterion) {
    // Spherical points paired with the origin of the face they fall on.
    let cells = common::sample_cells(10, BATCH, 42);
    let sphericals: Vec<Spherical> = cells
        .iter()
        .map(|&cell| cell_to_spherical(cell).unwrap())
        .collect();
    let origin_ids: Vec<u8> = cells
        .iter()
        .map(|&cell| deserialize(cell).unwrap().origin_id)
        .collect();

    let mut dodec = DodecahedronProjection::new().unwrap();
    let faces: Vec<Face> = (0..BATCH)
        .map(|n| dodec.forward(sphericals[n], origin_ids[n]).unwrap())
        .collect();

    let mut g = c.benchmark_group("dodecahedron projection");
    g.bench_function("forward ×100", |b| {
        b.iter(|| {
            for (&s, &origin_id) in sphericals.iter().zip(&origin_ids) {
                black_box(dodec.forward(black_box(s), black_box(origin_id)).unwrap());
            }
        })
    });
    g.bench_function("inverse ×100", |b| {
        b.iter(|| {
            for (&face, &origin_id) in faces.iter().zip(&origin_ids) {
                black_box(
                    dodec
                        .inverse(black_box(face), black_box(origin_id))
                        .unwrap(),
                );
            }
        })
    });
    g.finish();

    let authalic = AuthalicProjection;
    let gnomonic = GnomonicProjection;
    let mut rng = common::Rng::new(7);
    let phis: Vec<Radians> = (0..BATCH)
        .map(|_| Radians::new_unchecked(PI * (rng.next() - 0.5)))
        .collect();

    let mut g = c.benchmark_group("authalic projection");
    g.bench_function("forward ×100", |b| {
        b.iter(|| {
            for &phi in &phis {
                black_box(authalic.forward(black_box(phi)));
            }
        })
    });
    g.bench_function("inverse ×100", |b| {
        b.iter(|| {
            for &phi in &phis {
                black_box(authalic.inverse(black_box(phi)));
            }
        })
    });
    g.finish();

    let polars: Vec<Polar> = sphericals.iter().map(|&s| gnomonic.forward(s)).collect();

    let mut g = c.benchmark_group("gnomonic projection");
    g.bench_function("forward ×100", |b| {
        b.iter(|| {
            for &s in &sphericals {
                black_box(gnomonic.forward(black_box(s)));
            }
        })
    });
    g.bench_function("inverse ×100", |b| {
        b.iter(|| {
            for &p in &polars {
                black_box(gnomonic.inverse(black_box(p)));
            }
        })
    });
    g.finish();
}

criterion_group!(benches, bench_projections);
criterion_main!(benches);
