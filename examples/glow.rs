//! # Additive Glow Example
//!
//! Demonstrates `BlendMode::Additive` - overlapping particles add up
//! to create bright, glowing effects like fire, magic, or starfields.
//!
//! ## What This Demonstrates
//!
//! - `BlendMode::Additive` - colors add together (brighter where dense)
//! - Small particle size for high-density glow accumulation
//! - Swirl motion via custom rule
//! - HSV color generation for rainbow effects
//!
//! ## How Additive Blending Works
//!
//! In additive mode, overlapping particle colors are summed:
//! - 1 particle: normal color
//! - 10 particles overlapping: 10x brighter (clamped to white)
//!
//! This creates natural "density visualization" - areas with more
//! particles glow brighter. Perfect for:
//! - Fire and explosions
//! - Magic effects and energy
//! - Stars and galaxies
//! - Glowing fog and plasma
//!
//! ## Additive vs Alpha
//!
//! - **Additive**: Colors add up, no occlusion, always brightens
//! - **Alpha**: Standard transparency, particles can occlude each other
//!
//! ## Try This
//!
//! - Increase particle count to 100k for denser glow
//! - Reduce particle size to 0.004 for finer detail
//! - Switch to `BlendMode::Alpha` to see the difference
//! - Add a custom fragment shader with radial glow falloff
//! - Try monochrome (all white particles) for pure brightness effect
//!
//! Run with: `cargo run --example glow`

use rdpe::prelude::*;

#[derive(rdpe::Particle, Clone)]
struct Spark {
    position: Vec3,
    velocity: Vec3,
    #[color]
    color: Vec3,
}

fn main() {
    Simulation::<Spark>::new()
        .with_particle_count(50_000)
        .with_bounds(2.0)
        .with_particle_size(0.008) // Smaller particles work well with additive
        .with_spawner(|ctx| {
            let pos = ctx.random_in_sphere(0.8);
            let speed = ctx.random_range(0.3, 0.6);
            Spark {
                position: pos,
                velocity: ctx.tangent_velocity(pos, speed),
                color: ctx.rainbow(0.8, 1.0),
            }
        })
        // Additive blending - overlapping particles glow brighter!
        .with_visuals(|v| {
            v.blend_mode(BlendMode::Additive);
        })
        // Pull toward center
        .with_rule(Rule::AttractTo {
            point: Vec3::ZERO,
            strength: 0.5,
        })
        // Some swirl
        .with_rule(Rule::Custom(
            r#"
            let r = length(p.position.xz);
            let swirl_strength = 0.3 / (r + 0.1);
            p.velocity += vec3<f32>(-p.position.z, 0.0, p.position.x) * swirl_strength * uniforms.delta_time;
            "#
            .into(),
        ))
        // Light drag to keep things stable
        .with_rule(Rule::Drag(0.3))
        // Speed limit
        .with_rule(Rule::SpeedLimit { min: 0.0, max: 1.5 })
        // Bounce off walls
        .with_rule(Rule::BounceWalls)
        .run().expect("Simulation failed");
}
