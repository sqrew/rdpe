//! Interactive Lorenz Strange Attractor visualization.
//!
//! Explore the famous butterfly-shaped Lorenz attractor with real-time
//! parameter controls. Discovered by Edward Lorenz in 1963 while studying
//! atmospheric convection.
//!
//! The attractor is defined by three coupled differential equations:
//!   dx/dt = σ(y - x)
//!   dy/dt = x(ρ - z) - y
//!   dz/dt = xy - βz
//!
//! Adjust σ, ρ, and β to explore different chaotic behaviors:
//! - Classic chaos: σ=10, ρ=28, β=8/3
//! - Periodic orbits: ρ < 24.74
//! - Fixed points: ρ < 1
//! - Transient chaos: ρ slightly above 24.74
//!
//! Run with: `cargo run --example lorenz_attractor_interactive --features egui`

use rdpe::prelude::*;
use std::sync::{Arc, Mutex};

#[derive(Particle, Clone)]
struct LorenzParticle {
    position: Vec3,
    velocity: Vec3,
    #[color]
    color: Vec3,
}

/// Lorenz attractor parameters
struct LorenzState {
    sigma: f32,
    rho: f32,
    beta: f32,
    speed: f32,
    color_mode: i32,
}

impl Default for LorenzState {
    fn default() -> Self {
        Self {
            sigma: 10.0,
            rho: 28.0,
            beta: 8.0 / 3.0,
            speed: 0.15,
            color_mode: 0,
        }
    }
}

