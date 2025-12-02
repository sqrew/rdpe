//! # Explosion
//!
//! A one-time burst of particles using the Burst emitter with lifecycle effects.
//!
//! ## What This Demonstrates
//!
//! - `Emitter::Burst` - spawns all particles at once
//! - `Rule::ColorOverLife` - transition from yellow to red
//! - `Rule::FadeOut` - dim to black over lifetime
//! - `Rule::ShrinkOut` - shrink particles as they age
//!
//! ## Lifecycle Rules
//!
//! These rules create dynamic visual effects based on particle age:
//!
//! ```ignore
//! .with_rule(Rule::Age)                    // Required: track age
//! .with_rule(Rule::ColorOverLife { ... })  // Color transition
//! .with_rule(Rule::FadeOut(2.0))           // Dim to black
//! .with_rule(Rule::ShrinkOut(2.0))         // Shrink to nothing
//! .with_rule(Rule::Lifetime(2.0))          // Die at end
//! ```
//!
//! ## Try This
//!
//! - Change ColorOverLife colors (blue to white for ice)
//! - Remove FadeOut but keep ShrinkOut (or vice versa)
//! - Increase speed for more violent explosion
//! - Remove gravity for space explosion
//!
//! Run with: `cargo run --example explosion`

use rdpe::prelude::*;

#[derive(Particle, Clone)]
struct Spark {
    position: Vec3,
    velocity: Vec3,
    #[color]
    color: Vec3,
}

fn main() {
    Simulation::<Spark>::new()
        .with_particle_count(5_000)
        .with_bounds(2.0)
        .with_spawner(|_, _| Spark {
            position: Vec3::ZERO,
            velocity: Vec3::ZERO,
            color: Vec3::new(1.0, 1.0, 0.8), // Bright yellow-white
        })
        // Burst emitter - fires once at start
        .with_emitter(Emitter::Burst {
            position: Vec3::ZERO,
            count: 5000,
            speed: 3.0,
        })
        // === Lifecycle Effects ===
        .with_rule(Rule::Age)
        // Yellow-white → Orange → Red over lifetime
        .with_rule(Rule::ColorOverLife {
            start: Vec3::new(1.0, 1.0, 0.6), // Bright yellow
            end: Vec3::new(1.0, 0.2, 0.0),   // Deep red
            duration: 2.0,
        })
        .with_rule(Rule::FadeOut(2.0))   // Dim to black
        .with_rule(Rule::ShrinkOut(2.0)) // Shrink as they die
        .with_rule(Rule::Lifetime(2.0))  // Die at 2 seconds
        // Physics
        .with_rule(Rule::Gravity(3.0))
        .with_rule(Rule::Drag(1.0))
        .with_rule(Rule::BounceWalls)
        .run();
}
