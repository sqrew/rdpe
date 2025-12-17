//! Spawn configuration panel

use crate::config::*;
use egui::Ui;

pub fn render_spawn_panel(ui: &mut Ui, config: &mut SimConfig) -> bool {
    let mut changed = false;

    ui.heading("Simulation");

    changed |= ui
        .add(
            egui::Slider::new(&mut config.particle_count, 100..=100_000)
                .text("Particles")
                .logarithmic(true),
        )
        .changed();

    changed |= ui
        .add(egui::Slider::new(&mut config.bounds, 0.1..=10.0).text("Bounds"))
        .changed();

    changed |= ui
        .add(
            egui::Slider::new(&mut config.particle_size, 0.0..=0.1)
                .text("Particle Size")
                .logarithmic(true),
        )
        .changed();

    // Speed slider - doesn't trigger rebuild, just changes simulation rate
    ui.add(egui::Slider::new(&mut config.speed, 0.01..=100.0).text("Speed"));

    ui.separator();
    ui.heading("Spawn Shape");

    let shapes = SpawnShape::variants();
    let mut shape_idx = match &config.spawn.shape {
        SpawnShape::Cube { .. } => 0,
        SpawnShape::Sphere { .. } => 1,
        SpawnShape::Shell { .. } => 2,
        SpawnShape::Ring { .. } => 3,
        SpawnShape::Point => 4,
        SpawnShape::Line { .. } => 5,
        SpawnShape::Plane { .. } => 6,
    };

    let shape_changed = ui
        .horizontal(|ui| {
            ui.label("Shape:");
            egui::ComboBox::from_id_salt("spawn_shape")
                .selected_text(shapes[shape_idx])
                .show_index(ui, &mut shape_idx, shapes.len(), |i| shapes[i])
                .changed()
        })
        .inner;

    if shape_changed {
        config.spawn.shape = match shape_idx {
            0 => SpawnShape::Cube { size: 0.5 },
            1 => SpawnShape::Sphere { radius: 0.5 },
            2 => SpawnShape::Shell {
                inner: 0.3,
                outer: 0.5,
            },
            3 => SpawnShape::Ring {
                radius: 0.5,
                thickness: 0.1,
            },
            4 => SpawnShape::Point,
            5 => SpawnShape::Line { length: 1.0 },
            6 => SpawnShape::Plane {
                width: 1.0,
                depth: 1.0,
            },
            _ => SpawnShape::Sphere { radius: 0.5 },
        };
        changed = true;
    }

    match &mut config.spawn.shape {
        SpawnShape::Cube { size } => {
            changed |= ui
                .add(egui::Slider::new(size, 0.1..=2.0).text("Size"))
                .changed();
        }
        SpawnShape::Sphere { radius } => {
            changed |= ui
                .add(egui::Slider::new(radius, 0.1..=2.0).text("Radius"))
                .changed();
        }
        SpawnShape::Shell { inner, outer } => {
            changed |= ui
                .add(egui::Slider::new(inner, 0.0..=1.9).text("Inner"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(outer, 0.1..=2.0).text("Outer"))
                .changed();
        }
        SpawnShape::Ring { radius, thickness } => {
            changed |= ui
                .add(egui::Slider::new(radius, 0.1..=2.0).text("Radius"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(thickness, 0.01..=0.5).text("Thickness"))
                .changed();
        }
        SpawnShape::Point => {}
        SpawnShape::Line { length } => {
            changed |= ui
                .add(egui::Slider::new(length, 0.1..=3.0).text("Length"))
                .changed();
        }
        SpawnShape::Plane { width, depth } => {
            changed |= ui
                .add(egui::Slider::new(width, 0.1..=3.0).text("Width"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(depth, 0.1..=3.0).text("Depth"))
                .changed();
        }
    }

    ui.separator();
    ui.heading("Initial Velocity");

    let vel_variants = InitialVelocity::variants();
    let mut vel_idx = match &config.spawn.velocity {
        InitialVelocity::Zero => 0,
        InitialVelocity::RandomDirection { .. } => 1,
        InitialVelocity::Outward { .. } => 2,
        InitialVelocity::Inward { .. } => 3,
        InitialVelocity::Swirl { .. } => 4,
        InitialVelocity::Directional { .. } => 5,
    };

    if egui::ComboBox::from_label("Mode")
        .show_index(ui, &mut vel_idx, vel_variants.len(), |i| vel_variants[i])
        .changed()
    {
        config.spawn.velocity = match vel_idx {
            0 => InitialVelocity::Zero,
            1 => InitialVelocity::RandomDirection { speed: 0.1 },
            2 => InitialVelocity::Outward { speed: 0.1 },
            3 => InitialVelocity::Inward { speed: 0.1 },
            4 => InitialVelocity::Swirl { speed: 0.1 },
            5 => InitialVelocity::Directional {
                direction: [1.0, 0.0, 0.0],
                speed: 0.1,
            },
            _ => InitialVelocity::Zero,
        };
        changed = true;
    }

    match &mut config.spawn.velocity {
        InitialVelocity::Zero => {}
        InitialVelocity::RandomDirection { speed }
        | InitialVelocity::Outward { speed }
        | InitialVelocity::Inward { speed }
        | InitialVelocity::Swirl { speed } => {
            changed |= ui
                .add(egui::Slider::new(speed, 0.0..=2.0).text("Speed"))
                .changed();
        }
        InitialVelocity::Directional { direction, speed } => {
            changed |= ui
                .add(egui::Slider::new(speed, 0.0..=2.0).text("Speed"))
                .changed();
            ui.horizontal(|ui| {
                ui.label("Dir:");
                changed |= ui
                    .add(
                        egui::DragValue::new(&mut direction[0])
                            .speed(0.01)
                            .prefix("x:"),
                    )
                    .changed();
                changed |= ui
                    .add(
                        egui::DragValue::new(&mut direction[1])
                            .speed(0.01)
                            .prefix("y:"),
                    )
                    .changed();
                changed |= ui
                    .add(
                        egui::DragValue::new(&mut direction[2])
                            .speed(0.01)
                            .prefix("z:"),
                    )
                    .changed();
            });
        }
    }

    ui.separator();
    ui.heading("Particle Types");

    // Ensure at least one type exists
    if config.spawn.type_weights.is_empty() {
        config.spawn.type_weights.push(1.0);
    }

    let total_weight: f32 = config.spawn.type_weights.iter().sum();
    let num_types = config.spawn.type_weights.len();

    // Show each type with weight slider
    let mut remove_idx: Option<usize> = None;
    for (i, weight) in config.spawn.type_weights.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            // Calculate percentage
            let pct = if total_weight > 0.0 {
                *weight / total_weight * 100.0
            } else {
                0.0
            };

            ui.label(format!("Type {}:", i));
            if ui
                .add(egui::DragValue::new(weight).speed(0.1).range(0.0..=100.0))
                .changed()
            {
                changed = true;
            }
            ui.label(format!("({:.0}%)", pct));

            // Remove button (only if more than 1 type)
            if num_types > 1 && ui.small_button("×").clicked() {
                remove_idx = Some(i);
            }
        });
    }

    // Remove type if requested
    if let Some(idx) = remove_idx {
        config.spawn.type_weights.remove(idx);
        changed = true;
    }

    // Add type button
    ui.horizontal(|ui| {
        if ui.button("+ Add Type").clicked() {
            // New type gets same weight as average
            let avg = total_weight / num_types as f32;
            config.spawn.type_weights.push(avg.max(1.0));
            changed = true;
        }

        // Quick presets
        ui.menu_button("Presets", |ui| {
            if ui.button("50/50 (2 types)").clicked() {
                config.spawn.type_weights = vec![1.0, 1.0];
                changed = true;
                ui.close_menu();
            }
            if ui.button("Predator/Prey (20/80)").clicked() {
                config.spawn.type_weights = vec![4.0, 1.0]; // 80% type 0, 20% type 1
                changed = true;
                ui.close_menu();
            }
            if ui.button("DLA (1/99 seed/mobile)").clicked() {
                config.spawn.type_weights = vec![1.0, 99.0]; // 1% seeds, 99% mobile
                changed = true;
                ui.close_menu();
            }
            if ui.button("3 Types Equal").clicked() {
                config.spawn.type_weights = vec![1.0, 1.0, 1.0];
                changed = true;
                ui.close_menu();
            }
            if ui.button("Reset (All Type 0)").clicked() {
                config.spawn.type_weights = vec![1.0];
                changed = true;
                ui.close_menu();
            }
        });
    });

    ui.separator();
    ui.heading("Adjacency Buffer");

    let needs_adjacency = config.needs_adjacency();

    // Auto-enable adjacency if required by rules
    if needs_adjacency && !config.adjacency_enabled {
        config.adjacency_enabled = true;
        changed = true;
    }

    ui.horizontal(|ui| {
        let checkbox = ui.checkbox(&mut config.adjacency_enabled, "Enable Adjacency");
        if needs_adjacency {
            ui.label("(required by rules)");
        }
        if checkbox.on_hover_text("Pre-compute neighbor indices for graph-based operations").changed() {
            changed = true;
        }
    });

    if config.adjacency_enabled {
        changed |= ui
            .add(
                egui::Slider::new(&mut config.adjacency_max_neighbors, 8..=64)
                    .text("Max Neighbors"),
            )
            .on_hover_text("Maximum neighbors to store per particle")
            .changed();

        changed |= ui
            .add(
                egui::Slider::new(&mut config.adjacency_radius, 0.01..=0.5)
                    .text("Radius"),
            )
            .on_hover_text("Distance threshold for neighbor detection")
            .changed();
    }

    ui.separator();
    ui.heading("Custom Spawn");

    // Checkbox to enable/disable custom spawn
    let has_custom_spawn = config.spawn.custom_spawn.is_some();
    let mut enable_custom = has_custom_spawn;
    if ui.checkbox(&mut enable_custom, "Custom Spawn Shader")
        .on_hover_text("Replace default spawn logic with custom WGSL code")
        .changed()
    {
        if enable_custom && config.spawn.custom_spawn.is_none() {
            // Enable with default template
            config.spawn.custom_spawn = Some(DEFAULT_CUSTOM_SPAWN.to_string());
            changed = true;
        } else if !enable_custom && config.spawn.custom_spawn.is_some() {
            config.spawn.custom_spawn = None;
            changed = true;
        }
    }

    // Code editor if custom spawn is enabled
    if let Some(ref mut code) = config.spawn.custom_spawn {
        ui.label("Set position, velocity, color, scale for each particle:");
        ui.label("Available: index, num_particles, bounds, t, seed, p, rand()");

        // Code editor
        egui::ScrollArea::vertical()
            .id_salt("custom_spawn_code")
            .max_height(200.0)
            .show(ui, |ui| {
                if ui.add(
                    egui::TextEdit::multiline(code)
                        .code_editor()
                        .desired_width(f32::INFINITY)
                        .desired_rows(10)
                ).changed() {
                    changed = true;
                }
            });

        // Quick templates
        ui.horizontal(|ui| {
            ui.label("Templates:");
            ui.menu_button("Insert", |ui| {
                ui.label("Basic Shapes");
                if ui.button("Random Sphere").clicked() {
                    *code = TEMPLATE_RANDOM_SPHERE.to_string();
                    changed = true;
                    ui.close_menu();
                }
                if ui.button("Hollow Shell").clicked() {
                    *code = TEMPLATE_SHELL.to_string();
                    changed = true;
                    ui.close_menu();
                }
                if ui.button("Grid Layout").clicked() {
                    *code = TEMPLATE_GRID.to_string();
                    changed = true;
                    ui.close_menu();
                }
                if ui.button("Plane").clicked() {
                    *code = TEMPLATE_PLANE.to_string();
                    changed = true;
                    ui.close_menu();
                }
                ui.separator();
                ui.label("Patterns");
                if ui.button("Spiral").clicked() {
                    *code = TEMPLATE_SPIRAL.to_string();
                    changed = true;
                    ui.close_menu();
                }
                if ui.button("Double Helix").clicked() {
                    *code = TEMPLATE_DOUBLE_HELIX.to_string();
                    changed = true;
                    ui.close_menu();
                }
                if ui.button("Vortex Ring").clicked() {
                    *code = TEMPLATE_VORTEX_RING.to_string();
                    changed = true;
                    ui.close_menu();
                }
                if ui.button("Galaxy Disk").clicked() {
                    *code = TEMPLATE_GALAXY.to_string();
                    changed = true;
                    ui.close_menu();
                }
                ui.separator();
                ui.label("Dynamic");
                if ui.button("Explosion").clicked() {
                    *code = TEMPLATE_EXPLOSION.to_string();
                    changed = true;
                    ui.close_menu();
                }
                if ui.button("Fountain").clicked() {
                    *code = TEMPLATE_FOUNTAIN.to_string();
                    changed = true;
                    ui.close_menu();
                }
                if ui.button("Collision").clicked() {
                    *code = TEMPLATE_COLLISION.to_string();
                    changed = true;
                    ui.close_menu();
                }
                if ui.button("Wave Surface").clicked() {
                    *code = TEMPLATE_WAVE.to_string();
                    changed = true;
                    ui.close_menu();
                }
            });
        });
    }

    changed
}

