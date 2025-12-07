//! # Ethereal Web
//!
//! A dreamlike visualization combining all major RDPE features into one
//! cohesive artistic piece. This is a showcase of what's possible.
//!
//! ## What This Demonstrates
//!
//! - **Fragment shader**: Multi-layered ethereal glow with animated pulse
//! - **Post-processing**: Chromatic aberration, radial blur, color grading,
//!   vignette, and breathing brightness effect
//! - **Trails**: 8-frame trails for flowing light ribbons
//! - **Connections**: Neural web structure between nearby particles
//! - **Movement**: Orbital + breathing + curl turbulence + vertical waves
//!
//! ## Visual Techniques
//!
//! The fragment shader uses three glow layers:
//! - Inner: Sharp exponential falloff (bright core)
//! - Mid: Softer falloff (main body)
//! - Outer: Inverse square (ambient glow)
//!
//! Post-processing creates the "dreamlike" quality:
//! - Edge chromatic aberration (color fringing)
//! - Subtle radial blur near edges
//! - "Film-like" contrast curve: `color / (color + 0.5) * 1.5`
//!
//! ## Motion Layering
//!
//! Five forces combine for organic motion:
//! 1. Orbital rotation around Y axis
//! 2. Radial breathing (expand/contract with time)
//! 3. Curl-like turbulence (pseudo-fluid motion)
//! 4. Vertical wave displacement
//! 5. Soft boundary containment
//!
//! ## Try This
//!
//! - Increase trail length to 20 for longer ribbons
//! - Reduce connection distance for tighter web
//! - Add `v.palette()` instead of per-particle colors
//! - Modify curl_scale for different turbulence patterns
//!
//! Run with: `cargo run --example ethereal_web`

use rand::Rng;
use rdpe::prelude::*;

#[derive(Particle, Clone)]
struct WebNode {
    position: Vec3,
    velocity: Vec3,
    #[color]
    color: Vec3,
    /// Phase offset for individual particle animation
    phase: f32,
    /// "Energy" level affects glow intensity
    energy: f32,
}

