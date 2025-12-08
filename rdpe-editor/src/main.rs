//! RDPE Editor - Visual editor for particle simulations
//!
//! This version embeds the simulation directly in the editor window
//! for live visual tweaking.

use eframe::egui;
use glam::Vec3;
use rdpe_editor::config::*;
use rdpe_editor::embedded::{EmbeddedSimulation, SimulationResources, ParsedParticle};
use rdpe_editor::ui::{
    render_custom_panel, render_effects_panel, render_export_button, render_export_window,
    render_fields_panel, render_rules_panel, render_spawn_panel, render_visuals_panel,
    render_volume_panel, AddUniformState, ExportPanelState, PRESETS,
};

/// Sidebar tabs for organizing the editor panels
#[derive(Default, Clone, Copy, PartialEq, Eq)]
enum SidebarTab {
    #[default]
    Spawn,
    Rules,
    Fields,
    Visuals,
    Custom,
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_title("RDPE Editor"),
        // Use wgpu renderer for custom painting
        renderer: eframe::Renderer::Wgpu,
        wgpu_options: eframe::egui_wgpu::WgpuConfiguration::default(),
        ..Default::default()
    };

    eframe::run_native(
        "RDPE Editor",
        options,
        Box::new(|cc| {
            // Initialize with creation context to get wgpu state
            Ok(Box::new(EditorApp::new(cc)))
        }),
    )
}

struct EditorApp {
    config: SimConfig,
    /// Config that's currently running in the simulation
    applied_config: SimConfig,
    /// Config from last frame (for detecting changes)
    previous_config: SimConfig,
    current_file: Option<String>,
    status_message: Option<(String, std::time::Instant)>,
    simulation: EmbeddedSimulation,
    needs_rebuild: bool,
    needs_reset: bool,
    /// Track previous background color for live updates
    last_background_color: [f32; 3],
    /// State for the add uniform UI
    add_uniform_state: AddUniformState,
    /// State for the export panel
    export_panel_state: ExportPanelState,
    /// Currently selected sidebar tab
    selected_tab: SidebarTab,
    /// Debounce timer for auto-rebuild (seconds remaining)
    rebuild_timer: Option<f32>,
}

impl EditorApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let config = SimConfig::default();
        let mut simulation = EmbeddedSimulation::new();

        // Initialize simulation with wgpu render state
        if let Some(ref wgpu_render_state) = cc.wgpu_render_state {
            simulation.initialize(wgpu_render_state, &config);
        }

        let last_background_color = config.visuals.background_color;
        let applied_config = config.clone();
        let previous_config = config.clone();

        Self {
            config,
            applied_config,
            previous_config,
            current_file: None,
            status_message: None,
            simulation,
            needs_rebuild: false,
            needs_reset: false,
            last_background_color,
            add_uniform_state: AddUniformState::default(),
            export_panel_state: ExportPanelState::default(),
            selected_tab: SidebarTab::default(),
            rebuild_timer: None,
        }
    }
}

impl EditorApp {
    fn show_status(&mut self, msg: impl Into<String>) {
        self.status_message = Some((msg.into(), std::time::Instant::now()));
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
                    self.needs_rebuild = true;
                    self.show_status(format!("Loaded {}", path_str));
                }
                Err(e) => self.show_status(format!("Load failed: {}", e)),
            }
        }
    }

    fn rebuild_simulation(&mut self, wgpu_render_state: &egui_wgpu::RenderState) {
        // Reinitialize with state preservation (if particle count unchanged)
        self.simulation.reinitialize(wgpu_render_state, &self.config);
        self.needs_rebuild = false;
        self.rebuild_timer = None;

        if self.simulation.shader_error().is_some() {
            self.show_status("Rebuild failed: shader error");
        } else {
            // Update applied config on success
            self.applied_config = self.config.clone();
            self.show_status("Simulation rebuilt");
        }
    }

    fn reset_simulation(&mut self, wgpu_render_state: &egui_wgpu::RenderState) {
        // Full reset: regenerate all particles
        self.simulation.reset(wgpu_render_state, &self.config);
        self.needs_reset = false;
        self.rebuild_timer = None;

        if self.simulation.shader_error().is_some() {
            self.show_status("Reset failed: shader error");
        } else {
            // Update applied config on success
            self.applied_config = self.config.clone();
            self.show_status("Simulation reset");
        }
    }
}

/// Debounce delay for auto-rebuild in seconds
const REBUILD_DEBOUNCE: f32 = 0.4;

