//! # Tornado
//!
//! A swirling vortex that pulls particles into a spiral around the Y axis.
//!
//! ## What This Demonstrates
//!
//! - `Rule::Vortex` - rotational force around an axis
//! - Combining vortex with attraction for tornado shape
//! - Vertical lift for rising spiral
//!
//! ## How Vortex Works
//!
//! The vortex creates tangential force perpendicular to both:
//! 1. The rotation axis
//! 2. The direction from axis to particle
//!
//! This makes particles orbit around the axis. Add attraction
//! toward the axis to pull them into a tighter spiral.
//!
//! ## Try This
//!
//! - Change `axis` to `Vec3::X` for horizontal rotation
//! - Remove the attraction for loose swirl
//! - Add `Rule::Gravity` to fight the lift
//! - Use negative strength to reverse rotation
//!
//! Run with: `cargo run --example vortex`

use rand::Rng;
use rdpe::prelude::*;

#[derive(Particle, Clone)]
struct Dust {
    position: Vec3,
    velocity: Vec3,
    #[color]
    color: Vec3,
}

fn main() {
    let mut rng = rand::thread_rng();

    let particles: Vec<Dust> = (0..15_000)
        .map(|_| {
            // Start scattered around the base
            let angle = rng.gen_range(0.0..std::f32::consts::TAU);
            let radius = rng.gen_range(0.2..1.0);
            Dust {
                position: Vec3::new(
                    angle.cos() * radius,
                    rng.gen_range(-1.0..0.0),
                    angle.sin() * radius,
                ),
                velocity: Vec3::ZERO,
                color: Vec3::new(0.6, 0.5, 0.4), // Dusty brown
            }
        })
        .collect();

    Simulation::<Dust>::new()
        .with_particle_count(15_000)
        .with_bounds(2.0)
        .with_spawner(move |i, _| particles[i as usize].clone())
        // === Tornado Forces ===
        // Vortex: spin around Y axis
        .with_rule(Rule::Vortex {
            center: Vec3::ZERO,
            axis: Vec3::Y,
            strength: 4.0,
        })
        // Pull toward center (creates funnel shape)
        .with_rule(Rule::AttractTo {
            point: Vec3::ZERO,
            strength: 1.5,
        })
        // Lift particles upward
        .with_rule(Rule::Acceleration(Vec3::new(0.0, 2.0, 0.0)))
        // Physics
        .with_rule(Rule::Drag(1.0))
        .with_rule(Rule::SpeedLimit { min: 0.0, max: 3.0 })
        .with_rule(Rule::WrapWalls)
        .run();
}
