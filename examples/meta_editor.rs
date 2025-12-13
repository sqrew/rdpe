//! # Meta Particle Editor
//!
//! A visual editor for designing particle simulations without writing code.
//!
//! This demonstrates how RDPE can be used to build a full simulation editor.
//! The MetaParticle has flexible fields that can be repurposed for different
//! simulation types.
//!
//! Run with: `cargo run --example meta_editor --features egui`

use rdpe::prelude::*;
use std::fs;
use std::path::PathBuf;

/// A flexible particle type with common fields that can be repurposed.
///
/// Users can treat `energy` as temperature, charge, health, etc.
/// The field names in WGSL remain the same, but display labels can be customized.
#[derive(Particle, Clone)]
struct MetaParticle {
    position: Vec3,
    velocity: Vec3,
    #[color]
    color: Vec3,
    particle_type: u32,

    // Scalar fields - repurpose as needed
    // Note: `age` and `scale` are auto-injected by Particle derive
    mass: f32,
    energy: f32,
    heat: f32,      // could be temperature, charge, etc.
    custom: f32,    // generic scalar

    // Extra vector field
    goal: Vec3,  // "target" is reserved in WGSL
}

/// Spawn shape configuration
#[derive(Clone, serde::Serialize, serde::Deserialize)]
enum SpawnShape {
    Cube { size: f32 },
    Sphere { radius: f32 },
    Shell { inner: f32, outer: f32 },
    Ring { radius: f32, thickness: f32 },
}

impl Default for SpawnShape {
    fn default() -> Self {
        SpawnShape::Sphere { radius: 0.5 }
    }
}

/// Initial velocity configuration
#[derive(Clone, serde::Serialize, serde::Deserialize)]
enum InitialVelocity {
    Zero,
    RandomDirection { speed: f32 },
    Outward { speed: f32 },
    Inward { speed: f32 },
    Swirl { speed: f32 },
}

impl Default for InitialVelocity {
    fn default() -> Self {
        InitialVelocity::RandomDirection { speed: 0.1 }
    }
}

/// Configuration for spawning particles
#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct SpawnConfig {
    shape: SpawnShape,
    velocity: InitialVelocity,

    // Initial field values (min, max for random range)
    mass_range: (f32, f32),
    energy_range: (f32, f32),
    color_mode: ColorMode,
}

impl Default for SpawnConfig {
    fn default() -> Self {
        Self {
            shape: SpawnShape::default(),
            velocity: InitialVelocity::default(),
            mass_range: (1.0, 1.0),
            energy_range: (1.0, 1.0),
            color_mode: ColorMode::default(),
        }
    }
}

/// How to assign particle colors
#[derive(Clone, serde::Serialize, serde::Deserialize)]
enum ColorMode {
    Uniform { r: f32, g: f32, b: f32 },
    RandomHue { saturation: f32, value: f32 },
    ByPosition,
    ByEnergy,
}

impl Default for ColorMode {
    fn default() -> Self {
        ColorMode::RandomHue { saturation: 0.8, value: 0.9 }
    }
}

/// Serializable rule configuration
#[derive(Clone, serde::Serialize, serde::Deserialize)]
enum RuleConfig {
    Gravity(f32),
    Drag(f32),
    BounceWalls,
    WrapWalls,
    Separate { radius: f32, strength: f32 },
    Cohere { radius: f32, strength: f32 },
    Align { radius: f32, strength: f32 },
    AttractTo { point: [f32; 3], strength: f32 },
    Wander { strength: f32, speed: f32 },
    SpeedLimit { min: f32, max: f32 },
    Custom { code: String, params: Vec<(String, f32)> },
}

