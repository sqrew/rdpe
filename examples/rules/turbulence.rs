//! # Turbulence
//!
//! Chaotic, organic motion driven by a 3D noise field.
//!
//! ## What This Demonstrates
//!
//! - `Rule::Turbulence` - noise-based force field
//! - Organic, unpredictable particle motion
//! - Time-evolving flow patterns
//!
//! ## How Turbulence Works
//!
//! Each particle samples 3D noise at its position to get a force vector.
//! The noise field slowly evolves over time, creating flowing, organic motion.
//!
//! - `scale` controls noise frequency (smaller = larger swirls)
//! - `strength` controls force magnitude
//!
//! ## Turbulence vs Curl
//!
//! - **Turbulence**: Simple noise → particles can bunch up
//! - **Curl**: Divergence-free → particles spread evenly (fluid-like)
//!
//! Use Turbulence for chaotic effects, Curl for smoke/fluid.
//!
//! ## Try This
//!
//! - Increase `scale` for finer, more chaotic motion
//! - Decrease `scale` for large, sweeping flows
//! - Combine with `Rule::AttractTo` to keep particles centered
//! - Add `Rule::ColorOverLife` for visual variety
//!
//! Run with: `cargo run --example turbulence`

use rand::Rng;
use rdpe::prelude::*;

#[derive(Particle, Clone)]
struct Mote {
    position: Vec3,
    velocity: Vec3,
    #[color]
    color: Vec3,
}

fn main() {
    let mut rng = rand::thread_rng();

    let particles: Vec<Mote> = (0..20_000)
        .map(|_| Mote {
            position: Vec3::new(
                rng.gen_range(-0.5..0.5),
                rng.gen_range(-0.5..0.5),
                rng.gen_range(-0.5..0.5),
            ),
            velocity: Vec3::ZERO,
            color: Vec3::new(0.3, 0.8, 1.0), // Cyan
        })
        .collect();

    Simulation::<Mote>::new()
        .with_particle_count(20_000)
        .with_bounds(2.0)
        .with_spawner(move |i, _| particles[i as usize].clone())
        // === Turbulence ===
        .with_rule(Rule::Turbulence {
            scale: 2.0,      // Medium-sized turbulent structures
            strength: 3.0,   // Strong chaotic force
        })
        // Gentle pull to center (keeps particles from escaping)
        .with_rule(Rule::Custom(
            r#"
            let to_center = -p.position;
            let dist = length(to_center);
            if dist > 0.3 {
                p.velocity += normalize(to_center) * (dist - 0.3) * 2.0 * uniforms.delta_time;
            }
"#
            .into(),
        ))
        // Color based on velocity
        .with_rule(Rule::Custom(
            r#"
            let speed = length(p.velocity);
            p.color = hsv_to_rgb(speed * 0.3, 0.8, 1.0);
"#
            .into(),
        ))
        .with_rule(Rule::Drag(1.5))
        .with_rule(Rule::SpeedLimit { min: 0.0, max: 2.0 })
        .with_rule(Rule::WrapWalls)
        .run();
}
