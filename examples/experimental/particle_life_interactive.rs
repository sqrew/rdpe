//! # Interactive Particle Life
//!
//! Explore emergent behavior by adjusting attraction/repulsion between particle
//! types in real-time. Watch as simple rules create complex lifelike patterns.
//!
//! ## Controls
//!
//! - Adjust strength sliders: positive = attract, negative = repel
//! - Adjust radius sliders: interaction range
//! - Use presets for interesting starting configurations
//! - Randomize to discover new behaviors
//!
//! ## Emergent Patterns to Look For
//!
//! - **Clustering**: Same-type attraction creates groups
//! - **Chasing**: A attracts B, B repels A creates pursuit
//! - **Orbiting**: Balanced attraction/repulsion creates stable orbits
//! - **Oscillation**: Cyclic relationships create pulsing patterns
//!
//! Run with: `cargo run --example particle_life_interactive --features egui`

use rand::Rng;
use rdpe::prelude::*;
use std::sync::{Arc, Mutex};

const NUM_TYPES: usize = 4;
const TYPE_NAMES: [&str; NUM_TYPES] = ["Red", "Green", "Blue", "Yellow"];
const TYPE_COLORS: [Vec3; NUM_TYPES] = [
    Vec3::new(1.0, 0.3, 0.3),  // Red
    Vec3::new(0.3, 1.0, 0.3),  // Green
    Vec3::new(0.4, 0.4, 1.0),  // Blue
    Vec3::new(1.0, 1.0, 0.3),  // Yellow
];

#[derive(Particle, Clone)]
struct Cell {
    position: Vec3,
    velocity: Vec3,
    particle_type: u32,
    #[color]
    color: Vec3,
}

/// Interaction between two particle types
#[derive(Clone, Copy)]
struct Interaction {
    strength: f32,  // positive = attract, negative = repel
    radius: f32,
}

impl Default for Interaction {
    fn default() -> Self {
        Self {
            strength: 0.0,
            radius: 0.3,
        }
    }
}

/// Full interaction matrix state
struct ParticleLifeState {
    /// interactions[self_type][other_type]
    interactions: [[Interaction; NUM_TYPES]; NUM_TYPES],
    drag: f32,
    speed_limit: f32,
}

impl Default for ParticleLifeState {
    fn default() -> Self {
        let mut state = Self {
            interactions: [[Interaction::default(); NUM_TYPES]; NUM_TYPES],
            drag: 2.0,
            speed_limit: 1.5,
        };
        // Start with an interesting configuration
        state.apply_preset_ecosystem();
        state
    }
}

impl ParticleLifeState {
    fn apply_preset_ecosystem(&mut self) {
        // Red: predator, chases green
        self.interactions[0][0] = Interaction { strength: 0.5, radius: 0.3 };
        self.interactions[0][1] = Interaction { strength: 1.0, radius: 0.4 };
        self.interactions[0][2] = Interaction { strength: -0.3, radius: 0.2 };
        self.interactions[0][3] = Interaction { strength: 0.2, radius: 0.3 };

        // Green: prey, flees red, follows blue
        self.interactions[1][0] = Interaction { strength: -0.8, radius: 0.3 };
        self.interactions[1][1] = Interaction { strength: 0.3, radius: 0.25 };
        self.interactions[1][2] = Interaction { strength: 0.6, radius: 0.35 };
        self.interactions[1][3] = Interaction { strength: 0.1, radius: 0.2 };

        // Blue: curious, attracted to red, avoids green
        self.interactions[2][0] = Interaction { strength: 0.4, radius: 0.3 };
        self.interactions[2][1] = Interaction { strength: -0.5, radius: 0.25 };
        self.interactions[2][2] = Interaction { strength: 0.2, radius: 0.2 };
        self.interactions[2][3] = Interaction { strength: 0.3, radius: 0.3 };

        // Yellow: social butterfly, likes everyone but itself
        self.interactions[3][0] = Interaction { strength: 0.3, radius: 0.4 };
        self.interactions[3][1] = Interaction { strength: 0.3, radius: 0.4 };
        self.interactions[3][2] = Interaction { strength: 0.3, radius: 0.4 };
        self.interactions[3][3] = Interaction { strength: -0.8, radius: 0.3 };
    }

    fn apply_preset_symmetric(&mut self) {
        // All types attract themselves, repel others
        for i in 0..NUM_TYPES {
            for j in 0..NUM_TYPES {
                if i == j {
                    self.interactions[i][j] = Interaction { strength: 0.6, radius: 0.3 };
                } else {
                    self.interactions[i][j] = Interaction { strength: -0.4, radius: 0.25 };
                }
            }
        }
    }

    fn apply_preset_chaos(&mut self) {
        // Everyone attracted to everyone
        for i in 0..NUM_TYPES {
            for j in 0..NUM_TYPES {
                self.interactions[i][j] = Interaction { strength: 0.5, radius: 0.35 };
            }
        }
    }