impl RuleConfig {
    fn to_rule(&self) -> Rule {
        match self {
            RuleConfig::Gravity(g) => Rule::Gravity(*g),
            RuleConfig::Drag(d) => Rule::Drag(*d),
            RuleConfig::BounceWalls => Rule::BounceWalls,
            RuleConfig::WrapWalls => Rule::WrapWalls,
            RuleConfig::Separate { radius, strength } => Rule::Separate {
                radius: *radius,
                strength: *strength
            },
            RuleConfig::Cohere { radius, strength } => Rule::Cohere {
                radius: *radius,
                strength: *strength
            },
            RuleConfig::Align { radius, strength } => Rule::Align {
                radius: *radius,
                strength: *strength
            },
            RuleConfig::AttractTo { point, strength } => Rule::AttractTo {
                point: Vec3::from_array(*point),
                strength: *strength
            },
            RuleConfig::Wander { strength, speed } => Rule::Wander {
                strength: *strength,
                frequency: *speed,  // speed maps to frequency
            },
            RuleConfig::SpeedLimit { min, max } => Rule::SpeedLimit {
                min: *min,
                max: *max
            },
            RuleConfig::Custom { code, params } => {
                let mut builder = Rule::custom_dynamic(code.clone());
                for (name, value) in params {
                    builder = builder.with_param(name, *value);
                }
                builder.into()
            }
        }
    }

    fn name(&self) -> &'static str {
        match self {
            RuleConfig::Gravity(_) => "Gravity",
            RuleConfig::Drag(_) => "Drag",
            RuleConfig::BounceWalls => "Bounce Walls",
            RuleConfig::WrapWalls => "Wrap Walls",
            RuleConfig::Separate { .. } => "Separate",
            RuleConfig::Cohere { .. } => "Cohere",
            RuleConfig::Align { .. } => "Align",
            RuleConfig::AttractTo { .. } => "Attract To",
            RuleConfig::Wander { .. } => "Wander",
            RuleConfig::SpeedLimit { .. } => "Speed Limit",
            RuleConfig::Custom { .. } => "Custom",
        }
    }
}

/// Complete simulation configuration - can be saved/loaded
#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct SimConfig {
    name: String,
    particle_count: u32,
    bounds: f32,
    spatial_cell_size: f32,
    spatial_resolution: u32,
    spawn: SpawnConfig,
    rules: Vec<RuleConfig>,

    // Display labels for generic fields
    field_labels: FieldLabels,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct FieldLabels {
    mass: String,
    energy: String,
    heat: String,
    custom: String,
    goal: String,
}

impl Default for FieldLabels {
    fn default() -> Self {
        Self {
            mass: "Mass".into(),
            energy: "Energy".into(),
            heat: "Heat".into(),
            custom: "Custom".into(),
            goal: "Goal".into(),
        }
    }
}

impl Default for SimConfig {
    fn default() -> Self {
        Self {
            name: "Untitled".into(),
            particle_count: 5000,
            bounds: 1.0,
            spatial_cell_size: 0.1,
            spatial_resolution: 32,
            spawn: SpawnConfig::default(),
            rules: vec![
                RuleConfig::Gravity(2.0),
                RuleConfig::Drag(0.5),
                RuleConfig::BounceWalls,
            ],
            field_labels: FieldLabels::default(),
        }
    }
}

impl SimConfig {
    fn save(&self, path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)?;
        Ok(())
    }

    fn load(path: &PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        let json = fs::read_to_string(path)?;
        let config = serde_json::from_str(&json)?;
        Ok(config)
    }
}

/// Editor state persisted across frames
struct EditorState {
    config: SimConfig,
    config_path: Option<PathBuf>,
    needs_restart: bool,
    show_spawn_panel: bool,
    show_rules_panel: bool,
    show_save_dialog: bool,
    status_message: Option<(String, std::time::Instant)>,

    // For adding new rules
    new_rule_type: usize,
}

impl Default for EditorState {
    fn default() -> Self {
        Self {
            config: SimConfig::default(),
            config_path: None,
            needs_restart: false,
            show_spawn_panel: true,
            show_rules_panel: true,
            show_save_dialog: false,
            status_message: None,
            new_rule_type: 0,
        }
    }
}

impl EditorState {
    fn set_status(&mut self, msg: impl Into<String>) {
        self.status_message = Some((msg.into(), std::time::Instant::now()));
    }
}

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> Vec3 {
    let c = v * s;
    let h = h * 6.0;
    let x = c * (1.0 - ((h % 2.0) - 1.0).abs());
    let m = v - c;

    let (r, g, b) = if h < 1.0 {
        (c, x, 0.0)
    } else if h < 2.0 {
        (x, c, 0.0)
    } else if h < 3.0 {
        (0.0, c, x)
    } else if h < 4.0 {
        (0.0, x, c)
    } else if h < 5.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };

    Vec3::new(r + m, g + m, b + m)
}

