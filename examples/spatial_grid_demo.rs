//! # Spatial Grid Visualization Demo
//!
//! Demonstrates the spatial hash grid visualization feature.
//! Shows how particles are organized in the spatial grid for neighbor queries.
//!
//! ## What This Demonstrates
//!
//! - `.spatial_grid(opacity)` - Enable grid wireframe visualization
//! - How spatial hashing divides space into cells
//! - Useful for debugging spatial configuration
//!
//! Run with: `cargo run --example spatial_grid_demo`

use rand::Rng;
use rdpe::prelude::*;

#[derive(Particle, Clone)]
struct Ball {
    position: Vec3,
    velocity: Vec3,
    #[color]
    color: Vec3,
}

fn main() {
    let mut rng = rand::thread_rng();

    let particles: Vec<(Vec3, Vec3)> = (0..2_000)
        .map(|_| {
            let pos = Vec3::new(
                rng.gen_range(-0.8..0.8),
                rng.gen_range(-0.8..0.8),
                rng.gen_range(-0.8..0.8),
            );
            let vel = Vec3::new(
                rng.gen_range(-0.3..0.3),
                rng.gen_range(-0.3..0.3),
                rng.gen_range(-0.3..0.3),
            );
            (pos, vel)
        })
        .collect();

    Simulation::<Ball>::new()
        .with_particle_count(2_000)
        .with_bounds(1.0)
        .with_particle_size(0.015)
        // Configure spatial hashing - this is what the grid visualizes
        .with_spatial_config(0.15, 16) // cell_size=0.15, 16x16x16 grid
        .with_spawner(move |i, _| {
            let (pos, vel) = particles[i as usize];
            Ball {
                position: pos,
                velocity: vel,
                color: Vec3::new(1.0, 0.8, 0.3),
            }
        })
        .with_visuals(|v| {
            v.spatial_grid(0.2); // Show grid at 20% opacity
            v.background(Vec3::new(0.05, 0.05, 0.1));
        })
        // Simple boids-like behavior that uses the spatial grid
        .with_rule(Rule::Separate {
            radius: 0.1,
            strength: 1.5,
        })
        .with_rule(Rule::Cohere {
            radius: 0.3,
            strength: 0.3,
        })
        .with_rule(Rule::Align {
            radius: 0.2,
            strength: 0.5,
        })
        .with_rule(Rule::SpeedLimit { min: 0.1, max: 0.8 })
        .with_rule(Rule::BounceWalls)
        .run();
}
