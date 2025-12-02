//! # Orbits
//!
//! Particles in circular orbits around a central point, like a simple
//! solar system or electron cloud.
//!
//! ## What This Demonstrates
//!
//! - `Rule::Orbit` - circular motion around a point
//! - Stable orbital mechanics from simple forces
//! - Particle rings and bands
//!
//! ## How Orbit Works
//!
//! The orbit rule applies two forces:
//! 1. **Centripetal**: Pulls toward center (prevents flying away)
//! 2. **Tangential correction**: Adjusts speed for stable orbit
//!
//! The result is particles that naturally settle into circular paths.
//!
//! ## Try This
//!
//! - Change orbit `strength` to make tighter/looser orbits
//! - Add `Rule::Vortex` for spinning disk effect
//! - Use different colors for different orbital bands
//! - Add a second orbit center for figure-8 paths
//!
//! Run with: `cargo run --example orbit`

use rand::Rng;
use rdpe::prelude::*;

#[derive(Particle, Clone)]
struct Planet {
    position: Vec3,
    velocity: Vec3,
    #[color]
    color: Vec3,
}

fn main() {
    let mut rng = rand::thread_rng();

    let particles: Vec<Planet> = (0..10_000)
        .map(|i| {
            // Start in rings at different distances
            let ring = (i % 5) as f32;
            let radius = 0.2 + ring * 0.15;
            let angle = rng.gen_range(0.0..std::f32::consts::TAU);
            let height = rng.gen_range(-0.1..0.1);

            // Color based on orbital ring
            let hue = ring / 5.0;
            let color = hsv_to_rgb(hue, 0.7, 1.0);

            // Initial tangential velocity for stable orbit
            let speed = (2.0 / radius).sqrt() * 0.5;
            let vel = Vec3::new(-angle.sin() * speed, 0.0, angle.cos() * speed);

            Planet {
                position: Vec3::new(angle.cos() * radius, height, angle.sin() * radius),
                velocity: vel,
                color,
            }
        })
        .collect();

    Simulation::<Planet>::new()
        .with_particle_count(10_000)
        .with_bounds(2.0)
        .with_spawner(move |i, _| particles[i as usize].clone())
        // === Orbital Motion ===
        .with_rule(Rule::Orbit {
            center: Vec3::ZERO,
            strength: 2.0,
        })
        // Flatten to disk (gentle Y damping)
        .with_rule(Rule::Custom(
            r#"
            p.velocity.y -= p.position.y * 0.5 * uniforms.delta_time;
"#
            .into(),
        ))
        .with_rule(Rule::Drag(0.3))
        .with_rule(Rule::SpeedLimit { min: 0.1, max: 2.0 })
        .with_rule(Rule::WrapWalls)
        .run();
}

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> Vec3 {
    let c = v * s;
    let x = c * (1.0 - ((h * 6.0) % 2.0 - 1.0).abs());
    let m = v - c;

    let (r, g, b) = if h < 1.0 / 6.0 {
        (c, x, 0.0)
    } else if h < 2.0 / 6.0 {
        (x, c, 0.0)
    } else if h < 3.0 / 6.0 {
        (0.0, c, x)
    } else if h < 4.0 / 6.0 {
        (0.0, x, c)
    } else if h < 5.0 / 6.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };

    Vec3::new(r + m, g + m, b + m)
}
