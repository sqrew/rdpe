//! # Post-Processing Example
//!
//! Demonstrates screen-space post-processing effects.
//! Shows vignette, color grading, and chromatic aberration.
//!
//! Run with: `cargo run --example post_process`

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

    // Pre-generate random positions in a spherical distribution
    let particles: Vec<_> = (0..3000)
        .map(|_| {
            let theta = rng.gen_range(0.0..std::f32::consts::TAU);
            let phi = rng.gen_range(-1.0_f32..1.0).acos();
            let r = rng.gen_range(0.2_f32..0.8).cbrt();

            let x = r * phi.sin() * theta.cos();
            let y = r * phi.cos();
            let z = r * phi.sin() * theta.sin();

            // Rainbow colors based on angle
            let hue = theta / std::f32::consts::TAU;
            let color = hsv_to_rgb(hue, 0.9, 1.0);

            (Vec3::new(x, y, z), color)
        })
        .collect();

    Simulation::<GlowParticle>::new()
        .with_particle_count(3000)
        .with_bounds(1.5)
        .with_particle_size(0.035)
        .with_spawner(move |i, _| {
            let (pos, color) = particles[i as usize];
            GlowParticle {
                position: pos,
                velocity: Vec3::ZERO,
                color,
            }
        })
        // Custom fragment shader for soft glowing particles
        .with_fragment_shader(r#"
            let dist = length(in.uv);
            let glow = 1.0 / (dist * dist * 6.0 + 0.2);
            let core = smoothstep(0.6, 0.0, dist) * 1.5;
            let intensity = glow + core;
            let color = in.color * intensity;
            let alpha = clamp(intensity * 0.5, 0.0, 1.0);
            return vec4<f32>(color, alpha);
        "#)
        // Post-processing: vignette, color grading, and chromatic aberration
        .with_visuals(|v| {
            v.blend_mode(BlendMode::Additive);
            v.background(Vec3::new(0.02, 0.02, 0.04)); // Dark blue-ish background
            v.post_process(r#"
                // Sample with slight chromatic aberration
                let aberration = 0.003;
                let uv_r = in.uv + vec2<f32>(aberration, 0.0);
                let uv_b = in.uv - vec2<f32>(aberration, 0.0);

                let r = textureSample(scene, scene_sampler, uv_r).r;
                let g = textureSample(scene, scene_sampler, in.uv).g;
                let b = textureSample(scene, scene_sampler, uv_b).b;

                var color = vec3<f32>(r, g, b);

                // Vignette effect
                let vignette_center = vec2<f32>(0.5, 0.5);
                let vignette_dist = length(in.uv - vignette_center);
                let vignette = 1.0 - smoothstep(0.3, 0.9, vignette_dist);
                color *= vignette;

                // Color grading: slight warm tint and contrast boost
                color = pow(color, vec3<f32>(0.95, 1.0, 1.05)); // Warm tint
                color = (color - 0.5) * 1.1 + 0.5; // Contrast boost
                color = clamp(color, vec3<f32>(0.0), vec3<f32>(1.0));

                // Subtle film grain (based on UV and time)
                let grain = fract(sin(dot(in.uv * 1000.0, vec2<f32>(12.9898, 78.233)) + uniforms.time) * 43758.5453);
                color += (grain - 0.5) * 0.02;

                return vec4<f32>(color, 1.0);
            "#);
        })
        // Gentle orbital motion
        .with_rule(Rule::Custom(r#"
            let dist = length(p.position);
            let tangent = normalize(cross(p.position, vec3<f32>(0.0, 1.0, 0.0)));
            p.velocity += tangent * 0.2;
            p.velocity *= 0.985;
        "#.into()))
        .run();
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
