//! # RDPE Standalone Editor
//!
//! A visual editor for designing particle simulations.
//! Edits config files and launches simulation processes.
//!
//! Run with: `cargo run --example editor --features egui`

use std::fs;
use std::path::PathBuf;
use std::process::{Child, Command};

// Re-use config types - in a real app these would be in a shared crate
mod config {
    use serde::{Deserialize, Serialize};
    use std::fs;
    use std::path::PathBuf;

    #[derive(Clone, Serialize, Deserialize)]
    pub enum SpawnShape {
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

    #[derive(Clone, Serialize, Deserialize)]
    pub enum InitialVelocity {
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

    #[derive(Clone, Serialize, Deserialize)]
    pub enum ColorMode {
        Uniform { r: f32, g: f32, b: f32 },
        RandomHue { saturation: f32, value: f32 },
        ByPosition,
        ByVelocity,
    }

    impl Default for ColorMode {
        fn default() -> Self {
            ColorMode::RandomHue {
                saturation: 0.8,
                value: 0.9,
            }
        }
    }

    #[derive(Clone, Serialize, Deserialize)]
    pub struct SpawnConfig {
        pub shape: SpawnShape,
        pub velocity: InitialVelocity,
        pub mass_range: (f32, f32),
        pub energy_range: (f32, f32),
        pub color_mode: ColorMode,
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

    #[derive(Clone, Serialize, Deserialize)]
    pub enum RuleConfig {
        Gravity(f32),
        Drag(f32),
        BounceWalls,
        WrapWalls,
        Separate { radius: f32, strength: f32 },
        Cohere { radius: f32, strength: f32 },
        Align { radius: f32, strength: f32 },
        AttractTo { point: [f32; 3], strength: f32 },
        Wander { strength: f32, frequency: f32 },
        SpeedLimit { min: f32, max: f32 },
        Custom { code: String, params: Vec<(String, f32)> },
    }

    impl RuleConfig {
        pub fn name(&self) -> &'static str {
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
                RuleConfig::Custom { .. } => "Custom WGSL",
            }
        }
    }

    #[derive(Clone, Serialize, Deserialize)]
    pub struct SimConfig {
        pub name: String,
        pub particle_count: u32,
        pub bounds: f32,
        pub particle_size: f32,
        pub spatial_cell_size: f32,
        pub spatial_resolution: u32,
        pub spawn: SpawnConfig,
        pub rules: Vec<RuleConfig>,
    }

    impl Default for SimConfig {
        fn default() -> Self {
            Self {
                name: "Untitled".into(),
                particle_count: 5000,
                bounds: 1.0,
                particle_size: 0.015,
                spatial_cell_size: 0.1,
                spatial_resolution: 32,
                spawn: SpawnConfig::default(),
                rules: vec![
                    RuleConfig::Gravity(2.0),
                    RuleConfig::Drag(0.5),
                    RuleConfig::BounceWalls,
                ],
            }
        }
    }

    impl SimConfig {
        pub fn save(&self, path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
            let json = serde_json::to_string_pretty(self)?;
            fs::write(path, json)?;
            Ok(())
        }

        pub fn load(path: &PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
            let json = fs::read_to_string(path)?;
            let config = serde_json::from_str(&json)?;
            Ok(config)
        }
    }
}

use config::*;

/// Editor application state
struct EditorApp {
    config: SimConfig,
    config_path: PathBuf,
    sim_process: Option<Child>,
    new_rule_type: usize,
    status: Option<(String, std::time::Instant)>,
    show_presets: bool,
}

impl Default for EditorApp {
    fn default() -> Self {
        let config_path = PathBuf::from("simulation.json");
        let config = if config_path.exists() {
            SimConfig::load(&config_path).unwrap_or_default()
        } else {
            SimConfig::default()
        };

        Self {
            config,
            config_path,
            sim_process: None,
            new_rule_type: 0,
            status: None,
            show_presets: false,
        }
    }
}

impl EditorApp {
    fn set_status(&mut self, msg: impl Into<String>) {
        self.status = Some((msg.into(), std::time::Instant::now()));
    }

    fn save_config(&mut self) {
        match self.config.save(&self.config_path) {
            Ok(_) => self.set_status(format!("Saved to {:?}", self.config_path)),
            Err(e) => self.set_status(format!("Save error: {}", e)),
        }
    }

