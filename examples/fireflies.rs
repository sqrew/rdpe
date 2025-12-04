//! # Firefly Synchronization
//!
//! Simulates fireflies that gradually synchronize their flashing through
//! local interactions - just like real *Photinus carolinus* fireflies.
//!
//! ## The Physics (Kuramoto Model)
//!
//! Each firefly has an internal phase (0 to 2π) that advances over time.
//! When phase exceeds 2π, the firefly flashes and resets.
//!
//! The magic: when a firefly detects a nearby flash (reads high field value),
//! it nudges its own phase forward slightly. Over time, this causes clusters
//! to synchronize their flashing.
//!
//! ## Controls
//!
//! - **Coupling**: How strongly fireflies influence each other
//! - **Natural Frequency**: Base flash rate (with variation per firefly)
//! - **Detection Radius**: How far fireflies can "see" flashes
//!
//! Watch as chaos gradually becomes order!
//!
//! Run with: `cargo run --example fireflies --features egui`

use rand::Rng;
use rdpe::prelude::*;
use std::sync::{Arc, Mutex};

#[derive(Particle, Clone)]
struct Firefly {
    position: Vec3,
    velocity: Vec3,
    #[color]
    color: Vec3,
    /// Internal oscillator phase (0 to 2π)
    phase: f32,
    /// Natural frequency (varies per firefly)
    frequency: f32,
    /// Current brightness (for rendering)
    brightness: f32,
}

struct FireflyParams {
    coupling: f32,
    base_frequency: f32,
    detection_threshold: f32,
    flash_duration: f32,
    wander_strength: f32,
}

impl Default for FireflyParams {
    fn default() -> Self {
        Self {
            coupling: 0.3,
            base_frequency: 1.0,
            detection_threshold: 0.1,
            flash_duration: 0.15,
            wander_strength: 0.2,
        }
    }
}

