//! # Spring Demo
//!
//! Demonstrates `Rule::Spring` - Hooke's law spring force tethering particles.
//! Particles are pulled back toward the origin with a bouncy, elastic motion.
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
            let color = Vec3::new(
                0.5 + 0.5 * (t * 6.28).sin(),
                0.5 + 0.5 * (t * 6.28 + 2.09).sin(),
                0.5 + 0.5 * (t * 6.28 + 4.19).sin(),
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