    fn apply_preset_chase(&mut self) {
        // Circular chase: R->G->B->Y->R
        self.clear();
        self.interactions[0][1] = Interaction { strength: 1.0, radius: 0.5 };  // R chases G
        self.interactions[1][2] = Interaction { strength: 1.0, radius: 0.5 };  // G chases B
        self.interactions[2][3] = Interaction { strength: 1.0, radius: 0.5 };  // B chases Y
        self.interactions[3][0] = Interaction { strength: 1.0, radius: 0.5 };  // Y chases R

        // Everyone flees their chaser
        self.interactions[1][0] = Interaction { strength: -0.8, radius: 0.3 };
        self.interactions[2][1] = Interaction { strength: -0.8, radius: 0.3 };
        self.interactions[3][2] = Interaction { strength: -0.8, radius: 0.3 };
        self.interactions[0][3] = Interaction { strength: -0.8, radius: 0.3 };
    }

    fn apply_preset_orbits(&mut self) {
        // Create orbital patterns
        self.clear();
        for i in 0..NUM_TYPES {
            // Attract same type weakly
            self.interactions[i][i] = Interaction { strength: 0.3, radius: 0.4 };
            // Attract next type strongly
            self.interactions[i][(i + 1) % NUM_TYPES] = Interaction { strength: 0.8, radius: 0.5 };
            // Repel at close range
            self.interactions[i][(i + 2) % NUM_TYPES] = Interaction { strength: -0.6, radius: 0.15 };
        }
    }

    fn clear(&mut self) {
        for i in 0..NUM_TYPES {
            for j in 0..NUM_TYPES {
                self.interactions[i][j] = Interaction { strength: 0.0, radius: 0.3 };
            }
        }
    }

    fn randomize(&mut self) {
        let mut rng = rand::thread_rng();
        for i in 0..NUM_TYPES {
            for j in 0..NUM_TYPES {
                self.interactions[i][j] = Interaction {
                    strength: rng.gen_range(-1.0..1.0),
                    radius: rng.gen_range(0.1..0.5),
                };
            }
        }
    }
}

