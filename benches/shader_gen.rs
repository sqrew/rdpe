//! Benchmarks for shader generation and CPU-side operations.
//!
//! Run with: `cargo bench`

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use glam::Vec3;

// Import rules directly
use rdpe::rules::Rule;

fn bench_rule_to_wgsl(c: &mut Criterion) {
    let mut group = c.benchmark_group("rule_to_wgsl");

    // Simple rules
    group.bench_function("gravity", |b| {
        let rule = Rule::Gravity(9.8);
        b.iter(|| black_box(rule.to_wgsl(1.0)))
    });

    group.bench_function("bounce_walls", |b| {
        let rule = Rule::BounceWalls;
        b.iter(|| black_box(rule.to_wgsl(1.0)))
    });

    group.bench_function("turbulence", |b| {
        let rule = Rule::Turbulence {
            scale: 2.0,
            strength: 1.5,
        };
        b.iter(|| black_box(rule.to_wgsl(1.0)))
    });

    group.bench_function("vortex", |b| {
        let rule = Rule::Vortex {
            center: Vec3::ZERO,
            axis: Vec3::Y,
            strength: 1.0,
        };
        b.iter(|| black_box(rule.to_wgsl(1.0)))
    });

    group.bench_function("custom_short", |b| {
        let rule = Rule::Custom("p.velocity *= 0.99;".into());
        b.iter(|| black_box(rule.to_wgsl(1.0)))
    });

    group.bench_function("custom_long", |b| {
        let rule = Rule::Custom(r#"
            let dist = length(p.position);
            let t = clamp(dist / 2.0, 0.0, 1.0);
            p.color = mix(vec3<f32>(1.0, 0.0, 0.0), vec3<f32>(0.0, 0.0, 1.0), t);
            p.velocity += normalize(p.position) * 0.1;
        "#.into());
        b.iter(|| black_box(rule.to_wgsl(1.0)))
    });

    group.finish();
}

fn bench_neighbor_wgsl(c: &mut Criterion) {
    let mut group = c.benchmark_group("neighbor_to_wgsl");

    group.bench_function("separate", |b| {
        let rule = Rule::Separate {
            radius: 0.1,
            strength: 2.0,
        };
        b.iter(|| black_box(rule.to_neighbor_wgsl()))
    });

    group.bench_function("cohere", |b| {
        let rule = Rule::Cohere {
            radius: 0.2,
            strength: 1.0,
        };
        b.iter(|| black_box(rule.to_neighbor_wgsl()))
    });

    group.bench_function("nbody", |b| {
        let rule = Rule::NBodyGravity {
            strength: 0.5,
            softening: 0.01,
            radius: 0.5,
        };
        b.iter(|| black_box(rule.to_neighbor_wgsl()))
    });

    group.finish();
}

fn bench_many_rules(c: &mut Criterion) {
    let mut group = c.benchmark_group("many_rules");

    let simple_rules = vec![
        Rule::Gravity(9.8),
        Rule::Drag(0.5),
        Rule::Turbulence { scale: 2.0, strength: 1.5 },
        Rule::Vortex { center: Vec3::ZERO, axis: Vec3::Y, strength: 1.0 },
        Rule::SpeedLimit { min: 0.0, max: 3.0 },
        Rule::BounceWalls,
    ];

    for count in [1, 3, 6] {
        group.bench_with_input(
            BenchmarkId::new("rules", count),
            &count,
            |b, &count| {
                let rules: Vec<_> = simple_rules.iter().take(count).collect();
                b.iter(|| {
                    for rule in &rules {
                        black_box(rule.to_wgsl(1.0));
                    }
                })
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_rule_to_wgsl,
    bench_neighbor_wgsl,
    bench_many_rules,
);
criterion_main!(benches);
