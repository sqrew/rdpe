//! # Texture Sampling Example
//!
//! Demonstrates sampling custom textures in fragment shaders for
//! procedural effects and color lookup tables.
//!
//! ## What This Demonstrates
//!
//! - `.with_texture(name, config)` - add textures to shaders
//! - `TextureConfig::noise()` - procedural noise texture
//! - `TextureConfig::gradient()` - color gradient lookup table
//! - `tex_<name>` and `tex_<name>_sampler` - access in WGSL
//! - Texture-based color modulation
//!
//! ## How Textures Work
//!
//! Textures added via `.with_texture()` become available in fragment
//! shaders as `tex_<name>` (the texture) and `tex_<name>_sampler`.
//!
//! ```wgsl
//! // Sample a texture
//! let color = textureSample(tex_myname, tex_myname_sampler, uv);
//! ```
//!
//! ## Built-in Texture Types
//!
//! - `TextureConfig::noise(size, seed)` - Perlin-like noise
//! - `TextureConfig::gradient(width, start_rgba, end_rgba)` - 1D gradient
//! - `TextureConfig::from_image(path)` - load from file (feature: image)
//!
//! ## Common Patterns
//!
//! ```wgsl
//! // Use noise for distortion
//! let offset = textureSample(tex_noise, tex_noise_sampler, in.uv).rg * 0.1;
//! let distorted_uv = in.uv + offset;
//!
//! // Use gradient as color lookup table (LUT)
//! let speed = length(particle.velocity);
//! let color = textureSample(tex_gradient, tex_gradient_sampler, vec2(speed, 0.5));
//! ```
//!
//! ## Try This
//!
//! - Change noise seed for different patterns
//! - Create a rainbow gradient with more color stops
//! - Use world position for texture coordinates: `in.world_pos.xy`
//! - Animate UV offset with `uniforms.time`
//! - Tile noise: `fract(in.uv * 4.0)` for repeating pattern
//!
//! Run with: `cargo run --example texture_example`

use rdpe::prelude::*;

#[derive(Particle, Clone)]
struct Particle {
    position: Vec3,
    velocity: Vec3,
    #[color]
    color: Vec3,
}

fn main() {
    // Create a procedural noise texture
    let noise_texture = TextureConfig::noise(256, 42);

    // Create a gradient texture for color lookup
    let gradient = TextureConfig::gradient(
        256,
        [50, 50, 200, 255],   // Blue start
        [255, 100, 50, 255],  // Orange end
    );

    Simulation::<Particle>::new()
        .with_particle_count(50_000)
        .with_bounds(1.0)
        .with_spawner(|ctx| {
            let t = ctx.progress();
            let angle = t * std::f32::consts::TAU * 20.0;
            let radius = 0.3 + t * 0.5;
            Particle {
                position: Vec3::new(
                    angle.cos() * radius,
                    (t - 0.5) * 0.5,
                    angle.sin() * radius,
                ),
                velocity: Vec3::new(
                    -angle.sin() * 0.2,
                    0.0,
                    angle.cos() * 0.2,
                ),
                color: Vec3::new(1.0, 1.0, 1.0),
            }
        })
        // Add textures - they become tex_noise and tex_gradient in shaders
        .with_texture("noise", noise_texture)
        .with_texture("gradient", gradient)
        // Simple orbital motion
        .with_rule(Rule::Custom(r#"
            let to_center = -p.position;
            let dist = length(to_center);
            let dir = normalize(to_center);
            // Orbital force
            p.velocity += dir * 0.5 * uniforms.delta_time;
            // Tangential velocity
            let tangent = vec3<f32>(-dir.z, 0.0, dir.x);
            p.velocity += tangent * 0.1 * uniforms.delta_time;
        "#.into()))
        .with_rule(Rule::Drag(0.5))
        // Custom fragment shader that uses textures
        .with_fragment_shader(r#"
            // Sample noise texture based on particle position
            let noise_uv = in.uv * 0.5 + 0.5;
            let noise_val = textureSample(tex_noise, tex_noise_sampler, noise_uv).r;

            // Use noise to modulate the gradient lookup
            let gradient_u = noise_val;
            let gradient_color = textureSample(tex_gradient, tex_gradient_sampler, vec2<f32>(gradient_u, 0.5));

            // Create a pulsing glow effect
            let dist_from_center = length(in.uv);
            let glow = 1.0 - smoothstep(0.0, 0.5, dist_from_center);

            // Mix the gradient color with glow
            let final_color = gradient_color.rgb * glow * (0.5 + noise_val * 0.5);

            return vec4<f32>(final_color, glow * 0.8);
        "#)
        .with_visuals(|v| {
            v.blend_mode(BlendMode::Additive);
            v.background(Vec3::new(0.02, 0.02, 0.05));
        })
        .run();
}
