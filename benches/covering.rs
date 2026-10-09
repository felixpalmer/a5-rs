// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use criterion::{black_box, criterion_group, criterion_main, Criterion};

mod common;

fn bench_set_operations(c: &mut Criterion) {
    // Coverings at resolution 12: two neighboring countries and a cap overlapping both
    let france = a5::polygon_to_cells(&common::load_country("France"), 12, None).unwrap();
    let uk = a5::polygon_to_cells(&common::load_country("United Kingdom"), 12, None).unwrap();
    let paris = a5::lonlat_to_cell(a5::LonLat::new(2.3522, 48.8566), 12).unwrap();
    let cap_paris = a5::spherical_cap(paris, 400_000.0).unwrap();

    // Point-in-polygon probes: cells, at the coverage's resolution, of random points in France's bounding box
    let mut rng = common::Rng::new(7);
    let probes: Vec<u64> = (0..1000)
        .map(|_| {
            let lon = -5.0 + 13.0 * rng.next();
            let lat = 42.0 + 9.0 * rng.next();
            a5::lonlat_to_cell(a5::LonLat::new(lon, lat), 12).unwrap()
        })
        .collect();

    let mut g = c.benchmark_group("set operations");
    g.bench_function(
        format!(
            "union France + UK res 12 ({} compacted cells)",
            france.len() + uk.len()
        ),
        |b| b.iter(|| black_box(a5::union(black_box(&france), black_box(&uk)).unwrap())),
    );
    g.bench_function("intersect France with Paris cap res 12", |b| {
        b.iter(|| black_box(a5::intersect(black_box(&france), black_box(&cap_paris)).unwrap()))
    });
    g.bench_function("difference France minus Paris cap res 12", |b| {
        b.iter(|| black_box(a5::difference(black_box(&france), black_box(&cap_paris)).unwrap()))
    });
    g.bench_function("count France res 12", |b| {
        b.iter(|| black_box(a5::count(black_box(&france)).unwrap()))
    });
    g.bench_function("contains France res 12, 1000 points", |b| {
        b.iter(|| {
            for &probe in &probes {
                black_box(a5::contains(black_box(&france), probe).unwrap());
            }
        })
    });
    g.finish();
}

criterion_group!(benches, bench_set_operations);
criterion_main!(benches);
