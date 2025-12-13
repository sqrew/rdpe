//! # Slime Mold with 3D Field System
//!
//! Physarum-inspired particle simulation using the 3D spatial field system.
//! Agents deposit pheromones into a volumetric field and follow gradients.
//! The field handles diffusion and decay automatically on the GPU.
//!
//! ## What This Demonstrates
//!
//! - `.with_field(name, config)` - add 3D spatial fields
//! - `FieldConfig::new(resolution)` - configure field grid
//! - `field_write(id, pos, value)` - deposit to field
//! - `field_read(id, pos)` - sample from field
//! - Built-in decay and blur for pheromone dynamics
//! - egui integration for real-time tuning
//!
//! ## Field System vs Inbox
//!
//! **Field system** (this example):
//! - Data stored in 3D volumetric grid
//! - Automatic diffusion via blur kernel
//! - Automatic decay over time
//! - Better for continuous pheromone trails
//! - GPU memory cost scales with resolution³
//!
//! **Inbox system** (`slime_mold.rs`):
//! - Data stored per-particle
//! - Requires explicit neighbor interactions
//! - More control over diffusion logic
//! - Better for discrete particle-to-particle messages
//!
//! ## Field Configuration
//!
//! ```rust
//! FieldConfig::new(64)          // 64³ voxel grid
//!     .with_extent(1.2)         // covers -1.2 to +1.2 world space
//!     .with_decay(0.98)         // multiply values by 0.98 each frame
//!     .with_blur(0.1)           // diffusion strength
//!     .with_blur_iterations(1)  // blur passes per frame
//! ```
//!
//! ## Try This
//!
//! - Lower resolution (32) for performance, higher (128) for detail
//! - Increase blur for faster diffusion
//! - Set decay to 0.90 for quickly fading trails
//! - Add a second field for competing species (see `multi_field.rs`)
//!
//! Run with: `cargo run --example slime_mold_field --features egui`

use rand::Rng;
use rdpe::prelude::*;
use std::sync::{Arc, Mutex};

#[derive(Particle, Clone)]
struct SlimeAgent {
    position: Vec3,
    velocity: Vec3,
    #[color]
    color: Vec3,
    /// Agent's heading angle (radians, in XZ plane)
    heading: f32,
}

struct SlimeParams {
    speed: f32,
    turn_speed: f32,
    sense_dist: f32,
    sense_angle: f32,
    deposit_amount: f32,
}

impl Default for SlimeParams {
    fn default() -> Self {
        Self {
            speed: 0.5,
            turn_speed: 4.0,
            sense_dist: 0.1,
            sense_angle: 0.4,
            deposit_amount: 0.2,
        }
    }
}

