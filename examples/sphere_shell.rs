//! # Sphere Shell
//!
//! An expanding shell of particles using the Sphere emitter.
//!
//! ## What This Demonstrates
//!
//! - `Emitter::Sphere` - spawns on sphere surface
//! - Outward velocity creates expanding shell
//! - Low speed + low drag = long-lasting expansion
//!
//! ## How Sphere Emitter Works
//!
//! Particles spawn at random points on a sphere surface:
//! 1. Random direction is chosen (uniform on sphere)
//! 2. Position = center + direction * radius
//! 3. Velocity = direction * speed
//!
//! Use negative speed for implosion (particles move inward).
//!
//! ## Try This
//!
//! - Use negative speed for collapsing shell
//! - Increase radius for larger shell
//! - Add `Rule::Gravity` for falling shell
//! - Use `Rule::Custom` to pulse the color
//!
//! Run with: `cargo run --example sphere_shell`

use rdpe::prelude::*;

#[derive(Particle, Clone)]
struct Orb {
    position: Vec3,
    velocity: Vec3,
    #[color]
    color: Vec3,
}

fn main() {
    Simulation::<Orb>::new()
        .with_particle_count(15_000)
        .with_bounds(2.0)
        .with_spawner(|_, _| Orb {
            position: Vec3::ZERO,
            velocity: Vec3::ZERO,
            color: Vec3::new(0.2, 0.8, 1.0), // Cyan glow
        })
        // Sphere emitter - particles spawn on surface
        .with_emitter(Emitter::Sphere {
            center: Vec3::ZERO,
            radius: 0.3,
            speed: 0.5,  // Slow expansion (negative = implosion)
            rate: 5000.0,
        })
        // Lifecycle
        .with_rule(Rule::Age)
        .with_rule(Rule::Lifetime(4.0))
        // Minimal drag for long expansion
        .with_rule(Rule::Drag(0.2))
        .with_rule(Rule::WrapWalls)
        .run();
}
