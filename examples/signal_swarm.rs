//! # Signal Swarm
//!
//! A network of signal-processing agents demonstrating the logic and data
//! transformation rules. Particles act as neurons/processors that:
//!
//! - Accumulate "charge" from neighbors
//! - Fire when charge exceeds threshold (with hysteresis to prevent jitter)
//! - Enter refractory period after firing (latch)
//! - Spread activation to neighbors
//! - Display state through color blending
//!
//! Showcases: Hysteresis, Latch, Edge, Threshold, Select, Blend, Remap,
//! Smooth, Noise, Clamp, Copy, And, Or, Diffuse
//!
//! Run with: `cargo run --example signal_swarm --features egui`

use rand::Rng;
use rdpe::prelude::*;
use std::f32::consts::TAU;
use std::sync::{Arc, Mutex};

#[derive(Particle, Clone)]
struct SignalAgent {
    position: Vec3,
    velocity: Vec3,
    #[color]
    color: Vec3,

    // Signal state
    charge: f32,           // Accumulated activation (0-1)
    charge_prev: f32,      // Previous frame's charge (for edge detection)
    firing: f32,           // 1.0 when actively firing, 0.0 otherwise
    refractory: f32,       // 1.0 during cooldown, 0.0 when ready

    // Derived/display values
    ready_to_fire: f32,    // AND of (charge high, not refractory)
    display_glow: f32,     // Smoothed visual brightness
    heat: f32,             // Accumulated firing history (for color)

    // Type (0 = normal, 1 = inhibitor, 2 = pacemaker)
    agent_type: f32,
}

struct SwarmParams {
    charge_rate: f32,
    fire_threshold: f32,
    refractory_time: f32,
    diffuse_rate: f32,
    noise_amount: f32,
    movement: f32,
}

impl Default for SwarmParams {
    fn default() -> Self {
        Self {
            charge_rate: 0.3,
            fire_threshold: 0.7,
            refractory_time: 0.5,
            diffuse_rate: 0.4,
            noise_amount: 0.1,
            movement: 0.2,
        }
    }
}