    fn run_simulation(&mut self) {
        // Kill existing simulation if running
        if let Some(mut child) = self.sim_process.take() {
            let _ = child.kill();
        }

        // Save config first
        self.save_config();

        // Spawn the simulation process
        match Command::new("cargo")
            .args([
                "run",
                "--example",
                "meta_sim",
                "--features",
                "egui",
                "--",
                self.config_path.to_str().unwrap_or("simulation.json"),
            ])
            .spawn()
        {
            Ok(child) => {
                self.sim_process = Some(child);
                self.set_status("Simulation started");
            }
            Err(e) => {
                self.set_status(format!("Failed to start: {}", e));
            }
        }
    }

    fn stop_simulation(&mut self) {
        if let Some(mut child) = self.sim_process.take() {
            let _ = child.kill();
            self.set_status("Simulation stopped");
        }
    }

    fn check_simulation_status(&mut self) {
        if let Some(ref mut child) = self.sim_process {
            match child.try_wait() {
                Ok(Some(_status)) => {
                    self.sim_process = None;
                    self.set_status("Simulation exited");
                }
                Ok(None) => {} // Still running
                Err(_) => {
                    self.sim_process = None;
                }
            }
        }
    }

    fn load_preset(&mut self, preset: &str) {
        self.config = match preset {
            "boids" => SimConfig {
                name: "Boids Flocking".into(),
                particle_count: 5000,
                bounds: 1.0,
                particle_size: 0.01,
                spatial_cell_size: 0.15,
                spatial_resolution: 32,
                spawn: SpawnConfig {
                    shape: SpawnShape::Sphere { radius: 0.5 },
                    velocity: InitialVelocity::RandomDirection { speed: 0.2 },
                    ..Default::default()
                },
                rules: vec![
                    RuleConfig::Separate { radius: 0.05, strength: 5.0 },
                    RuleConfig::Cohere { radius: 0.15, strength: 1.0 },
                    RuleConfig::Align { radius: 0.1, strength: 2.0 },
                    RuleConfig::SpeedLimit { min: 0.1, max: 0.5 },
                    RuleConfig::BounceWalls,
                ],
            },
            "gravity" => SimConfig {
                name: "Gravity Well".into(),
                particle_count: 10000,
                bounds: 1.0,
                particle_size: 0.008,
                spatial_cell_size: 0.1,
                spatial_resolution: 32,
                spawn: SpawnConfig {
                    shape: SpawnShape::Shell { inner: 0.3, outer: 0.8 },
                    velocity: InitialVelocity::Swirl { speed: 0.3 },
                    color_mode: ColorMode::ByVelocity,
                    ..Default::default()
                },
                rules: vec![
                    RuleConfig::AttractTo { point: [0.0, 0.0, 0.0], strength: 3.0 },
                    RuleConfig::Drag(0.1),
                    RuleConfig::SpeedLimit { min: 0.0, max: 2.0 },
                ],
            },
            "explosion" => SimConfig {
                name: "Explosion".into(),
                particle_count: 20000,
                bounds: 2.0,
                particle_size: 0.005,
                spatial_cell_size: 0.1,
                spatial_resolution: 32,
                spawn: SpawnConfig {
                    shape: SpawnShape::Sphere { radius: 0.1 },
                    velocity: InitialVelocity::Outward { speed: 1.5 },
                    color_mode: ColorMode::RandomHue { saturation: 1.0, value: 1.0 },
                    ..Default::default()
                },
                rules: vec![
                    RuleConfig::Gravity(3.0),
                    RuleConfig::Drag(0.3),
                    RuleConfig::BounceWalls,
                ],
            },
            _ => SimConfig::default(),
        };
        self.set_status(format!("Loaded preset: {}", preset));
    }
}

impl eframe::App for EditorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.check_simulation_status();