fn spawn_particle(ctx: &mut SpawnContext, config: &SpawnConfig) -> MetaParticle {
    // Position based on shape
    let position = match &config.shape {
        SpawnShape::Cube { size } => ctx.random_in_cube(*size),
        SpawnShape::Sphere { radius } => ctx.random_in_sphere(*radius),
        SpawnShape::Shell { inner, outer } => {
            let dir = ctx.random_direction();
            let r = *inner + ctx.random() * (*outer - *inner);
            dir * r
        }
        SpawnShape::Ring { radius, thickness } => {
            let angle = ctx.random() * std::f32::consts::TAU;
            let r = *radius + (ctx.random() - 0.5) * *thickness;
            Vec3::new(angle.cos() * r, (ctx.random() - 0.5) * *thickness, angle.sin() * r)
        }
    };

    // Velocity based on mode
    let velocity = match &config.velocity {
        InitialVelocity::Zero => Vec3::ZERO,
        InitialVelocity::RandomDirection { speed } => ctx.random_direction() * *speed,
        InitialVelocity::Outward { speed } => {
            if position.length() > 0.001 {
                position.normalize() * *speed
            } else {
                ctx.random_direction() * *speed
            }
        }
        InitialVelocity::Inward { speed } => {
            if position.length() > 0.001 {
                -position.normalize() * *speed
            } else {
                ctx.random_direction() * *speed
            }
        }
        InitialVelocity::Swirl { speed } => {
            let tangent = Vec3::new(-position.z, 0.0, position.x);
            if tangent.length() > 0.001 {
                tangent.normalize() * *speed
            } else {
                ctx.random_direction() * *speed
            }
        }
    };

    // Color based on mode
    let color = match &config.color_mode {
        ColorMode::Uniform { r, g, b } => Vec3::new(*r, *g, *b),
        ColorMode::RandomHue { saturation, value } => {
            hsv_to_rgb(ctx.random(), *saturation, *value)
        }
        ColorMode::ByPosition => {
            Vec3::new(
                position.x * 0.5 + 0.5,
                position.y * 0.5 + 0.5,
                position.z * 0.5 + 0.5,
            )
        }
        ColorMode::ByEnergy => {
            let e = (config.energy_range.0 + config.energy_range.1) / 2.0;
            hsv_to_rgb(e.fract(), 0.8, 0.9)
        }
    };

    // Random values in ranges
    let mass = config.mass_range.0 + ctx.random() * (config.mass_range.1 - config.mass_range.0);
    let energy = config.energy_range.0 + ctx.random() * (config.energy_range.1 - config.energy_range.0);

    MetaParticle {
        position,
        velocity,
        color,
        particle_type: 0,
        mass,
        energy,
        heat: 0.0,
        custom: 0.0,
        goal: Vec3::ZERO,
    }
}

