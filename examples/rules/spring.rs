//! # Spring Demo
//!
//! Demonstrates `Rule::Spring` - Hooke's law spring force tethering particles.
//! Particles are pulled back toward the origin with a bouncy, elastic motion.
//!
//! ## What This Demonstrates
//!
//! - `Rule::Spring` - Hooke's law elastic force
//! - `stiffness` parameter (spring constant k)
//! - `damping` to reduce oscillation over time
//! - Combining with `Rule::Turbulence` for dynamic behavior
//! - Distance-based color in `Rule::Custom`
//!
//! ## The Physics
//!
//! **Hooke's Law**: F = -k * x, where k is stiffness and x is displacement
//! from the anchor point. Greater displacement = stronger restoring force.
//!
//! **Damping**: Without damping, particles would oscillate forever.
//! The damping coefficient removes energy each frame, causing oscillations
//! to decay toward equilibrium.
//!
//! **Underdamped vs Overdamped**: Low damping (< 1.0) creates bouncy, oscillatory
//! motion. High damping (> 2.0) creates sluggish, overdamped motion where
//! particles slowly return without overshooting.
//!
//! ## Try This
//!
//! - Set `damping: 0.1` for long-lasting bouncy oscillation
//! - Increase `stiffness` to 10.0 for snappy, high-frequency vibration
//! - Move `anchor` off-center: `Vec3::new(0.3, 0.2, 0.0)`
//! - Remove `Turbulence` to see pure spring dynamics
//! - Add `Rule::Gravity(1.0)` to see springs fighting gravity
//! - Try very high damping (5.0) for viscous, gooey motion
//!
//! Run with: `cargo run --example spring`

use rand::Rng;
use rdpe::prelude::*;

#[derive(Particle, Clone)]
struct Bouncy {
    position: Vec3,
    velocity: Vec3,
    #[color]
    color: Vec3,
}

fn main() {
    let mut rng = rand::thread_rng();

    // Spawn particles scattered randomly, they'll oscillate back
    let particles: Vec<Bouncy> = (0..10_000)
        .map(|i| {
            // Start at random positions
            let pos = Vec3::new(
                rng.gen_range(-0.8..0.8),
                rng.gen_range(-0.8..0.8),
                rng.gen_range(-0.8..0.8),
            );

            // Give some initial velocity to make it dynamic
            let vel = Vec3::new(
                rng.gen_range(-1.0..1.0),
                rng.gen_range(-1.0..1.0),
                rng.gen_range(-1.0..1.0),
            );

            // Color based on starting position
            let t = i as f32 / 10_000.0;
            let tau = std::f32::consts::TAU;
            let color = Vec3::new(
                0.5 + 0.5 * (t * tau).sin(),
                0.5 + 0.5 * (t * tau + 2.09).sin(),
                0.5 + 0.5 * (t * tau + 4.19).sin(),
            );

            Bouncy {
                position: pos,
                velocity: vel,
                color,
            }
        })
        .collect();

    Simulation::<Bouncy>::new()
        .with_particle_count(10_000)
        .with_particle_size(0.012)
        .with_bounds(1.2)
        .with_spawner(move |i, _| particles[i as usize].clone())

        // Spring pulls particles back to origin
        .with_rule(Rule::Spring {
            anchor: Vec3::ZERO,
            stiffness: 3.0,  // How snappy the spring is
            damping: 0.5,    // Reduces oscillation over time
        })

        // Add some turbulence to keep things interesting
        .with_rule(Rule::Turbulence {
            scale: 2.0,
            strength: 1.5,
        })

        // Color based on displacement from center
        .with_rule(Rule::Custom(r#"
            let dist = length(p.position);
            let stretch = clamp(dist / 0.8, 0.0, 1.0);
            p.color = mix(vec3<f32>(0.2, 0.5, 1.0), vec3<f32>(1.0, 0.3, 0.2), stretch);
        "#.into()))

        .with_rule(Rule::SpeedLimit { min: 0.0, max: 4.0 })
        .with_rule(Rule::BounceWalls)
        .run();
}