// Default custom spawn code
const DEFAULT_CUSTOM_SPAWN: &str = r#"// Custom spawn initialization
// Available: index, num_particles, bounds, t, seed, rand()
// Set: p.position, p.velocity, p.color, p.scale

// Random position in sphere
p.position = rand_in_sphere(&seed, bounds);

// Random velocity
p.velocity = rand_direction(&seed) * 0.1;

// Color based on position
p.color = normalize(p.position) * 0.5 + 0.5;

// Default scale
p.scale = 1.0;
"#;

const TEMPLATE_RANDOM_SPHERE: &str = r#"// Random positions in a sphere
p.position = rand_in_sphere(&seed, bounds);
p.velocity = vec3<f32>(0.0);
p.color = hsv_to_rgb(rand(&seed), 0.8, 0.9);
p.scale = 1.0;
"#;

const TEMPLATE_GRID: &str = r#"// 3D grid layout
let grid_size = u32(ceil(pow(f32(num_particles), 1.0/3.0)));
let ix = index % grid_size;
let iy = (index / grid_size) % grid_size;
let iz = index / (grid_size * grid_size);

let spacing = bounds * 2.0 / f32(grid_size);
let offset = -bounds + spacing * 0.5;

p.position = vec3<f32>(
    offset + f32(ix) * spacing,
    offset + f32(iy) * spacing,
    offset + f32(iz) * spacing
);
p.velocity = vec3<f32>(0.0);
p.color = vec3<f32>(f32(ix), f32(iy), f32(iz)) / f32(grid_size);
p.scale = 1.0;
"#;