impl eframe::App for EditorApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // Get wgpu render state for the viewport
        let wgpu_render_state = frame.wgpu_render_state();
        let delta_time = ctx.input(|i| i.stable_dt);

        // Auto-rebuild: detect config changes from previous frame and start/reset debounce timer
        // Compare against previous_config (not applied_config) so we only reset timer on actual changes
        let config_changed = {
            // Check everything except background_color and custom_uniforms (which are hot-swapped)
            self.config.name != self.previous_config.name
                || self.config.particle_count != self.previous_config.particle_count
                || self.config.bounds != self.previous_config.bounds
                || self.config.particle_size != self.previous_config.particle_size
                || self.config.spatial_cell_size != self.previous_config.spatial_cell_size
                || self.config.spatial_resolution != self.previous_config.spatial_resolution
                || self.config.spawn != self.previous_config.spawn
                || self.config.rules != self.previous_config.rules
                || self.config.vertex_effects != self.previous_config.vertex_effects
                || self.config.visuals.blend_mode != self.previous_config.visuals.blend_mode
                || self.config.visuals.shape != self.previous_config.visuals.shape
                || self.config.visuals.palette != self.previous_config.visuals.palette
                || self.config.visuals.color_mapping != self.previous_config.visuals.color_mapping
                || self.config.visuals.trail_length != self.previous_config.visuals.trail_length
                || self.config.visuals.connections_enabled != self.previous_config.visuals.connections_enabled
                || self.config.visuals.connections_radius != self.previous_config.visuals.connections_radius
                || self.config.visuals.velocity_stretch != self.previous_config.visuals.velocity_stretch
                || self.config.visuals.velocity_stretch_factor != self.previous_config.visuals.velocity_stretch_factor
                || self.config.visuals.spatial_grid_opacity != self.previous_config.visuals.spatial_grid_opacity
                || self.config.visuals.wireframe != self.previous_config.visuals.wireframe
                || self.config.visuals.wireframe_thickness != self.previous_config.visuals.wireframe_thickness
                || self.config.custom_shaders != self.previous_config.custom_shaders
                || self.config.fields != self.previous_config.fields
                || self.config.volume_render != self.previous_config.volume_render
        };

        if config_changed {
            // Start or reset debounce timer when config changes
            self.rebuild_timer = Some(REBUILD_DEBOUNCE);
            // Update previous_config to track this change
            self.previous_config = self.config.clone();
        }

        // Tick down rebuild timer
        if let Some(ref mut timer) = self.rebuild_timer {
            *timer -= delta_time;
            if *timer <= 0.0 {
                self.needs_rebuild = true;
                self.rebuild_timer = None;
            }
        }

        // Check if rebuild needed (either from timer or manual)
        if self.needs_rebuild {
            if let Some(ref state) = wgpu_render_state {
                self.rebuild_simulation(state);
            }
        }

        // Check if full reset needed
        if self.needs_reset {
            if let Some(ref state) = wgpu_render_state {
                self.reset_simulation(state);
            }
        }

        // Live update: background color (hot-swappable)
        if self.config.visuals.background_color != self.last_background_color {
            if let Some(ref state) = wgpu_render_state {
                if let Some(sim) = state.renderer.write().callback_resources.get_mut::<rdpe_editor::embedded::SimulationResources>() {
                    sim.set_background_color(Vec3::from_array(self.config.visuals.background_color));
                }
            }
            self.last_background_color = self.config.visuals.background_color;
        }

        // Live update: custom uniform values (hot-swappable)
        if let Some(ref state) = wgpu_render_state {
            if let Some(sim) = state.renderer.write().callback_resources.get_mut::<rdpe_editor::embedded::SimulationResources>() {
                sim.sync_custom_uniforms(&self.config.custom_uniforms);
            }
        }

        // Export window (floating)
        render_export_window(ctx, &mut self.export_panel_state, &self.config, delta_time);

        // Menu bar
        egui::TopBottomPanel::top("menu").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("New").clicked() {
                        self.config = SimConfig::default();
                        self.current_file = None;
                        self.needs_rebuild = true;
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
                            self.needs_rebuild = true;
                            self.show_status(format!("Loaded preset: {}", preset.name));
                            ui.close_menu();
                        }
                    }
                });

                // Spacer
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Reset button (full reset with fresh particles)
                    if ui.button("Reset").on_hover_text("Full reset: regenerate all particles").clicked() {
                        self.needs_reset = true;
                    }

                    // Show pending rebuild indicator
                    if self.rebuild_timer.is_some() {
                        ui.label(egui::RichText::new("⟳").color(egui::Color32::YELLOW))
                            .on_hover_text("Rebuild pending...");
                    }

                    // Pause/Play
                    if let Some(ref state) = wgpu_render_state {
                        let is_paused = state.renderer.read()
                            .callback_resources
                            .get::<rdpe_editor::embedded::SimulationResources>()
                            .map(|s| s.is_paused())
                            .unwrap_or(false);

                        let btn_text = if is_paused { "▶ Play" } else { "⏸ Pause" };
                        if ui.button(btn_text).clicked() {
                            if let Some(sim) = state.renderer.write()
                                .callback_resources
                                .get_mut::<rdpe_editor::embedded::SimulationResources>()
                            {
                                sim.set_paused(!is_paused);
                            }
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
                    // FPS counter
                    ui.label(format!("{:.0} FPS", 1.0 / ctx.input(|i| i.stable_dt)));

                    ui.separator();

                    // Show current file
                    if let Some(file) = &self.current_file {
                        ui.label(egui::RichText::new(file).small().weak());
                    } else {
                        ui.label(egui::RichText::new("(unsaved)").small().weak());
                    }
                });
            });
        });

        // Particle Inspector panel (shows when a particle is selected)
        let selected_particle = wgpu_render_state.as_ref().and_then(|state| {
            state.renderer.read().callback_resources.get::<SimulationResources>()
                .and_then(|sim| {
                    let idx = sim.selected_particle()?;
                    let data = sim.selected_particle_data()?;
                    let parsed = ParsedParticle::from_bytes(data)?;
                    Some((idx, parsed))
                })
        });

        if let Some((idx, particle)) = selected_particle {
            egui::TopBottomPanel::bottom("particle_inspector")
                .resizable(true)
                .min_height(60.0)
                .max_height(200.0)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.heading(format!("Particle #{}", idx));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.small_button("Clear Selection").clicked() {
                                if let Some(ref state) = wgpu_render_state {
                                    if let Some(sim) = state.renderer.write().callback_resources.get_mut::<SimulationResources>() {
                                        sim.clear_selection();
                                    }
                                }
                            }
                        });
                    });
                    ui.separator();

                    ui.horizontal(|ui| {
                        // Left column
                        ui.vertical(|ui| {
                            ui.label(format!("Position: ({:.3}, {:.3}, {:.3})",
                                particle.position[0], particle.position[1], particle.position[2]));
                            ui.label(format!("Velocity: ({:.3}, {:.3}, {:.3})",
                                particle.velocity[0], particle.velocity[1], particle.velocity[2]));
                            ui.label(format!("Goal: ({:.3}, {:.3}, {:.3})",
                                particle.goal[0], particle.goal[1], particle.goal[2]));
                        });

                        ui.separator();

                        // Middle column
                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                ui.label("Color:");
                                let color = egui::Color32::from_rgb(
                                    (particle.color[0] * 255.0) as u8,
                                    (particle.color[1] * 255.0) as u8,
                                    (particle.color[2] * 255.0) as u8,
                                );
                                ui.colored_label(color, format!("({:.2}, {:.2}, {:.2})",
                                    particle.color[0], particle.color[1], particle.color[2]));
                            });
                            ui.label(format!("Type: {}", particle.particle_type));
                            ui.label(format!("Alive: {} | Scale: {:.2}", particle.alive, particle.scale));
                        });

                        ui.separator();

                        // Right column
                        ui.vertical(|ui| {
                            ui.label(format!("Mass: {:.3}", particle.mass));
                            ui.label(format!("Energy: {:.3}", particle.energy));
                            ui.label(format!("Heat: {:.3}", particle.heat));
                            ui.label(format!("Custom: {:.3}", particle.custom));
                        });
                    });
                });
        }

        // Right panel: Settings with tabs
        egui::SidePanel::right("settings")
            .min_width(350.0)
            .default_width(400.0)
            .show(ctx, |ui| {
                // Simulation name at top (always visible)
                ui.horizontal(|ui| {
                    ui.label("Name:");
                    ui.text_edit_singleline(&mut self.config.name);
                });
                ui.separator();

                // Tab bar
                ui.horizontal_wrapped(|ui| {
                    ui.selectable_value(&mut self.selected_tab, SidebarTab::Spawn, "Spawn");
                    ui.selectable_value(&mut self.selected_tab, SidebarTab::Rules, "Rules");
                    ui.selectable_value(&mut self.selected_tab, SidebarTab::Fields, "Fields");
                    ui.selectable_value(&mut self.selected_tab, SidebarTab::Visuals, "Visuals");
                    ui.selectable_value(&mut self.selected_tab, SidebarTab::Custom, "Custom");
                });
                ui.separator();

                // Tab content
                egui::ScrollArea::vertical().show(ui, |ui| {
                    match self.selected_tab {
                        SidebarTab::Spawn => {
                            render_spawn_panel(ui, &mut self.config);

                            // Spatial settings (if needed)
                            if self.config.needs_spatial() {
                                ui.separator();
                                ui.heading("Spatial Hashing");
                                ui.add(egui::Slider::new(&mut self.config.spatial_cell_size, 0.01..=0.5)
                                    .text("Cell Size"));

                                // Resolution must be a power of 2
                                const VALID_RESOLUTIONS: &[u32] = &[8, 16, 32, 64, 128];
                                let mut res_idx = VALID_RESOLUTIONS
                                    .iter()
                                    .position(|&r| r == self.config.spatial_resolution)
                                    .unwrap_or(2); // Default to 32

                                egui::ComboBox::from_label("Resolution")
                                    .selected_text(format!("{}", VALID_RESOLUTIONS[res_idx]))
                                    .show_ui(ui, |ui| {
                                        for (i, &res) in VALID_RESOLUTIONS.iter().enumerate() {
                                            if ui.selectable_value(&mut res_idx, i, format!("{}", res)).clicked() {
                                                self.config.spatial_resolution = res;
                                            }
                                        }
                                    });
                            }
                        }
                        SidebarTab::Rules => {
                            render_rules_panel(ui, &mut self.config.rules);
                        }
                        SidebarTab::Fields => {
                            render_fields_panel(ui, &mut self.config.fields);

                            ui.separator();

                            // Volume rendering panel
                            let num_fields = self.config.fields.len();
                            render_volume_panel(ui, &mut self.config.volume_render, num_fields);
                        }
                        SidebarTab::Visuals => {
                            render_visuals_panel(ui, &mut self.config.visuals);

                            ui.separator();

                            // Vertex effects
                            render_effects_panel(ui, &mut self.config.vertex_effects);
                        }
                        SidebarTab::Custom => {
                            render_custom_panel(
                                ui,
                                &mut self.config.custom_uniforms,
                                &mut self.config.custom_shaders,
                                &mut self.add_uniform_state,
                            );

                            ui.separator();

                            // Export button
                            ui.horizontal(|ui| {
                                render_export_button(ui, &mut self.export_panel_state, &self.config);
                            });
                        }
                    }
                });
            });

        // Central panel: Simulation viewport
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(egui::Color32::from_rgb(
                (self.config.visuals.background_color[0] * 255.0) as u8,
                (self.config.visuals.background_color[1] * 255.0) as u8,
                (self.config.visuals.background_color[2] * 255.0) as u8,
            )))
            .show(ctx, |ui| {
                // Show the simulation viewport
                if let Some(ref state) = wgpu_render_state {
                    self.simulation.show(ui, state);
                } else {
                    ui.centered_and_justified(|ui| {
                        ui.label("wgpu not available - simulation requires GPU");
                    });
                }

                // Show shader error overlay if there's an error
                if let Some(error) = self.simulation.shader_error().map(|s| s.to_string()) {
                    let screen_rect = ui.ctx().screen_rect();
                    let mut should_clear = false;

                    egui::Area::new(egui::Id::new("shader_error_overlay"))
                        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                        .show(ui.ctx(), |ui| {
                            egui::Frame::popup(ui.style())
                                .fill(egui::Color32::from_rgba_unmultiplied(40, 0, 0, 230))
                                .stroke(egui::Stroke::new(2.0, egui::Color32::RED))
                                .inner_margin(16.0)
                                .show(ui, |ui| {
                                    ui.set_max_width(screen_rect.width() * 0.7);

                                    ui.horizontal(|ui| {
                                        ui.heading(egui::RichText::new("Shader Error").color(egui::Color32::RED));
                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                            if ui.button("Dismiss").clicked() {
                                                should_clear = true;
                                            }
                                        });
                                    });

                                    ui.separator();

                                    egui::ScrollArea::vertical()
                                        .max_height(screen_rect.height() * 0.5)
                                        .show(ui, |ui| {
                                            ui.add(egui::Label::new(
                                                egui::RichText::new(&error)
                                                    .monospace()
                                                    .color(egui::Color32::LIGHT_RED)
                                            ).wrap());
                                        });

                                    ui.separator();

                                    ui.label(
                                        egui::RichText::new("Fix the error in your custom shader code - will auto-rebuild when corrected")
                                            .small()
                                            .italics()
                                            .color(egui::Color32::GRAY)
                                    );
                                });
                        });

                    if should_clear {
                        self.simulation.clear_error();
                    }
                }
            });
    }
}
