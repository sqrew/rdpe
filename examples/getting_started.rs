//! # Getting Started
//!
//! The simplest possible RDPE simulation - just particles floating in space.
//!
//! ## What This Demonstrates
//!
//! - Defining a particle struct with `#[derive(Particle)]`
//! - Required fields: `position` and `velocity` (both `Vec3`)
//! - Using `with_spawner()` to initialize particles
//! - Running the simulation
//!
//! ## Controls
//!
//! - **Left-click + drag**: Rotate camera
//! - **Scroll wheel**: Zoom in/out
//!
//! ## Try This
//!
//! Add some behavior by uncommenting the rules below!
//!
//! Run with: `cargo run --example getting_started`

use rand::Rng;
use rdpe::prelude::*;

// Every particle needs position and velocity.
// The derive macro generates GPU-compatible code automatically.
#[derive(Particle, Clone)]
struct Ball {
    position: Vec3,
    velocity: Vec3,
}

fn main() {
    let mut rng = rand::thread_rng();

    // Pre-generate particles with random positions
    let particles: Vec<Ball> = (0..10_000)
        .map(|_| Ball {
            position: Vec3::new(
                rng.gen_range(-1.0..1.0),
                rng.gen_range(-1.0..1.0),
                rng.gen_range(-1.0..1.0),
            ),
            velocity: Vec3::ZERO,
        })
        .collect();

    Simulation::<Ball>::new()
        .with_particle_count(10_000)
        .with_spawner(move |i, _| particles[i as usize].clone())
        // Try uncommenting these rules one at a time:
        // .with_rule(Rule::Gravity(9.8))      // Particles fall down
        // .with_rule(Rule::BounceWalls)       // Bounce off boundaries
        // .with_rule(Rule::Drag(1.0))         // Air resistance
        .run();
}
