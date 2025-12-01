//! Basic bouncing particles example
//!
//! Run with: cargo run --example bouncing

use rdpe::prelude::*;

/// Our particle type - just position and velocity
/// No manual padding needed - the derive macro handles it!
#[derive(Particle, Clone)]
struct BouncingParticle {
    position: Vec3,
    velocity: Vec3,
}

/// Simple pseudo-random number generator
fn random(seed: u32) -> f32 {
    let x = seed.wrapping_mul(1103515245).wrapping_add(12345);
    let x = x ^ (x >> 16);
    (x & 0x7FFFFFFF) as f32 / 0x7FFFFFFF as f32
}

fn main() {
    Simulation::<BouncingParticle>::new()
        .with_particle_count(10_000)
        .with_bounds(1.0)
        .with_spawner(|i, count| {
            // Spawn in a spiral shell pattern
            let t = i as f32 / count as f32;
            let theta = t * std::f32::consts::TAU * 20.0;
            let phi = (t * 2.0 - 1.0).acos();
            let r = 0.5 + 0.5 * ((t * 50.0).sin() * 0.5 + 0.5);

            // Random velocity
            let vx = (random(i * 3) - 0.5) * 2.0;
            let vy = (random(i * 3 + 1) - 0.5) * 2.0;
            let vz = (random(i * 3 + 2) - 0.5) * 2.0;

            BouncingParticle {
                position: Vec3::new(
                    r * phi.sin() * theta.cos(),
                    r * phi.sin() * theta.sin(),
                    r * phi.cos(),
                ),
                velocity: Vec3::new(vx, vy, vz),
            }
        })
        .with_rule(Rule::BounceWalls)
        .run();
}