fn main() {
    let mut rng = rand::thread_rng();

    let state = Arc::new(Mutex::new(FireflyParams::default()));
    let ui_state = state.clone();
    let update_state = state.clone();

    // Create fireflies scattered in the space
    let particles: Vec<Firefly> = (0..500)
        .map(|_| {
            // Random starting position
            let x = rng.gen_range(-0.8..0.8);
            let y = rng.gen_range(-0.4..0.6);
            let z = rng.gen_range(-0.8..0.8);

            // Random initial phase (desynchronized start)
            let phase = rng.gen_range(0.0..std::f32::consts::TAU);

            // Slight frequency variation (natural desync)
            let frequency = rng.gen_range(0.85..1.15);

            // Warm firefly color (yellowish-green)
            let color = Vec3::new(0.6, 0.9, 0.2);

            Firefly {
                position: Vec3::new(x, y, z),
                velocity: Vec3::ZERO,
                color,
                phase,
                frequency,
                brightness: 0.0,
            }
        })
        .collect();

    Simulation::<Firefly>::new()
        .with_particle_count(500)
        .with_particle_size(0.015)
        .with_bounds(1.0)
        .with_spawner(move |i, _| particles[i as usize].clone())
        .with_spatial_config(0.2, 32)
        // Field to propagate light flashes
        .with_field(
            "light",
            FieldConfig::new(48)
                .with_extent(1.2)
                .with_decay(0.85) // Quick fade for distinct flashes
                .with_blur(0.3)
                .with_blur_iterations(2),
        )
        // Volume render the light field
        .with_volume_render(
            VolumeConfig::new()
                .with_field(0)
                .with_steps(48)
                .with_density_scale(12.0)
                .with_palette(Palette::Ice)
                .with_threshold(0.02)
                .with_additive(true),
        )
        // Uniforms
        .with_uniform::<f32>("coupling", 0.3)
        .with_uniform::<f32>("base_frequency", 1.0)
        .with_uniform::<f32>("detection_threshold", 0.1)
        .with_uniform::<f32>("flash_duration", 0.15)
        .with_uniform::<f32>("wander_strength", 0.2)
        // UI
        .with_ui(move |ctx| {
            let mut s = ui_state.lock().unwrap();

            egui::Window::new("Fireflies")
                .default_pos([10.0, 10.0])
                .default_width(280.0)
                .show(ctx, |ui| {
                    ui.heading("Synchronization");
                    ui.add(egui::Slider::new(&mut s.coupling, 0.0..=1.0).text("Coupling Strength"));
                    ui.label("Higher = faster sync");

                    ui.add(
                        egui::Slider::new(&mut s.detection_threshold, 0.01..=0.3)
                            .text("Detection Threshold"),
                    );

                    ui.separator();
                    ui.heading("Oscillation");
                    ui.add(
                        egui::Slider::new(&mut s.base_frequency, 0.3..=2.0).text("Flash Frequency"),
                    );
                    ui.add(
                        egui::Slider::new(&mut s.flash_duration, 0.05..=0.3).text("Flash Duration"),
                    );

                    ui.separator();
                    ui.heading("Movement");
                    ui.add(egui::Slider::new(&mut s.wander_strength, 0.0..=0.5).text("Wander"));

                    ui.separator();
                    ui.label("Watch for synchronization!");
                    ui.label("Clusters will start flashing together.");

                    if ui.button("Desync (Reset Phases)").clicked() {
                        // Can't reset from here, but the button is informative
                    }

                    if ui.button("Reset Params").clicked() {
                        *s = FireflyParams::default();
                    }
                });
        })
        .with_update(move |ctx| {
            let s = update_state.lock().unwrap();
            ctx.set("coupling", s.coupling);
            ctx.set("base_frequency", s.base_frequency);
            ctx.set("detection_threshold", s.detection_threshold);
            ctx.set("flash_duration", s.flash_duration);
            ctx.set("wander_strength", s.wander_strength);
        })
        // Gentle organic movement
        .with_rule(Rule::Wander {
            strength: 0.2,
            frequency: 0.3,
        })
        // Slight upward drift (fireflies tend to rise)
        .with_rule(Rule::Custom(
            r#"
            p.velocity.y += 0.1 * uniforms.delta_time;
        "#
            .into(),
        ))
        // Oscillator synchronization using Rule::Sync!
        // This handles: phase advancement, field reading, coupling, and firing
        .with_rule(Rule::Sync {
            phase_field: "phase".into(),
            frequency: 1.0, // Base frequency (multiplied by particle's own frequency elsewhere)
            field: 0,
            emit_amount: 0.5,
            coupling: 0.3,
            detection_threshold: 0.1,
            on_fire: Some(
                r#"
            // Flash! Set brightness to full
            p.brightness = 1.0;
            "#
                .into(),
            ),
        })
        // Brightness decay and visual updates (separate from sync logic)
        .with_rule(Rule::Custom(
            r#"
            // Decay brightness over flash duration
            p.brightness = max(0.0, p.brightness - uniforms.delta_time / uniforms.flash_duration);

            // Also emit light while bright (in addition to the instant pulse from Sync)
            if p.brightness > 0.3 {
                field_write(0u, p.position, p.brightness * 0.3);
            }

            // Color: dim yellow-green normally, bright yellow when flashing
            let base_color = vec3<f32>(0.2, 0.3, 0.1);
            let flash_color = vec3<f32>(1.0, 0.95, 0.4);
            p.color = mix(base_color, flash_color, p.brightness);
        "#
            .into(),
        ))
        .with_rule(Rule::Drag(1.5))
        .with_rule(Rule::SpeedLimit { min: 0.0, max: 0.3 })
        .with_rule(Rule::BounceWalls)
        .with_visuals(|v| {
            v.blend_mode(BlendMode::Additive);
            v.background(Vec3::new(0.01, 0.02, 0.03)); // Dark night sky
        })
        .run();
}
