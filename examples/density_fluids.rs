//! # Density-Stratified Fluid Simulation
//!
//! Multiple liquids with different densities that settle into layers.
//! Stir with the mouse and watch them separate back out!
//!
//! ## Physics
//!
//! - Heavy (blue) particles sink to the bottom
//! - Medium (green) particles settle in the middle
//! - Light (orange) particles float to the top
//! - Buoyancy pushes particles toward their equilibrium layer
//! - Viscosity creates smooth, fluid-like motion
//!
//! ## Controls
//!
//! - **Left-click + drag**: Stir the fluids
//! - **Scroll**: Zoom in/out
//! - **Right-click + drag**: Rotate camera
//!
//! Run with: `cargo run --example density_fluids --features egui`

use rand::Rng;
use rdpe::prelude::*;
use std::sync::{Arc, Mutex};

#[derive(Particle, Clone)]
struct FluidParticle {
    position: Vec3,
    velocity: Vec3,
    #[color]
    color: Vec3,
    /// Density determines buoyancy (higher = sinks, lower = floats)
    density: f32,
}

/// UI-controlled parameters
struct FluidParams {
    buoyancy_strength: f32,
    viscosity: f32,
    stir_radius: f32,
    stir_strength: f32,
    field_deposit: f32,
    volume_density: f32,
}

impl Default for FluidParams {
    fn default() -> Self {
        Self {
            buoyancy_strength: 8.0,
            viscosity: 0.15,
            stir_radius: 0.25,
            stir_strength: 3.0,
            field_deposit: 0.08,
            volume_density: 4.0,
        }
    }
}

