//! # Neon Assault Interactive
//!
//! The full neon assault experience with egui controls to tweak everything in real-time.
//!
//! Run with: `cargo run --example neon_assault_interactive --features egui`

use rand::Rng;
use rdpe::prelude::*;
use std::sync::{Arc, Mutex};

#[derive(Particle, Clone)]
struct GridEntity {
    position: Vec3,
    velocity: Vec3,
    #[color]
    color: Vec3,
    entity_type: f32,
    intensity: f32,
}

/// Shared state for UI controls (movement + visual parameters)
struct NeonState {
    // Movement parameters
    chaos_intensity: f32,
    orbital_speed: f32,
    hunt_aggression: f32,
    flee_speed: f32,
    // Visual parameters (now available in fragment/post-process shaders!)
    aberration: f32,
    scanline_intensity: f32,
    bloom_amount: f32,
    saturation: f32,
}

impl Default for NeonState {
    fn default() -> Self {
        Self {
            chaos_intensity: 0.15,
            orbital_speed: 0.4,
            hunt_aggression: 0.2,
            flee_speed: 0.3,
            aberration: 0.006,
            scanline_intensity: 0.15,
            bloom_amount: 0.4,
            saturation: 1.4,
        }
    }
}

fn main() {
    let mut rng = rand::thread_rng();

    let neon_colors = [
        Vec3::new(1.0, 0.0, 0.5),   // Hot pink
        Vec3::new(0.0, 1.0, 1.0),   // Cyan
        Vec3::new(1.0, 1.0, 0.0),   // Electric yellow
        Vec3::new(0.5, 0.0, 1.0),   // Purple
        Vec3::new(0.0, 1.0, 0.3),   // Toxic green
        Vec3::new(1.0, 0.3, 0.0),   // Orange
    ];

    let particles: Vec<_> = (0..3500)
        .map(|i| {
            let pattern = i % 5;
            let pos = match pattern {
                0 => {
                    let angle = rng.gen_range(0.0..std::f32::consts::TAU);
                    let r = rng.gen_range(0.3..0.7);
                    Vec3::new(angle.cos() * r, rng.gen_range(-0.3..0.3), angle.sin() * r)
                }
                1 => {
                    let x = (rng.gen_range(0..20) as f32 - 10.0) * 0.08;
                    let z = (rng.gen_range(0..20) as f32 - 10.0) * 0.08;
                    Vec3::new(x, rng.gen_range(-0.2..0.2), z)
                }
                2 => {
                    if rng.gen_bool(0.5) {
                        Vec3::new(rng.gen_range(-0.8..0.8), rng.gen_range(-0.1..0.1), rng.gen_range(-0.05..0.05))
                    } else {
                        Vec3::new(rng.gen_range(-0.05..0.05), rng.gen_range(-0.1..0.1), rng.gen_range(-0.8..0.8))
                    }
                }
                3 => {
                    let center = Vec3::new(rng.gen_range(-0.5..0.5), 0.0, rng.gen_range(-0.5..0.5));
                    center + Vec3::new(rng.gen_range(-0.1..0.1), rng.gen_range(-0.1..0.1), rng.gen_range(-0.1..0.1))
                }
                _ => Vec3::new(rng.gen_range(-0.8..0.8), rng.gen_range(-0.4..0.4), rng.gen_range(-0.8..0.8)),
            };

            let color = neon_colors[rng.gen_range(0..neon_colors.len())];
            let entity_type = rng.gen_range(0.0_f32..3.0).floor();
            let intensity = rng.gen_range(0.7..1.5);
            let vel = match entity_type as i32 {
                0 => Vec3::new(rng.gen_range(-0.5..0.5), 0.0, rng.gen_range(-0.5..0.5)),
                1 => Vec3::ZERO,
                _ => Vec3::new(rng.gen_range(-1.0..1.0), 0.0, rng.gen_range(-1.0..1.0)),
            };

            (pos, vel, color, entity_type, intensity)
        })
        .collect();

    // Shared state
    let state = Arc::new(Mutex::new(NeonState::default()));
    let ui_state = state.clone();
    let update_state = state.clone();

    let defaults = NeonState::default();

    Simulation::<GridEntity>::new()
        .with_particle_count(3500)
        .with_bounds(2.0)
        .with_particle_size(0.018)
        .with_spawner(move |ctx| {
            let (pos, vel, color, entity_type, intensity) = particles[ctx.index as usize];
            GridEntity { position: pos, velocity: vel, color, entity_type, intensity }
        })
        // Define all uniforms (now available in compute, fragment, AND post-process shaders!)
        .with_uniform("chaos_intensity", defaults.chaos_intensity)
        .with_uniform("orbital_speed", defaults.orbital_speed)
        .with_uniform("hunt_aggression", defaults.hunt_aggression)
        .with_uniform("flee_speed", defaults.flee_speed)
        .with_uniform("aberration", defaults.aberration)
        .with_uniform("scanline_intensity", defaults.scanline_intensity)
        .with_uniform("bloom_amount", defaults.bloom_amount)
        .with_uniform("saturation", defaults.saturation)
        // Fragment shader (custom uniforms now available here too!)
        .with_fragment_shader(r#"
            let dist = length(in.uv);
            let core = 1.0 - smoothstep(0.0, 0.3, dist);
            let halo = sin(dist * 30.0 - uniforms.time * 8.0) * 0.5 + 0.5;
            let halo_fade = exp(-dist * 3.0);
            let electric = halo * halo_fade * 0.5;
            let glow = 1.0 / (dist * dist * 10.0 + 0.2);
            let pulse = sin(uniforms.time * 8.0 + length(in.color) * 20.0) * 0.3 + 1.0;
            let intensity = (core * 2.0 + electric + glow * 0.3) * pulse;
            let boosted_color = in.color * 1.5;
            let alpha = clamp(intensity * 0.6, 0.0, 1.0);
            return vec4<f32>(boosted_color * intensity, alpha);
        "#)
        .with_visuals(|v| {
            v.blend_mode(BlendMode::Additive);
            v.background(Vec3::new(0.0, 0.0, 0.02));
            v.trails(5);
            v.connections(0.08);
            // CRT post-processing (NOW USING CUSTOM UNIFORMS!)
            v.post_process(r#"
                let center = vec2<f32>(0.5, 0.5);
                var uv = in.uv;
                let uv_centered = uv - center;
                let dist_sq = dot(uv_centered, uv_centered);
                uv = center + uv_centered * (1.0 + 0.1 * dist_sq);

                // Use uniforms.aberration instead of hardcoded value!
                let aberr = uniforms.aberration;
                let r = textureSample(scene, scene_sampler, uv + vec2<f32>(aberr, 0.0)).r;
                let g = textureSample(scene, scene_sampler, uv).g;
                let b = textureSample(scene, scene_sampler, uv - vec2<f32>(aberr, 0.0)).b;
                var color = vec3<f32>(r, g, b);

                // Use uniforms.scanline_intensity instead of hardcoded value!
                let scanline_freq = 400.0;
                let scanline = sin(in.uv.y * scanline_freq) * 0.5 + 0.5;
                color *= 1.0 - uniforms.scanline_intensity * (1.0 - scanline);

                let noise_y = floor(in.uv.y * 100.0 + uniforms.time * 50.0);
                let interference = fract(sin(noise_y * 12.9898) * 43758.5453);
                if interference > 0.97 {
                    color *= 1.3;
                }

                let pixel_x = floor(in.uv.x * 800.0);
                let subpixel = pixel_x % 3.0;
                if subpixel < 1.0 {
                    color.g *= 0.9; color.b *= 0.8;
                } else if subpixel < 2.0 {
                    color.r *= 0.9; color.b *= 0.8;
                } else {
                    color.r *= 0.8; color.g *= 0.9;
                }

                // Use uniforms.bloom_amount instead of hardcoded value!
                let luminance = dot(color, vec3<f32>(0.299, 0.587, 0.114));
                let bloom = smoothstep(0.4, 1.0, luminance) * uniforms.bloom_amount;
                color += color * bloom;

                let vignette_dist = length(in.uv - center);
                let vignette = 1.0 - smoothstep(0.4, 1.0, vignette_dist);
                color *= vignette;

                // Use uniforms.saturation instead of hardcoded value!
                let gray = dot(color, vec3<f32>(0.3, 0.3, 0.3));
                color = mix(vec3<f32>(gray), color, uniforms.saturation);

                let flicker = sin(uniforms.time * 60.0) * 0.02 + 1.0;
                color *= flicker;

                color = clamp(color, vec3<f32>(0.0), vec3<f32>(1.5));
                return vec4<f32>(color, 1.0);
            "#);
        })
        // Movement rule
        .with_rule(Rule::Custom(r#"
            let pos = p.position;
            let entity = floor(p.entity_type);

            let chaos_freq = 5.0;
            let chaos = vec3<f32>(
                sin(pos.z * chaos_freq + uniforms.time * 3.0),
                sin(pos.x * chaos_freq + uniforms.time * 2.5) * 0.3,
                cos(pos.x * chaos_freq + uniforms.time * 3.5)
            );
            p.velocity += chaos * uniforms.chaos_intensity;

            if entity == 0.0 {
                let to_center = -normalize(pos);
                let tangent = cross(to_center, vec3<f32>(0.0, 1.0, 0.0));
                p.velocity += tangent * uniforms.orbital_speed;
                let dive = step(0.98, fract(sin(uniforms.time * 2.0 + p.intensity * 100.0) * 0.5 + 0.5));
                p.velocity += to_center * dive * 0.5;
            } else if entity == 1.0 {
                let to_origin = -pos;
                let dist = length(to_origin);
                let hunt_dir = normalize(to_origin);
                let lunge = step(0.95, fract(sin(uniforms.time * 5.0 + p.intensity * 50.0) * 0.5 + 0.5));
                p.velocity += hunt_dir * (uniforms.hunt_aggression + lunge * 1.5);
                if dist < 0.3 {
                    let orbit = cross(hunt_dir, vec3<f32>(0.0, 1.0, 0.0));
                    p.velocity += orbit * 0.8;
                }
            } else {
                let away = normalize(pos + vec3<f32>(0.001, 0.0, 0.0));
                p.velocity += away * uniforms.flee_speed;
                let jitter = vec3<f32>(
                    sin(uniforms.time * 10.0 + p.intensity * 100.0),
                    0.0,
                    cos(uniforms.time * 12.0 + p.intensity * 80.0)
                );
                p.velocity += jitter * 0.2;
            }

            let bounds = 0.9;
            if abs(pos.x) > bounds { p.velocity.x = -p.velocity.x * 1.2; }
            if abs(pos.z) > bounds { p.velocity.z = -p.velocity.z * 1.2; }
            if abs(pos.y) > 0.5 { p.velocity.y = -p.velocity.y * 0.8; }

            let speed = length(p.velocity);
            if speed > 2.0 { p.velocity = normalize(p.velocity) * 2.0; }
            p.velocity *= 0.97;
        "#.into()))
        // UI callback
        .with_ui(move |ctx| {
            let mut s = ui_state.lock().unwrap();

            egui::Window::new("NEON ASSAULT CONTROLS")
                .default_width(280.0)
                .show(ctx, |ui| {
                    ui.heading("Movement Parameters");
                    ui.add(egui::Slider::new(&mut s.chaos_intensity, 0.0..=0.5).text("Chaos"));
                    ui.add(egui::Slider::new(&mut s.orbital_speed, 0.0..=1.0).text("Orbital Speed"));
                    ui.add(egui::Slider::new(&mut s.hunt_aggression, 0.0..=1.0).text("Hunt Aggression"));
                    ui.add(egui::Slider::new(&mut s.flee_speed, 0.0..=1.0).text("Flee Speed"));

                    ui.separator();
                    ui.heading("Visual Effects (via Custom Uniforms!)");
                    ui.add(egui::Slider::new(&mut s.aberration, 0.0..=0.02).text("Chromatic Aberration"));
                    ui.add(egui::Slider::new(&mut s.scanline_intensity, 0.0..=0.5).text("Scanlines"));
                    ui.add(egui::Slider::new(&mut s.bloom_amount, 0.0..=1.0).text("Bloom"));
                    ui.add(egui::Slider::new(&mut s.saturation, 0.5..=2.0).text("Saturation"));

                    ui.separator();
                    if ui.button("Reset to Defaults").clicked() {
                        *s = NeonState::default();
                    }

                    ui.separator();
                    ui.heading("Presets");
                    ui.horizontal(|ui| {
                        if ui.button("Calm").clicked() {
                            s.chaos_intensity = 0.05;
                            s.orbital_speed = 0.2;
                            s.hunt_aggression = 0.1;
                            s.flee_speed = 0.15;
                        }
                        if ui.button("Aggressive").clicked() {
                            s.chaos_intensity = 0.4;
                            s.orbital_speed = 0.8;
                            s.hunt_aggression = 0.8;
                            s.flee_speed = 0.7;
                        }
                    });
                    ui.horizontal(|ui| {
                        if ui.button("Maximum Chaos").clicked() {
                            s.chaos_intensity = 0.5;
                            s.orbital_speed = 1.0;
                            s.hunt_aggression = 1.0;
                            s.flee_speed = 1.0;
                        }
                        if ui.button("Balanced").clicked() {
                            s.chaos_intensity = 0.15;
                            s.orbital_speed = 0.4;
                            s.hunt_aggression = 0.2;
                            s.flee_speed = 0.3;
                        }
                    });
                    ui.horizontal(|ui| {
                        if ui.button("Retro CRT").clicked() {
                            s.aberration = 0.012;
                            s.scanline_intensity = 0.3;
                            s.bloom_amount = 0.6;
                            s.saturation = 1.2;
                        }
                        if ui.button("Clean").clicked() {
                            s.aberration = 0.0;
                            s.scanline_intensity = 0.0;
                            s.bloom_amount = 0.2;
                            s.saturation = 1.0;
                        }
                    });
                });
        })
        // Update callback - sync all state to uniforms
        .with_update(move |ctx| {
            let s = update_state.lock().unwrap();
            // Movement uniforms (used in compute shader)
            ctx.set("chaos_intensity", s.chaos_intensity);
            ctx.set("orbital_speed", s.orbital_speed);
            ctx.set("hunt_aggression", s.hunt_aggression);
            ctx.set("flee_speed", s.flee_speed);
            // Visual uniforms (now available in post-process shader too!)
            ctx.set("aberration", s.aberration);
            ctx.set("scanline_intensity", s.scanline_intensity);
            ctx.set("bloom_amount", s.bloom_amount);
            ctx.set("saturation", s.saturation);
        })
        .run().expect("Simulation failed");
}
