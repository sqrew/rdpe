//! # Custom Fragment Shader Example
//!
//! Demonstrates custom fragment shaders for particle rendering effects.
//! Shows glowing particles with animated pulsing.
//!
//! ## What This Demonstrates
//!
//! - `.with_fragment_shader()` - custom WGSL for particle appearance
//! - `in.uv` - UV coordinates relative to particle center (-0.5 to 0.5)
//! - `in.color` - particle's color from the simulation
//! - `uniforms.time` - animated effects (pulsing glow)
//! - `BlendMode::Additive` - overlapping particles add brightness
//! - `smoothstep()` for soft falloff effects
//!
//! ## How Fragment Shaders Work
//!
//! Each particle is rendered as a screen-facing quad. The fragment shader
//! runs for every pixel in the quad, receiving:
//!
//! - `in.uv` - position within quad (center = 0,0)
//! - `in.color` - RGB from the particle struct
//! - `in.world_pos` - world-space position
//!
//! Return `vec4<f32>(r, g, b, alpha)` to set the pixel color.
//!
//! ## Common Techniques
//!
//! ```wgsl
//! // Soft circle (default behavior)
//! let dist = length(in.uv);
//! let alpha = 1.0 - smoothstep(0.0, 0.5, dist);
//!
//! // Glowing orb
//! let glow = 1.0 / (dist * dist * 8.0 + 0.1);
//!
//! // Hard circle with outline
//! let circle = step(dist, 0.4);
//! let ring = step(0.35, dist) * circle;
//! ```
//!
//! ## Try This
//!
//! - Change pulse frequency: `sin(uniforms.time * 5.0)` for faster
//! - Add color shifting: `in.color * hsv_shift(uniforms.time)`
//! - Create a ring: `smoothstep(0.3, 0.35, dist) * smoothstep(0.45, 0.4, dist)`
//! - Add sparkle: multiply by noise based on `in.world_pos + time`
//!
//! Run with: `cargo run --example custom_shader`

use rand::Rng;
use rdpe::prelude::*;

#[derive(Particle, Clone)]
struct GlowParticle {
    position: Vec3,
    velocity: Vec3,
    #[color]
    color: Vec3,
}

fn main() {
    let mut rng = rand::thread_rng();

    // Pre-generate random positions
    let particles: Vec<_> = (0..5000)
        .map(|_| {
            let theta = rng.gen_range(0.0..std::f32::consts::TAU);
            let phi = rng.gen_range(-1.0_f32..1.0).acos();
            let r = rng.gen_range(0.3_f32..0.9).cbrt();

            let x = r * phi.sin() * theta.cos();
            let y = r * phi.cos();
            let z = r * phi.sin() * theta.sin();

            // Rainbow colors based on angle
            let hue = theta / std::f32::consts::TAU;
            let color = hsv_to_rgb(hue, 0.8, 1.0);

            (Vec3::new(x, y, z), color)
        })
        .collect();

    Simulation::<GlowParticle>::new()
        .with_particle_count(5000)
        .with_bounds(1.5)
        .with_particle_size(0.04)  // Larger base size for glow effect
        .with_spawner(move |ctx| {
            let (pos, color) = particles[ctx.index as usize];
            GlowParticle {
                position: pos,
                velocity: Vec3::ZERO,
                color,
            }
        })
        // Custom fragment shader with glowing effect
        .with_fragment_shader(r#"
            // Distance from center of particle quad
            let dist = length(in.uv);

            // Animated pulse based on time
            let pulse = sin(uniforms.time * 2.0) * 0.3 + 0.7;

            // Radial glow falloff (inverse square with offset for softness)
            let glow = 1.0 / (dist * dist * 8.0 + 0.15) * pulse;

            // Add a brighter core
            let core = smoothstep(0.5, 0.0, dist) * 2.0;
            let intensity = glow + core;

            // Final color with glow
            let color = in.color * intensity;

            // Alpha based on glow intensity
            let alpha = clamp(intensity * 0.6, 0.0, 1.0);

            return vec4<f32>(color, alpha);
        "#)
        // Additive blending for glowing effect, pure black background
        .with_visuals(|v| {
            v.blend_mode(BlendMode::Additive);
            v.background(Vec3::ZERO); // Pure black for maximum glow contrast
        })
        // Gentle orbital motion
        .with_rule(Rule::Custom(r#"
            let dist = length(p.position);
            let tangent = normalize(cross(p.position, vec3<f32>(0.0, 1.0, 0.0)));
            p.velocity += tangent * 0.3;
            p.velocity *= 0.98;
        "#.into()))
        .run().expect("Simulation failed");
}

// Helper function to convert HSV to RGB
fn hsv_to_rgb(h: f32, s: f32, v: f32) -> Vec3 {
    let c = v * s;
    let x = c * (1.0 - ((h * 6.0) % 2.0 - 1.0).abs());
    let m = v - c;

    let (r, g, b) = match (h * 6.0).floor() as i32 % 6 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };

    Vec3::new(r + m, g + m, b + m)
}
