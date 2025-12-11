//! # Biological Neural Network Simulation
//!
//! 100,000+ neurons forming a brain-like structure with realistic
//! activation dynamics, wave propagation, and emergent synchronization.
//!
//! ## Architecture
//!
//! - **Cortical structure**: Layered organization like real brain tissue
//! - **Excitatory neurons** (80%): Spread activation to neighbors
//! - **Inhibitory neurons** (20%): Dampen local activity (prevents seizures)
//! - **Pacemaker cells**: Spontaneous firing to seed activity
//!
//! ## Dynamics
//!
//! - Membrane potential accumulates from neighbor activity
//! - Fires when threshold crossed (Rule::Sync)
//! - Refractory period prevents immediate re-firing
//! - Activity waves propagate through the network
//!
//! ## Controls
//!
//! - Adjust excitability, inhibition balance, wave speed
//! - Watch for spontaneous pattern formation
//! - See how parameter changes affect network dynamics
//!
//! Run with: `cargo run --example neural_network --release --features egui`

use rand::Rng;
use rdpe::prelude::*;
use std::sync::{Arc, Mutex};

#[derive(Particle, Clone)]
struct Neuron {
    position: Vec3,
    velocity: Vec3,
    #[color]
    color: Vec3,
    /// Membrane potential (accumulates toward threshold)
    potential: f32,
    /// Refractory timer (can't fire while > 0)
    refractory: f32,
    /// Neuron type: 0 = excitatory, 1 = inhibitory, 2 = pacemaker
    neuron_type: u32,
    /// Intrinsic firing rate (varies per neuron)
    intrinsic_rate: f32,
    /// Current activation level (for visualization)
    activation: f32,
    /// Synaptic fatigue (reduces output when high - prevents seizures)
    fatigue: f32,
    /// Adaptive threshold (increases after firing - stabilizes network)
    adaptation: f32,
}

struct NeuralParams {
    // Network dynamics
    excitability: f32,
    inhibition_strength: f32,
    threshold: f32,
    refractory_period: f32,
    decay_rate: f32,

    // Propagation
    coupling_strength: f32,
    propagation_speed: f32,

    // Visualization
    volume_density: f32,

    // Stimulation
    stimulate: bool,
}

impl Default for NeuralParams {
    fn default() -> Self {
        Self {
            // Tuned for "edge of criticality" - interesting dynamics without seizure
            excitability: 0.8,           // Slightly reduced
            inhibition_strength: 1.0,    // Increased for better balance
            threshold: 0.6,              // Lower threshold = more responsive
            refractory_period: 0.15,     // Slightly longer = more stable
            decay_rate: 3.0,             // Faster leak = harder to accumulate
            coupling_strength: 0.35,     // Slightly reduced coupling
            propagation_speed: 0.8,      // Slower propagation
            volume_density: 6.0,
            stimulate: false,
        }
    }
}

