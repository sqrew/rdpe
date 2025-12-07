//! RDPE Editor - Visual editor for particle simulations

use eframe::egui;
use rdpe_editor::config::*;
use rdpe_editor::ui::{render_rules_panel, render_spawn_panel, PRESETS};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([420.0, 850.0])
            .with_title("RDPE Editor"),
        ..Default::default()
    };

    eframe::run_native(
        "RDPE Editor",
        options,
        Box::new(|_cc| Ok(Box::new(EditorApp::new()))),
    )
}

struct EditorApp {
    config: SimConfig,
    current_file: Option<String>,
    simulation_process: Option<Child>,
    status_message: Option<(String, std::time::Instant)>,
    workspace_root: Option<PathBuf>,
}

impl EditorApp {
    fn new() -> Self {
        // Find workspace root by looking for Cargo.toml with [workspace]
        let workspace_root = Self::find_workspace_root();

        Self {
            config: SimConfig::default(),
            current_file: None,
            simulation_process: None,
            status_message: None,
            workspace_root,
        }
    }

    fn find_workspace_root() -> Option<PathBuf> {
        // Start from current exe location or current dir
        let start = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .or_else(|| std::env::current_dir().ok())?;

        let mut current = start.as_path();

        // Walk up looking for workspace Cargo.toml
        loop {
            let cargo_toml = current.join("Cargo.toml");
            if cargo_toml.exists() {
                if let Ok(contents) = std::fs::read_to_string(&cargo_toml) {
                    if contents.contains("[workspace]") {
                        return Some(current.to_path_buf());
                    }
                }
            }

            current = current.parent()?;
        }
    }
}

impl EditorApp {
    fn show_status(&mut self, msg: impl Into<String>) {
        self.status_message = Some((msg.into(), std::time::Instant::now()));
    }

    fn is_running(&self) -> bool {
        self.simulation_process.is_some()
    }

    fn run_simulation(&mut self) {
        // Kill existing simulation if running
        self.stop_simulation();

        // Save config to temp file
        let temp_path = std::env::temp_dir().join("rdpe_editor_temp.json");
        if let Err(e) = self.config.save(&temp_path) {
            self.show_status(format!("Failed to save temp config: {}", e));
            return;
        }

        // Try different methods to launch the runner
        if let Some(ref workspace) = self.workspace_root {
            // Method 1: Direct binary path
            let profile = if cfg!(debug_assertions) { "debug" } else { "release" };
            let runner_path = workspace.join("target").join(profile).join("rdpe-runner");

            if runner_path.exists() {
                match Command::new(&runner_path)
                    .arg(&temp_path)
                    .current_dir(workspace)
                    .stdout(Stdio::inherit())
                    .stderr(Stdio::inherit())
                    .spawn()
                {
                    Ok(child) => {
                        self.simulation_process = Some(child);
                        self.show_status("Simulation started");
                        return;
                    }
                    Err(_) => {}
                }
            }

            // Method 2: cargo run from workspace
            match Command::new("cargo")
                .args(["run", "--bin", "rdpe-runner", "-p", "rdpe-editor", "--"])
                .arg(&temp_path)
                .current_dir(workspace)
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .spawn()
            {
                Ok(child) => {
                    self.simulation_process = Some(child);
                    self.show_status("Simulation started (building...)");
                    return;
                }
                Err(e) => {
                    self.show_status(format!("Failed to start: {}", e));
                    return;
                }
            }
        }

        // Fallback: try cargo run from current directory
        match Command::new("cargo")
            .args(["run", "--bin", "rdpe-runner", "-p", "rdpe-editor", "--"])
            .arg(&temp_path)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()
        {
            Ok(child) => {
                self.simulation_process = Some(child);
                self.show_status("Simulation started (building...)");
            }
            Err(e) => {
                self.show_status(format!("Failed to start simulation: {}", e));
            }
        }
    }

    fn stop_simulation(&mut self) {
        if let Some(mut child) = self.simulation_process.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }

    fn save_config(&mut self) {
        if let Some(path) = &self.current_file {
            match self.config.save(path) {
                Ok(()) => self.show_status(format!("Saved to {}", path)),
                Err(e) => self.show_status(format!("Save failed: {}", e)),
            }
        } else {
            self.save_config_as();
        }
    }