fn main() {
    let scale: f32 = 0.04;

    let state = Arc::new(Mutex::new(LorenzState::default()));
    let ui_state = state.clone();
    let update_state = state.clone();

    Simulation::<LorenzParticle>::new()
        .with_particle_count(20_000)
        .with_bounds(3.0)
        // Spawn particles in a small cloud near one of the attractor's lobes
        .with_spawner(move |ctx| {
            let i = ctx.index;
            let count = ctx.count;
            let t = i as f32 / count as f32;
            let angle = t * std::f32::consts::TAU * 100.0;
            let r = 0.5 + (i % 100) as f32 * 0.01;

            let x = 1.0 + r * angle.cos() * 0.3;
            let y = 1.0 + r * angle.sin() * 0.3;
            let z = 25.0 + (i % 50) as f32 * 0.1;

            let hue = t * 2.0;
            let color = hue_to_rgb(hue);

            LorenzParticle {
                position: Vec3::new(x * scale, (z - 25.0) * scale, y * scale),
                velocity: Vec3::ZERO,
                color,
            }
        })
        // Uniforms for dynamic control
        .with_uniform::<f32>("sigma", 10.0)
        .with_uniform::<f32>("rho", 28.0)
        .with_uniform::<f32>("beta", 8.0 / 3.0)
        .with_uniform::<f32>("speed", 0.15)
        .with_uniform::<f32>("lorenz_scale", scale)
        .with_uniform::<i32>("color_mode", 0)
        // UI Controls
        .with_ui(move |ctx| {
            let mut s = ui_state.lock().unwrap();

            egui::Window::new("Lorenz Attractor")
                .default_pos([10.0, 10.0])
                .resizable(false)
                .show(ctx, |ui| {
                    ui.heading("Parameters");

                    ui.add(egui::Slider::new(&mut s.sigma, 0.0..=30.0)
                        .text("σ (sigma)")
                        .step_by(0.1));
                    ui.label("Rate of rotation - higher = faster spiral");

                    ui.add_space(4.0);
                    ui.add(egui::Slider::new(&mut s.rho, 0.0..=50.0)
                        .text("ρ (rho)")
                        .step_by(0.1));
                    ui.label("Rayleigh number - controls chaos onset");

                    ui.add_space(4.0);
                    ui.add(egui::Slider::new(&mut s.beta, 0.0..=10.0)
                        .text("β (beta)")
                        .step_by(0.01));
                    ui.label("Geometric factor of convection cell");

                    ui.separator();
                    ui.heading("Simulation");
                    ui.add(egui::Slider::new(&mut s.speed, 0.01..=0.5)
                        .text("Speed")
                        .step_by(0.01));

                    ui.separator();
                    ui.heading("Color Mode");
                    ui.horizontal(|ui| {
                        ui.selectable_value(&mut s.color_mode, 0, "Position");
                        ui.selectable_value(&mut s.color_mode, 1, "Velocity");
                        ui.selectable_value(&mut s.color_mode, 2, "Wing");
                    });

                    ui.separator();
                    ui.heading("Presets");
                    ui.horizontal(|ui| {
                        if ui.button("Classic").clicked() {
                            s.sigma = 10.0;
                            s.rho = 28.0;
                            s.beta = 8.0 / 3.0;
                        }
                        if ui.button("Calm").clicked() {
                            s.sigma = 10.0;
                            s.rho = 14.0;
                            s.beta = 8.0 / 3.0;
                        }
                    });
                    ui.horizontal(|ui| {
                        if ui.button("Wild").clicked() {
                            s.sigma = 14.0;
                            s.rho = 45.0;
                            s.beta = 4.0;
                        }
                        if ui.button("Tight").clicked() {
                            s.sigma = 10.0;
                            s.rho = 28.0;
                            s.beta = 1.0;
                        }
                    });
                    ui.horizontal(|ui| {
                        if ui.button("Stretched").clicked() {
                            s.sigma = 16.0;
                            s.rho = 45.92;
                            s.beta = 4.0;
                        }
                        if ui.button("Collapse").clicked() {
                            s.sigma = 10.0;
                            s.rho = 0.5;
                            s.beta = 8.0 / 3.0;
                        }
                    });

                    ui.separator();
                    ui.label("ρ < 1: Fixed point");
                    ui.label("ρ ≈ 24.74: Chaos onset");
                    ui.label("ρ = 28: Classic butterfly");
                });
        })
        // Sync state to uniforms
        .with_update(move |ctx| {
            let s = update_state.lock().unwrap();
            ctx.set("sigma", s.sigma);
            ctx.set("rho", s.rho);
            ctx.set("beta", s.beta);
            ctx.set("speed", s.speed);
            ctx.set("color_mode", s.color_mode);
        })
        // The Lorenz system dynamics using uniforms
        .with_rule(Rule::Custom(r#"
    // Lorenz attractor dynamics
    let scale = uniforms.lorenz_scale;
    let lx = p.position.x / scale;
    let ly = p.position.z / scale;
    let lz = p.position.y / scale + 25.0;

    // Lorenz equations with uniform parameters
    let sigma = uniforms.sigma;
    let rho = uniforms.rho;
    let beta = uniforms.beta;

    let dx = sigma * (ly - lx);
    let dy = lx * (rho - lz) - ly;
    let dz = lx * ly - beta * lz;

    // Apply as velocity
    let spd = uniforms.speed;
    p.velocity = vec3<f32>(dx, dz, dy) * scale * spd;

    // Color based on selected mode
    let color_mode = uniforms.color_mode;

    if color_mode == 0 {
        // Position-based coloring
        let wing = lx / 20.0;
        let hue = (wing + 1.0) * 0.5;
        let h = (lz - 10.0) / 40.0;

        let r = clamp(abs(hue * 6.0 - 3.0) - 1.0, 0.0, 1.0);
        let g = clamp(2.0 - abs(hue * 6.0 - 2.0), 0.0, 1.0);
        let b = clamp(2.0 - abs(hue * 6.0 - 4.0), 0.0, 1.0);

        let brightness = 0.7 + clamp(h, 0.0, 1.0) * 0.3;
        p.color = vec3<f32>(r, g, b) * brightness;
    } else if color_mode == 1 {
        // Velocity-based coloring
        let vel_mag = length(vec3<f32>(dx, dy, dz));
        let hue = clamp(vel_mag / 50.0, 0.0, 1.0);

        // Blue (slow) -> Cyan -> Green -> Yellow -> Red (fast)
        let r = clamp(abs(hue * 6.0 - 3.0) - 1.0, 0.0, 1.0);
        let g = clamp(2.0 - abs(hue * 6.0 - 2.0), 0.0, 1.0);
        let b = clamp(2.0 - abs(hue * 6.0 - 4.0), 0.0, 1.0);

        p.color = vec3<f32>(r, g, b);
    } else {
        // Wing-based coloring (left wing vs right wing)
        if lx < 0.0 {
            // Left wing: cyan/blue
            let intensity = clamp(1.0 - abs(lx) / 20.0, 0.3, 1.0);
            p.color = vec3<f32>(0.2, 0.6, 1.0) * intensity;
        } else {
            // Right wing: orange/red
            let intensity = clamp(1.0 - lx / 20.0, 0.3, 1.0);
            p.color = vec3<f32>(1.0, 0.5, 0.2) * intensity;
        }
    }
"#.into()))
        // Slow drag to smooth out motion
        .with_rule(Rule::Drag(0.1))
        // Visual settings
        .with_visuals(|v| {
            v.blend_mode(BlendMode::Additive);
            v.background(Vec3::new(0.01, 0.01, 0.02));
            v.trails(100);
        })
        .with_particle_size(0.008)
        .run();
}

/// Convert hue (0-1) to RGB color
fn hue_to_rgb(h: f32) -> Vec3 {
    let h = h.fract();
    let r = (h * 6.0 - 3.0).abs().clamp(0.0, 1.0);
    let g = (2.0 - (h * 6.0 - 2.0).abs()).clamp(0.0, 1.0);
    let b = (2.0 - (h * 6.0 - 4.0).abs()).clamp(0.0, 1.0);
    Vec3::new(r, g, b)
}
