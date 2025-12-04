//! # Particle Computation Demo
//!
//! An experiment in making particles compute. Particles form a signal-processing
//! chain where signals propagate left-to-right, with NOT gates that invert.
//!
//! This is a proof-of-concept for particle-based computation:
//! - Wire particles: pass signal unchanged
//! - NOT particles: invert signal
//! - AND particles: output 1 only if both inputs are 1
//!
//! Run with: `cargo run --example compute_demo --features egui`

use rdpe::prelude::*;
use std::sync::{Arc, Mutex};

#[derive(Particle, Clone)]
struct GateParticle {
    position: Vec3,
    velocity: Vec3,
    #[color]
    color: Vec3,

    // Computation state
    signal: f32,           // Current signal (0.0 or 1.0)
    signal_in: f32,        // Accumulated input this frame
    input_count: f32,      // Number of inputs received

    // Gate type: 0=wire, 1=NOT, 2=AND, 3=OR, 4=input source
    gate_type: f32,

    // For input sources: oscillation phase
    phase: f32,
}

struct ComputeParams {
    propagation_speed: f32,
    input_frequency: f32,
    show_labels: bool,
}

impl Default for ComputeParams {
    fn default() -> Self {
        Self {
            propagation_speed: 1.0,
            input_frequency: 1.0,
            show_labels: true,
        }
    }
}

