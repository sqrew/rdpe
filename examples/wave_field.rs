//! # 3D Wave Field Visualization
//!
//! Particles act as wave sources, creating rippling interference patterns
//! visualized as volumetric fog. Watch waves propagate, interfere, and create
//! beautiful standing wave patterns.
//!
//! ## Physics
//!
//! - Each particle emits circular waves into the field
//! - Waves propagate outward based on time and distance
//! - Multiple sources create interference (constructive/destructive)
//! - Field blur simulates wave diffusion
//!
//! ## Controls
//!
//! - **Left-click**: Spawn wave sources at mouse position
//! - **Right-click + drag**: Rotate camera
//! - **Scroll**: Zoom in/out
//!
//! Run with: `cargo run --example wave_field --features egui`

use rand::Rng;
use rdpe::prelude::*;
use std::sync::{Arc, Mutex};

#[derive(Particle, Clone)]
struct WaveSource {
    position: Vec3,
    velocity: Vec3,
    #[color]
    color: Vec3,
    /// Phase offset for this wave source
    phase: f32,
    /// Wave frequency
    frequency: f32,
    /// Wave amplitude
    amplitude: f32,
}

/// UI-controlled parameters
struct WaveParams {
    wave_speed: f32,
    wavelength: f32,
    amplitude: f32,
    decay_rate: f32,
    volume_density: f32,
    source_movement: f32,
}

impl Default for WaveParams {
    fn default() -> Self {
        Self {
            wave_speed: 2.0,
            wavelength: 0.15,
            amplitude: 0.5,
            decay_rate: 0.92,
            volume_density: 8.0,
            source_movement: 0.3,
        }
    }
}

