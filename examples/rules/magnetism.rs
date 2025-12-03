//! # Magnetism Demo
//!
//! Demonstrates `Rule::Magnetism` - charge-based attraction and repulsion.
//! Red and blue particles have opposite polarity: opposites attract,
//! same repels.
//!
//! ## What This Demonstrates
//!
//! - `Rule::Magnetism` - type-based attraction/repulsion
//! - `particle_type` field for distinguishing particle polarities
//! - `same_repel: true` to make same-type particles push away
//! - `Rule::Collide` for soft collision response
//! - Color-coding to visualize particle types
//!
//! ## The Physics
//!
//! **Electromagnetic Analogy**: Like charges repel, opposite charges attract.
//! Particles with `particle_type = 0` (red/positive) attract particles with
//! `particle_type = 1` (blue/negative), while same-type particles repel.
//!
//! **Cluster Formation**: Over time, alternating red-blue clusters form as
//! opposites pair up while pushing away same-polarity neighbors. This
//! creates crystal-like structures.
//!
//! **Collision Response**: The `Collide` rule prevents particles from
//! overlapping when attraction brings them too close, maintaining
//! distinct particles rather than collapse.
//!
//! ## Try This
//!
//! - Set `same_repel: false` to make everything attract (black hole effect)
//! - Increase `strength` to 5.0 for violent pairing dynamics
//! - Add a third type with `particle_type = 2` for complex interactions
//! - Remove `Collide` to see particles overlap at attraction points
//! - Add `Rule::Turbulence` to prevent stable configurations
//!
//! Run with: `cargo run --example magnetism`

use rand::Rng;
use rdpe::prelude::*;

#[derive(Particle, Clone)]
struct Magnet {
    position: Vec3,
    velocity: Vec3,
    particle_type: u32, // 0 = positive (red), 1 = negative (blue)
    #[color]
    color: Vec3,
}

fn main() {
    let mut rng = rand::thread_rng();

    let particles: Vec<Magnet> = (0..8_000)
        .map(|i| {
            // Alternate between positive and negative charges
            let polarity = (i % 2) as u32;
            let color = if polarity == 0 {
                Vec3::new(1.0, 0.3, 0.2) // Red = positive
            } else {
                Vec3::new(0.2, 0.4, 1.0) // Blue = negative
            };

            Magnet {
                position: Vec3::new(
                    rng.gen_range(-0.8..0.8),
                    rng.gen_range(-0.8..0.8),
                    rng.gen_range(-0.8..0.8),
                ),
                velocity: Vec3::new(
                    rng.gen_range(-0.2..0.2),
                    rng.gen_range(-0.2..0.2),
                    rng.gen_range(-0.2..0.2),
                ),
                particle_type: polarity,
                color,
            }
        })
        .collect();

    Simulation::<Magnet>::new()
        .with_particle_count(8_000)
        .with_particle_size(0.015)
        .with_bounds(1.0)
        .with_spawner(move |i, _| particles[i as usize].clone())
        .with_spatial_config(0.25, 32)
        // Magnetism: same types repel, opposites attract
        .with_rule(Rule::Magnetism {
            radius: 0.3,
            strength: 2.0,
            same_repel: true,
        })
        // Soft collision to prevent overlap
        .with_rule(Rule::Collide {
            radius: 0.03,
            response: 0.5,
        })
        .with_rule(Rule::Drag(1.0))
        .with_rule(Rule::SpeedLimit { min: 0.0, max: 2.0 })
        .with_rule(Rule::BounceWalls)
        .run();
}
