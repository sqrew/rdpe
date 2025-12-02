//! # Bouncing Particles
//!
//! Particles with initial velocities bouncing off the simulation boundaries.
//!
//! ## What This Demonstrates
//!
//! - `Rule::BounceWalls` - reflects velocity when hitting bounds
//! - `with_bounds()` - sets the simulation cube size
//! - Creative spawner patterns (spiral shell)
//!
//! ## The Physics
//!
//! `BounceWalls` checks if a particle is outside the bounds and:
//! 1. Clamps position back inside
//! 2. Reflects velocity component (multiplied by -1)
//!
//! ## Try This
//!
//! - Add `Rule::Gravity(5.0)` to see them fall and bounce
//! - Add `Rule::Drag(0.5)` to slow them down over time
//! - Change bounds to `2.0` for a larger space
//!
//! Run with: `cargo run --example bounce_walls`

use rand::Rng;
use rdpe::prelude::*;

#[derive(Particle, Clone)]
struct BouncingParticle {
    position: Vec3,
    velocity: Vec3,
}

fn main() {
    let mut rng = rand::thread_rng();

    // Pre-generate random velocities
    let velocities: Vec<Vec3> = (0..10_000)
        .map(|_| {
            Vec3::new(
                rng.gen_range(-1.0..1.0),
                rng.gen_range(-1.0..1.0),
                rng.gen_range(-1.0..1.0),
            )
        })
        .collect();

    Simulation::<BouncingParticle>::new()
        .with_particle_count(10_000)
        .with_bounds(1.0) // Cube from -1 to +1
        .with_spawner(move |i, count| {
            // Spawn in a spiral shell pattern for visual interest
            let t = i as f32 / count as f32;
            let theta = t * std::f32::consts::TAU * 20.0;
            let phi = (t * 2.0 - 1.0).acos();
            let r = 0.5 + 0.5 * ((t * 50.0).sin() * 0.5 + 0.5);

            BouncingParticle {
                position: Vec3::new(
                    r * phi.sin() * theta.cos(),
                    r * phi.sin() * theta.sin(),
                    r * phi.cos(),
                ),
                velocity: velocities[i as usize],
            }
        })
        .with_rule(Rule::BounceWalls)
        .run();
}
