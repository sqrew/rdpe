//! # Custom Vertex Shader Example
//!
//! Demonstrates using custom vertex shaders for advanced particle effects.
//!
//! ## What This Demonstrates
//!
//! - Custom vertex shader with `.with_vertex_shader()`
//! - Per-particle rotation using quad coordinate rotation
//! - Wobble/wave effects using sin/cos
//! - Access to uniforms.time for animation
//!
//! ## Available Variables in Vertex Shader
//!
//! **Inputs:**
//! - `vertex_index` - Which quad vertex (0-5)
//! - `instance_index` - Which particle
//! - `particle_pos` - World position
//! - `particle_color` - Color (if defined)
//! - `scale` - Per-particle scale
//! - `quad_pos` - Quad vertex offset (-1 to 1)
//! - `base_size` / `particle_size` - Size values
//! - `uniforms.time`, `uniforms.view_proj`, etc.
//!
//! **Must Set:**
//! - `out.clip_position` - Final screen position
//! - `out.color` - Color for fragment shader
//! - `out.uv` - UV coordinates
//!
//! ## Try This
//!
//! - Modify the rotation speed or wobble amplitude
//! - Try different wave patterns (sin, cos, noise)
//! - Combine with custom fragment shaders
//!
//! Run with: `cargo run --example custom_vertex`

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
        .with_particle_count(10_000)
        .with_bounds(1.5)
        .with_particle_size(0.015)
        .with_spawner(|ctx| {
            let pos = ctx.random_in_sphere(0.6);
            let speed = ctx.random_range(0.2, 0.5);
            Spark {
                position: pos,
                velocity: ctx.tangent_velocity(pos, speed),
                color: ctx.rainbow(0.9, 1.0),
            }
        })
        .with_visuals(|v| {
            v.blend_mode(BlendMode::Additive);
            v.shape(ParticleShape::Square); // Rotation looks best with squares
        })
        // Custom vertex shader: rotating + wobbling particles
        .with_vertex_shader(r#"
            // Per-particle rotation based on time and index
            let rotation_speed = 2.0 + f32(instance_index % 10u) * 0.3;
            let angle = uniforms.time * rotation_speed + f32(instance_index) * 0.1;
            let cos_a = cos(angle);
            let sin_a = sin(angle);

            // Rotate the quad coordinates
            let rotated_quad = vec2<f32>(
                quad_pos.x * cos_a - quad_pos.y * sin_a,
                quad_pos.x * sin_a + quad_pos.y * cos_a
            );

            // Wobble effect: offset position based on time
            let wobble_freq = 3.0;
            let wobble_amp = 0.02;
            let wobble = vec3<f32>(
                sin(uniforms.time * wobble_freq + f32(instance_index) * 0.5) * wobble_amp,
                cos(uniforms.time * wobble_freq * 1.3 + f32(instance_index) * 0.7) * wobble_amp,
                sin(uniforms.time * wobble_freq * 0.7 + f32(instance_index) * 0.3) * wobble_amp
            );

            // Size pulsing
            let pulse = 1.0 + sin(uniforms.time * 4.0 + f32(instance_index) * 0.2) * 0.2;
            let final_size = particle_size * pulse;

            // Transform to clip space
            let world_pos = vec4<f32>(particle_pos + wobble, 1.0);
            var clip_pos = uniforms.view_proj * world_pos;

            // Apply rotated quad with pulsing size
            clip_pos.x += rotated_quad.x * final_size * clip_pos.w;
            clip_pos.y += rotated_quad.y * final_size * clip_pos.w;

            out.clip_position = clip_pos;
            out.color = particle_color;
            out.uv = rotated_quad; // Pass rotated UVs for fragment shader

            return out;
        "#)
        // Swirl motion
        .with_rule(Rule::AttractTo {
            point: Vec3::ZERO,
            strength: 0.3,
        })
        .with_rule(Rule::Custom(
            r#"
            let r = length(p.position.xz);
            let swirl = 0.4 / (r + 0.1);
            p.velocity += vec3<f32>(-p.position.z, 0.0, p.position.x) * swirl * uniforms.delta_time;
            "#
            .into(),
        ))
        .with_rule(Rule::Drag(0.3))
        .with_rule(Rule::SpeedLimit { min: 0.0, max: 1.0 })
        .with_rule(Rule::BounceWalls)
        .run().expect("Simulation failed");
}
