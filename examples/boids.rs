//! Boids flocking simulation
//!
//! Tests neighbor-based rules: separation, cohesion, and alignment.
//! Run with: cargo run --example boids

use rdpe::prelude::*;

#[derive(Particle, Clone)]
struct Boid {
    position: Vec3,
    velocity: Vec3,
}

fn random(seed: u32) -> f32 {
    let x = seed.wrapping_mul(1103515245).wrapping_add(12345);
    let x = x ^ (x >> 16);
    (x & 0x7FFFFFFF) as f32 / 0x7FFFFFFF as f32
}

fn main() {
    Simulation::<Boid>::new()
        .with_particle_count(5_000)
        .with_bounds(1.0)
        .with_spatial_config(0.1, 32) // Cell size 0.1, 32^3 grid
        .with_spawner(|i, _count| {
            let x = (random(i * 3) - 0.5) * 1.5;
            let y = (random(i * 3 + 1) - 0.5) * 1.5;
            let z = (random(i * 3 + 2) - 0.5) * 1.5;

            let vx = (random(i * 3 + 100) - 0.5) * 0.5;
            let vy = (random(i * 3 + 101) - 0.5) * 0.5;
            let vz = (random(i * 3 + 102) - 0.5) * 0.5;

            Boid {
                position: Vec3::new(x, y, z),
                velocity: Vec3::new(vx, vy, vz),
            }
        })
        // Neighbor-based flocking rules
        .with_rule(Rule::Separate { radius: 0.05, strength: 5.0 })
        .with_rule(Rule::Cohere { radius: 0.15, strength: 1.0 })
        .with_rule(Rule::Align { radius: 0.1, strength: 2.0 })
        // Basic physics
        .with_rule(Rule::Drag(2.0))
        .with_rule(Rule::BounceWalls)
        .run();
}
