//! # Slime Mold Simulation
//!
//! Physarum-inspired particle simulation where agents deposit and follow
//! pheromone trails, creating emergent network structures.
//!
//! ## What This Demonstrates
//!
//! - **Inbox system** - particle-to-particle messaging
//! - **Emergent behavior** - simple rules creating complex patterns
//! - **egui integration** - real-time parameter tuning
//! - **Custom neighbor interactions** - sensing pheromones from neighbors
//! - **Heading-based movement** - agents maintain direction state
//!
//! ## The Biology
//!
//! This simulates *Physarum polycephalum* (slime mold) behavior:
//! 1. Agents deposit pheromone trails
//! 2. Agents sense pheromones in three directions (left, center, right)
//! 3. Agents turn toward strongest pheromone concentration
//! 4. Pheromones decay and diffuse over time
//!
//! The result: organic network patterns emerge from simple local rules.
//!
//! ## Key Parameters
//!
//! - **Speed/Turn Speed**: How fast agents move and rotate
//! - **Sense Distance/Angle**: How far and wide agents sense
//! - **Deposit/Decay Rate**: Pheromone dynamics
//! - **Repulsion**: Prevents agents from bunching up
//!
//! ## Technical Notes
//!
//! Uses the **inbox system** for pheromone communication:
//! - `inbox_send(particle_idx, slot, value)` - send data to a particle
//! - `inbox_receive_at(idx, slot)` - receive accumulated data
//!
//! Compare with `slime_mold_field.rs` which uses 3D spatial fields instead.
//!
//! ## Try This
//!
//! - Increase deposit rate for thicker trails
//! - Reduce sense angle for straighter paths
//! - Increase repulsion for more spread-out networks
//! - Set decay very high to see trails form and fade quickly
//!
//! Run with: `cargo run --example slime_mold --features egui`

use rand::Rng;
use rdpe::prelude::*;
use std::sync::{Arc, Mutex};

#[derive(Particle, Clone)]
struct SlimeAgent {
    position: Vec3,
    velocity: Vec3,
    #[color]
    color: Vec3,
    /// Pheromone concentration at this particle's location
    pheromone: f32,
    /// Agent's heading angle (radians, in XZ plane)
    heading: f32,
}

struct SlimeParams {
    // Movement
    speed: f32,
    turn_speed: f32,
    wiggle: f32,

    // Pheromone
    deposit_rate: f32,
    decay_rate: f32,
    diffuse_rate: f32,

    // Sensing
    sense_dist: f32,
    sense_angle: f32,

    // Repulsion
    repel_dist: f32,
    repel_strength: f32,
}

impl Default for SlimeParams {
    fn default() -> Self {
        Self {
            speed: 0.4,
            turn_speed: 4.0,
            wiggle: 0.5,
            deposit_rate: 0.3,
            decay_rate: 0.5,
            diffuse_rate: 0.01,
            sense_dist: 0.12,
            sense_angle: 0.3,
            repel_dist: 0.025,
            repel_strength: 0.5,
        }
    }
}

