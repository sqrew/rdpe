//! RDPE Editor - Visual editor for particle simulations

use eframe::egui;
use rdpe_editor::config::*;
use rdpe_editor::ui::{render_rules_panel, render_spawn_panel, PRESETS};
use std::process::{Child, Command};

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([400.0, 800.0])
            .with_title("RDPE Editor"),
        ..Default::default()
    };

    eframe::run_native(
        "RDPE Editor",
        options,
        Box::new(|_cc| Ok(Box::new(EditorApp::default()))),
    )
}

struct EditorApp {
    config: SimConfig,
    current_file: Option<String>,
    simulation_process: Option<Child>,
    status_message: Option<(String, std::time::Instant)>,
}

impl Default for EditorApp {
    fn default() -> Self {
        Self {
            config: SimConfig::default(),
            current_file: None,
            simulation_process: None,
            status_message: None,
        }
    }
}

impl EditorApp {
    fn show_status(&mut self, msg: impl Into<String>) {
        self.status_message = Some((msg.into(), std::time::Instant::now()));
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

        // Find the runner binary
        let runner = if cfg!(debug_assertions) {
            "target/debug/rdpe-runner"
        } else {
            "target/release/rdpe-runner"
        };

        // Try to spawn the runner process
        match Command::new(runner)
            .arg(&temp_path)
            .spawn()
        {
            Ok(child) => {
                self.simulation_process = Some(child);
                self.show_status("Simulation started");
            }
            Err(e) => {
                // Try cargo run as fallback
                match Command::new("cargo")
                    .args(["run", "--bin", "rdpe-runner", "-p", "rdpe-editor", "--"])
                    .arg(&temp_path)
                    .spawn()
                {
                    Ok(child) => {
                        self.simulation_process = Some(child);
                        self.show_status("Simulation started (via cargo)");
                    }
                    Err(_) => {
                        self.show_status(format!("Failed to start simulation: {}", e));
                    }
                }
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
                    let is_running = self.simulation_process.is_some();

                    if is_running {
                        if ui.button("Stop").clicked() {
                            self.stop_simulation();
                        }
                    }

                    if ui.button(if is_running { "Restart" } else { "Run" }).clicked() {
                        self.run_simulation();
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
