//! # Noise Flow
//!
//! Organic, flowing particle motion using built-in simplex noise functions.
//! Particles move through a 3D noise field with colors that shift based on position.
//!
//! ## What This Demonstrates
//!
//! - Built-in `noise3()` function for 3D simplex noise
//! - `fbm3()` for fractal Brownian motion (layered noise)
//! - `hsv_to_rgb()` for smooth color transitions
//!
//! ## Built-in Noise Functions
//!
//! These are automatically available in all shaders:
//!
//! ```wgsl
//! noise3(pos)           // 3D simplex noise, returns [-1, 1]
//! noise2(pos)           // 2D version
//! fbm3(pos, octaves)    // Layered noise for more detail
//! fbm2(pos, octaves)    // 2D version
//! ```
//!
//! ## Creating a Flow Field
//!
//! Sample noise at offset positions for uncorrelated XYZ forces:
//!
//! ```wgsl
//! let force = vec3<f32>(
//!     noise3(p.position * scale + vec3<f32>(time, 0.0, 0.0)),
//!     noise3(p.position * scale + vec3<f32>(0.0, time, 100.0)),
//!     noise3(p.position * scale + vec3<f32>(0.0, 100.0, time))
//! );
//! ```
//!
//! The large offsets (100.0) ensure each axis samples different noise values.
//!
//! ## Try This
//!
//! - Increase noise scale for finer turbulence
//! - Add more FBM octaves for detail (costs performance)
//! - Implement curl noise for divergence-free flow
//! - Use noise for particle size or alpha
//!
//! Run with: `cargo run --example noisy`

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

    let particles: Vec<Mote> = (0..25_000)
        .map(|_| Mote {
            position: Vec3::new(
                rng.gen_range(-1.0..1.0),
                rng.gen_range(-1.0..1.0),
                rng.gen_range(-1.0..1.0),
            ),
            velocity: Vec3::ZERO,
            color: Vec3::new(1.0, 1.0, 1.0),
        })
        .collect();

    Simulation::<Mote>::new()
        .with_particle_count(25_000)
        .with_bounds(2.0)
        .with_spawner(move |i, _| particles[i as usize].clone())
        // Noise-driven flow and color
        .with_rule(Rule::Custom(
            r#"
            // Create a 3D noise-based force field
            let noise_scale = 2.0;
            let time_offset = uniforms.time * 0.3;

            // Sample noise at 3 offset positions for xyz components
            let force = vec3<f32>(
                noise3(p.position * noise_scale + vec3<f32>(time_offset, 0.0, 0.0)),
                noise3(p.position * noise_scale + vec3<f32>(0.0, time_offset, 100.0)),
                noise3(p.position * noise_scale + vec3<f32>(0.0, 100.0, time_offset))
            );

            // Apply noise force
            p.velocity += force * uniforms.delta_time * 2.0;

            // Color based on FBM noise (more detail)
            let color_noise = fbm3(p.position * 1.5 + uniforms.time * 0.2, 3);
            let hue = (color_noise + 1.0) * 0.25 + 0.5;
            p.color = hsv_to_rgb(hue, 0.8, 1.0);
"#
            .into(),
        ))
        // Keep particles from escaping
        .with_rule(Rule::Custom(
            r#"
            let to_center = -p.position;
            let dist = length(to_center);
            if dist > 0.3 {
                p.velocity += normalize(to_center) * dist * 0.5 * uniforms.delta_time;
            }
"#
            .into(),
        ))
        .with_rule(Rule::Drag(1.0))
        .with_rule(Rule::SpeedLimit { min: 0.0, max: 1.5 })
        .with_rule(Rule::WrapWalls)
        .run();
}
