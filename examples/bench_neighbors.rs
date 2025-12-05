//! # Neighbor Benchmark (With Spatial Hashing)
//!
//! Tests spatial hashing + neighbor query performance.
//! This is the expensive path - radix sort + cell table + neighbor iteration.
//!
//! Run with: `cargo run --example bench_neighbors --release`

use rand::Rng;
use rdpe::prelude::*;

#[derive(Particle, Clone)]
struct Boid {
    position: Vec3,
    velocity: Vec3,
    #[color]
    color: Vec3,
}

fn main() {
    let count: u32 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(50_000);

    println!("=== RDPE Neighbor Benchmark ===");
    println!("Particles: {}", count);
    println!("Rules: Separate, Cohere, Align, Collide, Drag, BounceWalls");
    println!("Spatial hashing: ON (cell_size=0.1, resolution=32, max_neighbors=48)");
    println!();
    println!("Watch the window title for FPS...");

    let mut rng = rand::thread_rng();

    let particles: Vec<Boid> = (0..count)
        .map(|_| Boid {
            position: Vec3::new(
                rng.gen_range(-0.8..0.8),
                rng.gen_range(-0.8..0.8),
                rng.gen_range(-0.8..0.8),
            ),
            velocity: Vec3::new(
                rng.gen_range(-0.3..0.3),
                rng.gen_range(-0.3..0.3),
                rng.gen_range(-0.3..0.3),
            ),
            color: Vec3::new(0.3, 0.6, 1.0),
        })
        .collect();

    Simulation::<Boid>::new()
        .with_particle_count(count)
        .with_particle_size(0.008)
        .with_bounds(1.0)
        .with_spawner(move |i, _| particles[i as usize].clone())
        .with_spatial_config(0.1, 32)
        .with_max_neighbors(48) // Limit neighbors for performance
        // Full boids rules - heavy neighbor usage
        .with_rule(Rule::Separate {
            radius: 0.05,
            strength: 3.0,
        })
        .with_rule(Rule::Cohere {
            radius: 0.15,
            strength: 1.0,
        })
        .with_rule(Rule::Align {
            radius: 0.1,
            strength: 2.0,
        })
        .with_rule(Rule::Collide {
            radius: 0.02,
            restitution: 0.8,
        })
        // Color based on local density (more neighbors = warmer)
        .with_rule(Rule::Custom(r#"
            let density_color = clamp(cohesion_count / 20.0, 0.0, 1.0);
            p.color = mix(vec3<f32>(0.2, 0.4, 1.0), vec3<f32>(1.0, 0.6, 0.2), density_color);
        "#.into()))
        .with_rule(Rule::Drag(1.0))
        .with_rule(Rule::SpeedLimit { min: 0.2, max: 1.5 })
        .with_rule(Rule::BounceWalls)
        .run();
}