fn main() {
    let mut rng = rand::thread_rng();

    let state = Arc::new(Mutex::new(NeuralParams::default()));
    let ui_state = state.clone();
    let update_state = state.clone();

    // Build a brain-like structure
    // Multiple cortical "columns" arranged in a curved sheet
    let num_neurons = 120_000;

    let particles: Vec<Neuron> = (0..num_neurons)
        .map(|i| {
            // Create a folded cortical sheet (like brain gyri)
            let t = i as f32 / num_neurons as f32;

            // Base position on a curved surface
            let theta = t * std::f32::consts::TAU * 3.0; // Wind around
            let phi = (t * 7.0).sin() * 0.5 + 0.5; // Undulating

            // Cortical sheet coordinates
            let sheet_x = theta.cos() * (0.4 + phi * 0.3);
            let sheet_z = theta.sin() * (0.4 + phi * 0.3);
            let sheet_y = (theta * 2.0).sin() * 0.15 + (theta * 5.0).cos() * 0.08;

            // Add cortical depth (6 layers, roughly)
            let layer = rng.gen_range(0.0..1.0_f32);
            let depth = layer * 0.12; // Cortical thickness

            // Radial direction for depth
            let radial = Vec3::new(sheet_x, 0.0, sheet_z).normalize();

            // Final position with noise
            let pos = Vec3::new(
                sheet_x + radial.x * depth + rng.gen_range(-0.02..0.02),
                sheet_y + rng.gen_range(-0.03..0.03) + layer * 0.05,
                sheet_z + radial.z * depth + rng.gen_range(-0.02..0.02),
            );

            // Neuron type distribution (80% excitatory, 15% inhibitory, 5% pacemaker)
            let type_roll: f32 = rng.gen();
            let neuron_type = if type_roll < 0.05 {
                2 // Pacemaker
            } else if type_roll < 0.20 {
                1 // Inhibitory
            } else {
                0 // Excitatory
            };

            // Color by type
            let color = match neuron_type {
                0 => Vec3::new(0.2, 0.3, 0.5), // Excitatory: blue-ish (dim)
                1 => Vec3::new(0.5, 0.2, 0.3), // Inhibitory: red-ish (dim)
                _ => Vec3::new(0.4, 0.5, 0.2), // Pacemaker: yellow-green (dim)
            };

            // Intrinsic rate (pacemakers have higher rates)
            let intrinsic_rate = match neuron_type {
                2 => rng.gen_range(0.3..0.8),  // Pacemakers fire spontaneously
                _ => rng.gen_range(0.0..0.1),  // Others mostly respond to input
            };

            // Random initial potential (some neurons start closer to threshold)
            let potential = rng.gen_range(0.0..0.5);

            Neuron {
                position: pos,
                velocity: Vec3::ZERO,
                color,
                potential,
                refractory: 0.0,
                neuron_type,
                intrinsic_rate,
                activation: 0.0,
                fatigue: 0.0,
                adaptation: 0.0,
            }
        })
        .collect();

    Simulation::<Neuron>::new()
        .with_particle_count(num_neurons)
        .with_particle_size(0.004)
        .with_bounds(1.0)
        .with_spawner(move |ctx| particles[ctx.index as usize].clone())
        .with_spatial_config(0.1, 64)
        // Neural activity field (for local signal propagation)
        // Excitation decays quickly and spreads moderately
        .with_field(
            "activity",
            FieldConfig::new(80)
                .with_extent(1.0)
                .with_decay(0.85)  // Faster decay - signals don't linger
                .with_blur(0.2)
                .with_blur_iterations(2),
        )
        // Inhibitory field - spreads FASTER and WIDER than excitation
        // This is key for stability (inhibition must "catch up" to excitation)
        .with_field(
            "inhibition",
            FieldConfig::new(64)
                .with_extent(1.0)
                .with_decay(0.80)  // Faster decay too
                .with_blur(0.5)    // Much wider spread!
                .with_blur_iterations(3),
        )
        // Volume render the activity
        .with_volume_render(
            VolumeConfig::new()
                .with_field(0)
                .with_steps(64)
                .with_density_scale(6.0)
                .with_palette(Palette::Inferno)
                .with_threshold(0.01)
                .with_additive(true),
        )
        // Uniforms - matched to default params
        .with_uniform::<f32>("excitability", 0.8)
        .with_uniform::<f32>("inhibition_strength", 1.0)
        .with_uniform::<f32>("threshold", 0.6)
        .with_uniform::<f32>("refractory_period", 0.15)
        .with_uniform::<f32>("decay_rate", 3.0)
        .with_uniform::<f32>("coupling_strength", 0.35)
        .with_uniform::<f32>("propagation_speed", 0.8)
        .with_uniform::<f32>("stimulate", 0.0)
        .with_uniform::<f32>("stim_x", 0.0)
        .with_uniform::<f32>("stim_y", 0.0)
        // UI
        .with_ui(move |ctx| {
            let mut s = ui_state.lock().unwrap();

            egui::Window::new("Neural Network")
                .default_pos([10.0, 10.0])
                .default_width(300.0)
                .show(ctx, |ui| {
                    ui.heading("Network Dynamics");

                    ui.add(
                        egui::Slider::new(&mut s.excitability, 0.3..=1.5)
                            .text("Excitability")
                    );
                    ui.add(
                        egui::Slider::new(&mut s.inhibition_strength, 0.3..=2.0)
                            .text("Inhibition")
                    );
                    ui.label("Homeostatic: network self-stabilizes!");

                    ui.separator();
                    ui.add(
                        egui::Slider::new(&mut s.threshold, 0.3..=1.0)
                            .text("Fire Threshold")
                    );
                    ui.add(
                        egui::Slider::new(&mut s.refractory_period, 0.05..=0.3)
                            .text("Refractory Period")
                    );
                    ui.add(
                        egui::Slider::new(&mut s.decay_rate, 1.0..=5.0)
                            .text("Potential Decay")
                    );

                    ui.separator();
                    ui.heading("Propagation");
                    ui.add(
                        egui::Slider::new(&mut s.coupling_strength, 0.1..=0.8)
                            .text("Coupling")
                    );
                    ui.add(
                        egui::Slider::new(&mut s.propagation_speed, 0.3..=2.0)
                            .text("Wave Speed")
                    );

                    ui.separator();
                    ui.heading("Visualization");
                    ui.add(
                        egui::Slider::new(&mut s.volume_density, 2.0..=15.0)
                            .text("Glow Intensity")
                    );

                    ui.separator();
                    ui.label("Left-click to stimulate a region!");

                    ui.separator();
                    ui.heading("Neuron Counts");
                    ui.label("Total: 120,000");
                    ui.label("Excitatory: ~96,000 (80%)");
                    ui.label("Inhibitory: ~18,000 (15%)");
                    ui.label("Pacemakers: ~6,000 (5%)");

                    if ui.button("Reset to Defaults").clicked() {
                        *s = NeuralParams::default();
                    }

                    if ui.button("Trigger Wave").clicked() {
                        s.stimulate = true;
                    }
                });
        })
        .with_update(move |ctx| {
            let mut s = update_state.lock().unwrap();
            ctx.set("excitability", s.excitability);
            ctx.set("inhibition_strength", s.inhibition_strength);
            ctx.set("threshold", s.threshold);
            ctx.set("refractory_period", s.refractory_period);
            ctx.set("decay_rate", s.decay_rate);
            ctx.set("coupling_strength", s.coupling_strength);
            ctx.set("propagation_speed", s.propagation_speed);

            // Handle mouse stimulation
            if ctx.input.mouse_held(MouseButton::Left) {
                let mouse = ctx.input.mouse_ndc();
                ctx.set("stimulate", 1.0);
                ctx.set("stim_x", mouse.x);
                ctx.set("stim_y", mouse.y);
            } else if s.stimulate {
                ctx.set("stimulate", 1.0);
                ctx.set("stim_x", 0.0);
                ctx.set("stim_y", 0.0);
                s.stimulate = false;
            } else {
                ctx.set("stimulate", 0.0);
            }
        })
        // Neural dynamics with homeostatic mechanisms
        .with_rule(Rule::Custom(r#"
            // === HOMEOSTATIC DECAY ===
            // Fatigue recovers slowly (synaptic vesicle replenishment)
            p.fatigue = max(0.0, p.fatigue - uniforms.delta_time * 0.5);

            // Adaptation decays back to baseline (ion channel recovery)
            p.adaptation = max(0.0, p.adaptation - uniforms.delta_time * 0.8);

            // Decrease refractory timer
            p.refractory = max(0.0, p.refractory - uniforms.delta_time);

            // Skip if in refractory period
            if p.refractory > 0.0 {
                p.activation = max(0.0, p.activation - uniforms.delta_time * 3.0);
                return;
            }

            // === READ FIELD INPUTS ===
            let excitation = field_read(0u, p.position);
            let inhibition = field_read(1u, p.position);

            // Net input with E/I balance
            // Inhibition is stronger and acts as a "brake"
            let net_input = excitation * uniforms.excitability
                          - inhibition * uniforms.inhibition_strength * 1.5;

            // Intrinsic activity (pacemakers generate their own input)
            // Reduced when fatigued
            let fatigue_factor = 1.0 - p.fatigue * 0.8;
            let intrinsic = p.intrinsic_rate * uniforms.delta_time * fatigue_factor;

            // External stimulation
            var stim = 0.0;
            if uniforms.stimulate > 0.5 {
                let stim_pos = vec3<f32>(uniforms.stim_x * 0.8, uniforms.stim_y * 0.5, 0.0);
                let stim_dist = length(p.position - stim_pos);
                if stim_dist < 0.25 {
                    stim = (0.25 - stim_dist) * 3.0;
                }
            }

            // === ACCUMULATE POTENTIAL ===
            // Only accumulate positive input (soft rectification)
            let input = max(0.0, net_input) + intrinsic + stim;
            p.potential += input * uniforms.coupling_strength * uniforms.propagation_speed;

            // Soft saturation - potential can't exceed 2x threshold (prevents runaway)
            let max_potential = uniforms.threshold * 2.0;
            p.potential = min(p.potential, max_potential);

            // Leak (decay toward resting potential)
            p.potential = p.potential * (1.0 - uniforms.decay_rate * uniforms.delta_time);

            // === ADAPTIVE THRESHOLD ===
            // Effective threshold increases with recent activity
            let effective_threshold = uniforms.threshold * (1.0 + p.adaptation * 0.5);

            // === CHECK FOR FIRING ===
            if p.potential >= effective_threshold {
                // FIRE!
                p.potential = 0.0;
                p.refractory = uniforms.refractory_period;
                p.activation = 1.0;

                // Increase fatigue and adaptation (homeostatic response)
                p.fatigue = min(1.0, p.fatigue + 0.3);
                p.adaptation = min(1.0, p.adaptation + 0.2);

                // Emit to field - OUTPUT REDUCED BY FATIGUE
                let output_strength = 1.0 - p.fatigue * 0.6;

                if p.neuron_type == 1u {
                    // Inhibitory neuron - emit to inhibition field
                    // Inhibitory neurons are slightly LESS affected by fatigue
                    field_write(1u, p.position, 0.9 * (1.0 - p.fatigue * 0.4));
                } else {
                    // Excitatory or pacemaker - emit to activity field
                    field_write(0u, p.position, output_strength * 0.8);
                }
            }

            // === VISUALIZATION ===
            p.activation = max(0.0, p.activation - uniforms.delta_time * 4.0);

            // Base colors (slightly brighter for better visibility)
            var base_color = vec3<f32>(0.12, 0.18, 0.35); // Excitatory: blue
            if p.neuron_type == 1u {
                base_color = vec3<f32>(0.3, 0.12, 0.18); // Inhibitory: red
            } else if p.neuron_type == 2u {
                base_color = vec3<f32>(0.18, 0.28, 0.12); // Pacemaker: green
            }

            // Firing colors
            var fire_color = vec3<f32>(1.0, 0.95, 0.6); // Yellow-white flash
            if p.neuron_type == 1u {
                fire_color = vec3<f32>(1.0, 0.5, 0.6); // Inhibitory: pink flash
            }

            // Show potential buildup AND fatigue state
            let potential_glow = p.potential / effective_threshold * 0.25;
            let fatigue_dim = 1.0 - p.fatigue * 0.3; // Fatigued neurons look dimmer

            p.color = mix(base_color * fatigue_dim, fire_color, p.activation + potential_glow);
        "#.into()))
        // Gentle drift to prevent static structure
        .with_rule(Rule::Wander {
            strength: 0.02,
            frequency: 0.2,
        })
        .with_rule(Rule::Drag(5.0))
        .with_rule(Rule::SpeedLimit { min: 0.0, max: 0.05 })
        .with_rule(Rule::BounceWalls)
        .with_visuals(|v| {
            v.blend_mode(BlendMode::Additive);
            v.background(Vec3::new(0.02, 0.02, 0.03));
        })
        .run();
}
