//! # Murmuration
//!
//! A starling flock simulation with dramatic swooping behavior.
//!
//! Run with: `cargo run --example murmuration --features egui`

use rand::Rng;
use rdpe::prelude::*;
use std::sync::{Arc, Mutex};

#[derive(Particle, Clone)]
struct Bird {
    position: Vec3,
    velocity: Vec3,
    #[color]
    color: Vec3,
    /// 0.0 = normal bird, 1.0 = predator
    is_predator: f32,
}

struct FlockState {
    separation: f32,
    alignment: f32,
    cohesion: f32,
    max_speed: f32,
    predator_fear: f32,
    time_scale: f32,
}

impl Default for FlockState {
    fn default() -> Self {
        Self {
            separation: 2.0,
            alignment: 1.0,
            cohesion: 1.0,
            max_speed: 1.2,
            predator_fear: 4.0,
            time_scale: 1.0,
        }
    }
}

fn main() {
    let mut rng = rand::thread_rng();

    let state = Arc::new(Mutex::new(FlockState::default()));
    let ui_state = state.clone();
    let update_state = state.clone();

    // Spawn birds spread out (first particle is the predator)
    let particles: Vec<_> = (0..8_000)
        .map(|i| {
            if i == 0 {
                // Predator - bright red, starts at orbit position
                (
                    Vec3::new(1.0, 0.0, 0.0), // Starting position
                    Vec3::ZERO,               // Velocity controlled by orbit
                    Vec3::new(1.0, 0.2, 0.1), // Bright red
                    1.0f32,                   // is_predator
                )
            } else {
                // Spawn birds in a larger area
                let pos = Vec3::new(
                    rng.gen_range(-0.8..0.8),
                    rng.gen_range(-0.8..0.8),
                    rng.gen_range(-0.8..0.8),
                );
                // All start moving in roughly the same direction
                let vel = Vec3::new(0.3, 0.1, 0.2).normalize() * 0.5
                    + Vec3::new(
                        rng.gen_range(-0.1..0.1),
                        rng.gen_range(-0.1..0.1),
                        rng.gen_range(-0.1..0.1),
                    );
                let color = Vec3::new(0.2, 0.2, 0.3);
                (pos, vel, color, 0.0f32)
            }
        })
        .collect();

    Simulation::<Bird>::new()
        .with_particle_count(8_000)
        .with_bounds(2.0) // Larger bounds
        .with_spawner(move |i, _| {
            let (pos, vel, color, is_predator) = particles[i as usize];
            Bird { position: pos, velocity: vel, color, is_predator }
        })
        .with_uniform::<f32>("separation", 2.0)
        .with_uniform::<f32>("alignment", 1.0)
        .with_uniform::<f32>("cohesion", 1.0)
        .with_uniform::<f32>("max_speed", 1.2)
        .with_uniform::<f32>("predator_fear", 4.0)
        .with_uniform::<f32>("speed_mult", 1.0)

        .with_ui(move |ctx| {
            let mut s = ui_state.lock().unwrap();
            egui::Window::new("Murmuration")
                .default_pos([10.0, 10.0])
                .show(ctx, |ui| {
                    ui.heading("Flocking");
                    ui.add(egui::Slider::new(&mut s.separation, 0.0..=5.0).text("Separation"));
                    ui.add(egui::Slider::new(&mut s.alignment, 0.0..=3.0).text("Alignment"));
                    ui.add(egui::Slider::new(&mut s.cohesion, 0.0..=3.0).text("Cohesion"));
                    ui.separator();
                    ui.add(egui::Slider::new(&mut s.max_speed, 0.5..=3.0).text("Max Speed"));
                    ui.add(egui::Slider::new(&mut s.predator_fear, 0.0..=8.0).text("Predator Fear"));
                    ui.separator();
                    ui.add(egui::Slider::new(&mut s.time_scale, 0.1..=2.0).text("Time Scale"));
                    if ui.button("Reset").clicked() {
                        *s = FlockState::default();
                    }
                });
        })
        .with_update(move |ctx| {
            let s = update_state.lock().unwrap();
            ctx.set("separation", s.separation);
            ctx.set("alignment", s.alignment);
            ctx.set("cohesion", s.cohesion);
            ctx.set("max_speed", s.max_speed);
            ctx.set("predator_fear", s.predator_fear);
            ctx.set("speed_mult", s.time_scale);
        })

        .with_spatial_config(0.5, 32)  // Larger cells for predator detection

        // All boids logic in one neighbor rule for simplicity
        .with_rule(Rule::NeighborCustom(r#"
            let perception = 0.2;
            let sep_dist = 0.08;
            let pred_radius = 0.6;

            // Only process if this particle is NOT the predator
            if p.is_predator < 0.5 {
                // Check if neighbor is the predator
                if other.is_predator > 0.5 {
                    // FLEE from predator!
                    if neighbor_dist < pred_radius && neighbor_dist > 0.001 {
                        let urgency = 1.0 - (neighbor_dist / pred_radius);
                        // neighbor_dir points FROM other TO self, so add it to flee
                        p.velocity += neighbor_dir * urgency * urgency * uniforms.predator_fear * 0.15;
                    }
                } else if neighbor_dist < perception && neighbor_dist > 0.001 {
                    // Normal bird-to-bird interactions

                    // SEPARATION - avoid crowding
                    if neighbor_dist < sep_dist {
                        let urgency = (sep_dist - neighbor_dist) / sep_dist;
                        p.velocity += neighbor_dir * urgency * uniforms.separation * 0.1;
                    }

                    // ALIGNMENT - match velocity
                    p.velocity += (other.velocity - p.velocity) * uniforms.alignment * 0.01;

                    // COHESION - move toward neighbors
                    p.velocity -= neighbor_dir * uniforms.cohesion * 0.01;
                }
            }
        "#.into()))

        // Main update: predator orbit, bird movement, bounds
        .with_rule(Rule::Custom(r#"
            let dt = uniforms.delta_time * uniforms.speed_mult;
            let t = uniforms.time;

            // === PREDATOR BEHAVIOR ===
            if p.is_predator > 0.5 {
                // Fast circular orbit that sweeps through the flock
                let orbit_speed = t * 1.2;
                let wobble = sin(t * 1.7) * 0.4;
                p.position = vec3<f32>(
                    cos(orbit_speed) * (1.0 + wobble),
                    sin(orbit_speed * 0.6) * 0.5,
                    sin(orbit_speed) * (1.0 + wobble)
                );
                // Keep bright red color
                p.color = vec3<f32>(1.0, 0.2, 0.1);
            } else {
                // === BIRD BEHAVIOR ===

                // === VERY GENTLE CENTER PULL ===
                let to_center = -p.position;
                let center_dist = length(to_center);
                if center_dist > 1.2 {
                    p.velocity += normalize(to_center) * (center_dist - 1.2) * 0.3 * dt;
                }

                // === SPEED LIMITS ===
                let speed = length(p.velocity);
                let min_speed = uniforms.max_speed * 0.4;

                if speed > uniforms.max_speed {
                    p.velocity = normalize(p.velocity) * uniforms.max_speed;
                } else if speed < min_speed && speed > 0.001 {
                    p.velocity = normalize(p.velocity) * min_speed;
                }

                // === COLOR ===
                let spd = length(p.velocity) / uniforms.max_speed;
                p.color = mix(
                    vec3<f32>(0.1, 0.1, 0.15),
                    vec3<f32>(0.8, 0.85, 0.9),
                    spd * 0.5 + 0.2
                );

                // === INTEGRATE ===
                p.position += p.velocity * dt;

                // === SOFT BOUNDS ===
                let bound = 1.5;
                if abs(p.position.x) > bound { p.velocity.x -= sign(p.position.x) * 2.0 * dt; }
                if abs(p.position.y) > bound { p.velocity.y -= sign(p.position.y) * 2.0 * dt; }
                if abs(p.position.z) > bound { p.velocity.z -= sign(p.position.z) * 2.0 * dt; }
            }
        "#.into()))

        .run();
}