fn main() {
    let state = Arc::new(Mutex::new(ComputeParams::default()));
    let ui_state = state.clone();
    let update_state = state.clone();

    // Build a simple circuit with tighter spacing so neighbors can communicate:
    //
    // INPUT A --[NOT]--+
    //                  +--[AND]--[WIRE]-- OUTPUT
    // INPUT B ---------+
    //
    // Spacing: 0.15 apart so particles stay within neighbor detection range (0.25)

    let mut particles = Vec::new();
    let spacing = 0.15;
    let y_top = 0.12;
    let y_bottom = -0.12;

    // Helper to create a gate particle
    let make_gate = |pos: Vec3, gate_type: f32, phase: f32| -> GateParticle {
        let color = match gate_type as u32 {
            4 => Vec3::new(0.9, 0.7, 0.1), // Input: yellow
            2 => Vec3::new(0.2, 0.9, 0.2), // AND: green
            1 => Vec3::new(0.9, 0.2, 0.2), // NOT: red
            _ => Vec3::new(0.3, 0.5, 0.9), // Wire: blue
        };
        GateParticle {
            position: pos,
            velocity: Vec3::ZERO,
            color,
            signal: 0.0,
            signal_in: 0.0,
            input_count: 0.0,
            gate_type,
            phase,
        }
    };

    // Top row: INPUT A -> NOT -> (diagonal down to AND)
    particles.push(make_gate(Vec3::new(-0.45, y_top, 0.0), 4.0, 0.0)); // Input A
    particles.push(make_gate(Vec3::new(-0.30, y_top, 0.0), 1.0, 0.0)); // NOT
    particles.push(make_gate(Vec3::new(-0.15, y_top * 0.5, 0.0), 0.0, 0.0)); // Wire (diagonal connector)

    // Bottom row: INPUT B -> WIRE -> (diagonal up to AND)
    particles.push(make_gate(Vec3::new(-0.45, y_bottom, 0.0), 4.0, std::f32::consts::PI * 0.5)); // Input B (phase offset)
    particles.push(make_gate(Vec3::new(-0.30, y_bottom, 0.0), 0.0, 0.0)); // Wire
    particles.push(make_gate(Vec3::new(-0.15, y_bottom * 0.5, 0.0), 0.0, 0.0)); // Wire (diagonal connector)

    // AND gate (center, takes inputs from both connectors)
    particles.push(make_gate(Vec3::new(0.0, 0.0, 0.0), 2.0, 0.0)); // AND

    // Output wires
    particles.push(make_gate(Vec3::new(0.15, 0.0, 0.0), 0.0, 0.0)); // Wire
    particles.push(make_gate(Vec3::new(0.30, 0.0, 0.0), 0.0, 0.0)); // Wire
    particles.push(make_gate(Vec3::new(0.45, 0.0, 0.0), 0.0, 0.0)); // Wire (final output)

    let particle_count = particles.len() as u32;

    Simulation::<GateParticle>::new()
        .with_particle_count(particle_count)
        .with_bounds(1.5)
        .with_spawner(move |i, _| {
            if (i as usize) < particles.len() {
                particles[i as usize].clone()
            } else {
                // Extra particles (shouldn't happen)
                GateParticle {
                    position: Vec3::new(10.0, 10.0, 10.0), // Off screen
                    velocity: Vec3::ZERO,
                    color: Vec3::ZERO,
                    signal: 0.0,
                    signal_in: 0.0,
                    input_count: 0.0,
                    gate_type: 0.0,
                    phase: 0.0,
                }
            }
        })
        .with_particle_size(0.04)
        .with_spatial_config(0.3, 16)

        .with_uniform::<f32>("propagation_speed", 1.0)
        .with_uniform::<f32>("input_frequency", 1.0)

        .with_ui(move |ctx| {
            let mut s = ui_state.lock().unwrap();

            egui::Window::new("Particle Compute")
                .default_pos([10.0, 10.0])
                .default_width(280.0)
                .show(ctx, |ui| {
                    ui.heading("Circuit: (NOT A) AND B");
                    ui.label("Two inputs oscillate at different phases.");
                    ui.label("A goes through NOT, B goes straight, then AND.");

                    ui.separator();
                    ui.add(egui::Slider::new(&mut s.input_frequency, 0.1..=2.0).text("Input Freq"));

                    ui.separator();
                    ui.heading("Gate Colors:");
                    ui.label("  Yellow: Input source");
                    ui.label("  Blue: Wire");
                    ui.label("  Red: NOT gate");
                    ui.label("  Green: AND gate");
                    ui.label("  Bright = signal ON");

                    ui.separator();
                    ui.label("Watch the signal propagate!");
                    ui.label("Output = (NOT A) AND B");
                });
        })

        .with_update(move |ctx| {
            let s = update_state.lock().unwrap();
            ctx.set("input_frequency", s.input_frequency);
        })

        // Reset accumulators each frame
        .with_rule(Rule::Custom(r#"
            p.signal_in = 0.0;
            p.input_count = 0.0;
        "#.into()))

        // Input sources oscillate
        .with_rule(Rule::Gate {
            condition: "p.gate_type > 3.5".into(), // Input source
            action: r#"
                p.phase += uniforms.delta_time * uniforms.input_frequency * 6.28318;
                p.signal = select(0.0, 1.0, sin(p.phase) > 0.0);
            "#.into(),
        })

        // Gather inputs from neighbors "behind" us (left side, or above/below for AND)
        .with_rule(Rule::NeighborCustom(r#"
            // Debug: count ALL neighbors first, then filter
            if neighbor_dist > 0.001 {
                // Count this as a potential neighbor for debugging
                p.input_count += 0.001;
            }

            // Only process neighbors in valid range
            if neighbor_dist < 0.5 && neighbor_dist > 0.01 {
                // Direction from neighbor to us
                let dir = p.position - other.position;

                // For wires/NOT: accept input from left (negative X direction)
                // For AND: accept from anywhere nearby
                var accept_input = false;

                if p.gate_type < 1.5 {
                    // Wire or NOT: input must be to our left
                    accept_input = dir.x > 0.02;
                } else if p.gate_type < 2.5 {
                    // AND: accept from anywhere nearby
                    accept_input = true;
                }

                if accept_input {
                    p.signal_in += other.signal;
                    p.input_count += 1.0;
                }
            }
        "#.into()))

        // Process gates based on accumulated inputs
        .with_rule(Rule::Custom(r#"
            // Skip input sources (they set their own signal)
            if p.gate_type < 3.5 {
                // Only process if we got any inputs
                if p.input_count > 0.5 {
                    if p.gate_type < 0.5 {
                        // WIRE: pass through
                        if p.signal_in > 0.5 {
                            p.signal = 1.0;
                        } else {
                            p.signal = 0.0;
                        }
                    } else if p.gate_type < 1.5 {
                        // NOT: invert
                        if p.signal_in > 0.5 {
                            p.signal = 0.0;
                        } else {
                            p.signal = 1.0;
                        }
                    } else if p.gate_type < 2.5 {
                        // AND: need multiple high inputs
                        if p.signal_in > 1.5 {
                            p.signal = 1.0;
                        } else {
                            p.signal = 0.0;
                        }
                    }
                }
            }
        "#.into()))

        // Set color based on gate type and signal
        .with_rule(Rule::Custom(r#"
            // Dim base color by gate type
            var base = vec3<f32>(0.15, 0.2, 0.4); // Wire: dim blue
            if p.gate_type > 3.5 {
                base = vec3<f32>(0.4, 0.3, 0.05); // Input: dim yellow
            } else if p.gate_type > 1.5 {
                base = vec3<f32>(0.1, 0.4, 0.1); // AND: dim green
            } else if p.gate_type > 0.5 {
                base = vec3<f32>(0.4, 0.1, 0.1); // NOT: dim red
            }

            // Bright version when signal is high
            var bright = vec3<f32>(0.4, 0.6, 1.0); // Wire: bright blue
            if p.gate_type > 3.5 {
                bright = vec3<f32>(1.0, 0.9, 0.3); // Input: bright yellow
            } else if p.gate_type > 1.5 {
                bright = vec3<f32>(0.3, 1.0, 0.3); // AND: bright green
            } else if p.gate_type > 0.5 {
                bright = vec3<f32>(1.0, 0.4, 0.3); // NOT: bright red
            }

            // Interpolate between dim and bright based on signal
            p.color = mix(base, bright, p.signal);

            // DEBUG: Add purple tint if we received ANY neighbor queries
            if p.input_count > 0.5 {
                p.color.x += 0.3; // Add red tint when receiving inputs
            }
        "#.into()))

        // Keep particles stationary
        .with_rule(Rule::Custom(r#"
            p.velocity = vec3<f32>(0.0, 0.0, 0.0);
        "#.into()))

        .with_visuals(|v| {
            v.blend_mode(BlendMode::Alpha);
            v.background(Vec3::new(0.05, 0.05, 0.08));
        })
        .run();
}
