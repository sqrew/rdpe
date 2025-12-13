//! # Particle Inspector
//!
//! Demonstrates the built-in particle and rule inspector panels.
//!
//! ## What This Demonstrates
//!
//! - **`.with_particle_inspector()`** - Zero-boilerplate particle inspector panel
//! - **`.with_rule_inspector()`** - Live-edit rule parameters without recompilation
//! - **Left-click** to select a particle
//! - **Live updates** - Selected particle's stats update in real-time
//! - **Drag values** in the Rule Inspector to adjust physics in real-time
//! - **Additive UI** - Inspectors work alongside custom `.with_ui()` panels
//!
//! ## Camera Controls
//!
//! - Right-drag: Orbit
//! - Scroll: Zoom
//! - WASD: Move
//! - Q/E: Up/Down
//! - Shift: Speed boost
//! - R: Reset camera
//!
//! Run with: `cargo run --example particle_inspector`

use rand::Rng;
use rdpe::prelude::*;

/// A particle with various properties to inspect.
#[derive(Particle, Clone)]
struct InspectableParticle {
    position: Vec3,
    velocity: Vec3,
    #[color]
    color: Vec3,
    /// Energy level (affects color intensity)
    energy: f32,
    /// Temperature (affects behavior)
    temperature: f32,
    /// Unique ID for display
    id: u32,
    /// Mass affects physics
    mass: f32,
}

fn main() {
    let mut rng = rand::thread_rng();

    // Pre-generate particles with varied properties
    let particles: Vec<InspectableParticle> = (0..5000)
        .map(|i| {
            // Spawn in clusters
            let cluster = i % 5;
            let cluster_center = match cluster {
                0 => Vec3::new(-0.8, 0.0, -0.8),
                1 => Vec3::new(0.8, 0.0, -0.8),
                2 => Vec3::new(-0.8, 0.0, 0.8),
                3 => Vec3::new(0.8, 0.0, 0.8),
                _ => Vec3::new(0.0, 0.5, 0.0),
            };

            let offset = Vec3::new(
                rng.gen_range(-0.4..0.4),
                rng.gen_range(-0.3..0.3),
                rng.gen_range(-0.4..0.4),
            );

            let energy = rng.gen_range(0.3..1.0);
            let temperature = rng.gen_range(0.0..1.0);
            let mass = rng.gen_range(0.5..2.0);

            // Color based on cluster
            let base_color = match cluster {
                0 => Vec3::new(1.0, 0.3, 0.3), // Red
                1 => Vec3::new(0.3, 1.0, 0.3), // Green
                2 => Vec3::new(0.3, 0.3, 1.0), // Blue
                3 => Vec3::new(1.0, 1.0, 0.3), // Yellow
                _ => Vec3::new(1.0, 0.3, 1.0), // Magenta
            };

            InspectableParticle {
                position: cluster_center + offset,
                velocity: Vec3::new(
                    rng.gen_range(-0.1..0.1),
                    rng.gen_range(-0.05..0.05),
                    rng.gen_range(-0.1..0.1),
                ),
                color: base_color * energy,
                energy,
                temperature,
                id: i,
                mass,
            }
        })
        .collect();

    Simulation::<InspectableParticle>::new()
        .with_particle_count(5000)
        .with_bounds(2.0)
        .with_particle_size(0.025)
        .with_spawner(move |ctx| particles[ctx.index as usize].clone())
        // Gentle movement rules
        .with_rule(Rule::Wander {
            strength: 0.3,
            frequency: 2.0,
        })
        .with_rule(Rule::Drag(2.0))
        .with_rule(Rule::SpeedLimit { min: 0.0, max: 0.5 })
        // Energy affects color brightness
        .with_rule(Rule::Custom(
            r#"
            // Slowly regenerate energy
            p.energy = min(p.energy + 0.01 * uniforms.delta_time, 1.0);

            // Temperature oscillates
            p.temperature = (sin(uniforms.time * 2.0 + f32(index) * 0.1) + 1.0) * 0.5;

            // Update color intensity based on energy
            let intensity = 0.3 + p.energy * 0.7;
            p.color = p.color * intensity / length(p.color);
            "#
            .into(),
        ))
        .with_rule(Rule::BounceWalls)
        // Visual setup
        .with_visuals(|v| {
            v.wireframe(WireframeMesh::cube(), 0.001);
            v.blend_mode(BlendMode::Additive);
            v.background(Vec3::new(0.02, 0.02, 0.04));
        })
        // Built-in particle inspector - just one line!
        .with_particle_inspector()
        // Built-in rule inspector - live edit rule parameters!
        .with_rule_inspector()
        // Custom UI can be added alongside the inspector
        .with_ui(|ctx| {
            // Show cluster legend
            egui::Window::new("Clusters")
                .default_pos([10.0, 550.0])
                .default_width(150.0)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.colored_label(egui::Color32::from_rgb(255, 80, 80), "●");
                        ui.label("Cluster 0 (red)");
                    });
                    ui.horizontal(|ui| {
                        ui.colored_label(egui::Color32::from_rgb(80, 255, 80), "●");
                        ui.label("Cluster 1 (green)");
                    });
                    ui.horizontal(|ui| {
                        ui.colored_label(egui::Color32::from_rgb(80, 80, 255), "●");
                        ui.label("Cluster 2 (blue)");
                    });
                    ui.horizontal(|ui| {
                        ui.colored_label(egui::Color32::from_rgb(255, 255, 80), "●");
                        ui.label("Cluster 3 (yellow)");
                    });
                    ui.horizontal(|ui| {
                        ui.colored_label(egui::Color32::from_rgb(255, 80, 255), "●");
                        ui.label("Cluster 4 (magenta)");
                    });
                });
        })
        .run().expect("Simulation failed");
}
