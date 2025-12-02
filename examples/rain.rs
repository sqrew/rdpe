//! # Rain
//!
//! Falling rain using a Box emitter at the top of the simulation.
//!
//! ## What This Demonstrates
//!
//! - `Emitter::Box` - spawns particles within a volume
//! - Uniform initial velocity for all particles
//! - High emission rate for dense effects
//!
//! ## How Box Emitter Works
//!
//! The Box emitter spawns particles at random positions within
//! a rectangular volume defined by `min` and `max` corners.
//! All particles get the same `velocity`.
//!
//! Great for:
//! - Rain (box at top, velocity down)
//! - Snow (box at top, slower velocity)
//! - Area fills (large box, zero velocity)
//!
//! ## Try This
//!
//! - Reduce velocity for slow snow
//! - Add `Rule::Wander` for wind effect
//! - Use `Rule::Custom` to fade color based on age
//! - Add horizontal velocity component for angled rain
//!
//! Run with: `cargo run --example rain`

use rdpe::prelude::*;

#[derive(Particle, Clone)]
struct Raindrop {
    position: Vec3,
    velocity: Vec3,
    #[color]
    color: Vec3,
}

fn main() {
    Simulation::<Raindrop>::new()
        .with_particle_count(20_000)
        .with_bounds(2.0)
        .with_spawner(|_, _| Raindrop {
            position: Vec3::ZERO,
            velocity: Vec3::ZERO,
            color: Vec3::new(0.5, 0.6, 0.9), // Light blue
        })
        // Box emitter spawning from ceiling
        .with_emitter(Emitter::Box {
            min: Vec3::new(-1.5, 0.8, -1.5),  // Thin slice at top
            max: Vec3::new(1.5, 0.9, 1.5),
            velocity: Vec3::new(0.0, -2.0, 0.0), // Falling down
            rate: 5000.0,
        })
        // Lifecycle
        .with_rule(Rule::Age)
        .with_rule(Rule::Lifetime(2.0))
        // Physics
        .with_rule(Rule::Gravity(3.0))
        .with_rule(Rule::WrapWalls) // Wrap so rain continues
        .run();
}
