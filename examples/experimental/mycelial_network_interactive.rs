//! # Mycelial Network (Interactive)
//!
//! Interactive version of the mycelial network simulation with egui controls.
//! Tweak parameters in real-time to explore how fungal networks form and behave.
//!
//! ## Controls
//!
//! - **Hypha Movement**: Speed, meandering intensity
//! - **Chemotropism**: How strongly hyphae are attracted to nodes
//! - **Nutrients**: Absorption, decay, and transfer rates
//! - **Network Structure**: Separation and cohesion between hyphae
//! - **Physics**: Drag and speed limits
//!
//! ## Try This
//!
//! - Crank up chemotropism to watch hyphae swarm toward nodes
//! - Increase nutrient transfer for visible "pulses" through the network
//! - Reduce separation for tighter hyphal bundles
//! - Set meandering to 0 for straight-line growth
//!
//! Run with: `cargo run --example mycelial_network_interactive --features egui`

use rand::Rng;
use rdpe::prelude::*;
use std::sync::{Arc, Mutex};

#[derive(Clone, Copy, PartialEq)]
#[repr(u32)]
enum CellType {
    Node = 0,
    Hypha = 1,
}

impl From<CellType> for u32 {
    fn from(t: CellType) -> u32 {
        t as u32
    }
}

#[derive(Particle, Clone)]
struct Cell {
    position: Vec3,
    velocity: Vec3,
    particle_type: u32,
    #[color]
    color: Vec3,
    nutrients: f32,
    heading: f32,
}

/// Shared state for UI controls
struct SimState {
    // Hypha movement
    base_speed: f32,
    nutrient_speed_boost: f32,
    meander_intensity: f32,

    // Chemotropism
    chemo_range: f32,
    chemo_strength: f32,
    absorption_rate: f32,

    // Nutrients
    decay_rate: f32,
    transfer_rate: f32,

    // Network structure
    separation_radius: f32,
    separation_strength: f32,
    cohesion_radius: f32,
    cohesion_strength: f32,

    // Physics
    drag: f32,
    max_speed: f32,
}

impl Default for SimState {
    fn default() -> Self {
        Self {
            base_speed: 0.06,
            nutrient_speed_boost: 0.03,
            meander_intensity: 1.2,

            chemo_range: 0.4,
            chemo_strength: 0.5,
            absorption_rate: 0.015,

            decay_rate: 0.003,
            transfer_rate: 0.03,

            separation_radius: 0.02,
            separation_strength: 0.15,
            cohesion_radius: 0.08,
            cohesion_strength: 0.05,

            drag: 3.0,
            max_speed: 0.12,
        }
    }
}