const TEMPLATE_SPIRAL: &str = r#"// Spiral/helix pattern
let angle = t * 20.0 * 3.14159;
let height = (t - 0.5) * bounds * 2.0;
let radius = bounds * (0.2 + t * 0.6);

p.position = vec3<f32>(
    cos(angle) * radius,
    height,
    sin(angle) * radius
);

// Tangent velocity for swirl
let tangent = vec3<f32>(-sin(angle), 0.0, cos(angle));
p.velocity = tangent * 0.2;

p.color = hsv_to_rgb(t, 0.9, 0.9);
p.scale = 1.0;
"#;

const TEMPLATE_VORTEX_RING: &str = r#"// Vortex ring (torus shape with swirl velocity)
let major_angle = t * 6.28318;
let minor_angle = rand(&seed) * 6.28318;
let major_r = bounds * 0.6;
let minor_r = bounds * 0.15;

// Position on torus
let ring_center = vec3<f32>(cos(major_angle) * major_r, 0.0, sin(major_angle) * major_r);
let minor_offset = vec3<f32>(
    cos(major_angle) * cos(minor_angle) * minor_r,
    sin(minor_angle) * minor_r,
    sin(major_angle) * cos(minor_angle) * minor_r
);
p.position = ring_center + minor_offset;

// Velocity: tangent to ring + some rotation
let tangent = vec3<f32>(-sin(major_angle), 0.0, cos(major_angle));
p.velocity = tangent * 0.15;