fn render_editor_ui(ctx: &egui::Context, state: &mut EditorState) {
    // Top menu bar
    egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
        egui::menu::bar(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("New").clicked() {
                    state.config = SimConfig::default();
                    state.config_path = None;
                    state.needs_restart = true;
                    state.set_status("New simulation");
                    ui.close_menu();
                }
                if ui.button("Save").clicked() {
                    if let Some(path) = &state.config_path {
                        match state.config.save(path) {
                            Ok(_) => state.set_status(format!("Saved to {:?}", path)),
                            Err(e) => state.set_status(format!("Save failed: {}", e)),
                        }
                    } else {
                        state.show_save_dialog = true;
                    }
                    ui.close_menu();
                }
                if ui.button("Save As...").clicked() {
                    state.show_save_dialog = true;
                    ui.close_menu();
                }
            });

            ui.menu_button("View", |ui| {
                ui.checkbox(&mut state.show_spawn_panel, "Spawn Config");
                ui.checkbox(&mut state.show_rules_panel, "Rules");
            });

            ui.separator();

            if ui.button("Restart Simulation").clicked() {
                state.needs_restart = true;
            }

            // Status message
            if let Some((msg, time)) = &state.status_message {
                if time.elapsed().as_secs() < 3 {
                    ui.separator();
                    ui.label(msg);
                }
            }
        });
    });

    // Spawn configuration panel
    if state.show_spawn_panel {
        egui::Window::new("Spawn Config")
            .default_width(250.0)
            .show(ctx, |ui| {
                let config = &mut state.config;

                ui.horizontal(|ui| {
                    ui.label("Particles:");
                    if ui.add(egui::DragValue::new(&mut config.particle_count)
                        .range(100..=100_000)
                        .speed(100)).changed() {
                        state.needs_restart = true;
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Bounds:");
                    if ui.add(egui::DragValue::new(&mut config.bounds)
                        .range(0.1..=10.0)
                        .speed(0.1)).changed() {
                        state.needs_restart = true;
                    }
                });

                ui.separator();
                ui.heading("Shape");

                let mut shape_idx = match &config.spawn.shape {
                    SpawnShape::Cube { .. } => 0,
                    SpawnShape::Sphere { .. } => 1,
                    SpawnShape::Shell { .. } => 2,
                    SpawnShape::Ring { .. } => 3,
                };

                if egui::ComboBox::from_label("Type")
                    .show_index(ui, &mut shape_idx, 4, |i| {
                        ["Cube", "Sphere", "Shell", "Ring"][i]
                    }).changed() {
                    config.spawn.shape = match shape_idx {
                        0 => SpawnShape::Cube { size: 0.5 },
                        1 => SpawnShape::Sphere { radius: 0.5 },
                        2 => SpawnShape::Shell { inner: 0.3, outer: 0.5 },
                        3 => SpawnShape::Ring { radius: 0.5, thickness: 0.1 },
                        _ => SpawnShape::Sphere { radius: 0.5 },
                    };
                    state.needs_restart = true;
                }

                match &mut config.spawn.shape {
                    SpawnShape::Cube { size } => {
                        if ui.add(egui::Slider::new(size, 0.1..=2.0).text("Size")).changed() {
                            state.needs_restart = true;
                        }
                    }
                    SpawnShape::Sphere { radius } => {
                        if ui.add(egui::Slider::new(radius, 0.1..=2.0).text("Radius")).changed() {
                            state.needs_restart = true;
                        }
                    }
                    SpawnShape::Shell { inner, outer } => {
                        if ui.add(egui::Slider::new(inner, 0.0..=1.9).text("Inner")).changed() {
                            state.needs_restart = true;
                        }
                        if ui.add(egui::Slider::new(outer, 0.1..=2.0).text("Outer")).changed() {
                            state.needs_restart = true;
                        }
                    }
                    SpawnShape::Ring { radius, thickness } => {
                        if ui.add(egui::Slider::new(radius, 0.1..=2.0).text("Radius")).changed() {
                            state.needs_restart = true;
                        }
                        if ui.add(egui::Slider::new(thickness, 0.01..=0.5).text("Thickness")).changed() {
                            state.needs_restart = true;
                        }
                    }
                }

                ui.separator();
                ui.heading("Initial Velocity");

                let mut vel_idx = match &config.spawn.velocity {
                    InitialVelocity::Zero => 0,
                    InitialVelocity::RandomDirection { .. } => 1,
                    InitialVelocity::Outward { .. } => 2,
                    InitialVelocity::Inward { .. } => 3,
                    InitialVelocity::Swirl { .. } => 4,
                };

                if egui::ComboBox::from_label("Mode")
                    .show_index(ui, &mut vel_idx, 5, |i| {
                        ["Zero", "Random", "Outward", "Inward", "Swirl"][i]
                    }).changed() {
                    config.spawn.velocity = match vel_idx {
                        0 => InitialVelocity::Zero,
                        1 => InitialVelocity::RandomDirection { speed: 0.1 },
                        2 => InitialVelocity::Outward { speed: 0.1 },
                        3 => InitialVelocity::Inward { speed: 0.1 },
                        4 => InitialVelocity::Swirl { speed: 0.1 },
                        _ => InitialVelocity::Zero,
                    };
                    state.needs_restart = true;
                }

                match &mut config.spawn.velocity {
                    InitialVelocity::Zero => {}
                    InitialVelocity::RandomDirection { speed } |
                    InitialVelocity::Outward { speed } |
                    InitialVelocity::Inward { speed } |
                    InitialVelocity::Swirl { speed } => {
                        if ui.add(egui::Slider::new(speed, 0.0..=2.0).text("Speed")).changed() {
                            state.needs_restart = true;
                        }
                    }
                }

                ui.separator();
                ui.heading("Color");

                let mut color_idx = match &config.spawn.color_mode {
                    ColorMode::Uniform { .. } => 0,
                    ColorMode::RandomHue { .. } => 1,
                    ColorMode::ByPosition => 2,
                    ColorMode::ByEnergy => 3,
                };

                if egui::ComboBox::from_label("Color Mode")
                    .show_index(ui, &mut color_idx, 4, |i| {
                        ["Uniform", "Random Hue", "By Position", "By Energy"][i]
                    }).changed() {
                    config.spawn.color_mode = match color_idx {
                        0 => ColorMode::Uniform { r: 1.0, g: 0.5, b: 0.2 },
                        1 => ColorMode::RandomHue { saturation: 0.8, value: 0.9 },
                        2 => ColorMode::ByPosition,
                        3 => ColorMode::ByEnergy,
                        _ => ColorMode::RandomHue { saturation: 0.8, value: 0.9 },
                    };
                    state.needs_restart = true;
                }

                match &mut config.spawn.color_mode {
                    ColorMode::Uniform { r, g, b } => {
                        let mut color = [*r, *g, *b];
                        if ui.color_edit_button_rgb(&mut color).changed() {
                            *r = color[0];
                            *g = color[1];
                            *b = color[2];
                            state.needs_restart = true;
                        }
                    }
                    ColorMode::RandomHue { saturation, value } => {
                        if ui.add(egui::Slider::new(saturation, 0.0..=1.0).text("Saturation")).changed() {
                            state.needs_restart = true;
                        }
                        if ui.add(egui::Slider::new(value, 0.0..=1.0).text("Value")).changed() {
                            state.needs_restart = true;
                        }
                    }
                    _ => {}
                }
            });
    }

    // Rules panel
    if state.show_rules_panel {
        egui::Window::new("Rules")
            .default_width(280.0)
            .show(ctx, |ui| {
                let mut remove_idx = None;

                for (i, rule) in state.config.rules.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        ui.label(format!("{}.", i + 1));
                        ui.strong(rule.name());
                        if ui.small_button("x").clicked() {
                            remove_idx = Some(i);
                        }
                    });

                    ui.indent(format!("rule_{}", i), |ui| {
                        match rule {
                            RuleConfig::Gravity(g) => {
                                ui.add(egui::Slider::new(g, 0.0..=20.0).text("Strength"));
                            }
                            RuleConfig::Drag(d) => {
                                ui.add(egui::Slider::new(d, 0.0..=10.0).text("Amount"));
                            }
                            RuleConfig::Separate { radius, strength } => {
                                ui.add(egui::Slider::new(radius, 0.01..=0.5).text("Radius"));
                                ui.add(egui::Slider::new(strength, 0.0..=10.0).text("Strength"));
                            }
                            RuleConfig::Cohere { radius, strength } => {
                                ui.add(egui::Slider::new(radius, 0.01..=0.5).text("Radius"));
                                ui.add(egui::Slider::new(strength, 0.0..=10.0).text("Strength"));
                            }
                            RuleConfig::Align { radius, strength } => {
                                ui.add(egui::Slider::new(radius, 0.01..=0.5).text("Radius"));
                                ui.add(egui::Slider::new(strength, 0.0..=10.0).text("Strength"));
                            }
                            RuleConfig::AttractTo { point, strength } => {
                                ui.horizontal(|ui| {
                                    ui.label("Point:");
                                    ui.add(egui::DragValue::new(&mut point[0]).speed(0.01));
                                    ui.add(egui::DragValue::new(&mut point[1]).speed(0.01));
                                    ui.add(egui::DragValue::new(&mut point[2]).speed(0.01));
                                });
                                ui.add(egui::Slider::new(strength, 0.0..=10.0).text("Strength"));
                            }
                            RuleConfig::Wander { strength, speed } => {
                                ui.add(egui::Slider::new(strength, 0.0..=5.0).text("Strength"));
                                ui.add(egui::Slider::new(speed, 0.0..=10.0).text("Speed"));
                            }
                            RuleConfig::SpeedLimit { min, max } => {
                                ui.add(egui::Slider::new(min, 0.0..=2.0).text("Min"));
                                ui.add(egui::Slider::new(max, 0.0..=5.0).text("Max"));
                            }
                            RuleConfig::Custom { code, params } => {
                                ui.label("WGSL Code:");
                                ui.text_edit_multiline(code);
                                for (name, value) in params.iter_mut() {
                                    ui.horizontal(|ui| {
                                        ui.label(format!("{}:", name));
                                        ui.add(egui::DragValue::new(value).speed(0.01));
                                    });
                                }
                            }
                            _ => {}
                        }
                    });

                    ui.separator();
                }

                if let Some(idx) = remove_idx {
                    state.config.rules.remove(idx);
                    state.needs_restart = true;
                }

                // Add new rule
                ui.heading("Add Rule");
                egui::ComboBox::from_label("Type")
                    .show_index(ui, &mut state.new_rule_type, 11, |i| {
                        ["Gravity", "Drag", "Bounce", "Wrap", "Separate",
                         "Cohere", "Align", "Attract", "Wander", "Speed Limit", "Custom"][i]
                    });

                if ui.button("Add").clicked() {
                    let new_rule = match state.new_rule_type {
                        0 => RuleConfig::Gravity(9.8),
                        1 => RuleConfig::Drag(1.0),
                        2 => RuleConfig::BounceWalls,
                        3 => RuleConfig::WrapWalls,
                        4 => RuleConfig::Separate { radius: 0.05, strength: 2.0 },
                        5 => RuleConfig::Cohere { radius: 0.15, strength: 1.0 },
                        6 => RuleConfig::Align { radius: 0.1, strength: 1.5 },
                        7 => RuleConfig::AttractTo { point: [0.0, 0.0, 0.0], strength: 1.0 },
                        8 => RuleConfig::Wander { strength: 1.0, speed: 2.0 },
                        9 => RuleConfig::SpeedLimit { min: 0.0, max: 1.0 },
                        10 => RuleConfig::Custom {
                            code: "// Custom WGSL\np.velocity.y += 0.01;".into(),
                            params: vec![],
                        },
                        _ => RuleConfig::Gravity(9.8),
                    };
                    state.config.rules.push(new_rule);
                    state.needs_restart = true;
                }
            });
    }

    // Restart indicator
    if state.needs_restart {
        egui::Window::new("Restart Required")
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label("Configuration changed. Restart to apply.");
                ui.label("Press 'R' or click the button in the menu bar.");
            });
    }
}

