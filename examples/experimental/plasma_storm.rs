//! # Plasma Storm
//!
//! A dynamic electromagnetic plasma simulation with real-time controls.
//! Charged particles swirl in turbulent electromagnetic fields, forming
//! beautiful vortices and filaments.
//!
//! Features:
//! - Dual charge types with electromagnetic attraction/repulsion
//! - Turbulent noise-driven motion
//! - Energy-based coloring (cool blues to hot whites)
//! - Interactive egui controls
//!
//! Run with: `cargo run --example plasma_storm --features egui`

use rand::Rng;
use rdpe::prelude::*;
use std::sync::{Arc, Mutex};

#[derive(Particle, Clone)]
struct PlasmaParticle {
    position: Vec3,
    velocity: Vec3,
    #[color]
    color: Vec3,
    /// Charge: positive (1.0) or negative (-1.0)
    charge: f32,
    /// Energy level affects color intensity
    energy: f32,
}

/// Shared simulation parameters
struct PlasmaState {
    // Electromagnetic parameters
    em_strength: f32,
    em_range: f32,

    // Turbulence
    turbulence: f32,
    turbulence_scale: f32,

    // Confinement (keeps plasma contained)
    confinement: f32,

    // Energy dynamics
    energy_transfer: f32,
    cooling_rate: f32,

    // Visual
    speed_multiplier: f32,
}

impl Default for PlasmaState {
    fn default() -> Self {
        Self {
            em_strength: 0.8,
            em_range: 0.25,
            turbulence: 0.4,
            turbulence_scale: 3.0,
            confinement: 1.5,
            energy_transfer: 0.3,
            cooling_rate: 0.1,
            speed_multiplier: 1.0,
        }
    }
}

