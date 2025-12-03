//! # Particle Trails Example
//!
//! Shows particles leaving trails behind them as they move,
//! like comets streaking across space.
//!
//! ## What This Demonstrates
//!
//! - `.with_visuals(|v| v.trails(frames))` - enable motion trails
//! - Trail length controlled by frame count
//! - Trails combined with `BlendMode::Additive` for glow effect
//! - Trails fade from current color to transparent
//!
//! ## How Trails Work
//!
//! The renderer stores the last N positions of each particle.
//! Each frame, lines are drawn connecting historical positions,
//! creating a "tail" effect. The trail fades from full opacity
//! at the particle to transparent at the oldest position.
//!
//! **Performance note**: Trails multiply vertex count by trail length.
//! Use shorter trails (10-20) for many particles, longer trails (50+)
//! for fewer particles.
//!
//! ## Visual Considerations
//!
//! - `BlendMode::Additive` makes trails glow and accumulate brightness
//! - `BlendMode::Alpha` gives solid trails that occlude
//! - Faster particles have more stretched trails
//! - Slow particles show as dots with short tails
//!
//! ## Try This
//!
//! - Increase trail length to 50 for long comet tails
//! - Switch to `BlendMode::Alpha` for solid ribbon trails
//! - Add `Rule::Turbulence` for chaotic, winding paths
//! - Add `Rule::ColorBySpeed` so faster particles have different trails
//! - Reduce particle count but increase trail length for ribbon effect
//!
//! Run with: `cargo run --example trails`

use rand::Rng;
use rdpe::prelude::*;

#[derive(rdpe::Particle, Clone)]
struct Comet {
    position: Vec3,
    velocity: Vec3,
    #[color]
    color: Vec3,
}

fn main() {
    let mut rng = rand::thread_rng();
    let count = 5_000;

    // Pre-generate comets
    let particles: Vec<Comet> = (0..count)
        .map(|_| {
            // Spawn randomly
            let x = rng.gen_range(-0.8..0.8);
            let y = rng.gen_range(-0.8..0.8);
            let z = rng.gen_range(-0.8..0.8);

            // Random initial velocity
            let vx = rng.gen_range(-0.3..0.3);
            let vy = rng.gen_range(-0.3..0.3);
            let vz = rng.gen_range(-0.3..0.3);

            // Random color
            let r = rng.gen_range(0.5..1.0);
            let g = rng.gen_range(0.5..1.0);
            let b = rng.gen_range(0.5..1.0);

            Comet {
                position: Vec3::new(x, y, z),
                velocity: Vec3::new(vx, vy, vz),
                color: Vec3::new(r, g, b),
            }
        })
        .collect();

    Simulation::<Comet>::new()
        .with_particle_count(count)
        .with_bounds(1.0)
        .with_particle_size(0.02)
        .with_spawner(move |i, _| particles[i as usize].clone())
        // Enable trails - 20 frames of history
        .with_visuals(|v| {
            v.trails(20);
            v.blend_mode(BlendMode::Additive); // Glowy trails!
        })
        // Central attraction
        .with_rule(Rule::AttractTo {
            point: Vec3::ZERO,
            strength: 0.5,
        })
        // Some drag
        .with_rule(Rule::Drag(0.3))
        // Bounce off walls
        .with_rule(Rule::BounceWalls)
        .run();
}
