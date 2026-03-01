// Performance benchmarks for tessellation

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use rvm_rs::export::cache::GeometryCache;
use rvm_rs::export::tessellator::{Tessellate, TessellateWithCaps};
use rvm_rs::store::geometry::*;

fn bench_cylinder_tessellation(c: &mut Criterion) {
    let cylinder = Cylinder {
        radius: 1.0,
        height: 2.0,
    };

    c.bench_function("cylinder_with_caps", |b| {
        b.iter(|| black_box(cylinder.tessellate(0.01, 1.0)))
    });

    c.bench_function("cylinder_without_caps", |b| {
        b.iter(|| black_box(cylinder.tessellate_with_caps(0.01, 1.0, &[false, false])))
    });
}

fn bench_sphere_tessellation(c: &mut Criterion) {
    let sphere = Sphere { radius: 1.0 };

    let mut group = c.benchmark_group("sphere_tessellation");

    for tolerance in [0.1, 0.01, 0.001].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("tol_{}", tolerance)),
            tolerance,
            |b, &tol| b.iter(|| black_box(sphere.tessellate(tol, 1.0))),
        );
    }

    group.finish();
}

fn bench_circular_torus_tessellation(c: &mut Criterion) {
    let torus = CircularTorus {
        offset: 2.0,
        radius: 0.5,
        angle: std::f32::consts::PI,
    };

    c.bench_function("circular_torus_with_caps", |b| {
        b.iter(|| black_box(torus.tessellate(0.01, 1.0)))
    });

    c.bench_function("circular_torus_without_caps", |b| {
        b.iter(|| black_box(torus.tessellate_with_caps(0.01, 1.0, &[false, false])))
    });
}

fn bench_snout_tessellation(c: &mut Criterion) {
    let snout = Snout {
        radius_bottom: 1.0,
        radius_top: 0.5,
        height: 2.0,
        offset_x: 0.1,
        offset_y: 0.1,
        bottom_shear_x: 0.1,
        bottom_shear_y: 0.0,
        top_shear_x: -0.1,
        top_shear_y: 0.0,
    };

    c.bench_function("snout_with_shear", |b| {
        b.iter(|| black_box(snout.tessellate(0.01, 1.0)))
    });
}

fn bench_geometry_cache(c: &mut Criterion) {
    let cylinder = Cylinder {
        radius: 1.0,
        height: 2.0,
    };

    let mut cache = GeometryCache::new();

    c.bench_function("cache_miss", |b| {
        b.iter(|| {
            cache.clear();
            black_box(cache.get_or_tessellate(&cylinder, 0.01, 1.0))
        })
    });

    c.bench_function("cache_hit", |b| {
        // Pre-populate cache
        cache.clear();
        cache.get_or_tessellate(&cylinder, 0.01, 1.0);

        b.iter(|| black_box(cache.get_or_tessellate(&cylinder, 0.01, 1.0)))
    });
}

fn bench_multiple_geometries(c: &mut Criterion) {
    c.bench_function("mixed_geometries", |b| {
        b.iter(|| {
            let cylinder = Cylinder {
                radius: 1.0,
                height: 2.0,
            };
            black_box(cylinder.tessellate(0.01, 1.0));

            let sphere = Sphere { radius: 1.0 };
            black_box(sphere.tessellate(0.01, 1.0));

            let bx = rvm_rs::store::geometry::Box {
                lengths: [2.0, 2.0, 2.0],
            };
            black_box(bx.tessellate(0.01, 1.0));

            let pyramid = Pyramid {
                bottom: [2.0, 2.0],
                top: [1.0, 1.0],
                offset: [0.0, 0.0],
                height: 3.0,
            };
            black_box(pyramid.tessellate(0.01, 1.0));
        })
    });
}

criterion_group!(
    benches,
    bench_cylinder_tessellation,
    bench_sphere_tessellation,
    bench_circular_torus_tessellation,
    bench_snout_tessellation,
    bench_geometry_cache,
    bench_multiple_geometries,
);

criterion_main!(benches);
