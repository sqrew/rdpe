//! # SPH-style Fluid Demo
//!
//! Demonstrates `Rule::Viscosity` and `Rule::Pressure` together to create
//! fluid-like behavior. Particles spread to fill space evenly while
//! maintaining smooth, coherent flow.
//!
//! ## What This Demonstrates
//!
//! - `Rule::Pressure` - particles push apart when crowded
//! - `Rule::Viscosity` - smooths velocity field (gooey feel)
//! - `Rule::Custom` - color based on velocity (visual feedback)
//! - SPH (Smoothed Particle Hydrodynamics) style simulation
//!
//! ## The Physics
//!
//! **Pressure**: When a particle has more neighbors than `target_density`,
//! it gets pushed outward. This creates incompressible fluid behavior -
//! particles spread to fill space evenly rather than clumping.
//!
//! **Viscosity**: Blends each particle's velocity with its neighbors,
//! creating smooth, coherent flow. Higher values = thicker fluid (honey),
//! lower values = thinner (water).
//!
//! ## Try This
//!
//! - Increase `target_density` to 10+ for denser fluid
//! - Set viscosity `strength` to 0.5 for water-like behavior
//! - Remove gravity to see surface tension effects
//! - Add `Rule::SurfaceTension` for blob cohesion
//!
//! Run with: `cargo run --example fluid`

use rand::Rng;
use rdpe::prelude::*;

#[derive(Particle, Clone)]
struct Drop {
    position: Vec3,
    velocity: Vec3,
    #[color]
    color: Vec3,
}

fn main() {
    let mut rng = rand::thread_rng();

    // Start particles in a clump that will spread out
    let particles: Vec<Drop> = (0..12_000)
        .map(|_| {
            let r: f32 = rng.gen_range(0.0..0.4);
            let theta: f32 = rng.gen_range(0.0..std::f32::consts::TAU);
            let phi: f32 = rng.gen_range(0.0..std::f32::consts::PI);

            Drop {
                position: Vec3::new(
                    r * phi.sin() * theta.cos(),
                    r * phi.cos() + 0.3, // Start above center
                    r * phi.sin() * theta.sin(),
                ),
                velocity: Vec3::ZERO,
                color: Vec3::new(0.3, 0.5, 1.0), // Water blue
            }
        })
        .collect();

    Simulation::<Drop>::new()
        .with_particle_count(12_000)
        .with_particle_size(0.01)
        .with_bounds(1.0)
        .with_spawner(move |i, _| particles[i as usize].clone())
        .with_spatial_config(0.12, 32)

        // Pressure: particles spread when crowded
        .with_rule(Rule::Pressure {
            radius: 0.08,
            strength: 3.0,
            target_density: 6.0, // Comfortable with ~6 neighbors
        })

        // Viscosity: smooth velocity field
        .with_rule(Rule::Viscosity {
            radius: 0.1,
            strength: 2.0,
        })

        // Gravity pulls the fluid down
        .with_rule(Rule::Gravity(4.0))

        // Color based on velocity (still = blue, fast = white)
        .with_rule(Rule::Custom(r#"
            let speed = length(p.velocity);
            let energy = clamp(speed / 2.0, 0.0, 1.0);
            p.color = mix(vec3<f32>(0.2, 0.4, 0.8), vec3<f32>(0.8, 0.9, 1.0), energy);
        "#.into()))

        .with_rule(Rule::Drag(0.5))
        .with_rule(Rule::SpeedLimit { min: 0.0, max: 3.0 })
        .with_rule(Rule::BounceWalls)
        .run();
}