fn main() {
    let mut rng = rand::thread_rng();

    let state = Arc::new(Mutex::new(SwarmParams::default()));
    let ui_state = state.clone();
    let update_state = state.clone();

    // Create agents in a spherical distribution
    let particles: Vec<SignalAgent> = (0..800)
        .map(|i| {
            // Spherical distribution
            let theta = rng.gen_range(0.0..TAU);
            let phi = rng.gen_range(0.0..std::f32::consts::PI);
            let r = rng.gen_range(0.2..0.7);

            let x = r * phi.sin() * theta.cos();
            let y = r * phi.cos();
            let z = r * phi.sin() * theta.sin();

            // Random initial charge (mostly low, some high to seed activity)
            let charge = if rng.gen_bool(0.1) {
                rng.gen_range(0.6..0.9)
            } else {
                rng.gen_range(0.0..0.3)
            };

            // Agent types: 90% normal, 5% inhibitor, 5% pacemaker
            let agent_type = if i % 20 == 0 {
                2.0 // Pacemaker
            } else if i % 20 == 1 {
                1.0 // Inhibitor
            } else {
                0.0 // Normal
            };

            SignalAgent {
                position: Vec3::new(x, y, z),
                velocity: Vec3::ZERO,
                color: Vec3::ONE,
                charge,
                charge_prev: charge,
                firing: 0.0,
                refractory: 0.0,
                ready_to_fire: 0.0,
                display_glow: 0.0,
                heat: 0.0,
                agent_type,
            }
        })
        .collect();

    Simulation::<SignalAgent>::new()
        .with_particle_count(800)
        .with_particle_size(0.012)
        .with_bounds(1.0)
        .with_spawner(move |i, _| particles[i as usize].clone())
        .with_spatial_config(0.15, 32)

        // Uniforms
        .with_uniform::<f32>("charge_rate", 0.3)
        .with_uniform::<f32>("fire_threshold", 0.7)
        .with_uniform::<f32>("refractory_time", 0.5)
        .with_uniform::<f32>("diffuse_rate", 0.4)
        .with_uniform::<f32>("noise_amount", 0.1)
        .with_uniform::<f32>("movement", 0.2)

        // UI
        .with_ui(move |ctx| {
            let mut s = ui_state.lock().unwrap();

            egui::Window::new("Signal Swarm")
                .default_pos([10.0, 10.0])
                .default_width(280.0)
                .show(ctx, |ui| {
                    ui.heading("Signal Dynamics");
                    ui.add(egui::Slider::new(&mut s.charge_rate, 0.1..=1.0).text("Charge Rate"));
                    ui.add(egui::Slider::new(&mut s.fire_threshold, 0.3..=0.9).text("Fire Threshold"));
                    ui.add(egui::Slider::new(&mut s.refractory_time, 0.1..=1.0).text("Refractory Time"));

                    ui.separator();
                    ui.heading("Propagation");
                    ui.add(egui::Slider::new(&mut s.diffuse_rate, 0.0..=1.0).text("Diffuse Rate"));
                    ui.add(egui::Slider::new(&mut s.noise_amount, 0.0..=0.3).text("Noise"));

                    ui.separator();
                    ui.heading("Movement");
                    ui.add(egui::Slider::new(&mut s.movement, 0.0..=0.5).text("Wander"));

                    ui.separator();
                    ui.label("Agent types:");
                    ui.label("  White/Cyan: Normal neurons");
                    ui.label("  Magenta: Inhibitors");
                    ui.label("  Yellow: Pacemakers");

                    if ui.button("Reset").clicked() {
                        *s = SwarmParams::default();
                    }
                });
        })

        .with_update(move |ctx| {
            let s = update_state.lock().unwrap();
            ctx.set("charge_rate", s.charge_rate);
            ctx.set("fire_threshold", s.fire_threshold);
            ctx.set("refractory_time", s.refractory_time);
            ctx.set("diffuse_rate", s.diffuse_rate);
            ctx.set("noise_amount", s.noise_amount);
            ctx.set("movement", s.movement);
        })

        // =====================================================================
        // SIGNAL PROCESSING RULES - showcasing the new rule system
        // =====================================================================

        // Store previous charge for edge detection
        .with_rule(Rule::Copy {
            from: "charge".into(),
            to: "charge_prev".into(),
            scale: 1.0,
            offset: 0.0,
        })

        // Pacemakers slowly auto-charge
        .with_rule(Rule::Gate {
            condition: "p.agent_type > 1.5".into(), // Pacemaker
            action: "p.charge += 0.15 * uniforms.delta_time;".into(),
        })

        // Add noise to charge (organic variation)
        .with_rule(Rule::Noise {
            field: "charge".into(),
            amplitude: 0.05,
            frequency: 3.0,
            time_scale: 2.0,
        })

        // Natural charge decay when not firing
        .with_rule(Rule::Gate {
            condition: "p.firing < 0.5".into(),
            action: "p.charge -= 0.05 * uniforms.delta_time;".into(),
        })

        // Clamp charge to valid range
        .with_rule(Rule::Clamp {
            field: "charge".into(),
            min: 0.0,
            max: 1.0,
        })

        // Check if ready to fire: charge high AND not refractory
        // Using NOT to invert refractory, then AND with charge threshold
        .with_rule(Rule::Threshold {
            input_field: "charge".into(),
            output_field: "ready_to_fire".into(),
            threshold: 0.7, // Will use uniform in custom rule
            above: 1.0,
            below: 0.0,
        })

        // Combine: ready = (charge_high AND NOT refractory)
        .with_rule(Rule::Custom(r#"
            let charge_ready = select(0.0, 1.0, p.charge > uniforms.fire_threshold);
            let not_refractory = 1.0 - p.refractory;
            p.ready_to_fire = min(charge_ready, not_refractory);
        "#.into()))

        // Hysteresis for firing state - prevents rapid on/off
        .with_rule(Rule::Hysteresis {
            input: "ready_to_fire".into(),
            output: "firing".into(),
            low_threshold: 0.3,
            high_threshold: 0.7,
            on_value: 1.0,
            off_value: 0.0,
        })

        // Edge detection: pulse on firing start
        .with_rule(Rule::Edge {
            input: "firing".into(),
            prev_field: "charge_prev".into(), // Reusing for edge detection
            output: "display_glow".into(),
            threshold: 0.5,
            rising: true,
            falling: false,
        })

        // When firing starts, reset charge and enter refractory
        .with_rule(Rule::Latch {
            output: "refractory".into(),
            set_condition: "p.firing > 0.5 && p.charge > 0.5".into(),
            reset_condition: "p.refractory > 0.0 && uniforms.time % uniforms.refractory_time < uniforms.delta_time * 2.0".into(),
            set_value: 1.0,
            reset_value: 0.0,
        })

        // Reset charge when firing
        .with_rule(Rule::Gate {
            condition: "p.firing > 0.5 && p.charge > 0.3".into(),
            action: "p.charge = 0.0;".into(),
        })

        // Accumulate heat (firing history) with decay
        .with_rule(Rule::Gate {
            condition: "p.firing > 0.5".into(),
            action: "p.heat += 0.3;".into(),
        })
        .with_rule(Rule::Smooth {
            field: "heat".into(),
            target: 0.0,
            rate: 0.5,
        })
        .with_rule(Rule::Clamp {
            field: "heat".into(),
            min: 0.0,
            max: 1.0,
        })

        // Smooth the display glow for nice visuals
        .with_rule(Rule::Custom(r#"
            // Boost glow when firing
            if p.firing > 0.5 {
                p.display_glow = 1.0;
            }
            // Smooth decay
            p.display_glow = mix(p.display_glow, 0.0, 2.0 * uniforms.delta_time);
        "#.into()))

        // Neighbor interaction: firing agents spread charge
        .with_rule(Rule::NeighborCustom(r#"
            if neighbor_dist < 0.12 && neighbor_dist > 0.001 {
                // Firing neighbors add charge (normal agents)
                if other.firing > 0.5 && other.agent_type < 0.5 {
                    p.charge += 0.4 * uniforms.delta_time * uniforms.diffuse_rate;
                }
                // Inhibitors suppress charge when firing
                if other.firing > 0.5 && other.agent_type > 0.5 && other.agent_type < 1.5 {
                    p.charge -= 0.6 * uniforms.delta_time * uniforms.diffuse_rate;
                }
            }
        "#.into()))

        // Color based on state using blending
        .with_rule(Rule::Custom(r#"
            // Base color by type
            var base_color = vec3<f32>(0.3, 0.4, 0.5); // Normal: blue-gray
            if p.agent_type > 1.5 {
                base_color = vec3<f32>(0.6, 0.5, 0.2); // Pacemaker: gold
            } else if p.agent_type > 0.5 {
                base_color = vec3<f32>(0.5, 0.2, 0.4); // Inhibitor: purple
            }

            // Glow color (white-cyan when firing)
            let glow_color = vec3<f32>(0.8, 1.0, 1.0);

            // Heat adds red tint
            let heat_color = vec3<f32>(1.0, 0.3, 0.2);

            // Blend based on states
            var c = base_color;
            c = mix(c, heat_color, p.heat * 0.5);
            c = mix(c, glow_color, p.display_glow);

            // Dim when refractory
            if p.refractory > 0.5 {
                c *= 0.4;
            }

            // Charge glow
            c += vec3<f32>(0.2, 0.3, 0.4) * p.charge;

            p.color = c;
        "#.into()))

        // Movement
        .with_rule(Rule::Wander {
            strength: 0.2,
            frequency: 0.5,
        })
        .with_rule(Rule::Custom(r#"
            p.velocity *= uniforms.movement / 0.2;
        "#.into()))

        // Firing agents get a little kick
        .with_rule(Rule::Gate {
            condition: "p.display_glow > 0.8".into(),
            action: "p.velocity += normalize(p.position) * 0.3;".into(),
        })

        .with_rule(Rule::Drag(2.0))
        .with_rule(Rule::SpeedLimit { min: 0.0, max: 0.4 })
        .with_rule(Rule::BounceWalls)

        .with_visuals(|v| {
            v.blend_mode(BlendMode::Additive);
            v.background(Vec3::new(0.02, 0.02, 0.04));
        })
        .run();
}
