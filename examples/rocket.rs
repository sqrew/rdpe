//! # Rocket Exhaust
//!
//! A rocket thruster effect using a Cone emitter.
//!
//! ## What This Demonstrates
//!
//! - `Emitter::Cone` pointing downward
//! - Tight spread for focused exhaust
//! - High speed for dramatic effect
//!
//! ## Cone Parameters
//!
//! - `position`: Nozzle location
//! - `direction`: Thrust direction (Vec3::NEG_Y = down)
//! - `speed`: How fast particles exit
//! - `spread`: Cone width in radians
//!   - 0.0 = laser beam
//!   - 0.2 = tight exhaust (~11°)
//!   - 0.5 = wider spray (~29°)
//!   - π/2 = hemisphere
//!
//! ## Try This
//!
//! - Move position to create flying rocket trail
//! - Add `Rule::Custom` to change color based on age (orange → gray)
//! - Use uniforms to make the rocket move over time
//! - Add multiple cones for multi-engine effect
//!
//! Run with: `cargo run --example rocket`

use rdpe::prelude::*;

#[derive(Particle, Clone)]
struct Exhaust {
    position: Vec3,
    velocity: Vec3,
    #[color]
    color: Vec3,
}

fn main() {
    Simulation::<Exhaust>::new()
        .with_particle_count(10_000)
        .with_bounds(2.0)
        .with_spawner(|_, _| Exhaust {
            position: Vec3::ZERO,
            velocity: Vec3::ZERO,
            color: Vec3::new(1.0, 0.4, 0.1), // Fiery orange
        })
        // Cone emitter pointing down (rocket exhaust)
        .with_emitter(Emitter::Cone {
            position: Vec3::new(0.0, 0.5, 0.0),
            direction: Vec3::NEG_Y, // Thrust downward
            speed: 3.0,
            spread: 0.2, // Tight cone
            rate: 3000.0,
        })
        // Lifecycle
        .with_rule(Rule::Age)
        .with_rule(Rule::Lifetime(1.5))
        // Physics - no gravity, just drag to slow exhaust
        .with_rule(Rule::Drag(1.0))
        .with_rule(Rule::BounceWalls)
        .run();
}
