//! # Lifecycle Demo
//!
//! Demonstrates the particle lifecycle system with presets and the builder API.
//!
//! ## What This Demonstrates
//!
//! - `Lifecycle::fire()` - Rising, fading embers
//! - `Lifecycle::fountain()` - Arcing water particles
//! - `with_lifecycle(|l| ...)` - Custom lifecycle configuration
//! - Auto-injected `age`, `alive`, `scale` fields
//!
//! ## Hidden Lifecycle Fields
//!
//! Every particle automatically has these fields (accessible in custom WGSL):
//! - `p.age` - Time since spawn (seconds)
//! - `p.alive` - 0 = dead, 1 = alive
//! - `p.scale` - Visual size multiplier
//!
//! ## WGSL Lifecycle Helpers
//!
//! Available in custom rules:
//! - `is_alive(p)` / `is_dead(p)` - Check particle state
//! - `kill_particle(&p)` - Mark particle as dead
//! - `respawn_particle(&p)` - Reset lifecycle state
//! - `respawn_at(&p, pos, vel)` - Respawn at position
//!
//! Run with: `cargo run --example lifecycle_demo`

use rdpe::prelude::*;

#[derive(rdpe::Particle, Clone)]
struct Spark {
    position: Vec3,
    velocity: Vec3,
    #[color]
    color: Vec3,
}

fn main() {
    Simulation::<Spark>::new()
        .with_particle_count(15_000)
        .with_bounds(1.5)
        .with_particle_size(0.012)
        // Start all particles dead - emitters will spawn them
        .with_spawner(|_, _| Spark {
            position: Vec3::ZERO,
            velocity: Vec3::ZERO,
            color: Vec3::ONE,
        })
        // Fire preset on the left
        .with_lifecycle_preset(Lifecycle::fire(Vec3::new(-0.5, -0.8, 0.0), 800.0))
        // Fountain preset on the right
        .with_lifecycle_preset(Lifecycle::fountain(Vec3::new(0.5, -0.8, 0.0), 600.0))
        // Custom sparkler in the center using builder API
        .with_lifecycle(|l| {
            l.lifetime(0.8)
                .fade_out()
                .shrink_out()
                .color_over_life(
                    Vec3::new(1.0, 1.0, 1.0), // White
                    Vec3::new(1.0, 0.3, 0.0), // Orange
                )
                .emitter(Emitter::Sphere {
                    center: Vec3::new(0.0, 0.0, 0.0),
                    radius: 0.05,
                    speed: 1.5,
                    rate: 1500.0,
                })
        })
        // Physics
        .with_rule(Rule::Gravity(2.0))
        .with_rule(Rule::Drag(0.5))
        // Custom rule showing lifecycle helpers
        .with_rule(Rule::Custom(
            r#"
            // Kill particles that escape bounds (using lifecycle helper)
            if length(p.position) > 1.8 {
                kill_particle(&p);
            }
        "#
            .into(),
        ))
        .with_visuals(|v| {
            v.blend_mode(BlendMode::Additive);
            v.background(Vec3::new(0.02, 0.02, 0.03));
        })
        .run();
}
