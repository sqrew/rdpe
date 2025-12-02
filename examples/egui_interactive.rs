//! # Interactive Egui Controls
//!
//! Demonstrates egui UI that actually controls the simulation in real-time.
//! Uses Arc<Mutex<T>> to share state between UI and update callbacks.
//!
//! Features:
//! - Gravity slider (pulls particles down)
//! - Speed multiplier
//! - Drag coefficient
//! - Reset button
//!
//! Run with: `cargo run --example egui_interactive --features egui`

use rand::Rng;
use rdpe::prelude::*;
use std::sync::{Arc, Mutex};

#[derive(Particle, Clone)]
struct Ball {
    position: Vec3,
    velocity: Vec3,
    #[color]
    color: Vec3,
}

/// Shared state between UI and simulation
struct SimState {
    gravity: f32,
    speed: f32,
    drag: f32,
}

impl Default for SimState {
    fn default() -> Self {
        Self {
            gravity: 0.3,
            speed: 1.0,
            drag: 0.1, // Low drag so particles keep moving
        }
    }
}

fn main() {
    let mut rng = rand::thread_rng();

    // Create shared state
    let state = Arc::new(Mutex::new(SimState::default()));
    let ui_state = state.clone();
    let update_state = state.clone();

    // Pre-generate particles
    let particles: Vec<(Vec3, Vec3, Vec3)> = (0..8_000)
        .map(|_| {
            let pos = Vec3::new(
                rng.gen_range(-0.8..0.8),
                rng.gen_range(0.0..0.8), // Start in upper half
                rng.gen_range(-0.8..0.8),
            );
            let vel = Vec3::new(
                rng.gen_range(-0.2..0.2),
                rng.gen_range(-0.1..0.1),
                rng.gen_range(-0.2..0.2),
            );
            // Rainbow based on height
            let hue = (pos.y + 1.0) / 2.0;
            let color = hsv_to_rgb(hue, 0.9, 1.0);
            (pos, vel, color)
        })
        .collect();

    Simulation::<Ball>::new()
        .with_particle_count(8_000)
        .with_bounds(1.0)
        .with_spawner(move |i, _| {
            let (pos, vel, color) = particles[i as usize];
            Ball {
                position: pos,
                velocity: vel,
                color,
            }
        })
        // Custom uniforms that rules will read (must match defaults)
        .with_uniform::<f32>("gravity", 0.3)
        .with_uniform::<f32>("speed", 1.0)
        .with_uniform::<f32>("drag", 0.1)

        // UI callback - modifies shared state
        .with_ui(move |ctx| {
            let mut s = ui_state.lock().unwrap();

            egui::Window::new("Simulation Controls")
                .default_pos([10.0, 10.0])
                .resizable(false)
                .show(ctx, |ui| {
                    ui.heading("Physics");
                    ui.separator();

                    ui.horizontal(|ui| {
                        ui.label("Gravity:");
                        ui.add(egui::Slider::new(&mut s.gravity, 0.0..=2.0));
                    });

                    ui.horizontal(|ui| {
                        ui.label("Speed:");
                        ui.add(egui::Slider::new(&mut s.speed, 0.1..=3.0));
                    });

                    ui.horizontal(|ui| {
                        ui.label("Drag:");
                        ui.add(egui::Slider::new(&mut s.drag, 0.0..=5.0));
                    });

                    ui.separator();

                    if ui.button("Reset to defaults").clicked() {
                        s.gravity = 0.5;
                        s.speed = 1.0;
                        s.drag = 0.5;
                    }

                    ui.separator();
                    ui.label("Drag 3D view to rotate camera");
                    ui.label("Scroll to zoom");
                });
        })

        // Update callback - syncs shared state to uniforms
        .with_update(move |ctx| {
            let s = update_state.lock().unwrap();
            ctx.set("gravity", s.gravity);
            ctx.set("speed", s.speed);
            ctx.set("drag", s.drag);
        })

        // Custom rule that uses the uniforms
        .with_rule(Rule::Custom(r#"
            // Apply gravity (controlled by slider)
            p.velocity.y -= uniforms.gravity * uniforms.delta_time;

            // Apply drag (controlled by slider)
            let drag_factor = 1.0 - uniforms.drag * uniforms.delta_time;
            p.velocity *= max(drag_factor, 0.0);

            // Apply speed multiplier
            let speed_mult = uniforms.speed;

            // Integrate position
            p.position += p.velocity * uniforms.delta_time * speed_mult;
        "#.into()))

        .with_rule(Rule::BounceWalls)
        .run();
}

/// Convert HSV to RGB
fn hsv_to_rgb(h: f32, s: f32, v: f32) -> Vec3 {
    let c = v * s;
    let h_prime = h * 6.0;
    let x = c * (1.0 - ((h_prime % 2.0) - 1.0).abs());
    let m = v - c;

    let (r, g, b) = if h_prime < 1.0 {
        (c, x, 0.0)
    } else if h_prime < 2.0 {
        (x, c, 0.0)
    } else if h_prime < 3.0 {
        (0.0, c, x)
    } else if h_prime < 4.0 {
        (0.0, x, c)
    } else if h_prime < 5.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };

    Vec3::new(r + m, g + m, b + m)
}
