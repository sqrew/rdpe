//! # Egui Controls Demo
//!
//! Basic egui UI integration showing a floating control panel.
//! This example demonstrates the UI framework works, but the controls
//! don't affect the simulation. See `egui_interactive` for connected controls.
//!
//! ## What This Demonstrates
//!
//! - `.with_ui(|ctx| { ... })` - add egui windows
//! - `egui::Window` - floating control panels
//! - `egui::Slider` - numeric controls
//! - State persistence in closures
//!
//! ## Basic egui Setup
//!
//! ```rust
//! .with_ui(|ctx| {
//!     egui::Window::new("Controls").show(ctx, |ui| {
//!         ui.label("Hello from egui!");
//!         if ui.button("Click me").clicked() {
//!             println!("Clicked!");
//!         }
//!     });
//! })
//! ```
//!
//! ## State in Closures
//!
//! For simple state that doesn't need to affect the simulation,
//! you can store it directly in the closure:
//!
//! ```rust
//! .with_ui({
//!     let mut value = 1.0f32;  // Captured by closure
//!     move |ctx| {
//!         egui::Window::new("Test").show(ctx, |ui| {
//!             ui.add(egui::Slider::new(&mut value, 0.0..=2.0));
//!         });
//!     }
//! })
//! ```
//!
//! ## Try This
//!
//! - Connect sliders to uniforms (see `egui_interactive`)
//! - Add color pickers for particle colors
//! - Create tabs with `egui::TopBottomPanel`
//! - Add graphs with `egui_plot` (external crate)
//!
//! Run with: `cargo run --example egui_controls --features egui`

use rand::Rng;
use rdpe::prelude::*;

#[derive(Particle, Clone)]
struct Ball {
    position: Vec3,
    velocity: Vec3,
    #[color]
    color: Vec3,
}

fn main() {
    let mut rng = rand::thread_rng();

    let particles: Vec<(Vec3, Vec3, Vec3)> = (0..5_000)
        .map(|_| {
            let pos = Vec3::new(
                rng.gen_range(-0.8..0.8),
                rng.gen_range(-0.8..0.8),
                rng.gen_range(-0.8..0.8),
            );
            let vel = Vec3::new(
                rng.gen_range(-0.3..0.3),
                rng.gen_range(-0.3..0.3),
                rng.gen_range(-0.3..0.3),
            );
            let hue = rng.gen_range(0.0..1.0);
            let color = hsv_to_rgb(hue, 0.8, 1.0);
            (pos, vel, color)
        })
        .collect();

    Simulation::<Ball>::new()
        .with_particle_count(5_000)
        .with_bounds(1.0)
        .with_spawner(move |ctx| {
            let (pos, vel, color) = particles[ctx.index as usize];
            Ball {
                position: pos,
                velocity: vel,
                color,
            }
        })
        .with_ui({
            // UI state stored in closure
            let mut demo_speed = 1.0f32;
            let mut demo_gravity = 0.5f32;
            let mut demo_bounce = true;

            move |ctx| {
                egui::Window::new("RDPE Controls")
                    .default_pos([10.0, 10.0])
                    .show(ctx, |ui| {
                        ui.heading("Particle Simulation");
                        ui.separator();

                        ui.label("This is a demo of egui integration with RDPE.");
                        ui.label("The sliders below don't affect the simulation yet,");
                        ui.label("but demonstrate that the UI is working.");

                        ui.separator();

                        // Demo sliders (state persists across frames)
                        ui.add(egui::Slider::new(&mut demo_speed, 0.1..=3.0).text("Speed"));
                        ui.add(egui::Slider::new(&mut demo_gravity, 0.0..=2.0).text("Gravity"));
                        ui.checkbox(&mut demo_bounce, "Bounce off walls");

                        ui.separator();

                        if ui.button("Reset (does nothing)").clicked() {
                            demo_speed = 1.0;
                            demo_gravity = 0.5;
                            demo_bounce = true;
                        }

                        ui.separator();
                        ui.label("Drag the window. Use mouse wheel in 3D view to zoom.");
                    });
            }
        })
        .with_rule(Rule::Drag(1.0))
        .with_rule(Rule::BounceWalls)
        .run();
}

/// Convert HSV to RGB (helper for colorful particles)
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
