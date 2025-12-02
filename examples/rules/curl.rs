//! # Smoke
//!
//! Fluid-like particle flow using curl noise for divergence-free motion.
//!
//! ## What This Demonstrates
//!
//! - `Rule::Curl` - divergence-free flow field
//! - Smoke/fluid-like motion without bunching
//! - Rising, swirling particle streams
//!
//! ## How Curl Noise Works
//!
//! Curl noise computes the mathematical "curl" of a noise field.
//! This creates flow that:
//! - Never converges to a point (divergence-free)
//! - Particles spread evenly through space
//! - Looks like smoke, water, or fog
//!
//! The trade-off: 6x more noise samples than regular turbulence.
//!
//! ## Curl vs Turbulence
//!
//! - **Turbulence**: Chaotic, particles can cluster
//! - **Curl**: Smooth flow, particles stay evenly distributed
//!
//! ## Try This
//!
//! - Decrease `scale` for larger, slower swirls
//! - Add upward `Acceleration` for rising smoke
//! - Use `ColorOverLife` for fading smoke trail
//! - Combine with `Vortex` for swirling chimney smoke
//!
//! Run with: `cargo run --example curl`

use rand::Rng;
use rdpe::prelude::*;

#[derive(Particle, Clone)]
struct Wisp {
    position: Vec3,
    velocity: Vec3,
    #[color]
    color: Vec3,
}

fn main() {
    let mut rng = rand::thread_rng();

    let particles: Vec<Wisp> = (0..25_000)
        .map(|_| Wisp {
            position: Vec3::new(
                rng.gen_range(-0.3..0.3),
                rng.gen_range(-1.0..-0.5),
                rng.gen_range(-0.3..0.3),
            ),
            velocity: Vec3::new(0.0, rng.gen_range(0.2..0.5), 0.0),
            color: Vec3::new(0.7, 0.7, 0.7), // Gray smoke
        })
        .collect();

    Simulation::<Wisp>::new()
        .with_particle_count(25_000)
        .with_bounds(2.0)
        .with_spawner(move |i, _| particles[i as usize].clone())
        // Respawn at bottom when particles reach top
        .with_emitter(Emitter::Box {
            min: Vec3::new(-0.15, -0.85, -0.15),
            max: Vec3::new(0.15, -0.75, 0.15),
            velocity: Vec3::new(0.0, 0.3, 0.0),
            rate: 500.0,
        })
        .with_rule(Rule::Age)
        .with_rule(Rule::Lifetime(4.0))
        // === Curl Noise Flow ===
        .with_rule(Rule::Curl {
            scale: 2.0,      // Medium flow structures
            strength: 1.5,   // Gentle swirling
        })
        // Rising motion
        .with_rule(Rule::Acceleration(Vec3::new(0.0, 0.8, 0.0)))
        // Fade out as smoke rises
        .with_rule(Rule::FadeOut(4.0))
        // Color: white to dark gray
        .with_rule(Rule::Custom(
            r#"
            let brightness = 0.9 - p.age * 0.15;
            p.color = vec3<f32>(brightness);
"#
            .into(),
        ))
        .with_rule(Rule::Drag(0.5))
        .with_rule(Rule::SpeedLimit { min: 0.0, max: 1.5 })
        .with_rule(Rule::WrapWalls)
        .run();
}
