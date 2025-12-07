//! # Fluid Galaxy
//!
//! A cosmic simulation where particles form a spiral galaxy through
//! fluid dynamics, differential rotation, and gravitational forces.
//!
//! Features:
//! - Central black hole gravity
//! - SPH-like pressure and viscosity
//! - Differential rotation (Keplerian-ish)
//! - Density waves that enhance spiral structure
//! - Cosmic color palette (deep space blues, hot stellar whites)
//! - Interactive egui controls
//!
//! Run with: `cargo run --example fluid_galaxy_interactive --features egui`

use rand::Rng;
use rdpe::prelude::*;
use std::sync::{Arc, Mutex};

#[derive(Particle, Clone)]
struct StarParticle {
    position: Vec3,
    velocity: Vec3,
    #[color]
    color: Vec3,
    /// Density estimate from neighbors
    density: f32,
    /// Temperature/energy for coloring
    temperature: f32,
}

/// Galaxy simulation parameters
struct GalaxyState {
    // Gravity
    central_mass: f32,
    softening: f32,

    // Fluid dynamics
    pressure: f32,
    viscosity: f32,
    rest_density: f32,

    // Spiral enhancement
    spiral_strength: f32,
    spiral_arms: f32,
    spiral_twist: f32,

    // General
    rotation_boost: f32,
    speed: f32,
}

impl Default for GalaxyState {
    fn default() -> Self {
        Self {
            central_mass: 2.0,
            softening: 0.1,
            pressure: 0.3,
            viscosity: 0.1,
            rest_density: 1.0,
            spiral_strength: 0.15,
            spiral_arms: 2.0,
            spiral_twist: 3.0,
            rotation_boost: 1.0,
            speed: 1.0,
        }
    }
}