        // Top panel with menu and controls
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("New").clicked() {
                        self.config = SimConfig::default();
                        self.set_status("New configuration");
                        ui.close_menu();
                    }
                    if ui.button("Save").clicked() {
                        self.save_config();
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Quit").clicked() {
                        std::process::exit(0);
                    }
                });

                ui.menu_button("Presets", |ui| {
                    if ui.button("Boids Flocking").clicked() {
                        self.load_preset("boids");
                        ui.close_menu();
                    }
                    if ui.button("Gravity Well").clicked() {
                        self.load_preset("gravity");
                        ui.close_menu();
                    }
                    if ui.button("Explosion").clicked() {
                        self.load_preset("explosion");
                        ui.close_menu();
                    }
                });

                ui.separator();

                let sim_running = self.sim_process.is_some();

                if sim_running {
                    if ui.button("Stop").clicked() {
                        self.stop_simulation();
                    }
                    if ui.button("Restart").clicked() {
                        self.run_simulation();
                    }
                    ui.label("Simulation running");
                } else {
                    if ui.button("Run").clicked() {
                        self.run_simulation();
                    }
                }

                // Status message
                if let Some((msg, time)) = &self.status {
                    if time.elapsed().as_secs() < 3 {
                        ui.separator();
                        ui.label(msg);
                    }
                }
            });
        });

        // Left panel - Simulation settings
        egui::SidePanel::left("settings_panel")
            .default_width(280.0)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.heading("Simulation");

                    ui.horizontal(|ui| {
                        ui.label("Name:");
                        ui.text_edit_singleline(&mut self.config.name);
                    });

                    ui.add(egui::Slider::new(&mut self.config.particle_count, 100..=100_000)
                        .text("Particles")
                        .logarithmic(true));

                    ui.add(egui::Slider::new(&mut self.config.bounds, 0.5..=5.0)
                        .text("Bounds"));

                    ui.add(egui::Slider::new(&mut self.config.particle_size, 0.001..=0.1)
                        .text("Particle Size")
                        .logarithmic(true));

                    ui.separator();
                    ui.heading("Spawn Shape");

                    let mut shape_idx = match &self.config.spawn.shape {
                        SpawnShape::Cube { .. } => 0,
                        SpawnShape::Sphere { .. } => 1,
                        SpawnShape::Shell { .. } => 2,
                        SpawnShape::Ring { .. } => 3,
                    };

                    if egui::ComboBox::from_label("Shape")
                        .show_index(ui, &mut shape_idx, 4, |i| {
                            ["Cube", "Sphere", "Shell", "Ring"][i]
                        })
                        .changed()
                    {
                        self.config.spawn.shape = match shape_idx {
                            0 => SpawnShape::Cube { size: 0.5 },
                            1 => SpawnShape::Sphere { radius: 0.5 },
                            2 => SpawnShape::Shell { inner: 0.3, outer: 0.5 },
                            3 => SpawnShape::Ring { radius: 0.5, thickness: 0.1 },
                            _ => SpawnShape::Sphere { radius: 0.5 },
                        };
                    }

                    match &mut self.config.spawn.shape {
                        SpawnShape::Cube { size } => {
                            ui.add(egui::Slider::new(size, 0.1..=2.0).text("Size"));
                        }
                        SpawnShape::Sphere { radius } => {
                            ui.add(egui::Slider::new(radius, 0.1..=2.0).text("Radius"));
                        }
                        SpawnShape::Shell { inner, outer } => {
                            ui.add(egui::Slider::new(inner, 0.0..=1.9).text("Inner"));
                            ui.add(egui::Slider::new(outer, 0.1..=2.0).text("Outer"));
                        }
                        SpawnShape::Ring { radius, thickness } => {
                            ui.add(egui::Slider::new(radius, 0.1..=2.0).text("Radius"));
                            ui.add(egui::Slider::new(thickness, 0.01..=0.5).text("Thickness"));
                        }
                    }

                    ui.separator();
                    ui.heading("Initial Velocity");

                    let mut vel_idx = match &self.config.spawn.velocity {
                        InitialVelocity::Zero => 0,
                        InitialVelocity::RandomDirection { .. } => 1,
                        InitialVelocity::Outward { .. } => 2,
                        InitialVelocity::Inward { .. } => 3,
                        InitialVelocity::Swirl { .. } => 4,
                    };

                    if egui::ComboBox::from_label("Mode")
                        .show_index(ui, &mut vel_idx, 5, |i| {
                            ["Zero", "Random", "Outward", "Inward", "Swirl"][i]
                        })
                        .changed()
                    {
                        self.config.spawn.velocity = match vel_idx {
                            0 => InitialVelocity::Zero,
                            1 => InitialVelocity::RandomDirection { speed: 0.1 },
                            2 => InitialVelocity::Outward { speed: 0.1 },
                            3 => InitialVelocity::Inward { speed: 0.1 },
                            4 => InitialVelocity::Swirl { speed: 0.1 },
                            _ => InitialVelocity::Zero,
                        };
                    }

                    match &mut self.config.spawn.velocity {
                        InitialVelocity::Zero => {}
                        InitialVelocity::RandomDirection { speed }
                        | InitialVelocity::Outward { speed }
                        | InitialVelocity::Inward { speed }
                        | InitialVelocity::Swirl { speed } => {
                            ui.add(egui::Slider::new(speed, 0.0..=2.0).text("Speed"));
                        }
                    }

                    ui.separator();
                    ui.heading("Color");

                    let mut color_idx = match &self.config.spawn.color_mode {
                        ColorMode::Uniform { .. } => 0,
                        ColorMode::RandomHue { .. } => 1,
                        ColorMode::ByPosition => 2,
                        ColorMode::ByVelocity => 3,
                    };

                    if egui::ComboBox::from_label("Color Mode")
                        .show_index(ui, &mut color_idx, 4, |i| {
                            ["Uniform", "Random Hue", "By Position", "By Velocity"][i]
                        })
                        .changed()
                    {
                        self.config.spawn.color_mode = match color_idx {
                            0 => ColorMode::Uniform { r: 1.0, g: 0.5, b: 0.2 },
                            1 => ColorMode::RandomHue { saturation: 0.8, value: 0.9 },
                            2 => ColorMode::ByPosition,
                            3 => ColorMode::ByVelocity,
                            _ => ColorMode::RandomHue { saturation: 0.8, value: 0.9 },
                        };
                    }

                    match &mut self.config.spawn.color_mode {
                        ColorMode::Uniform { r, g, b } => {
                            let mut color = [*r, *g, *b];
                            if ui.color_edit_button_rgb(&mut color).changed() {
                                *r = color[0];
                                *g = color[1];
                                *b = color[2];
                            }
                        }
                        ColorMode::RandomHue { saturation, value } => {
                            ui.add(egui::Slider::new(saturation, 0.0..=1.0).text("Saturation"));
                            ui.add(egui::Slider::new(value, 0.0..=1.0).text("Value"));
                        }
                        _ => {}
                    }
                });
            });

        // Right panel - Rules
        egui::SidePanel::right("rules_panel")
            .default_width(300.0)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.heading("Rules");
                    ui.label("Rules execute in order, top to bottom.");
                    ui.separator();

                    let mut remove_idx = None;
                    let mut move_up_idx = None;
                    let mut move_down_idx = None;
                    let rules_len = self.config.rules.len();

                    for (i, rule) in self.config.rules.iter_mut().enumerate() {
                        ui.horizontal(|ui| {
                            ui.strong(format!("{}. {}", i + 1, rule.name()));
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.small_button("x").clicked() {
                                    remove_idx = Some(i);
                                }
                                if i < rules_len - 1 {
                                    if ui.small_button("v").clicked() {
                                        move_down_idx = Some(i);
                                    }
                                }
                                if i > 0 {
                                    if ui.small_button("^").clicked() {
                                        move_up_idx = Some(i);
                                    }
                                }
                            });
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
                                    ui.add(egui::Slider::new(strength, 0.0..=20.0).text("Strength"));
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
                                        ui.add(egui::DragValue::new(&mut point[0]).speed(0.01).prefix("x:"));
                                        ui.add(egui::DragValue::new(&mut point[1]).speed(0.01).prefix("y:"));
                                        ui.add(egui::DragValue::new(&mut point[2]).speed(0.01).prefix("z:"));
                                    });
                                    ui.add(egui::Slider::new(strength, -10.0..=10.0).text("Strength"));
                                }
                                RuleConfig::Wander { strength, frequency } => {
                                    ui.add(egui::Slider::new(strength, 0.0..=5.0).text("Strength"));
                                    ui.add(egui::Slider::new(frequency, 0.0..=10.0).text("Frequency"));
                                }
                                RuleConfig::SpeedLimit { min, max } => {
                                    ui.add(egui::Slider::new(min, 0.0..=2.0).text("Min"));
                                    ui.add(egui::Slider::new(max, 0.0..=5.0).text("Max"));
                                }
                                RuleConfig::Custom { code, params } => {
                                    ui.label("WGSL Code:");
                                    ui.add(egui::TextEdit::multiline(code).code_editor().desired_rows(4));
                                    for (name, value) in params.iter_mut() {
                                        ui.horizontal(|ui| {
                                            ui.label(format!("{}:", name));
                                            ui.add(egui::DragValue::new(value).speed(0.01));
                                        });
                                    }
                                    if ui.small_button("+ Add Param").clicked() {
                                        params.push(("param".into(), 1.0));
                                    }
                                }
                                RuleConfig::BounceWalls | RuleConfig::WrapWalls => {
                                    ui.label("(no parameters)");
                                }
                            }
                        });

                        ui.separator();
                    }

                    // Handle reordering
                    if let Some(idx) = remove_idx {
                        self.config.rules.remove(idx);
                    }
                    if let Some(idx) = move_up_idx {
                        self.config.rules.swap(idx, idx - 1);
                    }
                    if let Some(idx) = move_down_idx {
                        self.config.rules.swap(idx, idx + 1);
                    }

                    // Add new rule
                    ui.heading("Add Rule");
                    egui::ComboBox::from_label("Type")
                        .show_index(ui, &mut self.new_rule_type, 11, |i| {
                            [
                                "Gravity", "Drag", "Bounce Walls", "Wrap Walls",
                                "Separate", "Cohere", "Align", "Attract To",
                                "Wander", "Speed Limit", "Custom WGSL"
                            ][i]
                        });

                    if ui.button("Add Rule").clicked() {
                        let new_rule = match self.new_rule_type {
                            0 => RuleConfig::Gravity(9.8),
                            1 => RuleConfig::Drag(1.0),
                            2 => RuleConfig::BounceWalls,
                            3 => RuleConfig::WrapWalls,
                            4 => RuleConfig::Separate { radius: 0.05, strength: 2.0 },
                            5 => RuleConfig::Cohere { radius: 0.15, strength: 1.0 },
                            6 => RuleConfig::Align { radius: 0.1, strength: 1.5 },
                            7 => RuleConfig::AttractTo { point: [0.0, 0.0, 0.0], strength: 1.0 },
                            8 => RuleConfig::Wander { strength: 1.0, frequency: 2.0 },
                            9 => RuleConfig::SpeedLimit { min: 0.0, max: 1.0 },
                            10 => RuleConfig::Custom {
                                code: "// Custom WGSL\np.velocity.y += 0.01;".into(),
                                params: vec![],
                            },
                            _ => RuleConfig::Gravity(9.8),
                        };
                        self.config.rules.push(new_rule);
                    }
                });
            });

        // Central panel - Preview/Help
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("RDPE Editor");
            ui.separator();

            ui.label("Configure your particle simulation using the panels on the left and right.");
            ui.label("");
            ui.label("Quick Start:");
            ui.label("1. Choose a preset from the menu, or configure from scratch");
            ui.label("2. Adjust spawn settings (shape, velocity, color)");
            ui.label("3. Add and configure rules");
            ui.label("4. Click 'Run' to launch the simulation");
            ui.label("");
            ui.label("In the simulation window:");
            ui.label("- Left-click + drag to rotate camera");
            ui.label("- Scroll to zoom");
            ui.label("- Click a particle to inspect it");
            ui.label("- Use the Rule Inspector for live parameter tweaks");

            ui.separator();
            ui.heading("Current Config Summary");
            ui.label(format!("Name: {}", self.config.name));
            ui.label(format!("Particles: {}", self.config.particle_count));
            ui.label(format!("Rules: {}", self.config.rules.len()));
        });

        // Request repaint for status updates
        ctx.request_repaint_after(std::time::Duration::from_millis(100));
    }
}

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1000.0, 700.0])
            .with_min_inner_size([800.0, 500.0]),
        ..Default::default()
    };

    eframe::run_native(
        "RDPE Editor",
        options,
        Box::new(|_cc| Ok(Box::new(EditorApp::default()))),
    )
}