p.color = hsv_to_rgb(major_angle / 6.28318, 0.8, 0.95);
p.scale = 1.0;
"#;

const TEMPLATE_SHELL: &str = r#"// Hollow sphere shell
p.position = rand_on_sphere(&seed, bounds * 0.8);
p.velocity = vec3<f32>(0.0);
p.color = hsv_to_rgb(rand(&seed), 0.7, 0.95);
p.scale = 1.0;
"#;

const TEMPLATE_PLANE: &str = r#"// Flat plane with slight noise
let x = (rand(&seed) - 0.5) * bounds * 2.0;
let z = (rand(&seed) - 0.5) * bounds * 2.0;
let y = (rand(&seed) - 0.5) * 0.05; // Slight thickness

p.position = vec3<f32>(x, y, z);
p.velocity = vec3<f32>(0.0);
p.color = vec3<f32>(0.3, 0.6, 0.9);
p.scale = 1.0;
"#;

const TEMPLATE_DOUBLE_HELIX: &str = r#"// DNA-like double helix
let angle = t * 12.0 * 3.14159;
let height = (t - 0.5) * bounds * 2.0;
let helix_r = bounds * 0.3;

// Alternate between two strands
let strand = select(0.0, 3.14159, index % 2u == 1u);

p.position = vec3<f32>(
    cos(angle + strand) * helix_r,
    height,
    sin(angle + strand) * helix_r
);

// Slight upward drift
p.velocity = vec3<f32>(0.0, 0.02, 0.0);