fn main() {
    let mut rng = rand::thread_rng();

    let state = Arc::new(Mutex::new(SlimeParams::default()));
    let ui_state = state.clone();
    let update_state = state.clone();

    // Spawn agents in a circular pattern on the XZ plane
    let particles: Vec<_> = (0..30_000)
        .map(|_| {
            let angle = rng.gen_range(0.0..std::f32::consts::TAU);
            let radius = rng.gen_range(0.0_f32..0.8).sqrt();
            let pos = Vec3::new(radius * angle.cos(), 0.0, radius * angle.sin());
            let heading = rng.gen_range(0.0..std::f32::consts::TAU);
            (pos, heading)
        })
        .collect();

    Simulation::<SlimeAgent>::new()
        .with_particle_count(30_000)
        .with_bounds(1.2)
        .with_spawner(move |ctx| {
            let (pos, heading) = particles[ctx.index as usize];
            SlimeAgent {
                position: pos,
                velocity: Vec3::ZERO,
                color: Vec3::new(0.2, 0.8, 0.3),
                heading,
            }
        })
        // Configure the pheromone field:
        // - 64^3 resolution
        // - decay of 0.98 = moderate fading
        // - blur of 0.1 = light diffusion
        .with_field(
            "pheromone",
            FieldConfig::new(64)
                .with_extent(1.2)
                .with_decay(0.98)
                .with_blur(0.1)
                .with_blur_iterations(1),
        )
        // Movement and sensing parameters
        .with_uniform::<f32>("speed", 0.5)
        .with_uniform::<f32>("turn_speed", 4.0)
        .with_uniform::<f32>("sense_dist", 0.1)
        .with_uniform::<f32>("sense_angle", 0.4)
        .with_uniform::<f32>("deposit_amount", 0.2)
        .with_ui(move |ctx| {
            let mut s = ui_state.lock().unwrap();
            egui::Window::new("Slime Mold Field Controls")
                .default_pos([10.0, 10.0])
                .show(ctx, |ui| {
                    ui.heading("Movement");
                    ui.add(egui::Slider::new(&mut s.speed, 0.1..=2.0).text("Speed"));
                    ui.add(egui::Slider::new(&mut s.turn_speed, 0.5..=10.0).text("Turn Speed"));

                    ui.separator();
                    ui.heading("Sensing");
                    ui.add(egui::Slider::new(&mut s.sense_dist, 0.02..=0.3).text("Sense Distance"));
                    ui.add(egui::Slider::new(&mut s.sense_angle, 0.1..=1.2).text("Sense Angle"));

                    ui.separator();
                    ui.heading("Pheromone");
                    ui.add(egui::Slider::new(&mut s.deposit_amount, 0.01..=1.0).text("Deposit Amount"));

                    ui.separator();
                    if ui.button("Reset to Defaults").clicked() {
                        *s = SlimeParams::default();
                    }

                    ui.separator();
                    ui.label("Tip: View from above (drag to rotate)");
                    ui.label("for best slime mold patterns!");
                });
        })
        .with_update(move |ctx| {
            let s = update_state.lock().unwrap();
            ctx.set("speed", s.speed);
            ctx.set("turn_speed", s.turn_speed);
            ctx.set("sense_dist", s.sense_dist);
            ctx.set("sense_angle", s.sense_angle);
            ctx.set("deposit_amount", s.deposit_amount);
        })
        // Slime mold behavior: sense pheromones, turn toward highest, deposit trail
        .with_rule(Rule::Custom(
            r#"
            let dt = uniforms.delta_time;
            let speed = uniforms.speed;
            let turn_speed = uniforms.turn_speed;
            let sense_dist = uniforms.sense_dist;
            let sense_angle = uniforms.sense_angle;
            let deposit = uniforms.deposit_amount;

            // Deposit pheromone at current position
            field_write(0u, p.position, deposit);

            // Calculate forward direction from heading
            let forward = vec3<f32>(cos(p.heading), 0.0, sin(p.heading));

            // Sense in three directions: forward, left, right
            let sense_fwd = p.position + forward * sense_dist;
            let left_angle = p.heading + sense_angle;
            let right_angle = p.heading - sense_angle;
            let sense_left = p.position + vec3<f32>(cos(left_angle), 0.0, sin(left_angle)) * sense_dist;
            let sense_right = p.position + vec3<f32>(cos(right_angle), 0.0, sin(right_angle)) * sense_dist;

            // Sample pheromone at each sensor
            let val_fwd = field_read(0u, sense_fwd);
            let val_left = field_read(0u, sense_left);
            let val_right = field_read(0u, sense_right);

            // Turn toward highest pheromone concentration
            if val_left > val_fwd && val_left > val_right {
                p.heading += turn_speed * dt;
            } else if val_right > val_fwd && val_right > val_left {
                p.heading -= turn_speed * dt;
            } else if val_left > val_right {
                p.heading += turn_speed * dt * 0.5;
            } else if val_right > val_left {
                p.heading -= turn_speed * dt * 0.5;
            }
            // Add small random wiggle for exploration
            p.heading += (sin(uniforms.time * 10.0 + f32(p.alive) * 123.456) * 0.1) * dt;

            // Move forward
            let new_forward = vec3<f32>(cos(p.heading), 0.0, sin(p.heading));
            p.position += new_forward * speed * dt;

            // Wrap at boundaries
            if p.position.x > 1.1 { p.position.x = -1.1; }
            if p.position.x < -1.1 { p.position.x = 1.1; }
            if p.position.z > 1.1 { p.position.z = -1.1; }
            if p.position.z < -1.1 { p.position.z = 1.1; }
            p.position.y = 0.0;

            // Color based on pheromone concentration at current position
            let pheromone = field_read(0u, p.position);
            let intensity = clamp(pheromone * 2.0, 0.0, 1.0);
            p.color = vec3<f32>(intensity * 0.2, 0.3 + intensity * 0.5, 0.1 + intensity * 0.3);

            // Zero velocity (we handle movement directly)
            p.velocity = vec3<f32>(0.0, 0.0, 0.0);
        "#
            .into(),
        ))
        .run().expect("Simulation failed");
}