fn main() {
    let mut rng = rand::thread_rng();

    let state = Arc::new(Mutex::new(WaveParams::default()));
    let ui_state = state.clone();
    let update_state = state.clone();

    // Create initial wave sources in interesting patterns
    let particles: Vec<WaveSource> = (0..50)
        .map(|i| {
            // Arrange in a few clusters for interesting interference
            let cluster = i % 5;
            let angle = (i as f32 / 10.0) * std::f32::consts::TAU;
            let radius = 0.3 + (cluster as f32) * 0.1;

            let x = angle.cos() * radius + rng.gen_range(-0.05..0.05);
            let z = angle.sin() * radius + rng.gen_range(-0.05..0.05);
            let y = rng.gen_range(-0.1..0.1);

            // Random phase creates more interesting patterns
            let phase = rng.gen_range(0.0..std::f32::consts::TAU);
            let frequency = rng.gen_range(0.8..1.2);

            // Color based on frequency (creates visual distinction)
            let hue: f32 = (frequency - 0.8) / 0.4;
            let color = Vec3::new(
                0.5 + 0.5 * (hue * 2.0_f32).sin(),
                0.5 + 0.5 * (hue * 2.0_f32 + 2.0).sin(),
                0.5 + 0.5 * (hue * 2.0_f32 + 4.0).sin(),
            );

            WaveSource {
                position: Vec3::new(x, y, z),
                velocity: Vec3::ZERO,
                color,
                phase,
                frequency,
                amplitude: 1.0,
            }
        })
        .collect();

    Simulation::<WaveSource>::new()
        .with_particle_count(200) // Room for spawning more
        .with_particle_size(0.02)
        .with_bounds(1.0)
        .with_spawner(move |i, _| {
            if (i as usize) < particles.len() {
                particles[i as usize].clone()
            } else {
                // Extra particles start dead
                WaveSource {
                    position: Vec3::ZERO,
                    velocity: Vec3::ZERO,
                    color: Vec3::ONE,
                    phase: 0.0,
                    frequency: 1.0,
                    amplitude: 0.0,
                }
            }
        })
        // Wave field for volumetric visualization
        .with_field(
            "waves",
            FieldConfig::new(64)
                .with_extent(1.2)
                .with_decay(0.96)  // Slower decay for persistent waves
                .with_blur(0.2)
                .with_blur_iterations(2),
        )
        // Volume render the wave field
        .with_volume_render(
            VolumeConfig::new()
                .with_field(0)
                .with_steps(80)
                .with_density_scale(8.0)
                .with_palette(Palette::Plasma)
                .with_threshold(0.01)
                .with_additive(true),
        )
        // Uniforms for dynamic control
        .with_uniform::<f32>("wave_speed", 2.0)
        .with_uniform::<f32>("wavelength", 0.15)
        .with_uniform::<f32>("amplitude", 0.5)
        .with_uniform::<f32>("source_movement", 0.3)
        // UI Panel
        .with_ui(move |ctx| {
            let mut s = ui_state.lock().unwrap();

            egui::Window::new("Wave Field")
                .default_pos([10.0, 10.0])
                .default_width(260.0)
                .show(ctx, |ui| {
                    ui.heading("Wave Properties");
                    ui.add(
                        egui::Slider::new(&mut s.wave_speed, 0.5..=5.0)
                            .text("Wave Speed")
                    );
                    ui.add(
                        egui::Slider::new(&mut s.wavelength, 0.05..=0.4)
                            .text("Wavelength")
                    );
                    ui.add(
                        egui::Slider::new(&mut s.amplitude, 0.1..=1.0)
                            .text("Amplitude")
                    );

                    ui.separator();
                    ui.heading("Visualization");
                    ui.add(
                        egui::Slider::new(&mut s.decay_rate, 0.8..=0.98)
                            .text("Decay Rate")
                    );
                    ui.add(
                        egui::Slider::new(&mut s.volume_density, 2.0..=15.0)
                            .text("Fog Density")
                    );

                    ui.separator();
                    ui.heading("Sources");
                    ui.add(
                        egui::Slider::new(&mut s.source_movement, 0.0..=1.0)
                            .text("Movement")
                    );

                    ui.separator();
                    ui.label("Watch interference patterns!");
                    ui.label("Bright = constructive");
                    ui.label("Dark = destructive");

                    if ui.button("Reset").clicked() {
                        *s = WaveParams::default();
                    }
                });
        })
        // Update uniforms from UI
        .with_update(move |ctx| {
            let s = update_state.lock().unwrap();
            ctx.set("wave_speed", s.wave_speed);
            ctx.set("wavelength", s.wavelength);
            ctx.set("amplitude", s.amplitude);
            ctx.set("source_movement", s.source_movement);
        })
        // Gentle wandering motion for sources
        .with_rule(Rule::Wander {
            strength: 0.3,
            frequency: 0.5,
        })
        // Custom wave emission to field
        .with_rule(Rule::Custom(r#"
            // Skip dead particles
            if p.amplitude < 0.01 {
                return;
            }

            // Wave parameters
            let k = 6.28318 / uniforms.wavelength;  // wave number
            let omega = uniforms.wave_speed * k;     // angular frequency
            let phase = uniforms.time * omega + p.phase * p.frequency;

            // Deposit at source (bright center)
            let center_intensity = p.amplitude * uniforms.amplitude * 0.5;
            field_write(0u, p.position, center_intensity);

            // Create multiple expanding rings (linear intensity - no distance falloff)
            // Each ring has constant brightness as it expands
            let max_radius = 0.8;
            let num_rings = 4u;

            for (var ring = 0u; ring < num_rings; ring++) {
                // Stagger rings in time so they create continuous waves
                let ring_phase = f32(ring) / f32(num_rings);
                let ring_radius = fract(uniforms.time * uniforms.wave_speed * 0.3 + ring_phase) * max_radius;

                // Sine wave modulation for interference pattern
                let wave_mod = sin(ring_radius * k - phase) * 0.5 + 0.5;
                let ring_intensity = p.amplitude * uniforms.amplitude * wave_mod * 0.4;

                // Sample ring at multiple angles
                for (var angle = 0.0; angle < 6.28; angle += 0.4) {
                    let offset = vec3<f32>(
                        cos(angle) * ring_radius,
                        sin(angle * 0.3) * 0.05,  // slight Y variation
                        sin(angle) * ring_radius
                    );
                    let ring_pos = p.position + offset;

                    // Bounds check
                    if abs(ring_pos.x) < 1.1 && abs(ring_pos.y) < 1.1 && abs(ring_pos.z) < 1.1 {
                        field_write(0u, ring_pos, ring_intensity);
                    }
                }
            }
        "#.into()))
        // Keep sources contained
        .with_rule(Rule::Drag(2.0))
        .with_rule(Rule::SpeedLimit { min: 0.0, max: 0.5 })
        .with_rule(Rule::BounceWalls)
        .with_visuals(|v| {
            v.blend_mode(BlendMode::Additive);
            v.background(Vec3::new(0.01, 0.01, 0.02));
        })
        .run();
}