// Different colors for each strand
p.color = select(vec3<f32>(0.9, 0.3, 0.3), vec3<f32>(0.3, 0.3, 0.9), index % 2u == 0u);
p.scale = 1.0;
"#;

const TEMPLATE_GALAXY: &str = r#"// Spiral galaxy disk
let arm_count = 3.0;
let base_angle = t * 6.28318 * arm_count;
let radius_t = sqrt(rand(&seed)); // Square root for uniform disk distribution
let radius = radius_t * bounds * 0.9;

// Spiral arm offset increases with radius
let spiral_offset = radius * 2.0;
let angle = base_angle + spiral_offset + rand(&seed) * 0.3;

// Disk with slight thickness that decreases toward edges
let thickness = (1.0 - radius_t) * 0.1;
let y = (rand(&seed) - 0.5) * thickness;

p.position = vec3<f32>(cos(angle) * radius, y, sin(angle) * radius);

// Orbital velocity
let orbital_speed = 0.15 / max(sqrt(radius), 0.1);
p.velocity = vec3<f32>(-sin(angle), 0.0, cos(angle)) * orbital_speed;

// Color: blue core, yellow/orange arms
let core_factor = 1.0 - radius_t;
p.color = mix(vec3<f32>(1.0, 0.8, 0.4), vec3<f32>(0.5, 0.7, 1.0), core_factor);
p.scale = 1.0;
"#;

const TEMPLATE_EXPLOSION: &str = r#"// Explosion - particles burst outward from center
let dir = rand_direction(&seed);
let speed = 0.3 + rand(&seed) * 0.4;

// Start near center with slight spread
p.position = dir * rand(&seed) * bounds * 0.1;

// Outward velocity
p.velocity = dir * speed;

// Hot colors near center, cooler further out
let heat = 1.0 - length(p.position) / bounds;
p.color = vec3<f32>(1.0, heat * 0.8, heat * heat * 0.3);
p.scale = 0.5 + rand(&seed) * 1.0;
"#;

const TEMPLATE_FOUNTAIN: &str = r#"// Fountain - upward stream that falls back
// Spawn at base with upward velocity
let angle = rand(&seed) * 6.28318;
let spread = rand(&seed) * 0.1;

p.position = vec3<f32>(
    cos(angle) * spread,
    -bounds * 0.8,
    sin(angle) * spread
);

// Upward velocity with slight outward spread
let up_speed = 0.4 + rand(&seed) * 0.2;
p.velocity = vec3<f32>(
    cos(angle) * 0.05,
    up_speed,
    sin(angle) * 0.05
);

p.color = vec3<f32>(0.4, 0.7, 1.0);
p.scale = 0.8 + rand(&seed) * 0.4;
"#;

const TEMPLATE_COLLISION: &str = r#"// Two groups about to collide
let group = select(0u, 1u, index < num_particles / 2u);

// Position in two separate clusters
let offset = select(vec3<f32>(-0.5, 0.0, 0.0), vec3<f32>(0.5, 0.0, 0.0), group == 0u);
let local_pos = rand_in_sphere(&seed, bounds * 0.3);
p.position = offset * bounds + local_pos;

// Velocities toward each other
let vel_dir = select(vec3<f32>(1.0, 0.0, 0.0), vec3<f32>(-1.0, 0.0, 0.0), group == 0u);
p.velocity = vel_dir * 0.15 + rand_direction(&seed) * 0.02;

// Different colors for each group
p.color = select(vec3<f32>(1.0, 0.3, 0.3), vec3<f32>(0.3, 0.5, 1.0), group == 0u);
p.scale = 1.0;
"#;

const TEMPLATE_WAVE: &str = r#"// Sine wave surface
let grid_size = u32(ceil(sqrt(f32(num_particles))));
let ix = index % grid_size;
let iz = index / grid_size;

let x = (f32(ix) / f32(grid_size) - 0.5) * bounds * 2.0;
let z = (f32(iz) / f32(grid_size) - 0.5) * bounds * 2.0;

// Multiple wave components
let wave1 = sin(x * 4.0) * 0.15;
let wave2 = sin(z * 3.0 + x * 2.0) * 0.1;
let y = (wave1 + wave2) * bounds;

p.position = vec3<f32>(x, y, z);
p.velocity = vec3<f32>(0.0);

// Color by height
let h = (y / bounds + 0.3) * 0.5;
p.color = hsv_to_rgb(0.55 + h * 0.15, 0.7, 0.9);
p.scale = 1.0;
"#;