fn main() {
    // Try to load saved config or use default
    let config_path = PathBuf::from("meta_sim.json");
    let initial_config = if config_path.exists() {
        SimConfig::load(&config_path).unwrap_or_default()
    } else {
        SimConfig::default()
    };

    // We need to clone config for the spawner closure
    let spawn_config = initial_config.spawn.clone();

    // Convert RuleConfig to Rule
    let rules: Vec<Rule> = initial_config.rules.iter().map(|r| r.to_rule()).collect();

    // Check if we need spatial hashing
    let needs_spatial = rules.iter().any(|r| r.requires_neighbors());

    // Build the simulation
    let mut sim = Simulation::<MetaParticle>::new()
        .with_particle_count(initial_config.particle_count)
        .with_bounds(initial_config.bounds)
        .with_spawner(move |ctx| spawn_particle(ctx, &spawn_config));

    // Add spatial config if needed
    if needs_spatial {
        sim = sim.with_spatial_config(
            initial_config.spatial_cell_size,
            initial_config.spatial_resolution
        );
    }

    // Add rules
    for rule in rules {
        sim = sim.with_rule(rule);
    }

    // Editor state
    let mut editor_state = EditorState {
        config: initial_config,
        config_path: Some(config_path),
        ..Default::default()
    };

    // Run with editor UI
    sim
        .with_particle_inspector()
        .with_rule_inspector()
        .with_ui(move |ctx| {
            render_editor_ui(ctx, &mut editor_state);
        })
        .run().expect("Simulation failed");
}
