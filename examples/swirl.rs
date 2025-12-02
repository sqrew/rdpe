//! # Swirl Effect
//!
//! A mesmerizing vortex using custom shader functions. Particles orbit
//! around the Y axis while bobbing up and down in a wave pattern.
//!
//! ## What This Demonstrates
//!
//! - `with_function()` - define reusable WGSL functions
//! - Custom functions for complex behavior
//! - Combining multiple custom functions
//!
//! ## Custom Functions
//!
//! Define WGSL functions that can be called from rules:
//!
//! ```ignore
//! .with_function(r#"
//!     fn my_force(pos: vec3<f32>, strength: f32) -> vec3<f32> {
//!         // Your WGSL code here
//!         return some_vector;
//!     }
//! "#)
//! .with_rule(Rule::Custom(r#"
//!     p.velocity += my_force(p.position, 2.0);
//! "#.into()))
//! ```
//!
//! ## The Swirl Force
//!
//! Creates tangential motion around the Y axis:
//! 1. Project position to XZ plane
//! 2. Calculate perpendicular (tangent) direction
//! 3. Scale by distance for consistent angular velocity
//!
//! ## Try This
//!
//! - Add second counter-rotating vortex
//! - Make swirl strength oscillate with time
//! - Add vertical component for tornado effect
//! - Use noise to perturb the swirl
//!
//! Run with: `cargo run --example swirl`

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
        .map(|_| {
            // Start in a ring formation
            let angle = rng.gen_range(0.0..std::f32::consts::TAU);
            let radius = rng.gen_range(0.3..0.8);
            let height = rng.gen_range(-0.2..0.2);

            // Color based on starting angle (rainbow ring)
            let hue = angle / std::f32::consts::TAU;
            let color = hsv_to_rgb(hue, 0.8, 1.0);

            Mote {
                position: Vec3::new(angle.cos() * radius, height, angle.sin() * radius),
                velocity: Vec3::ZERO,
                color,
            }
        })
        .collect();

    Simulation::<Mote>::new()
        .with_particle_count(20_000)
        .with_bounds(2.0)
        .with_spawner(move |i, _| particles[i as usize].clone())
        // === Custom Functions ===
        // Swirl force: tangential motion around Y axis
        .with_function(
            r#"
            fn swirl_force(pos: vec3<f32>, strength: f32, falloff: f32) -> vec3<f32> {
                let radial = vec2<f32>(pos.x, pos.z);
                let dist = length(radial);
                if dist < 0.01 {
                    return vec3<f32>(0.0);
                }
                // Tangential direction (perpendicular to radial)
                let tangent = vec3<f32>(-pos.z, 0.0, pos.x) / dist;
                let force = strength / (1.0 + dist * falloff);
                return tangent * force;
            }
        "#,
        )
        // Wave lift: outward-propagating vertical wave
        .with_function(
            r#"
            fn wave_lift(pos: vec3<f32>, time: f32, amplitude: f32) -> f32 {
                let dist = length(vec2<f32>(pos.x, pos.z));
                return sin(dist * 5.0 - time * 2.0) * amplitude;
            }
        "#,
        )
        // Apply custom functions
        .with_rule(Rule::Custom(
            r#"
            // Swirling motion
            let swirl = swirl_force(p.position, 3.0, 0.5);
            p.velocity += swirl * uniforms.delta_time;

            // Wave-like vertical motion
            let lift = wave_lift(p.position, uniforms.time, 0.5);
            p.velocity.y += (lift - p.position.y) * uniforms.delta_time * 2.0;
"#
            .into(),
        ))
        // Keep particles from flying out
        .with_rule(Rule::Custom(
            r#"
            let to_center = -p.position;
            let dist = length(to_center);
            if dist > 0.5 {
                p.velocity += normalize(to_center) * (dist - 0.5) * uniforms.delta_time;
            }
"#
            .into(),
        ))
        .with_rule(Rule::Drag(1.5))
        .with_rule(Rule::SpeedLimit { min: 0.0, max: 2.0 })
        .with_rule(Rule::WrapWalls)
        .run();
}

// Helper for Rust-side HSV to RGB (used in spawner)
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