fn main() {
    let mut rng = rand::thread_rng();

    let state = Arc::new(Mutex::new(FluidParams::default()));
    let ui_state = state.clone();
    let update_state = state.clone();

    // Define three fluid layers with different densities
    // Initially mixed randomly, they'll settle into layers
    let particles: Vec<FluidParticle> = (0..15_000)
        .map(|_| {
            // Random position in the container
            let x = rng.gen_range(-0.8..0.8);
            let y = rng.gen_range(-0.8..0.8);
            let z = rng.gen_range(-0.3..0.3);

            // Randomly assign to one of three fluid types
            let fluid_type = rng.gen_range(0..3);
            let (density, color) = match fluid_type {
                0 => (2.0, Vec3::new(0.2, 0.4, 1.0)), // Heavy - blue (sinks)
                1 => (1.0, Vec3::new(0.3, 0.9, 0.4)), // Medium - green (middle)
                _ => (0.3, Vec3::new(1.0, 0.6, 0.2)), // Light - orange (floats)
            };

            FluidParticle {
                position: Vec3::new(x, y, z),
                velocity: Vec3::ZERO,
                color,
                density,
            }
        })
        .collect();

    Simulation::<FluidParticle>::new()
        .with_particle_count(150_00)
        .with_particle_size(0.012)
        .with_bounds(1.0)
        .with_spawner(move |i, _| particles[i as usize].clone())
        .with_spatial_config(0.08, 32)
        // Fluid density field for volumetric visualization
        .with_field(
            "fluid",
            FieldConfig::new(64)
                .with_extent(1.0)
                .with_decay(0.85)
                .with_blur(0.3)
                .with_blur_iterations(2),
        )
        // Volume render the fluid field
        .with_volume_render(
            VolumeConfig::new()
                .with_field(0)
                .with_steps(64)
                .with_density_scale(4.0)
                .with_palette(Palette::Viridis)
                .with_threshold(0.02)
                .with_additive(true),
        )
        // Uniforms for dynamic control
        .with_uniform::<f32>("buoyancy_strength", 8.0)
        .with_uniform::<f32>("viscosity", 0.15)
        .with_uniform::<f32>("stir_radius", 0.25)
        .with_uniform::<f32>("stir_strength", 3.0)
        .with_uniform::<f32>("field_deposit", 0.08)
        .with_uniform::<f32>("mouse_x", 0.0)
        .with_uniform::<f32>("mouse_y", 0.0)
        .with_uniform::<f32>("mouse_active", 0.0)
        // UI Panel
        .with_ui(move |ctx| {
            let mut s = ui_state.lock().unwrap();

            egui::Window::new("Density Fluids")
                .default_pos([10.0, 10.0])
                .default_width(260.0)
                .show(ctx, |ui| {
                    ui.heading("Physics");
                    ui.add(
                        egui::Slider::new(&mut s.buoyancy_strength, 1.0..=20.0)
                            .text("Buoyancy")
                    );
                    ui.add(
                        egui::Slider::new(&mut s.viscosity, 0.01..=0.5)
                            .text("Viscosity")
                    );

                    ui.separator();
                    ui.heading("Stirring");
                    ui.add(
                        egui::Slider::new(&mut s.stir_radius, 0.1..=0.5)
                            .text("Stir Radius")
                    );
                    ui.add(
                        egui::Slider::new(&mut s.stir_strength, 1.0..=10.0)
                            .text("Stir Strength")
                    );

                    ui.separator();
                    ui.heading("Visualization");
                    ui.add(
                        egui::Slider::new(&mut s.field_deposit, 0.01..=0.2)
                            .text("Field Deposit")
                    );
                    ui.add(
                        egui::Slider::new(&mut s.volume_density, 1.0..=10.0)
                            .text("Fog Density")
                    );

                    ui.separator();
                    ui.label("Left-click to stir");
                    ui.label("Watch layers separate!");

                    if ui.button("Reset Params").clicked() {
                        *s = FluidParams::default();
                    }
                });
        })
        // Update uniforms from UI and mouse
        .with_update(move |ctx| {
            let s = update_state.lock().unwrap();
            ctx.set("buoyancy_strength", s.buoyancy_strength);
            ctx.set("viscosity", s.viscosity);
            ctx.set("stir_radius", s.stir_radius);
            ctx.set("stir_strength", s.stir_strength);
            ctx.set("field_deposit", s.field_deposit);

            // Pass mouse position for stirring
            if ctx.input.mouse_held(MouseButton::Left) {
                let mouse = ctx.input.mouse_ndc();
                ctx.set("mouse_x", mouse.x);
                ctx.set("mouse_y", mouse.y);
                ctx.set("mouse_active", 1.0);
            } else {
                ctx.set("mouse_active", 0.0);
            }
        })
        // Buoyancy: particles rise/sink based on density
        // Density 1.0 is neutral, < 1.0 floats, > 1.0 sinks
        .with_rule(Rule::Custom(r#"
            // Buoyancy force based on density difference from neutral (1.0)
            let buoyancy = (1.0 - p.density) * uniforms.buoyancy_strength;
            p.velocity.y += buoyancy * uniforms.delta_time;
        "#.into()))
        // Viscosity: smooth velocity between neighbors
        .with_rule(Rule::Viscosity {
            radius: 0.08,
            strength: 0.15,
        })
        // Pressure: prevent particles from overlapping
        .with_rule(Rule::Pressure {
            radius: 0.06,
            strength: 2.0,
            target_density: 1.0,
        })
        // Soft collision for fluid feel
        .with_rule(Rule::Collide {
            radius: 0.025,
            restitution: 0.3,
        })
        // Mouse stirring interaction
        .with_rule(Rule::Custom(r#"
            if uniforms.mouse_active > 0.5 {
                // Convert mouse NDC to world space (bounds = 1.0)
                let mouse_world = vec3<f32>(
                    uniforms.mouse_x,
                    uniforms.mouse_y,
                    0.0
                );

                let to_mouse = mouse_world - p.position;
                let dist = length(to_mouse);

                if dist < uniforms.stir_radius && dist > 0.01 {
                    // Circular stirring motion + inward pull
                    let dir = normalize(to_mouse);
                    let tangent = vec3<f32>(-dir.y, dir.x, 0.0);
                    let falloff = 1.0 - dist / uniforms.stir_radius;

                    // Swirl around mouse + slight attraction
                    p.velocity += tangent * uniforms.stir_strength * falloff * uniforms.delta_time;
                    p.velocity += dir * uniforms.stir_strength * 0.3 * falloff * uniforms.delta_time;
                }
            }
        "#.into()))
        // Deposit to field based on density (heavier = brighter in field)
        .with_rule(Rule::Custom(r#"
            let deposit = uniforms.field_deposit * p.density;
            field_write(0u, p.position, deposit);
        "#.into()))
        // Damping
        .with_rule(Rule::Drag(2.0))
        .with_rule(Rule::SpeedLimit { min: 0.0, max: 20.0 })
        .with_rule(Rule::BounceWalls)
        .with_visuals(|v| {
            v.blend_mode(BlendMode::Additive);
            v.background(Vec3::new(0.02, 0.02, 0.04));
        })
        .run();
}
