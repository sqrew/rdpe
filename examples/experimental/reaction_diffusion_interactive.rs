//! # Reaction-Diffusion Particles
//!
//! Two particle types (Activator and Inhibitor) interact to create
//! emergent Turing patterns - spots, stripes, waves, and more.
//!
//! Run with: `cargo run --example reaction_diffusion_interactive --features egui`

use rand::Rng;
use rdpe::prelude::*;
use std::sync::{Arc, Mutex};

const PARTICLE_COUNT: u32 = 6000;

#[derive(Particle, Clone)]
struct Chemical {
    position: Vec3,
    velocity: Vec3,
    #[color]
    color: Vec3,
    particle_type: u32,  // 0 = Activator (U), 1 = Inhibitor (V)
    concentration: f32,
}

struct RDState {
    // Interaction matrix (2x2)
    uu: f32,  // U reacts to U
    uv: f32,  // U reacts to V
    vu: f32,  // V reacts to U
    vv: f32,  // V reacts to V
    // Dynamics
    separation: f32,
    speed: f32,
    drag: f32,
}

impl Default for RDState {
    fn default() -> Self {
        Self {
            uu: 0.5,   // U attracts U (clustering)
            uv: -0.8,  // U avoids V
            vu: 0.6,   // V chases U
            vv: 0.2,   // V weakly attracts V
            separation: 0.5,
            speed: 2.0,
            drag: 3.0,
        }
    }
}

impl RDState {
    fn preset_chase(&mut self) {
        // Classic predator-prey chase
        self.uu = 0.3;
        self.uv = -1.0;  // U flees V
        self.vu = 0.8;   // V chases U
        self.vv = -0.2;  // V spreads out
        self.separation = 0.4;
        self.speed = 2.5;
    }

    fn preset_clusters(&mut self) {
        // Both types cluster with own kind, avoid other
        self.uu = 0.8;
        self.uv = -0.6;
        self.vu = -0.6;
        self.vv = 0.8;
        self.separation = 0.6;
        self.speed = 1.5;
    }

    fn preset_orbits(&mut self) {
        // Orbital dynamics
        self.uu = 0.2;
        self.uv = 0.4;   // U attracted to V
        self.vu = 0.4;   // V attracted to U
        self.vv = 0.2;
        self.separation = 0.8;
        self.speed = 2.0;
    }

    fn preset_waves(&mut self) {
        // Wave-like patterns
        self.uu = 0.6;
        self.uv = -0.3;
        self.vu = 0.5;
        self.vv = -0.4;  // V repels V (spreads)
        self.separation = 0.3;
        self.speed = 3.0;
    }

    fn preset_segregation(&mut self) {
        // Both types repel each other, attract own kind
        self.uu = -0.3;  // U repels U
        self.uv = 0.7;   // U chases V away
        self.vu = 0.7;   // V chases U away
        self.vv = -0.3;  // V repels V
        self.separation = 0.5;
        self.speed = 2.0;
    }

    fn preset_symbiosis(&mut self) {
        // Both types cluster with own, coexist
        self.uu = 0.9;
        self.uv = -1.2;
        self.vu = -1.2;
        self.vv = 0.9;
        self.separation = 0.4;
        self.speed = 2.5;
    }
}

