//! # Magnetism Demo
//!
//! Demonstrates `Rule::Magnetism` - charge-based attraction and repulsion.
//! Red and blue particles have opposite polarity: opposites attract,
//! same repels.
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
