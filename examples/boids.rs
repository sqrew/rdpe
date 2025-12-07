//! # Boids Flocking
//!
//! Classic boids algorithm: separation, cohesion, and alignment create
//! emergent flocking behavior from simple local rules.
//!
//! ## What This Demonstrates
//!
//! - `with_spatial_config()` - enables efficient neighbor queries
//! - `Rule::Separate` - avoid crowding nearby boids
//! - `Rule::Cohere` - steer toward average position of neighbors
//! - `Rule::Align` - match velocity with nearby boids
//!
//! ## The Algorithm
//!
//! Each boid looks at neighbors within a radius and:
//! 1. **Separate**: Steer away from very close neighbors (avoid collision)
//! 2. **Cohere**: Steer toward the center of nearby neighbors (stay together)
//! 3. **Align**: Match the average heading of neighbors (move as a group)
//!
//! ## Spatial Hashing
//!
//! `with_spatial_config(cell_size, resolution)` divides space into a grid
//! for O(1) neighbor lookups instead of O(n²) brute force.
//!
//! - `cell_size` should be >= your largest interaction radius
//! - `resolution` must be power of 2 (8, 16, 32, 64)
//!
//! ## Try This
//!
//! - Adjust strengths to change flocking tightness
//! - Add `Rule::SpeedLimit { min: 0.1, max: 1.0 }` for more natural motion
//! - Try `Rule::WrapWalls` instead of `BounceWalls` for toroidal space
//!
//! Run with: `cargo run --example boids`

use rdpe::prelude::*;

#[derive(Particle, Clone)]
struct Boid {
    position: Vec3,
    velocity: Vec3,
}

fn main() {
    Simulation::<Boid>::new()
        .with_particle_count(5_000)
        .with_bounds(1.0)
        // Enable spatial hashing for neighbor queries
        // Cell size 0.1 (>= largest radius), 32^3 grid
        .with_spatial_config(0.1, 32)
        .with_spawner(|ctx| Boid {
            position: ctx.random_in_cube(0.75),
            velocity: ctx.random_direction() * 0.25,
        })
        // The three classic boids rules
        .with_rule(Rule::Separate {
            radius: 0.05,   // Avoid neighbors within this distance
            strength: 5.0,  // How hard to push away
        })
        .with_rule(Rule::Cohere {
            radius: 0.15,   // Consider neighbors within this distance
            strength: 1.0,  // How strongly to move toward center
        })
        .with_rule(Rule::Align {
            radius: 0.1,    // Match velocity of neighbors within this distance
            strength: 2.0,  // How quickly to align
        })
        // Keep things stable
        .with_rule(Rule::Drag(2.0))
        .with_rule(Rule::BounceWalls)
        .run();
}