fn main() {
    let mut rng = rand::thread_rng();

    let particles: Vec<_> = (0..PARTICLE_COUNT)
        .map(|i| {
            // Uniform distribution in a square
            let x = rng.gen_range(-0.9..0.9);
            let z = rng.gen_range(-0.9..0.9);
            let y = rng.gen_range(-0.05..0.05);

            let particle_type = if i % 2 == 0 { 0u32 } else { 1u32 };

            (x, y, z, particle_type)
        })
        .collect();

    let state = Arc::new(Mutex::new(RDState::default()));
    let ui_state = state.clone();
    let update_state = state.clone();

    Simulation::<Chemical>::new()
        .with_particle_count(PARTICLE_COUNT)
        .with_bounds(2.0)
        .with_spawner(move |i, _| {
            let (x, y, z, ptype) = particles[i as usize];

            let color = if ptype == 0 {
                Vec3::new(0.3, 0.7, 1.0)  // Cyan for U
            } else {
                Vec3::new(1.0, 0.5, 0.2)  // Orange for V
            };

            Chemical {
                position: Vec3::new(x, y, z),
                velocity: Vec3::ZERO,
                color,
                particle_type: ptype,
                concentration: 0.0,
            }
        })
        .with_spatial_config(0.25, 64)
        .with_uniform("uu", 0.5f32)
        .with_uniform("uv", -0.8f32)
        .with_uniform("vu", 0.6f32)
        .with_uniform("vv", 0.2f32)
        .with_uniform("separation", 0.5f32)
        .with_uniform("speed", 2.0f32)
        .with_uniform("drag", 3.0f32)
        // Reset concentration
        .with_rule(Rule::Custom("p.concentration = 0.0;".into()))
        // Neighbor interactions
        .with_rule(Rule::NeighborCustom(r#"
            let interact_radius = 0.2;
            let sep_radius = 0.04;

            if neighbor_dist < interact_radius && neighbor_dist > 0.001 {
                // Separation force (always repel when too close)
                if neighbor_dist < sep_radius {
                    let sep_strength = (1.0 - neighbor_dist / sep_radius) * uniforms.separation * 5.0;
                    p.velocity -= neighbor_dir * sep_strength * uniforms.delta_time;
                }

                // Type-based interaction
                let falloff = 1.0 - neighbor_dist / interact_radius;
                var strength = 0.0;

                if p.particle_type == 0u {
                    if other.particle_type == 0u {
                        strength = uniforms.uu;
                    } else {
                        strength = uniforms.uv;
                    }
                } else {
                    if other.particle_type == 0u {
                        strength = uniforms.vu;
                    } else {
                        strength = uniforms.vv;
                    }
                }

                p.velocity += neighbor_dir * strength * falloff * uniforms.speed * uniforms.delta_time;

                // Track same-type neighbors for coloring
                if p.particle_type == other.particle_type {
                    p.concentration += falloff;
                }
            }
        "#.into()))
        // Boundary and damping
        .with_rule(Rule::Custom(r#"
            // Wrap around boundaries
            if p.position.x > 0.95 { p.position.x = -0.95; }
            if p.position.x < -0.95 { p.position.x = 0.95; }
            if p.position.z > 0.95 { p.position.z = -0.95; }
            if p.position.z < -0.95 { p.position.z = 0.95; }

            // Keep flat
            p.velocity.y -= p.position.y * 5.0 * uniforms.delta_time;
            p.position.y *= 0.95;

            // Drag
            p.velocity *= 1.0 - uniforms.drag * uniforms.delta_time;

            // Color based on concentration
            let conc = min(p.concentration / 8.0, 1.0);
            if p.particle_type == 0u {
                p.color = vec3<f32>(0.2 + conc * 0.3, 0.5 + conc * 0.4, 0.9 + conc * 0.1);
            } else {
                p.color = vec3<f32>(1.0, 0.35 + conc * 0.35, 0.15 + conc * 0.2);
            }
        "#.into()))
        .with_ui(move |ctx| {
            let mut s = ui_state.lock().unwrap();

            egui::Window::new("🧪 Reaction-Diffusion")
                .default_pos([10.0, 10.0])
                .show(ctx, |ui| {
                    ui.heading("Type Interactions");
                    ui.label("+ = attract, - = repel");

                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.label("U→U:");
                        ui.add(egui::Slider::new(&mut s.uu, -1.5..=1.5));
                    });
                    ui.horizontal(|ui| {
                        ui.label("U→V:");
                        ui.add(egui::Slider::new(&mut s.uv, -1.5..=1.5));
                    });
                    ui.horizontal(|ui| {
                        ui.label("V→U:");
                        ui.add(egui::Slider::new(&mut s.vu, -1.5..=1.5));
                    });
                    ui.horizontal(|ui| {
                        ui.label("V→V:");
                        ui.add(egui::Slider::new(&mut s.vv, -1.5..=1.5));
                    });

                    ui.add_space(8.0);
                    ui.separator();
                    ui.heading("Physics");

                    ui.add(egui::Slider::new(&mut s.separation, 0.0..=2.0).text("Separation"));
                    ui.add(egui::Slider::new(&mut s.speed, 0.5..=5.0).text("Speed"));
                    ui.add(egui::Slider::new(&mut s.drag, 0.5..=6.0).text("Drag"));

                    ui.add_space(12.0);
                    ui.separator();
                    ui.heading("Presets");

                    ui.horizontal_wrapped(|ui| {
                        if ui.button("Chase").clicked() { s.preset_chase(); }
                        if ui.button("Clusters").clicked() { s.preset_clusters(); }
                        if ui.button("Orbits").clicked() { s.preset_orbits(); }
                    });
                    ui.horizontal_wrapped(|ui| {
                        if ui.button("Waves").clicked() { s.preset_waves(); }
                        if ui.button("Symbiosis").clicked() { s.preset_symbiosis(); }
                        if ui.button("Segregation").clicked() { s.preset_segregation(); }
                    });

                    ui.add_space(8.0);
                    ui.label("Cyan = U, Orange = V");
                });
        })
        .with_update(move |ctx| {
            let s = update_state.lock().unwrap();
            ctx.set("uu", s.uu);
            ctx.set("uv", s.uv);
            ctx.set("vu", s.vu);
            ctx.set("vv", s.vv);
            ctx.set("separation", s.separation);
            ctx.set("speed", s.speed);
            ctx.set("drag", s.drag);
        })
        .with_visuals(|v| {
            v.background(Vec3::new(0.015, 0.015, 0.025));
        })
        .with_particle_size(0.01)
        .run();
}