    fn save_config_as(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("JSON", &["json"])
            .set_file_name(&format!("{}.json", self.config.name))
            .save_file()
        {
            let path_str = path.display().to_string();
            match self.config.save(&path) {
                Ok(()) => {
                    self.show_status(format!("Saved to {}", path_str));
                    self.current_file = Some(path_str);
                }
                Err(e) => self.show_status(format!("Save failed: {}", e)),
            }
        }
    }

    fn load_config(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("JSON", &["json"])
            .pick_file()
        {
            let path_str = path.display().to_string();
            match SimConfig::load(&path) {
                Ok(config) => {
                    self.config = config;
                    self.current_file = Some(path_str.clone());
                    self.show_status(format!("Loaded {}", path_str));
                }
                Err(e) => self.show_status(format!("Load failed: {}", e)),
            }
        }
    }
}

impl eframe::App for EditorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Check if simulation process has exited
        if let Some(child) = &mut self.simulation_process {
            match child.try_wait() {
                Ok(Some(_status)) => {
                    self.simulation_process = None;
                }
                Ok(None) => {} // Still running
                Err(_) => {
                    self.simulation_process = None;
                }
            }
        }

        // Menu bar
        egui::TopBottomPanel::top("menu").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("New").clicked() {
                        self.config = SimConfig::default();
                        self.current_file = None;
                        ui.close_menu();
                    }
                    if ui.button("Open...").clicked() {
                        self.load_config();
                        ui.close_menu();
                    }
                    if ui.button("Save").clicked() {
                        self.save_config();
                        ui.close_menu();
                    }
                    if ui.button("Save As...").clicked() {
                        self.save_config_as();
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Quit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });

                ui.menu_button("Presets", |ui| {
                    for preset in PRESETS {
                        if ui.button(preset.name).on_hover_text(preset.description).clicked() {
                            self.config = (preset.config)();
                            self.current_file = None;
                            self.show_status(format!("Loaded preset: {}", preset.name));
                            ui.close_menu();
                        }
                    }
                });

                // Spacer
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let is_running = self.is_running();

                    // Stop button (red-ish)
                    if is_running {
                        if ui.add(egui::Button::new("Stop").fill(egui::Color32::from_rgb(180, 60, 60))).clicked() {
                            self.stop_simulation();
                        }
                    }

                    // Run/Restart button (green-ish)
                    let run_text = if is_running { "Restart" } else { "Run" };
                    let run_color = if is_running {
                        egui::Color32::from_rgb(60, 120, 180)
                    } else {
                        egui::Color32::from_rgb(60, 160, 60)
                    };

                    if ui.add(egui::Button::new(run_text).fill(run_color)).clicked() {
                        self.run_simulation();
                    }

                    // Running indicator
                    if is_running {
                        ui.label(egui::RichText::new("Running").color(egui::Color32::GREEN));
                    }
                });
            });
        });

        // Status bar
        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.horizontal(|ui| {
                // Status message with timeout
                if let Some((msg, time)) = &self.status_message {
                    if time.elapsed().as_secs() < 5 {
                        ui.label(msg);
                    } else {
                        self.status_message = None;
                    }
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Show current file
                    if let Some(file) = &self.current_file {
                        ui.label(egui::RichText::new(file).small().weak());
                    } else {
                        ui.label(egui::RichText::new("(unsaved)").small().weak());
                    }
                });
            });
        });

        // Main content
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                // Simulation name
                ui.horizontal(|ui| {
                    ui.label("Name:");
                    ui.text_edit_singleline(&mut self.config.name);
                });
                ui.separator();

                // Spawn panel
                render_spawn_panel(ui, &mut self.config);

                ui.separator();

                // Spatial settings (if needed)
                if self.config.needs_spatial() {
                    ui.heading("Spatial Hashing");
                    ui.add(egui::Slider::new(&mut self.config.spatial_cell_size, 0.01..=0.5)
                        .text("Cell Size"));
                    ui.add(egui::Slider::new(&mut self.config.spatial_resolution, 8..=128)
                        .text("Resolution"));
                    ui.separator();
                }

                // Rules panel
                render_rules_panel(ui, &mut self.config.rules);
            });
        });
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.stop_simulation();
    }
}