fn main() {
    let mut rng = rand::thread_rng();

    // Create initial particle distribution - clustered in organic blobs
    let particles: Vec<_> = (0..2000)
        .map(|i| {
            // Create several cluster centers
            let cluster = (i % 7) as f32;
            let cluster_angle = cluster * std::f32::consts::TAU / 7.0;
            let cluster_center = Vec3::new(
                cluster_angle.cos() * 0.4,
                (cluster * 0.3).sin() * 0.2,
                cluster_angle.sin() * 0.4,
            );

            // Scatter around cluster center
            let scatter = Vec3::new(
                rng.gen_range(-0.25..0.25),
                rng.gen_range(-0.25..0.25),
                rng.gen_range(-0.25..0.25),
            );
            let pos = cluster_center + scatter;

            // Color based on cluster with variation
            let base_hue = cluster / 7.0;
            let hue = (base_hue + rng.gen_range(-0.1..0.1)).rem_euclid(1.0);
            let color = hsv_to_rgb(hue, 0.7, 1.0);

            let phase = rng.gen_range(0.0..std::f32::consts::TAU);
            let energy = rng.gen_range(0.5..1.0);

            (pos, color, phase, energy)
        })
        .collect();

    Simulation::<WebNode>::new()
        .with_particle_count(2000)
        .with_bounds(1.8)
        .with_particle_size(0.025)
        .with_spawner(move |ctx| {
            let (pos, color, phase, energy) = particles[ctx.index as usize];
            WebNode {
                position: pos,
                velocity: Vec3::ZERO,
                color,
                phase,
                energy,
            }
        })
        // Custom fragment shader - ethereal pulsing glow
        .with_fragment_shader(r#"
            let dist = length(in.uv);

            // Animated pulse using particle's phase (encoded in color brightness variation)
            let pulse = sin(uniforms.time * 3.0 + length(in.color) * 10.0) * 0.3 + 0.7;

            // Multi-layered glow effect
            let inner_glow = exp(-dist * dist * 15.0) * 2.0;
            let mid_glow = exp(-dist * dist * 5.0) * 1.0;
            let outer_glow = 1.0 / (dist * dist * 8.0 + 0.3);

            let intensity = (inner_glow + mid_glow + outer_glow * 0.5) * pulse;

            // Color shift based on intensity - brighter = whiter
            let white_mix = smoothstep(1.5, 3.0, intensity);
            let final_color = mix(in.color, vec3<f32>(1.0, 0.95, 0.9), white_mix);

            let alpha = clamp(intensity * 0.4, 0.0, 1.0);
            return vec4<f32>(final_color * intensity * 0.6, alpha);
        "#)
        // Full visual suite
        .with_visuals(|v| {
            v.blend_mode(BlendMode::Additive);
            v.background(Vec3::new(0.01, 0.01, 0.02));
            v.trails(8);           // Flowing light trails
            v.connections(0.12);   // Neural web connections

            // Dreamlike post-processing
            v.post_process(r#"
                let center = vec2<f32>(0.5, 0.5);
                let uv_centered = in.uv - center;
                let dist_from_center = length(uv_centered);

                // Chromatic aberration - stronger at edges
                let aberration = 0.003 + dist_from_center * 0.008;
                let angle = atan2(uv_centered.y, uv_centered.x);
                let aberr_dir = vec2<f32>(cos(angle), sin(angle));

                let uv_r = in.uv + aberr_dir * aberration;
                let uv_g = in.uv;
                let uv_b = in.uv - aberr_dir * aberration;

                let r = textureSample(scene, scene_sampler, uv_r).r;
                let g = textureSample(scene, scene_sampler, uv_g).g;
                let b = textureSample(scene, scene_sampler, uv_b).b;

                var color = vec3<f32>(r, g, b);

                // Subtle radial blur near edges (sample nearby pixels)
                let blur_amount = smoothstep(0.3, 0.8, dist_from_center) * 0.002;
                let blur_samples = 4.0;
                var blurred = color;
                for (var i = 0.0; i < blur_samples; i += 1.0) {
                    let blur_angle = i * 6.28318 / blur_samples;
                    let blur_offset = vec2<f32>(cos(blur_angle), sin(blur_angle)) * blur_amount;
                    blurred += textureSample(scene, scene_sampler, in.uv + blur_offset).rgb;
                }
                color = mix(color, blurred / (blur_samples + 1.0), smoothstep(0.4, 0.7, dist_from_center));

                // Dreamy glow - boost bright areas
                let luminance = dot(color, vec3<f32>(0.299, 0.587, 0.114));
                let glow_boost = smoothstep(0.2, 0.8, luminance) * 0.3;
                color += color * glow_boost;

                // Vignette with color tint
                let vignette = 1.0 - smoothstep(0.2, 0.9, dist_from_center);
                let vignette_color = vec3<f32>(0.02, 0.01, 0.04); // Slight purple in shadows
                color = mix(vignette_color, color, vignette);

                // Color grading - cool shadows, warm highlights
                let shadows = smoothstep(0.3, 0.0, luminance);
                let highlights = smoothstep(0.5, 1.0, luminance);
                color += vec3<f32>(-0.02, 0.0, 0.03) * shadows;  // Blue shadows
                color += vec3<f32>(0.03, 0.02, -0.01) * highlights; // Warm highlights

                // Subtle breathing effect on overall brightness
                let breathe = sin(uniforms.time * 0.5) * 0.05 + 1.0;
                color *= breathe;

                // Film-like contrast curve
                color = color / (color + 0.5) * 1.5;

                return vec4<f32>(clamp(color, vec3<f32>(0.0), vec3<f32>(1.0)), 1.0);
            "#);
        })
        // Complex organic movement
        .with_rule(Rule::Custom(r#"
            // Get position info
            let pos = p.position;
            let dist_from_center = length(pos);

            // 1. Gentle orbital motion around Y axis
            let tangent = normalize(cross(pos, vec3<f32>(0.0, 1.0, 0.0)));
            let orbital_speed = 0.15 / (dist_from_center + 0.3);
            p.velocity += tangent * orbital_speed;

            // 2. Breathing - expand/contract based on time
            let breathe = sin(uniforms.time * 0.3) * 0.02;
            let radial = normalize(pos);
            p.velocity += radial * breathe;

            // 3. Curl-like turbulence
            let curl_scale = 3.0;
            let px = pos * curl_scale;
            let curl = vec3<f32>(
                sin(px.y + uniforms.time * 0.5) * cos(px.z),
                sin(px.z + uniforms.time * 0.4) * cos(px.x),
                sin(px.x + uniforms.time * 0.6) * cos(px.y)
            );
            p.velocity += curl * 0.03;

            // 4. Vertical wave motion
            let wave = sin(pos.x * 5.0 + pos.z * 5.0 + uniforms.time * 2.0) * 0.02;
            p.velocity.y += wave;

            // 5. Soft attraction to neighbors (flocking-lite)
            // This creates organic clustering without tight packing

            // 6. Damping to keep things smooth
            p.velocity *= 0.95;

            // 7. Soft boundary - gentle push back
            if dist_from_center > 0.8 {
                let push = (dist_from_center - 0.8) * 0.5;
                p.velocity -= radial * push;
            }
        "#.into()))
        .run();
}

// HSV to RGB conversion
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
