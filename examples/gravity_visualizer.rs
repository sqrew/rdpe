//! # Interactive Gravity Visualizer
//!
//! A comprehensive showcase of RDPE's capabilities: n-body gravity simulation
//! with volumetric field visualization showing gravitational potential wells.
//!
//! ## What This Demonstrates
//!
//! - `Rule::NBodyGravity` - inverse-square gravitational attraction
//! - `.with_field()` - 3D spatial field for potential visualization
//! - `.with_volume_render()` - ray-marched volumetric fog
//! - `.with_ui()` - egui integration for real-time parameter tuning
//! - `.spatial_grid()` - optional debug grid overlay
//!
//! ## The Visualization
//!
//! Each particle deposits its "gravitational influence" into a 3D field.
//! The field's blur naturally creates smooth potential gradients around masses.
//! Volume rendering makes these invisible gravity wells visible as glowing halos.
//!
//! ## Controls
//!
//! - **Left-click + drag**: Rotate camera
//! - **Scroll**: Zoom in/out
//! - **UI Panel**: Adjust gravity, visualization, spawn new masses
//!
//! Run with: `cargo run --example gravity_visualizer --features egui`

use rand::Rng;
use rdpe::prelude::*;
use std::sync::{Arc, Mutex};

#[derive(Particle, Clone)]
struct Mass {
    position: Vec3,
    velocity: Vec3,
    #[color]
    color: Vec3,
    /// Particle mass (affects gravity and field deposit)
    mass: f32,
}

/// Simulation parameters controlled by UI
struct GravityParams {
    // Physics
    gravity_strength: f32,
    softening: f32,
    interaction_radius: f32,
    drag: f32,

    // Visualization
    field_deposit: f32,
    volume_density: f32,
    show_grid: bool,

    // Spawn
    spawn_mass: f32,
    spawn_count: u32,
    spawn_requested: bool,
}

impl Default for GravityParams {
    fn default() -> Self {
        Self {
            gravity_strength: 0.2,
            softening: 0.05,
            interaction_radius: 0.8,
            drag: 0.5,
            field_deposit: 0.15,
            volume_density: 6.0,
            show_grid: false,
            spawn_mass: 1.0,
            spawn_count: 100,
            spawn_requested: false,
        }
    }
}

