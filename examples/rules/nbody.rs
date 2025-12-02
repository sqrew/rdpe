//! # N-Body Gravity Demo
//!
//! Demonstrates `Rule::NBodyGravity` - inverse-square gravitational
//! attraction between particles. Watch clusters form and orbit!
//!
//! Run with: `cargo run --example nbody`

use rand::Rng;
use rdpe::prelude::*;

#[derive(Particle, Clone)]
struct Star {
    position: Vec3,
    velocity: Vec3,
    #[color]
    color: Vec3,
}

fn main() {
    let mut rng = rand::thread_rng();

    let particles: Vec<Star> = (0..10_000)
        .map(|_| {
            // Spawn in a disk shape with some initial orbital velocity
            let angle: f32 = rng.gen_range(0.0..std::f32::consts::TAU);
            let r: f32 = rng.gen_range(0.1..0.8);
            let height: f32 = rng.gen_range(-0.05..0.05) * (1.0 - r); // Flatter at edges

            // Initial orbital velocity (counter-clockwise)
            let orbital_speed = 0.3 * r.sqrt();

            // Color based on distance from center (hot core, cool edges)
            let temp = 1.0 - r;
            let color = if temp > 0.7 {
                Vec3::new(1.0, 1.0, 0.9) // White-hot core
            } else if temp > 0.4 {
                Vec3::new(1.0, 0.8, 0.4) // Yellow
            } else {
                Vec3::new(0.6, 0.4, 0.8) // Cool purple edges
            };

            Star {
                position: Vec3::new(angle.cos() * r, height, angle.sin() * r),
                velocity: Vec3::new(-angle.sin() * orbital_speed, 0.0, angle.cos() * orbital_speed),
                color,
            }
        })
        .collect();

    Simulation::<Star>::new()
        .with_particle_count(10_000)
        .with_particle_size(0.008)
        .with_bounds(2.0)
        .with_spawner(move |i, _| particles[i as usize].clone())
        .with_spatial_config(0.2, 32)

        // The main attraction - n-body gravity!
        .with_rule(Rule::NBodyGravity {
            strength: 0.15,
            softening: 0.03,  // Prevents extreme forces at close range
            radius: 0.6,      // Only interact within this range
        })

        // Gentle drag to prevent runaway velocities
        .with_rule(Rule::Drag(0.3))
        .with_rule(Rule::SpeedLimit { min: 0.0, max: 2.0 })
        .with_rule(Rule::WrapWalls)
        .run();
}