fn main() {
    let mut rng = rand::thread_rng();

    let state = Arc::new(Mutex::new(SlimeParams::default()));
    let ui_state = state.clone();
    let update_state = state.clone();

    // Spawn agents in a circular pattern
    let particles: Vec<_> = (0..10_000)
        .map(|_| {
            let angle = rng.gen_range(0.0..std::f32::consts::TAU);
            let radius = rng.gen_range(0.0_f32..0.8).sqrt();
            let pos = Vec3::new(radius * angle.cos(), 0.0, radius * angle.sin());
            let heading = rng.gen_range(0.0..std::f32::consts::TAU);
            (pos, heading)
        })
        .collect();

    Simulation::<SlimeAgent>::new()
        .with_particle_count(10_000)
        .with_bounds(1.2)
        .with_spawner(move |i, _| {
            let (pos, heading) = particles[i as usize];
            SlimeAgent {
                position: pos,
                velocity: Vec3::ZERO,
                color: Vec3::new(0.2, 0.8, 0.3),
                pheromone: 0.1,
                heading,
            }
        })
        .with_inbox()
        .with_spatial_config(0.15, 32)

        // Custom uniforms for all parameters
        .with_uniform::<f32>("speed", 0.4)
        .with_uniform::<f32>("turn_speed", 4.0)
        .with_uniform::<f32>("wiggle", 0.5)
        .with_uniform::<f32>("deposit_rate", 0.3)
        .with_uniform::<f32>("decay_rate", 0.5)
        .with_uniform::<f32>("diffuse_rate", 0.01)
        .with_uniform::<f32>("sense_dist", 0.12)
        .with_uniform::<f32>("sense_angle", 0.3)
        .with_uniform::<f32>("repel_dist", 0.025)
        .with_uniform::<f32>("repel_strength", 0.5)

        .with_ui(move |ctx| {
            let mut s = ui_state.lock().unwrap();
            egui::Window::new("Slime Mold Controls")
                .default_pos([10.0, 10.0])
                .show(ctx, |ui| {
                    ui.heading("Movement");
                    ui.add(egui::Slider::new(&mut s.speed, 0.1..=1.0).text("Speed"));
                    ui.add(egui::Slider::new(&mut s.turn_speed, 1.0..=10.0).text("Turn Speed"));
                    ui.add(egui::Slider::new(&mut s.wiggle, 0.0..=2.0).text("Wiggle"));

                    ui.separator();
                    ui.heading("Pheromone");
                    ui.add(egui::Slider::new(&mut s.deposit_rate, 0.0..=1.0).text("Deposit Rate"));
                    ui.add(egui::Slider::new(&mut s.decay_rate, 0.0..=2.0).text("Decay Rate"));
                    ui.add(egui::Slider::new(&mut s.diffuse_rate, 0.0..=0.1).text("Diffuse Rate"));

                    ui.separator();
                    ui.heading("Sensing");
                    ui.add(egui::Slider::new(&mut s.sense_dist, 0.02..=0.3).text("Sense Distance"));
                    ui.add(egui::Slider::new(&mut s.sense_angle, 0.1..=0.8).text("Sense Angle"));

                    ui.separator();
                    ui.heading("Repulsion");
                    ui.add(egui::Slider::new(&mut s.repel_dist, 0.0..=0.1).text("Repel Distance"));
                    ui.add(egui::Slider::new(&mut s.repel_strength, 0.0..=2.0).text("Repel Strength"));

                    ui.separator();
                    if ui.button("Reset").clicked() {
                        *s = SlimeParams::default();
                    }

                    ui.separator();
                    ui.label("View from above for best effect!");
                });
        })

        .with_update(move |ctx| {
            let s = update_state.lock().unwrap();
            ctx.set("speed", s.speed);
            ctx.set("turn_speed", s.turn_speed);
            ctx.set("wiggle", s.wiggle);
            ctx.set("deposit_rate", s.deposit_rate);
            ctx.set("decay_rate", s.decay_rate);
            ctx.set("diffuse_rate", s.diffuse_rate);
            ctx.set("sense_dist", s.sense_dist);
            ctx.set("sense_angle", s.sense_angle);
            ctx.set("repel_dist", s.repel_dist);
            ctx.set("repel_strength", s.repel_strength);
        })

        // Neighbor rule: sense pheromones, share them, and avoid crowding
        .with_rule(Rule::NeighborCustom(r#"
            // Get my forward direction
            let forward = vec3<f32>(cos(p.heading), 0.0, sin(p.heading));
            let to_neighbor = -neighbor_dir;

            // === REPULSION ===
            if neighbor_dist < uniforms.repel_dist && neighbor_dist > 0.001 {
                p.velocity += neighbor_dir * uniforms.repel_strength;
            }

            // === PHEROMONE SENSING ===
            if neighbor_dist < uniforms.sense_dist && neighbor_dist > 0.01 {
                let dot_forward = dot(forward, to_neighbor);
                let right = vec3<f32>(-sin(p.heading), 0.0, cos(p.heading));
                let dot_right = dot(right, to_neighbor);

                let weight = (1.0 - neighbor_dist / uniforms.sense_dist);
                let signal = other.pheromone * weight * 0.5;

                if dot_forward > 0.0 {
                    if dot_right < -uniforms.sense_angle {
                        inbox_send(index, 0u, signal);  // Left
                    } else if dot_right > uniforms.sense_angle {
                        inbox_send(index, 2u, signal);  // Right
                    } else {
                        inbox_send(index, 1u, signal);  // Center
                    }
                }
            }

            // === PHEROMONE DIFFUSION ===
            if neighbor_dist < 0.05 && neighbor_dist > 0.001 {
                inbox_send(other_idx, 3u, p.pheromone * uniforms.diffuse_rate);
            }
        "#.into()))

        // Main update: steering, movement, pheromone dynamics
        .with_rule(Rule::Custom(r#"
            let dt = uniforms.delta_time;

            // Read sensor values
            let sense_left = inbox_receive_at(index, 0u);
            let sense_center = inbox_receive_at(index, 1u);
            let sense_right = inbox_receive_at(index, 2u);
            let diffused_in = inbox_receive_at(index, 3u);

            // Steering
            let total_sense = sense_left + sense_center + sense_right + 0.001;

            if sense_center >= sense_left && sense_center >= sense_right {
                let w = sin(uniforms.time * 30.0 + p.heading * 20.0) * uniforms.wiggle;
                p.heading += w * dt;
            } else if sense_left > sense_right {
                p.heading -= uniforms.turn_speed * dt;
            } else {
                p.heading += uniforms.turn_speed * dt;
            }

            // Random exploration when no pheromone
            if total_sense < 0.1 {
                let rand = sin(uniforms.time * 50.0 + p.position.x * 100.0 + p.position.z * 77.0);
                p.heading += rand * 2.0 * dt;
            }

            // Movement
            let heading_vel = vec3<f32>(cos(p.heading), 0.0, sin(p.heading)) * uniforms.speed;
            p.velocity = heading_vel + p.velocity * 0.3;
            p.position += p.velocity * dt;
            p.velocity = vec3<f32>(0.0);

            // Wrap edges
            let bound = 1.0;
            if p.position.x > bound { p.position.x = -bound; }
            if p.position.x < -bound { p.position.x = bound; }
            if p.position.z > bound { p.position.z = -bound; }
            if p.position.z < -bound { p.position.z = bound; }
            p.position.y = 0.0;

            // Pheromone dynamics
            p.pheromone += uniforms.deposit_rate * dt;
            p.pheromone += diffused_in;
            p.pheromone *= 1.0 - uniforms.decay_rate * dt;
            p.pheromone = clamp(p.pheromone, 0.0, 3.0);

            // Color gradient
            let t = clamp(p.pheromone / 2.0, 0.0, 1.0);
            if t < 0.5 {
                p.color = mix(vec3<f32>(0.1, 0.02, 0.15), vec3<f32>(0.1, 0.5, 0.2), t * 2.0);
            } else {
                p.color = mix(vec3<f32>(0.1, 0.5, 0.2), vec3<f32>(0.4, 1.0, 0.8), (t - 0.5) * 2.0);
            }
        "#.into()))

        .run();
}
