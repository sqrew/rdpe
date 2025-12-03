//! # Point Gravity Demo
//!
//! Demonstrates `Rule::PointGravity` - inverse-square attraction to a fixed point.
//! Particles orbit around the center like planets around a star or matter
//! spiraling into a black hole.
//!
//! ## What This Demonstrates
//!
//! - `Rule::PointGravity` - attraction toward a single point
//! - `softening` to prevent singularity at the center
//! - Initial tangential velocity for orbital motion
//! - `Rule::ColorBySpeed` for velocity-based coloring
//! - Disk-shaped spawning (flattened in Y) for galaxy appearance
//!
//! ## The Physics
//!
//! **Inverse-Square Law**: Force ~ 1/r². Particles closer to the center
//! experience much stronger pull. Combined with tangential velocity,
//! this creates Keplerian orbits where inner particles move faster.
//!
//! **Orbital Mechanics**: Initial velocity is set perpendicular to the
//! radius (tangential) and scaled by 1/sqrt(r). This approximates
//! circular orbital velocity, creating a stable disk.
//!
//! **Softening**: Without it, particles passing very close to the
//! center would experience near-infinite acceleration. Softening
//! adds a minimum effective distance.
//!
//! ## Try This
//!
//! - Increase `strength` to 3.0+ for faster spiral-in
//! - Remove tangential velocity (`velocity: Vec3::ZERO`) for direct collapse
//! - Move `point` off-center: `Vec3::new(0.3, 0.0, 0.0)`
//! - Add a second `PointGravity` for binary star dynamics
//! - Remove drag entirely to see pure orbital mechanics
//! - Use input handling to move the gravity point with mouse
//!
//! Run with: `cargo run --example point_gravity`

use rand::Rng;
use rdpe::prelude::*;

#[derive(Particle, Clone)]
struct Orbiter {
    position: Vec3,
    velocity: Vec3,
    #[color]
    color: Vec3,
}

fn main() {
    let mut rng = rand::thread_rng();

    // Spawn particles in a ring around the center with tangential velocity
    let particles: Vec<Orbiter> = (0..15_000)
        .map(|_| {
            // Random position in a shell
            let r: f32 = rng.gen_range(0.3..0.9);
            let theta: f32 = rng.gen_range(0.0..std::f32::consts::TAU);
            let phi: f32 = rng.gen_range(-0.3..0.3); // Flatten to disk

            let pos = Vec3::new(
                r * theta.cos() * phi.cos(),
                r * phi.sin() * 0.3, // Flatten Y
                r * theta.sin() * phi.cos(),
            );

            // Tangential velocity for orbital motion
            let tangent = Vec3::new(-pos.z, 0.0, pos.x).normalize();
            let orbital_speed = 0.8 / r.sqrt(); // Kepler-ish

            // Color based on distance (closer = hotter)
            let heat = 1.0 - r;
            let color = Vec3::new(0.5 + heat * 0.5, 0.3 + heat * 0.4, 1.0 - heat * 0.5);

            Orbiter {
                position: pos,
                velocity: tangent * orbital_speed,
                color,
            }
        })
        .collect();

    Simulation::<Orbiter>::new()
        .with_particle_count(15_000)
        .with_particle_size(0.008)
        .with_bounds(1.5)
        .with_spawner(move |i, _| particles[i as usize].clone())
        // Central gravity well
        .with_rule(Rule::PointGravity {
            point: Vec3::ZERO,
            strength: 1.5,
            softening: 0.1, // Prevent singularity at center
        })
        // Color by speed (fast = bright, slow = dim)
        .with_rule(Rule::ColorBySpeed {
            slow_color: Vec3::new(0.2, 0.1, 0.4),
            fast_color: Vec3::new(1.0, 0.8, 0.3),
            max_speed: 2.0,
        })
        .with_rule(Rule::Drag(0.1)) // Very light drag
        .with_rule(Rule::SpeedLimit { min: 0.0, max: 3.0 })
        .run();
}