fn main() {
    let mut rng = rand::thread_rng();

    // Shared state
    let state = Arc::new(Mutex::new(SimState::default()));
    let ui_state = state.clone();
    let update_state = state.clone();

    let num_nodes = 10;
    let num_hyphae = 3000;
    let total = num_nodes + num_hyphae;

    let node_positions: Vec<Vec3> = (0..num_nodes)
        .map(|i| {
            let angle =
                (i as f32 / num_nodes as f32) * std::f32::consts::TAU + rng.gen_range(-0.2..0.2);
            let r = 0.25 + rng.gen_range(0.0..0.55);
            Vec3::new(angle.cos() * r, 0.0, angle.sin() * r)
        })
        .collect();

    let node_positions_clone = node_positions.clone();

    let particles: Vec<Cell> = (0..total)
        .map(|i| {
            if i < num_nodes {
                Cell {
                    position: node_positions[i],
                    velocity: Vec3::ZERO,
                    particle_type: CellType::Node.into(),
                    color: Vec3::new(0.5, 0.3, 0.15),
                    nutrients: 1.0,
                    heading: 0.0,
                }
            } else {
                let source_node = &node_positions_clone[rng.gen_range(0..num_nodes)];
                let offset_angle = rng.gen_range(0.0..std::f32::consts::TAU);
                let offset_r = rng.gen_range(0.02..0.08);

                Cell {
                    position: *source_node
                        + Vec3::new(
                            offset_angle.cos() * offset_r,
                            0.0,
                            offset_angle.sin() * offset_r,
                        ),
                    velocity: Vec3::ZERO,
                    particle_type: CellType::Hypha.into(),
                    color: Vec3::new(0.85, 0.8, 0.65),
                    nutrients: 0.4,
                    heading: offset_angle,
                }
            }
        })
        .collect();

    Simulation::<Cell>::new()
        .with_particle_count(total as u32)
        .with_particle_size(0.01)
        .with_bounds(1.3)
        .with_spawner(move |ctx| particles[ctx.index as usize].clone())
        .with_spatial_config(0.12, 32)
        .with_inbox()
        // Register all uniforms with defaults
        .with_uniform::<f32>("base_speed", 0.06)
        .with_uniform::<f32>("nutrient_speed_boost", 0.03)
        .with_uniform::<f32>("meander_intensity", 1.2)
        .with_uniform::<f32>("chemo_range", 0.4)
        .with_uniform::<f32>("chemo_strength", 0.5)
        .with_uniform::<f32>("absorption_rate", 0.015)
        .with_uniform::<f32>("decay_rate", 0.003)
        .with_uniform::<f32>("transfer_rate", 0.03)
        .with_uniform::<f32>("separation_radius", 0.02)
        .with_uniform::<f32>("separation_strength", 0.15)
        .with_uniform::<f32>("cohesion_radius", 0.08)
        .with_uniform::<f32>("cohesion_strength", 0.05)
        .with_uniform::<f32>("drag", 3.0)
        .with_uniform::<f32>("max_speed", 0.12)
        .with_visuals(|v| {
            v.trails(12);
            v.connections(0.055);
            v.background(Vec3::new(0.012, 0.008, 0.004));
        })
        // === UI ===
        .with_ui(move |ctx| {
            let mut s = ui_state.lock().unwrap();

            egui::Window::new("Mycelial Network")
                .default_pos([10.0, 10.0])
                .default_width(280.0)
                .show(ctx, |ui| {
                    ui.heading("Hypha Movement");
                    ui.add(
                        egui::Slider::new(&mut s.base_speed, 0.0..=0.2).text("Base Speed"),
                    );
                    ui.add(
                        egui::Slider::new(&mut s.nutrient_speed_boost, 0.0..=0.1)
                            .text("Nutrient Boost"),
                    );
                    ui.add(
                        egui::Slider::new(&mut s.meander_intensity, 0.0..=3.0)
                            .text("Meandering"),
                    );

                    ui.separator();
                    ui.heading("Chemotropism");
                    ui.add(
                        egui::Slider::new(&mut s.chemo_range, 0.1..=1.0).text("Sensing Range"),
                    );
                    ui.add(
                        egui::Slider::new(&mut s.chemo_strength, 0.0..=2.0)
                            .text("Attraction Strength"),
                    );
                    ui.add(
                        egui::Slider::new(&mut s.absorption_rate, 0.0..=0.1)
                            .text("Absorption Rate"),
                    );

                    ui.separator();
                    ui.heading("Nutrient Flow");
                    ui.add(
                        egui::Slider::new(&mut s.decay_rate, 0.0..=0.02).text("Decay Rate"),
                    );
                    ui.add(
                        egui::Slider::new(&mut s.transfer_rate, 0.0..=0.15)
                            .text("Transfer Rate"),
                    );

                    ui.separator();
                    ui.heading("Network Structure");
                    ui.add(
                        egui::Slider::new(&mut s.separation_radius, 0.005..=0.1)
                            .text("Separation Radius"),
                    );
                    ui.add(
                        egui::Slider::new(&mut s.separation_strength, 0.0..=1.0)
                            .text("Separation Strength"),
                    );
                    ui.add(
                        egui::Slider::new(&mut s.cohesion_radius, 0.02..=0.2)
                            .text("Cohesion Radius"),
                    );
                    ui.add(
                        egui::Slider::new(&mut s.cohesion_strength, 0.0..=0.3)
                            .text("Cohesion Strength"),
                    );

                    ui.separator();
                    ui.heading("Physics");
                    ui.add(egui::Slider::new(&mut s.drag, 0.0..=10.0).text("Drag"));
                    ui.add(
                        egui::Slider::new(&mut s.max_speed, 0.01..=0.5).text("Max Speed"),
                    );

                    ui.separator();
                    if ui.button("Reset to Defaults").clicked() {
                        *s = SimState::default();
                    }

                    ui.separator();
                    ui.label("Drag to rotate | Scroll to zoom");
                });
        })
        // === Sync state to uniforms ===
        .with_update(move |ctx| {
            let s = update_state.lock().unwrap();
            ctx.set("base_speed", s.base_speed);
            ctx.set("nutrient_speed_boost", s.nutrient_speed_boost);
            ctx.set("meander_intensity", s.meander_intensity);
            ctx.set("chemo_range", s.chemo_range);
            ctx.set("chemo_strength", s.chemo_strength);
            ctx.set("absorption_rate", s.absorption_rate);
            ctx.set("decay_rate", s.decay_rate);
            ctx.set("transfer_rate", s.transfer_rate);
            ctx.set("separation_radius", s.separation_radius);
            ctx.set("separation_strength", s.separation_strength);
            ctx.set("cohesion_radius", s.cohesion_radius);
            ctx.set("cohesion_strength", s.cohesion_strength);
            ctx.set("drag", s.drag);
            ctx.set("max_speed", s.max_speed);
        })
        // === NODE BEHAVIOR ===
        .with_rule(Rule::Custom(
            r#"
            if p.particle_type == 0u {
                p.velocity = vec3<f32>(0.0);
                p.nutrients = 0.7 + sin(uniforms.time * 0.4 + p.position.x * 3.0) * 0.3;
                p.color = vec3<f32>(0.35, 0.2, 0.08) + vec3<f32>(0.25, 0.15, 0.07) * p.nutrients;
            }
        "#
            .into(),
        ))
        // === HYPHA EXPLORATION ===
        .with_rule(Rule::Custom(
            r#"
            if p.particle_type == 1u {
                let speed = uniforms.base_speed + p.nutrients * uniforms.nutrient_speed_boost;

                let dir = vec3<f32>(cos(p.heading), 0.0, sin(p.heading));
                p.velocity = dir * speed;

                let noise1 = sin(uniforms.time * 1.5 + p.position.x * 12.0 + p.heading * 3.0);
                let noise2 = cos(uniforms.time * 1.1 + p.position.z * 10.0);
                let meander = (noise1 * 0.6 + noise2 * 0.4) * uniforms.delta_time * uniforms.meander_intensity;
                p.heading += meander;

                p.position.y = 0.0;

                let dist_from_center = length(p.position.xz);
                if dist_from_center > 0.85 {
                    let to_center = -normalize(p.position);
                    let target_heading = atan2(to_center.z, to_center.x);
                    let diff = atan2(sin(target_heading - p.heading), cos(target_heading - p.heading));
                    p.heading += diff * 0.1;
                }

                p.nutrients *= (1.0 - uniforms.decay_rate);
                p.nutrients = max(p.nutrients, 0.05);

                let t = p.nutrients;
                p.color = mix(
                    vec3<f32>(0.5, 0.45, 0.35),
                    vec3<f32>(0.55, 0.85, 0.4),
                    t
                );
            }
        "#
            .into(),
        ))
        // === CHEMOTROPISM ===
        .with_rule(Rule::NeighborCustom(
            r#"
            if p.particle_type == 1u && other.particle_type == 0u {
                if neighbor_dist < uniforms.chemo_range && neighbor_dist > 0.025 {
                    let to_node = -neighbor_dir;
                    let target_heading = atan2(to_node.z, to_node.x);
                    let heading_diff = atan2(
                        sin(target_heading - p.heading),
                        cos(target_heading - p.heading)
                    );

                    let hunger = 1.0 - p.nutrients;
                    let attraction = (uniforms.chemo_strength / (neighbor_dist + 0.1)) * (0.5 + hunger * 0.5);
                    p.heading += heading_diff * attraction * uniforms.delta_time;
                }

                if neighbor_dist < 0.045 {
                    p.nutrients = min(p.nutrients + uniforms.absorption_rate, 1.0);
                }
            }
        "#
            .into(),
        ))
        // === NUTRIENT TRANSPORT ===
        .with_rule(Rule::NeighborCustom(
            r#"
            if p.particle_type == 1u && other.particle_type == 1u {
                if neighbor_dist < 0.07 && neighbor_dist > 0.003 {
                    let gradient = other.nutrients - p.nutrients;
                    if gradient > 0.02 {
                        let transfer = gradient * uniforms.transfer_rate;
                        inbox_send(index, 0u, transfer);
                    }
                }
            }
        "#
            .into(),
        ))
        // Receive nutrient transfers
        .with_rule(Rule::Custom(
            r#"
            if p.particle_type == 1u {
                let received = inbox_receive_at(index, 0u);
                p.nutrients = clamp(p.nutrients + received, 0.0, 1.0);
            }
        "#
            .into(),
        ))
        // === NETWORK STRUCTURE ===
        // Dynamic separation using uniforms
        .with_rule(Rule::NeighborCustom(
            r#"
            if p.particle_type == 1u && other.particle_type == 1u {
                if neighbor_dist < uniforms.separation_radius && neighbor_dist > 0.001 {
                    p.velocity += neighbor_dir * uniforms.separation_strength * (uniforms.separation_radius - neighbor_dist);
                }
            }
        "#
            .into(),
        ))
        // Dynamic cohesion using uniforms
        .with_rule(Rule::NeighborCustom(
            r#"
            if p.particle_type == 1u && other.particle_type == 1u {
                if neighbor_dist < uniforms.cohesion_radius && neighbor_dist > uniforms.separation_radius {
                    p.velocity -= neighbor_dir * uniforms.cohesion_strength * uniforms.delta_time;
                }
            }
        "#
            .into(),
        ))
        // === PHYSICS ===
        .with_rule(Rule::Custom(
            r#"
            p.velocity *= (1.0 - uniforms.drag * uniforms.delta_time);
            let speed = length(p.velocity);
            if speed > uniforms.max_speed {
                p.velocity = normalize(p.velocity) * uniforms.max_speed;
            }
        "#
            .into(),
        ))
        .run().expect("Simulation failed");
}