fn main() {
    let mut rng = rand::thread_rng();

    let state = Arc::new(Mutex::new(GalaxyState::default()));
    let ui_state = state.clone();
    let update_state = state.clone();

    // Generate galaxy disk distribution
    let particles: Vec<_> = (0..15_000)
        .map(|_| {
            // Exponential disk profile - more particles near center
            let r = rng.gen_range(0.0_f32..1.0).powf(0.5) * 0.85;
            let theta = rng.gen_range(0.0..std::f32::consts::TAU);

            // Flat disk with slight thickness
            let disk_height = 0.02 * (1.0 - r * 0.5); // Thinner at edges
            let z = rng.gen_range(-disk_height..disk_height);

            let pos = Vec3::new(r * theta.cos(), z, r * theta.sin());

            // Keplerian rotation: v = sqrt(GM/r), but simplified
            // Inner particles orbit faster
            let orbital_speed = if r > 0.05 {
                (0.8 / r.sqrt()).min(2.0)
            } else {
                0.5
            };

            // Tangential velocity (perpendicular to radius in xz plane)
            let vel = Vec3::new(
                -theta.sin() * orbital_speed * 0.3,
                rng.gen_range(-0.01..0.01),
                theta.cos() * orbital_speed * 0.3,
            );

            // Temperature based on radius (hotter near center)
            let temperature = (1.0 - r).powf(1.5) * 0.8 + 0.2;

            // Color: inner hot (white/blue) -> outer cool (blue/purple)
            let color = cosmic_color(r, temperature);

            (pos, vel, color, temperature)
        })
        .collect();

    Simulation::<StarParticle>::new()
        .with_particle_count(15_000)
        .with_bounds(1.0)
        .with_spawner(|ctx| {
            let (pos, vel, color, temperature) = particles[i as usize];
            StarParticle {
                position: pos,
                velocity: vel,
                color,
                density: 1.0,
                temperature,
            }
        })

        // Custom uniforms
        .with_uniform::<f32>("central_mass", 2.0)
        .with_uniform::<f32>("softening", 0.1)
        .with_uniform::<f32>("pressure", 0.3)
        .with_uniform::<f32>("viscosity", 0.1)
        .with_uniform::<f32>("rest_density", 1.0)
        .with_uniform::<f32>("spiral_strength", 0.15)
        .with_uniform::<f32>("spiral_arms", 2.0)
        .with_uniform::<f32>("spiral_twist", 3.0)
        .with_uniform::<f32>("rotation_boost", 1.0)
        .with_uniform::<f32>("speed_mult", 1.0)

        // UI Controls
        .with_ui(move |ctx| {
            let mut s = ui_state.lock().unwrap();

            egui::Window::new("Galaxy Controls")
                .default_pos([10.0, 10.0])
                .resizable(false)
                .show(ctx, |ui| {
                    ui.heading("Gravity");
                    ui.add(egui::Slider::new(&mut s.central_mass, 0.0..=5.0).text("Black Hole Mass"));
                    ui.add(egui::Slider::new(&mut s.softening, 0.01..=0.3).text("Softening"));

                    ui.separator();
                    ui.heading("Fluid Dynamics");
                    ui.add(egui::Slider::new(&mut s.pressure, 0.0..=1.0).text("Pressure"));
                    ui.add(egui::Slider::new(&mut s.viscosity, 0.0..=0.5).text("Viscosity"));
                    ui.add(egui::Slider::new(&mut s.rest_density, 0.5..=3.0).text("Rest Density"));

                    ui.separator();
                    ui.heading("Spiral Structure");
                    ui.add(egui::Slider::new(&mut s.spiral_strength, 0.0..=0.5).text("Spiral Force"));
                    ui.add(egui::Slider::new(&mut s.spiral_arms, 1.0..=6.0).text("Arm Count"));
                    ui.add(egui::Slider::new(&mut s.spiral_twist, 1.0..=8.0).text("Twist"));

                    ui.separator();
                    ui.add(egui::Slider::new(&mut s.rotation_boost, 0.5..=2.0).text("Rotation"));
                    ui.add(egui::Slider::new(&mut s.speed, 0.1..=3.0).text("Speed"));

                    ui.separator();
                    if ui.button("Reset").clicked() {
                        *s = GalaxyState::default();
                    }

                    ui.separator();
                    ui.label("View from above for best spiral view");
                });
        })

        // Sync state to uniforms
        .with_update(move |ctx| {
            let s = update_state.lock().unwrap();
            ctx.set("central_mass", s.central_mass);
            ctx.set("softening", s.softening);
            ctx.set("pressure", s.pressure);
            ctx.set("viscosity", s.viscosity);
            ctx.set("rest_density", s.rest_density);
            ctx.set("spiral_strength", s.spiral_strength);
            ctx.set("spiral_arms", s.spiral_arms);
            ctx.set("spiral_twist", s.spiral_twist);
            ctx.set("rotation_boost", s.rotation_boost);
            ctx.set("speed_mult", s.speed);
        })

        // Main galaxy dynamics
        .with_rule(Rule::Custom(r#"
            let dt = uniforms.delta_time * uniforms.speed_mult;
            let t = uniforms.time;

            // Position in xz plane (galaxy disk)
            let r = length(vec2<f32>(p.position.x, p.position.z));
            let theta = atan2(p.position.z, p.position.x);

            // === CENTRAL GRAVITY ===
            let to_center = -p.position;
            let dist = length(to_center);
            let grav_dir = to_center / max(dist, 0.001);

            // Softened gravity: F = GM / (r^2 + softening^2)
            let grav_strength = uniforms.central_mass / (dist * dist + uniforms.softening * uniforms.softening);
            p.velocity += grav_dir * grav_strength * dt;

            // === DIFFERENTIAL ROTATION MAINTENANCE ===
            // Add slight tangential boost to maintain rotation
            if r > 0.05 {
                let target_orbital = sqrt(uniforms.central_mass / (r + uniforms.softening)) * 0.5;
                let tangent = vec3<f32>(-sin(theta), 0.0, cos(theta));
                let current_tangent_vel = dot(p.velocity, tangent);
                let rotation_diff = target_orbital * uniforms.rotation_boost - current_tangent_vel;
                p.velocity += tangent * rotation_diff * dt * 0.5;
            }

            // === SPIRAL DENSITY WAVE ===
            // Creates spiral arm structure through periodic forcing
            let spiral_phase = uniforms.spiral_arms * (theta - uniforms.spiral_twist * r + t * 0.1);
            let spiral_force = sin(spiral_phase) * uniforms.spiral_strength;

            // Force pushes particles toward spiral arm positions
            let spiral_tangent = vec3<f32>(-sin(theta), 0.0, cos(theta));
            p.velocity += spiral_tangent * spiral_force * dt;

            // === DISK CONFINEMENT ===
            // Keep particles in disk plane
            p.velocity.y -= p.position.y * 2.0 * dt;
            p.velocity.y *= 0.95;

            // === TEMPERATURE/COLOR UPDATE ===
            let speed = length(p.velocity);
            p.temperature = mix(p.temperature, 0.3 + speed * 0.5 + (1.0 - min(r, 1.0)) * 0.5, 0.05);

            // Color: cool outer (purple/blue) to hot inner (cyan/white)
            let temp = clamp(p.temperature, 0.0, 1.5);
            if temp < 0.4 {
                // Cool: deep purple to blue
                p.color = mix(vec3<f32>(0.2, 0.1, 0.4), vec3<f32>(0.3, 0.4, 0.8), temp / 0.4);
            } else if temp < 0.8 {
                // Medium: blue to cyan
                p.color = mix(vec3<f32>(0.3, 0.4, 0.8), vec3<f32>(0.5, 0.8, 1.0), (temp - 0.4) / 0.4);
            } else {
                // Hot: cyan to white
                p.color = mix(vec3<f32>(0.5, 0.8, 1.0), vec3<f32>(1.0, 1.0, 1.0), (temp - 0.8) / 0.7);
            }

            // === DAMPING ===
            p.velocity *= 0.999;

            // === INTEGRATE ===
            p.position += p.velocity * dt;
        "#.into()))

        // Spatial hashing for fluid interactions
        .with_spatial_config(0.1, 32)

        // SPH-like neighbor interactions
        .with_rule(Rule::NeighborCustom(r#"
            let dt = uniforms.delta_time * uniforms.speed_mult;

            // SPH kernel weight (simple linear falloff)
            let h = 0.1; // smoothing radius
            if neighbor_dist < h && neighbor_dist > 0.001 {
                let q = neighbor_dist / h;
                let weight = 1.0 - q;

                // Accumulate density
                p.density += weight;

                // === PRESSURE FORCE ===
                // Pushes apart when density > rest density
                let pressure_self = max(p.density - uniforms.rest_density, 0.0) * uniforms.pressure;
                let pressure_other = max(other.density - uniforms.rest_density, 0.0) * uniforms.pressure;
                let pressure_force = (pressure_self + pressure_other) * 0.5;

                p.velocity -= neighbor_dir * pressure_force * weight * dt * 0.1;

                // === VISCOSITY ===
                // Smooths velocity differences
                let vel_diff = other.velocity - p.velocity;
                p.velocity += vel_diff * uniforms.viscosity * weight * dt;
            }
        "#.into()))

        // Reset density each frame (done via a simple custom rule)
        .with_rule(Rule::Custom(r#"
            // Decay density toward base (will be rebuilt by neighbor pass)
            p.density = max(p.density * 0.8, 0.5);
        "#.into()))

        .run();
}

/// Generate cosmic colors based on radius and temperature
fn cosmic_color(_r: f32, temperature: f32) -> Vec3 {
    let temp = temperature.clamp(0.0, 1.5);
    if temp < 0.4 {
        // Cool: deep purple to blue
        Vec3::new(0.2, 0.1, 0.4).lerp(Vec3::new(0.3, 0.4, 0.8), temp / 0.4)
    } else if temp < 0.8 {
        // Medium: blue to cyan
        Vec3::new(0.3, 0.4, 0.8).lerp(Vec3::new(0.5, 0.8, 1.0), (temp - 0.4) / 0.4)
    } else {
        // Hot: cyan to white
        Vec3::new(0.5, 0.8, 1.0).lerp(Vec3::new(1.0, 1.0, 1.0), (temp - 0.8) / 0.7)
    }
}
