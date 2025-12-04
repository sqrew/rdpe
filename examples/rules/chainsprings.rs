//! # Rope Simulation
//!
//! Demonstrates `Rule::ChainSprings` - the simplest way to make a rope.
//! No bond fields needed - just spawn particles in order!
//!
//! Run with: `cargo run --example rope`

use rdpe::prelude::*;

const SEGMENTS: u32 = 40;
const SPACING: f32 = 0.025;

#[derive(Particle, Clone)]
struct RopePoint {
    position: Vec3,
    velocity: Vec3,
    #[color]
    color: Vec3,
    pinned: f32,
}

fn main() {
    Simulation::<RopePoint>::new()
        .with_particle_count(SEGMENTS)
        .with_bounds(2.0)
        .with_spawner(|i, _| {
            // Spawn in a vertical line
            let y = 0.5 - (i as f32 * SPACING);

            // Color gradient along rope
            let t = i as f32 / SEGMENTS as f32;
            let color = Vec3::new(1.0 - t * 0.5, 0.3 + t * 0.4, 0.2 + t * 0.6);

            RopePoint {
                position: Vec3::new(0.0, y, 0.0),
                velocity: Vec3::ZERO,
                color,
                pinned: if i == 0 { 1.0 } else { 0.0 },  // Pin first point
            }
        })
        // Skip pinned particles
        .with_rule(Rule::Custom("if p.pinned > 0.5 { return; }".into()))
        // THE MAGIC - one line for rope physics!
        .with_rule(Rule::ChainSprings {
            stiffness: 600.0,
            damping: 12.0,
            rest_length: SPACING,
            max_stretch: Some(1.3),
        })
        // Gravity
        .with_rule(Rule::Gravity(4.0))
        // Wind
        .with_rule(Rule::Custom(r#"
            let wind = sin(uniforms.time * 2.0 + p.position.y * 10.0) * 1.5;
            p.velocity.x += wind * uniforms.delta_time;
        "#.into()))
        // Damping
        .with_rule(Rule::Custom("p.velocity *= 0.98;".into()))
        .with_visuals(|v| {
            v.background(Vec3::new(0.02, 0.02, 0.05));
            v.connections(SPACING * 1.5);
        })
        .with_particle_size(0.015)
        .run();
}