fn main() {
    let mut rng = rand::thread_rng();

    // Shared state for UI
    let state = Arc::new(Mutex::new(PlasmaState::default()));
    let ui_state = state.clone();
    let update_state = state.clone();

    // Generate initial particles in a spherical distribution
    let particles: Vec<_> = (0..12_000)
        .map(|_| {
            // Spherical distribution
            let theta = rng.gen_range(0.0..std::f32::consts::TAU);
            let phi = rng.gen_range(0.0..std::f32::consts::PI);
            let r = rng.gen_range(0.1..0.7_f32).powf(0.33); // Cube root for uniform volume

            let pos = Vec3::new(
                r * phi.sin() * theta.cos(),
                r * phi.sin() * theta.sin(),
                r * phi.cos(),
            );

            // Initial velocity - slight rotation
            let vel = Vec3::new(
                -pos.y * 0.3 + rng.gen_range(-0.05..0.05),
                pos.x * 0.3 + rng.gen_range(-0.05..0.05),
                rng.gen_range(-0.05..0.05),
            );

            // Random charge
            let charge = if rng.gen_bool(0.5) { 1.0 } else { -1.0 };

            // Initial energy based on distance from center (hotter core)
            let energy = 0.3 + (1.0 - r) * 0.7;

            // Color will be computed in shader based on energy
            let color = energy_to_color(energy, charge);

            (pos, vel, color, charge, energy)
        })
        .collect();

    Simulation::<PlasmaParticle>::new()
        .with_particle_count(12_000)
        .with_bounds(1.0)
        .with_spawner(move |i, _| {
            let (pos, vel, color, charge, energy) = particles[i as usize];
            PlasmaParticle {
                position: pos,
                velocity: vel,
                color,
                charge,
                energy,
            }
        })

        // Custom uniforms for interactive control
        .with_uniform::<f32>("em_strength", 0.8)
        .with_uniform::<f32>("em_range", 0.25)
        .with_uniform::<f32>("turbulence", 0.4)
        .with_uniform::<f32>("turbulence_scale", 3.0)
        .with_uniform::<f32>("confinement", 1.5)
        .with_uniform::<f32>("energy_transfer", 0.3)
        .with_uniform::<f32>("cooling_rate", 0.1)
        .with_uniform::<f32>("speed_mult", 1.0)

        // UI Controls
        .with_ui(move |ctx| {
            let mut s = ui_state.lock().unwrap();

            egui::Window::new("Plasma Storm Controls")
                .default_pos([10.0, 10.0])
                .resizable(false)
                .show(ctx, |ui| {
                    ui.heading("Electromagnetic");
                    ui.add(egui::Slider::new(&mut s.em_strength, 0.0..=3.0).text("EM Strength"));
                    ui.add(egui::Slider::new(&mut s.em_range, 0.05..=0.5).text("EM Range"));

                    ui.separator();
                    ui.heading("Turbulence");
                    ui.add(egui::Slider::new(&mut s.turbulence, 0.0..=2.0).text("Intensity"));
                    ui.add(egui::Slider::new(&mut s.turbulence_scale, 1.0..=10.0).text("Scale"));

                    ui.separator();
                    ui.heading("Plasma Dynamics");
                    ui.add(egui::Slider::new(&mut s.confinement, 0.0..=5.0).text("Confinement"));
                    ui.add(egui::Slider::new(&mut s.energy_transfer, 0.0..=1.0).text("Energy Transfer"));
                    ui.add(egui::Slider::new(&mut s.cooling_rate, 0.0..=0.5).text("Cooling Rate"));

                    ui.separator();
                    ui.add(egui::Slider::new(&mut s.speed_multiplier, 0.1..=3.0).text("Speed"));

                    ui.separator();
                    if ui.button("Reset").clicked() {
                        *s = PlasmaState::default();
                    }

                    ui.separator();
                    ui.label("Drag to orbit, scroll to zoom");
                });
        })

        // Sync UI state to uniforms
        .with_update(move |ctx| {
            let s = update_state.lock().unwrap();
            ctx.set("em_strength", s.em_strength);
            ctx.set("em_range", s.em_range);
            ctx.set("turbulence", s.turbulence);
            ctx.set("turbulence_scale", s.turbulence_scale);
            ctx.set("confinement", s.confinement);
            ctx.set("energy_transfer", s.energy_transfer);
            ctx.set("cooling_rate", s.cooling_rate);
            ctx.set("speed_mult", s.speed_multiplier);
        })

        // Main plasma dynamics rule
        .with_rule(Rule::Custom(r#"
            let dt = uniforms.delta_time * uniforms.speed_mult;
            let t = uniforms.time;

            // === TURBULENT NOISE FORCE ===
            // 3D simplex-like noise for turbulent motion
            let noise_pos = p.position * uniforms.turbulence_scale + vec3<f32>(t * 0.5, t * 0.3, t * 0.4);
            let turb = vec3<f32>(
                sin(noise_pos.x * 2.1 + noise_pos.y * 1.3) * cos(noise_pos.z * 1.7),
                sin(noise_pos.y * 2.3 + noise_pos.z * 1.1) * cos(noise_pos.x * 1.9),
                sin(noise_pos.z * 1.9 + noise_pos.x * 1.5) * cos(noise_pos.y * 2.1)
            );
            p.velocity += turb * uniforms.turbulence * dt;

            // === CONFINEMENT FORCE ===
            // Soft boundary that pushes particles back toward center
            let dist_from_center = length(p.position);
            if dist_from_center > 0.6 {
                let push_strength = (dist_from_center - 0.6) * uniforms.confinement;
                p.velocity -= normalize(p.position) * push_strength * dt;
            }

            // === ENERGY DYNAMICS ===
            // Energy based on velocity magnitude
            let speed = length(p.velocity);
            p.energy = mix(p.energy, speed * 2.0, 0.1);

            // Cooling
            p.energy = max(p.energy - uniforms.cooling_rate * dt, 0.1);

            // === COLOR UPDATE ===
            // Hot plasma: blue -> cyan -> white based on energy
            let e = clamp(p.energy, 0.0, 1.5);
            if p.charge > 0.0 {
                // Positive: blue to cyan to white
                if e < 0.5 {
                    p.color = mix(vec3<f32>(0.1, 0.2, 0.8), vec3<f32>(0.2, 0.7, 1.0), e * 2.0);
                } else {
                    p.color = mix(vec3<f32>(0.2, 0.7, 1.0), vec3<f32>(1.0, 1.0, 1.0), (e - 0.5) * 2.0);
                }
            } else {
                // Negative: purple to magenta to white
                if e < 0.5 {
                    p.color = mix(vec3<f32>(0.5, 0.1, 0.8), vec3<f32>(1.0, 0.3, 0.8), e * 2.0);
                } else {
                    p.color = mix(vec3<f32>(1.0, 0.3, 0.8), vec3<f32>(1.0, 1.0, 1.0), (e - 0.5) * 2.0);
                }
            }

            // === VELOCITY DAMPING ===
            p.velocity *= 0.995;

            // === INTEGRATE ===
            p.position += p.velocity * dt;
        "#.into()))

        // Enable spatial hashing for neighbor queries
        .with_spatial_config(0.25, 32)

        // Neighbor-based electromagnetic interactions
        .with_rule(Rule::NeighborCustom(r#"
            let dt = uniforms.delta_time * uniforms.speed_mult;

            // Electromagnetic force: like charges repel, opposite attract
            if neighbor_dist > 0.001 && neighbor_dist < uniforms.em_range {
                // Coulomb-like force (inverse square, but softened)
                let force_mag = uniforms.em_strength / (neighbor_dist * neighbor_dist + 0.01);

                // Charge interaction: same sign = repel (-), opposite = attract (+)
                let charge_factor = -p.charge * other.charge;

                p.velocity += neighbor_dir * force_mag * charge_factor * dt * 0.01;

                // Energy transfer between nearby particles
                let energy_diff = other.energy - p.energy;
                p.energy += energy_diff * uniforms.energy_transfer * dt * 0.1;
            }
        "#.into()))

        .with_rule(Rule::BounceWalls)

        .run();
}

/// Convert energy level to plasma color (for initial spawn)
fn energy_to_color(energy: f32, charge: f32) -> Vec3 {
    let e = energy.clamp(0.0, 1.5);
    if charge > 0.0 {
        // Positive: blue to cyan to white
        if e < 0.5 {
            Vec3::new(0.1, 0.2, 0.8).lerp(Vec3::new(0.2, 0.7, 1.0), e * 2.0)
        } else {
            Vec3::new(0.2, 0.7, 1.0).lerp(Vec3::new(1.0, 1.0, 1.0), (e - 0.5) * 2.0)
        }
    } else {
        // Negative: purple to magenta to white
        if e < 0.5 {
            Vec3::new(0.5, 0.1, 0.8).lerp(Vec3::new(1.0, 0.3, 0.8), e * 2.0)
        } else {
            Vec3::new(1.0, 0.3, 0.8).lerp(Vec3::new(1.0, 1.0, 1.0), (e - 0.5) * 2.0)
        }
    }
}
