//! # Aurora Borealis
//!
//! Charged particles spiraling along magnetic field lines.
//! Uses Lorentz force (v × B) for realistic magnetic dynamics.
//!
//! Run with: `cargo run --example aurora --features egui`

use rand::Rng;
use rdpe::prelude::*;
use std::sync::{Arc, Mutex};

const PARTICLE_COUNT: u32 = 150_000;

#[derive(Particle, Clone)]
struct ChargedParticle {
    position: Vec3,
    velocity: Vec3,
    #[color]
    color: Vec3,
    charge: f32,      // Positive or negative
    energy: f32,      // Affects brightness
}

struct AuroraState {
    field_strength: f32,
    dipole_height: f32,
    injection_speed: f32,
    drag: f32,
    color_mode: u32,
}

impl Default for AuroraState {
    fn default() -> Self {
        Self {
            field_strength: 2.0,
            dipole_height: 0.8,
            injection_speed: 0.5,
            drag: 0.3,
            color_mode: 0,
        }
    }
}

fn main() {
    let mut rng = rand::thread_rng();

    let particles: Vec<_> = (0..PARTICLE_COUNT)
        .map(|_| {
            // Start particles in upper region (solar wind incoming)
            let theta = rng.gen_range(0.0..std::f32::consts::TAU);
            let r = rng.gen_range(0.3..0.9);
            let x = theta.cos() * r;
            let z = theta.sin() * r;
            let y = rng.gen_range(0.5..1.0);

            // Random charge (affects spiral direction)
            let charge = if rng.gen_bool(0.5) { 1.0 } else { -1.0 };

            // Initial downward velocity (solar wind)
            let vy = rng.gen_range(-0.3..-0.1);
            let vx = rng.gen_range(-0.05..0.05);
            let vz = rng.gen_range(-0.05..0.05);

            (x, y, z, vx, vy, vz, charge)
        })
        .collect();

    let state = Arc::new(Mutex::new(AuroraState::default()));
    let ui_state = state.clone();
    let update_state = state.clone();

    Simulation::<ChargedParticle>::new()
        .with_particle_count(PARTICLE_COUNT)
        .with_bounds(2.0)
        .with_spawner(move |ctx| {
            let (x, y, z, vx, vy, vz, charge) = particles[ctx.index as usize];

            ChargedParticle {
                position: Vec3::new(x, y, z),
                velocity: Vec3::new(vx, vy, vz),
                color: Vec3::new(0.2, 1.0, 0.4), // Will be updated by rules
                charge,
                energy: 1.0,
            }
        })
        .with_uniform("field_strength", 2.0f32)
        .with_uniform("dipole_height", 0.8f32)
        .with_uniform("injection_speed", 0.5f32)
        .with_uniform("drag", 0.3f32)
        .with_uniform("color_mode", 0u32)
        // Magnetic field dynamics
        .with_rule(Rule::Custom(r#"
            // Dipole magnetic field approximation
            // Field points upward near poles, curves around
            let dipole_pos = vec3<f32>(0.0, -uniforms.dipole_height, 0.0);
            let r = p.position - dipole_pos;
            let r_len = max(length(r), 0.1);
            let r_norm = r / r_len;

            // Simplified dipole: B field curving from pole
            // Stronger near the dipole, pointing along r with vertical bias
            let field_falloff = 1.0 / (r_len * r_len);

            // Magnetic field direction (simplified dipole)
            let B = vec3<f32>(
                -r_norm.x * r_norm.y,
                1.0 - r_norm.y * r_norm.y,
                -r_norm.z * r_norm.y
            ) * uniforms.field_strength * field_falloff;

            // Lorentz force: F = q(v × B)
            let lorentz = vec3<f32>(
                p.velocity.y * B.z - p.velocity.z * B.y,
                p.velocity.z * B.x - p.velocity.x * B.z,
                p.velocity.x * B.y - p.velocity.y * B.x
            ) * p.charge;

            p.velocity += lorentz * uniforms.delta_time;

            // Gentle downward drift (gravity-like, particles falling into atmosphere)
            p.velocity.y -= 0.1 * uniforms.delta_time;

            // Energy based on speed and altitude
            let speed = length(p.velocity);
            p.energy = min(speed * 2.0 + max(0.0, 0.5 - p.position.y), 2.0);

            // Drag
            p.velocity *= 1.0 - uniforms.drag * uniforms.delta_time;
        "#.into()))
        // Respawn at top when falling too low (continuous aurora)
        .with_rule(Rule::Custom(r#"
            if p.position.y < -0.8 {
                // Respawn at top with fresh velocity
                p.position.y = 0.9;
                p.position.x = p.position.x * 0.5 + sin(uniforms.time * 0.3 + p.position.z * 2.0) * 0.3;
                p.velocity = vec3<f32>(
                    sin(p.position.z * 10.0) * 0.05,
                    -uniforms.injection_speed,
                    cos(p.position.x * 10.0) * 0.05
                );
                p.energy = 0.5;
            }

            // Soft horizontal bounds
            let bounds = 1.2;
            if abs(p.position.x) > bounds {
                p.position.x = sign(p.position.x) * bounds;
                p.velocity.x *= -0.3;
            }
            if abs(p.position.z) > bounds {
                p.position.z = sign(p.position.z) * bounds;
                p.velocity.z *= -0.3;
            }
        "#.into()))
        // Aurora coloring
        .with_rule(Rule::Custom(r#"
            let altitude = (p.position.y + 1.0) / 2.0; // 0 to 1
            let intensity = min(p.energy, 1.5);

            if uniforms.color_mode == 0u {
                // Classic green aurora with purple/pink at edges
                let green = vec3<f32>(0.1, 0.9, 0.3) * intensity;
                let purple = vec3<f32>(0.6, 0.2, 0.8) * intensity;
                let pink = vec3<f32>(0.9, 0.3, 0.5) * intensity;

                // Green dominant in middle altitudes, purple/pink at extremes
                if altitude > 0.6 {
                    p.color = mix(green, purple, (altitude - 0.6) * 2.5);
                } else if altitude < 0.3 {
                    p.color = mix(pink, green, altitude * 3.3);
                } else {
                    p.color = green;
                }
            } else if uniforms.color_mode == 1u {
                // Blue-cyan mode
                let cyan = vec3<f32>(0.2, 0.8, 0.9) * intensity;
                let blue = vec3<f32>(0.3, 0.4, 1.0) * intensity;
                p.color = mix(blue, cyan, altitude);
            } else if uniforms.color_mode == 2u {
                // Fire mode (red/orange aurora - rare but real!)
                let red = vec3<f32>(1.0, 0.2, 0.1) * intensity;
                let orange = vec3<f32>(1.0, 0.6, 0.2) * intensity;
                let yellow = vec3<f32>(1.0, 0.9, 0.3) * intensity;
                p.color = mix(red, mix(orange, yellow, altitude), altitude);
            } else {
                // Charge-based coloring
                if p.charge > 0.0 {
                    p.color = vec3<f32>(0.2, 0.8, 1.0) * intensity;
                } else {
                    p.color = vec3<f32>(1.0, 0.4, 0.7) * intensity;
                }
            }

            // Fade based on speed (stationary = dimmer)
            let speed_factor = min(length(p.velocity) * 3.0, 1.0);
            p.color *= 0.3 + speed_factor * 0.7;
        "#.into()))
        .with_ui(move |ctx| {
            let mut s = ui_state.lock().unwrap();

            egui::Window::new("Aurora Controls")
                .default_pos([10.0, 10.0])
                .show(ctx, |ui| {
                    ui.heading("Magnetic Field");
                    ui.add(egui::Slider::new(&mut s.field_strength, 0.5..=5.0).text("Field Strength"));
                    ui.add(egui::Slider::new(&mut s.dipole_height, 0.2..=1.5).text("Dipole Depth"));

                    ui.add_space(8.0);
                    ui.heading("Dynamics");
                    ui.add(egui::Slider::new(&mut s.injection_speed, 0.1..=1.0).text("Solar Wind"));
                    ui.add(egui::Slider::new(&mut s.drag, 0.0..=1.0).text("Drag"));

                    ui.add_space(8.0);
                    ui.heading("Color Mode");
                    ui.horizontal_wrapped(|ui| {
                        if ui.button("Classic").clicked() { s.color_mode = 0; }
                        if ui.button("Cyan").clicked() { s.color_mode = 1; }
                        if ui.button("Fire").clicked() { s.color_mode = 2; }
                        if ui.button("Charge").clicked() { s.color_mode = 3; }
                    });
                });
        })
        .with_update(move |ctx| {
            let s = update_state.lock().unwrap();
            ctx.set("field_strength", s.field_strength);
            ctx.set("dipole_height", s.dipole_height);
            ctx.set("injection_speed", s.injection_speed);
            ctx.set("drag", s.drag);
            ctx.set("color_mode", s.color_mode);
        })
        .with_visuals(|v| {
            v.background(Vec3::new(0.01, 0.01, 0.03));
        })
        .with_particle_size(0.004)
        .run();
}
