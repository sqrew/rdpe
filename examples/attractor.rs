//! # Mouse Attractor
//!
//! Interactive simulation where particles are attracted to your mouse
//! cursor when you click.
//!
//! ## What This Demonstrates
//!
//! - `with_uniform()` - pass custom values to shaders
//! - `with_update()` - update uniforms each frame
//! - Mouse interaction via `UpdateContext`
//! - `Rule::Custom` - custom WGSL force calculation
//!
//! ## Custom Uniforms
//!
//! Uniforms are values passed to the GPU every frame:
//!
//! ```ignore
//! .with_uniform("attractor", Vec3::ZERO)  // Define with initial value
//! .with_uniform("strength", 0.0f32)
//! ```
//!
//! Access in shaders as `uniforms.attractor`, `uniforms.strength`.
//!
//! ## Update Context
//!
//! The `with_update()` callback runs every frame:
//!
//! ```ignore
//! .with_update(|ctx| {
//!     ctx.time()          // Simulation time
//!     ctx.delta_time()    // Time since last frame
//!     ctx.mouse_ndc()     // Mouse position (-1 to 1)
//!     ctx.mouse_pressed() // Is left button down?
//!     ctx.set(name, value) // Update uniform
//! })
//! ```
//!
//! ## Try This
//!
//! - Use negative strength for repulsion
//! - Add multiple attractors that orbit
//! - Make strength oscillate with time
//! - Add color change based on distance to attractor
//!
//! Run with: `cargo run --example attractor`

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

    let particles: Vec<Mote> = (0..15_000)
        .map(|_| Mote {
            position: Vec3::new(
                rng.gen_range(-1.5..1.5),
                rng.gen_range(-1.5..1.5),
                rng.gen_range(-1.5..1.5),
            ),
            velocity: Vec3::ZERO,
            color: Vec3::new(0.4, 0.7, 1.0),
        })
        .collect();

    Simulation::<Mote>::new()
        .with_particle_count(15_000)
        .with_bounds(2.0)
        .with_spawner(move |i, _| particles[i as usize].clone())
        // Define custom uniforms
        .with_uniform("attractor", Vec3::ZERO)
        .with_uniform("strength", 0.0f32)
        // Update uniforms based on mouse input
        .with_update(|ctx| {
            if ctx.mouse_pressed() {
                if let Some(mouse) = ctx.mouse_ndc() {
                    // Map NDC (-1 to 1) to world space (approximate)
                    ctx.set("attractor", Vec3::new(mouse.x * 2.0, mouse.y * 2.0, 0.0));
                    ctx.set("strength", 5.0f32);
                }
            } else {
                ctx.set("strength", 0.0f32);
            }
        })
        // Custom force toward attractor
        .with_rule(Rule::Custom(
            r#"
            if uniforms.strength > 0.0 {
                let to_attractor = uniforms.attractor - p.position;
                let dist = length(to_attractor);
                if dist > 0.01 {
                    let dir = to_attractor / dist;
                    // Inverse square falloff with softening
                    let force = uniforms.strength / (dist * dist + 0.5);
                    p.velocity += dir * force * uniforms.delta_time;
                }
            }
"#
            .into(),
        ))
        .with_rule(Rule::Drag(1.5))
        .with_rule(Rule::SpeedLimit { min: 0.0, max: 3.0 })
        .with_rule(Rule::WrapWalls)
        .run();
}
