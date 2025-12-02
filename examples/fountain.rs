//! # Fountain
//!
//! A classic fountain effect using a cone emitter to continuously
//! spray particles upward.
//!
//! ## What This Demonstrates
//!
//! - `Emitter::Cone` - directional particle emission
//! - `Rule::Age` + `Rule::Lifetime` - particle lifecycle
//! - Continuous particle spawning and recycling
//!
//! ## How Emitters Work
//!
//! Emitters find dead particles (`alive == 0`) and respawn them:
//! 1. Set `alive = 1` and `age = 0`
//! 2. Position at emitter location
//! 3. Set velocity based on emitter type
//!
//! The `rate` controls particles per second. Combined with `Lifetime`,
//! this creates a steady-state particle count.
//!
//! ## Cone Emitter Parameters
//!
//! - `position`: Where particles spawn
//! - `direction`: Primary emission direction
//! - `speed`: Initial particle speed
//! - `spread`: Cone half-angle in radians (0 = laser, π/2 = hemisphere)
//! - `rate`: Particles per second
//!
//! ## Try This
//!
//! - Change `direction` to `Vec3::X` for a horizontal jet
//! - Increase `spread` to `0.8` for a wider spray
//! - Add `Rule::WrapWalls` instead of `BounceWalls` for infinite fall
//!
//! Run with: `cargo run --example fountain`

use rdpe::prelude::*;

#[derive(Particle, Clone)]
struct Droplet {
    position: Vec3,
    velocity: Vec3,
    #[color]
    color: Vec3,
}

fn main() {
    // All particles start dead - the emitter will spawn them
    Simulation::<Droplet>::new()
        .with_particle_count(20_000)
        .with_bounds(2.0)
        .with_spawner(|_, _| Droplet {
            position: Vec3::ZERO,
            velocity: Vec3::ZERO,
            color: Vec3::new(0.3, 0.5, 1.0), // Blue water
        })
        // Cone emitter shooting upward
        .with_emitter(Emitter::Cone {
            position: Vec3::new(0.0, -0.5, 0.0),
            direction: Vec3::Y,  // Point up
            speed: 2.5,
            spread: 0.3,         // ~17 degree cone
            rate: 2000.0,        // 2000 particles/second
        })
        // Lifecycle rules
        .with_rule(Rule::Age)            // Increment age each frame
        .with_rule(Rule::Lifetime(5.0))  // Die after 5 seconds
        // Physics
        .with_rule(Rule::Gravity(4.0))
        .with_rule(Rule::Drag(0.3))
        .with_rule(Rule::BounceWalls)
        .run();
}