fn main() {
    let mut rng = rand::thread_rng();

    let state = Arc::new(Mutex::new(GravityParams::default()));
    let ui_state = state.clone();
    let update_state = state.clone();

    // Create initial galaxy disk
    let particles: Vec<Mass> = (0..8_000)
        .map(|_| {
            let angle: f32 = rng.gen_range(0.0..std::f32::consts::TAU);
            let r: f32 = rng.gen_range(0.05..0.7_f32).sqrt(); // sqrt for uniform disk
            let height: f32 = rng.gen_range(-0.03..0.03) * (1.0 - r);

            // Orbital velocity for stable rotation
            let orbital_speed = 0.25 * r.sqrt();

            // Mass varies - some heavier "stars"
            let mass = if rng.gen_bool(0.02) {
                rng.gen_range(2.0..5.0) // Heavy stars
            } else {
                rng.gen_range(0.5..1.5) // Normal
            };

            // Color by mass - heavier = hotter
            let temp = (mass - 0.5) / 4.5;
            let color = if temp > 0.6 {
                Vec3::new(0.9, 0.95, 1.0) // Blue-white (massive)
            } else if temp > 0.3 {
                Vec3::new(1.0, 0.9, 0.6) // Yellow
            } else {
                Vec3::new(1.0, 0.6, 0.4) // Orange-red (small)
            };

            Mass {
                position: Vec3::new(angle.cos() * r, height, angle.sin() * r),
                velocity: Vec3::new(
                    -angle.sin() * orbital_speed,
                    0.0,
                    angle.cos() * orbital_speed,
                ),
                color,
                mass,
            }
        })
        .collect();

    Simulation::<Mass>::new()
        .with_particle_count(8_000)
        .with_particle_size(0.006)
        .with_bounds(1.5)
        .with_spawner(|ctx| particles[ctx.index as usize].clone())
        .with_spatial_config(0.15, 32)
        // Gravitational potential field
        .with_field(
            "potential",
            FieldConfig::new(48)
                .with_extent(1.5)
                .with_decay(0.92) // Quick fade for responsive viz
                .with_blur(0.25) // Smooth potential wells
                .with_blur_iterations(2),
        )
        // Volume render the gravitational field
        .with_volume_render(
            VolumeConfig::new()
                .with_field(0)
                .with_steps(48)
                .with_density_scale(6.0)
                .with_palette(Palette::Inferno)
                .with_threshold(0.01)
                .with_additive(true),
        )
        // Uniforms for dynamic control
        .with_uniform::<f32>("gravity_strength", 0.2)
        .with_uniform::<f32>("softening", 0.05)
        .with_uniform::<f32>("interaction_radius", 0.8)
        .with_uniform::<f32>("drag", 0.5)
        .with_uniform::<f32>("field_deposit", 0.15)
        // UI Panel
        .with_ui(move |ctx| {
            let mut s = ui_state.lock().unwrap();

            egui::Window::new("Gravity Visualizer")
                .default_pos([10.0, 10.0])
                .default_width(280.0)
                .show(ctx, |ui| {
                    ui.heading("Physics");
                    ui.add(
                        egui::Slider::new(&mut s.gravity_strength, 0.01..=1.0)
                            .text("Gravity Strength")
                            .logarithmic(true),
                    );
                    ui.add(
                        egui::Slider::new(&mut s.softening, 0.01..=0.2)
                            .text("Softening"),
                    );
                    ui.add(
                        egui::Slider::new(&mut s.interaction_radius, 0.2..=1.5)
                            .text("Interaction Radius"),
                    );
                    ui.add(
                        egui::Slider::new(&mut s.drag, 0.0..=2.0)
                            .text("Drag"),
                    );

                    ui.separator();
                    ui.heading("Visualization");
                    ui.add(
                        egui::Slider::new(&mut s.field_deposit, 0.01..=0.5)
                            .text("Field Deposit"),
                    );
                    ui.add(
                        egui::Slider::new(&mut s.volume_density, 1.0..=20.0)
                            .text("Volume Density"),
                    );
                    ui.checkbox(&mut s.show_grid, "Show Spatial Grid");

                    ui.separator();
                    ui.heading("Info");
                    ui.label("Particles: 8,000");
                    ui.label("Field: 48³ voxels");
                    ui.label("");
                    ui.label("Drag to rotate, scroll to zoom");
                    ui.label("Watch gravity wells form!");

                    ui.separator();
                    if ui.button("Reset to Defaults").clicked() {
                        *s = GravityParams::default();
                    }
                });
        })
        // Update uniforms from UI
        .with_update(move |ctx| {
            let s = update_state.lock().unwrap();
            ctx.set("gravity_strength", s.gravity_strength);
            ctx.set("softening", s.softening);
            ctx.set("interaction_radius", s.interaction_radius);
            ctx.set("drag", s.drag);
            ctx.set("field_deposit", s.field_deposit);
            // Toggle grid visibility based on checkbox
            ctx.set_grid_opacity(if s.show_grid { 0.15 } else { 0.0 });
        })
        // N-body gravity with dynamic parameters
        .with_rule(Rule::NBodyGravity {
            strength: 0.2,
            softening: 0.05,
            radius: 0.8,
        })
        // Deposit gravitational influence into field
        .with_rule(Rule::Custom(
            r#"
            // Deposit mass-weighted influence into potential field
            // Heavier masses create brighter wells
            let deposit = uniforms.field_deposit * p.mass;
            field_write(0u, p.position, deposit);
            "#
            .into(),
        ))
        // Apply drag
        .with_rule(Rule::Custom(
            r#"
            p.velocity *= 1.0 - uniforms.drag * uniforms.delta_time;
            "#
            .into(),
        ))
        .with_rule(Rule::SpeedLimit { min: 0.0, max: 2.0 })
        .with_rule(Rule::BounceWalls)
        .with_visuals(|v| {
            v.blend_mode(BlendMode::Additive);
            v.background(Vec3::new(0.01, 0.01, 0.02));
        })
        .run();
}