fn main() {
    let mut rng = rand::thread_rng();

    let state = Arc::new(Mutex::new(ParticleLifeState::default()));
    let ui_state = state.clone();
    let update_state = state.clone();

    // Pre-generate particles
    let particles: Vec<Cell> = (0..6_000)
        .map(|i| {
            let type_idx = (i % NUM_TYPES as u32) as usize;
            Cell {
                position: Vec3::new(
                    rng.gen_range(-1.0..1.0),
                    rng.gen_range(-1.0..1.0),
                    rng.gen_range(-1.0..1.0),
                ),
                velocity: Vec3::ZERO,
                particle_type: type_idx as u32,
                color: TYPE_COLORS[type_idx],
            }
        })
        .collect();

    // Build uniform names for all interactions
    let mut sim = Simulation::<Cell>::new()
        .with_particle_count(6_000)
        .with_bounds(1.5)
        .with_spawner(|ctx| particles[ctx.index as usize].clone());

    // Register uniforms for each interaction pair
    for i in 0..NUM_TYPES {
        for j in 0..NUM_TYPES {
            sim = sim
                .with_uniform::<f32>(&format!("str_{i}_{j}"), 0.0)
                .with_uniform::<f32>(&format!("rad_{i}_{j}"), 0.3);
        }
    }

    // Also register drag and speed limit
    sim = sim
        .with_uniform::<f32>("drag", 2.0)
        .with_uniform::<f32>("speed_limit", 1.5);

    sim
        // UI controls
        .with_ui(move |ctx| {
            let mut s = ui_state.lock().unwrap();

            egui::Window::new("Particle Life")
                .default_pos([10.0, 10.0])
                .default_width(320.0)
                .show(ctx, |ui| {
                    // Presets
                    ui.heading("Presets");
                    ui.horizontal(|ui| {
                        if ui.button("Ecosystem").clicked() {
                            s.apply_preset_ecosystem();
                        }
                        if ui.button("Symmetric").clicked() {
                            s.apply_preset_symmetric();
                        }
                        if ui.button("Chaos").clicked() {
                            s.apply_preset_chaos();
                        }
                    });
                    ui.horizontal(|ui| {
                        if ui.button("Chase").clicked() {
                            s.apply_preset_chase();
                        }
                        if ui.button("Orbits").clicked() {
                            s.apply_preset_orbits();
                        }
                        if ui.button("Random").clicked() {
                            s.randomize();
                        }
                    });
                    if ui.button("Clear All").clicked() {
                        s.clear();
                    }

                    ui.separator();
                    ui.heading("Physics");
                    ui.add(egui::Slider::new(&mut s.drag, 0.5..=5.0).text("Drag"));
                    ui.add(egui::Slider::new(&mut s.speed_limit, 0.5..=3.0).text("Speed Limit"));

                    ui.separator();
                    ui.heading("Interaction Matrix");
                    ui.label("Strength: + attract, - repel");

                    // Compact matrix display
                    egui::ScrollArea::vertical()
                        .max_height(400.0)
                        .show(ui, |ui| {
                            for i in 0..NUM_TYPES {
                                ui.collapsing(format!("{} reacts to...", TYPE_NAMES[i]), |ui| {
                                    for j in 0..NUM_TYPES {
                                        ui.horizontal(|ui| {
                                            ui.label(format!("{}:", TYPE_NAMES[j]));
                                        });
                                        ui.horizontal(|ui| {
                                            ui.add(egui::Slider::new(
                                                &mut s.interactions[i][j].strength,
                                                -1.5..=1.5,
                                            ).text("str").step_by(0.05));
                                        });
                                        ui.horizontal(|ui| {
                                            ui.add(egui::Slider::new(
                                                &mut s.interactions[i][j].radius,
                                                0.05..=0.6,
                                            ).text("rad").step_by(0.01));
                                        });
                                        ui.add_space(4.0);
                                    }
                                });
                            }
                        });
                });
        })

        // Sync state to uniforms
        .with_update(move |ctx| {
            let s = update_state.lock().unwrap();

            for i in 0..NUM_TYPES {
                for j in 0..NUM_TYPES {
                    ctx.set(&format!("str_{i}_{j}"), s.interactions[i][j].strength);
                    ctx.set(&format!("rad_{i}_{j}"), s.interactions[i][j].radius);
                }
            }
            ctx.set("drag", s.drag);
            ctx.set("speed_limit", s.speed_limit);
        })

        // Custom neighbor rule that reads from uniforms
        .with_spatial_config(0.6, 32)
        .with_rule(Rule::NeighborCustom(r#"
            // Get interaction parameters based on particle types
            let self_type = p.particle_type;
            let other_type = other.particle_type;

            // Look up strength and radius for this pair
            var strength = 0.0;
            var radius = 0.3;

            // Type 0 (Red) interactions
            if self_type == 0u {
                if other_type == 0u { strength = uniforms.str_0_0; radius = uniforms.rad_0_0; }
                else if other_type == 1u { strength = uniforms.str_0_1; radius = uniforms.rad_0_1; }
                else if other_type == 2u { strength = uniforms.str_0_2; radius = uniforms.rad_0_2; }
                else if other_type == 3u { strength = uniforms.str_0_3; radius = uniforms.rad_0_3; }
            }
            // Type 1 (Green) interactions
            else if self_type == 1u {
                if other_type == 0u { strength = uniforms.str_1_0; radius = uniforms.rad_1_0; }
                else if other_type == 1u { strength = uniforms.str_1_1; radius = uniforms.rad_1_1; }
                else if other_type == 2u { strength = uniforms.str_1_2; radius = uniforms.rad_1_2; }
                else if other_type == 3u { strength = uniforms.str_1_3; radius = uniforms.rad_1_3; }
            }
            // Type 2 (Blue) interactions
            else if self_type == 2u {
                if other_type == 0u { strength = uniforms.str_2_0; radius = uniforms.rad_2_0; }
                else if other_type == 1u { strength = uniforms.str_2_1; radius = uniforms.rad_2_1; }
                else if other_type == 2u { strength = uniforms.str_2_2; radius = uniforms.rad_2_2; }
                else if other_type == 3u { strength = uniforms.str_2_3; radius = uniforms.rad_2_3; }
            }
            // Type 3 (Yellow) interactions
            else if self_type == 3u {
                if other_type == 0u { strength = uniforms.str_3_0; radius = uniforms.rad_3_0; }
                else if other_type == 1u { strength = uniforms.str_3_1; radius = uniforms.rad_3_1; }
                else if other_type == 2u { strength = uniforms.str_3_2; radius = uniforms.rad_3_2; }
                else if other_type == 3u { strength = uniforms.str_3_3; radius = uniforms.rad_3_3; }
            }

            // Apply interaction force
            if radius > 0.0 && neighbor_dist < radius && neighbor_dist > 0.001 {
                let falloff = 1.0 - (neighbor_dist / radius);
                let force_mag = strength * falloff * falloff;
                p.velocity += neighbor_dir * force_mag * uniforms.delta_time;
            }
        "#.into()))

        // Apply drag and speed limit via custom rule using uniforms
        .with_rule(Rule::Custom(r#"
            // Apply drag
            p.velocity *= 1.0 - uniforms.drag * uniforms.delta_time;

            // Speed limit
            let speed = length(p.velocity);
            if speed > uniforms.speed_limit {
                p.velocity = normalize(p.velocity) * uniforms.speed_limit;
            }
        "#.into()))

        .with_rule(Rule::WrapWalls)
        .with_particle_size(0.012)
        .with_visuals(|v| {
            v.background(Vec3::new(0.02, 0.02, 0.04));
        })
        .run();
}
