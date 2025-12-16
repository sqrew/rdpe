//! RDPE Editor - Visual editor for particle simulations
//!
//! This version embeds the simulation directly in the editor window
//! for live visual tweaking.

use eframe::egui;
use glam::Vec3;

// Use web-time on WASM for Instant compatibility
#[cfg(target_arch = "wasm32")]
use web_time::Instant;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;
use rdpe_editor::config::*;
use rdpe_editor::embedded::{EmbeddedSimulation, SimulationResources, ParsedParticle};
use rdpe_editor::ui::{
    render_custom_panel, render_effects_panel, render_emitters_panel, render_export_button,
    render_export_window, render_fields_panel, render_interactions_panel, render_mouse_panel,
    render_particle_fields_panel, render_post_process_panel, render_rules_panel, render_spawn_panel,
    render_stats_panel, render_visuals_panel, render_volume_panel, AddUniformState, ExportPanelState,
    InteractionsPanelState, ShaderContext, PRESETS,
};

/// Sidebar tabs for organizing the editor panels
#[derive(Default, Clone, Copy, PartialEq, Eq)]
enum SidebarTab {
    #[default]
    Spawn,
    Rules,
    Interactions,
    Emitters,
    Particle,
    Fields,
    Visuals,
    PostProcess,
    Mouse,
    Custom,
    Stats,
}

/// Which side the settings panel is on
#[derive(Default, Clone, Copy, PartialEq, Eq)]
enum PanelSide {
    #[default]
    Left,
    Right,
}

/// Editor appearance settings (not persisted)
#[derive(Clone)]
struct EditorSettings {
    /// Which side the panel is on
    panel_side: PanelSide,
    /// Accent color hue (0.0-1.0)
    accent_hue: f32,
    /// Background color hue (0.0-1.0)
    background_hue: f32,
    /// Background tint strength (0.0 = no tint, 1.0 = full tint)
    background_tint: f32,
    /// Dark mode (true) or light mode (false)
    dark_mode: bool,
    /// UI scale factor (1.0 = default)
    ui_scale: f32,
    /// Panel opacity (0.0 = transparent, 1.0 = opaque)
    panel_opacity: f32,
    /// Brightness adjustment (0.5 = darker, 1.0 = default, 1.5 = brighter)
    brightness: f32,
    /// Whether the options popup is open
    show_options: bool,
}

impl Default for EditorSettings {
    fn default() -> Self {
        Self {
            panel_side: PanelSide::Left,
            accent_hue: 0.6, // Default blue-ish
            background_hue: 0.6,
            background_tint: 0.0, // No tint by default
            dark_mode: true,
            ui_scale: 1.0,
            panel_opacity: 1.0, // Fully opaque by default
            brightness: 1.0,
            show_options: false,
        }
    }
}

// ============================================================================
// Native entry point
// ============================================================================

#[cfg(not(target_arch = "wasm32"))]
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

// ============================================================================
// WASM entry point
// ============================================================================

#[cfg(target_arch = "wasm32")]
fn main() {
    use wasm_bindgen::JsCast;

    // Redirect panic messages to console.error
    console_error_panic_hook::set_once();

    wasm_bindgen_futures::spawn_local(async {
        // Create WebGPU instance and adapter
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::BROWSER_WEBGPU,
            ..Default::default()
        });

        let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        }).await.expect("WebGPU adapter required");

        // Request device from adapter
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("RDPE Device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::downlevel_webgl2_defaults(),
                    memory_hints: wgpu::MemoryHints::default(),
                },
                None,
            )
            .await
            .expect("Failed to create device");

        // Configure eframe to use our existing wgpu setup
        use eframe::egui_wgpu::{WgpuConfiguration, WgpuSetup, WgpuSetupExisting};

        let web_options = eframe::WebOptions {
            wgpu_options: WgpuConfiguration {
                wgpu_setup: WgpuSetup::Existing(WgpuSetupExisting {
                    instance,
                    adapter,
                    device,
                    queue,
                }),
                ..Default::default()
            },
            ..Default::default()
        };

        // Get the canvas element from the DOM
        let document = web_sys::window()
            .expect("No window")
            .document()
            .expect("No document");

        let canvas = document
            .get_element_by_id("rdpe-canvas")
            .expect("No canvas element with id 'rdpe-canvas'")
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .expect("Element is not a canvas");

        let result = eframe::WebRunner::new()
            .start(
                canvas,
                web_options,
                Box::new(|cc| Ok(Box::new(EditorApp::new(cc)))),
            )
            .await;

        if let Err(e) = result {
            web_sys::console::error_1(&format!("RDPE: eframe failed: {:?}", e).into());
        }
    });
}

struct EditorApp {
    config: SimConfig,
    /// Config that's currently running in the simulation
    applied_config: SimConfig,
    /// Config from last frame (for detecting changes)
    previous_config: SimConfig,
    current_file: Option<String>,
    status_message: Option<(String, Instant)>,
    simulation: EmbeddedSimulation,
    needs_rebuild: bool,
    needs_reset: bool,
    /// Track previous background color for live updates
    last_background_color: [f32; 3],
    /// Track previous grid opacity for live updates
    last_grid_opacity: f32,
    /// Track previous glyph config for live updates
    last_glyph_config: GlyphsConfig,
    /// State for the add uniform UI
    add_uniform_state: AddUniformState,
    /// State for the export panel
    export_panel_state: ExportPanelState,
    /// State for the interactions (rule matrix) panel
    interactions_panel_state: InteractionsPanelState,
    /// Currently selected sidebar tab
    selected_tab: SidebarTab,
    /// Debounce timer for auto-rebuild (seconds remaining)
    rebuild_timer: Option<f32>,
    /// Editable copy of selected particle (for live editing)
    editing_particle: Option<(u32, ParsedParticle)>,
    /// Fullscreen mode - hides all UI panels
    fullscreen_mode: bool,
    /// Editor appearance settings
    editor_settings: EditorSettings,
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
        let last_grid_opacity = config.visuals.spatial_grid_opacity;
        let last_glyph_config = config.visuals.glyphs.clone();
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
            last_grid_opacity,
            last_glyph_config,
            add_uniform_state: AddUniformState::default(),
            export_panel_state: ExportPanelState::default(),
            interactions_panel_state: InteractionsPanelState::default(),
            selected_tab: SidebarTab::default(),
            rebuild_timer: None,
            editing_particle: None,
            fullscreen_mode: false,
            editor_settings: EditorSettings::default(),
        }
    }
}

impl EditorApp {
    fn show_status(&mut self, msg: impl Into<String>) {
        self.status_message = Some((msg.into(), Instant::now()));
    }

    // ========================================================================
    // Native file operations (using rfd)
    // ========================================================================

    #[cfg(not(target_arch = "wasm32"))]
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

    #[cfg(not(target_arch = "wasm32"))]
    fn save_config_as(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("JSON", &["json"])
            .set_file_name(format!("{}.json", self.config.name))
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

    #[cfg(not(target_arch = "wasm32"))]
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

    // ========================================================================
    // WASM file operations (using browser APIs)
    // ========================================================================

    #[cfg(target_arch = "wasm32")]
    fn save_config(&mut self) {
        // On web, always do "Save As" (download)
        self.save_config_as();
    }

    #[cfg(target_arch = "wasm32")]
    fn save_config_as(&mut self) {
        use wasm_bindgen::JsCast;

        let json = match serde_json::to_string_pretty(&self.config) {
            Ok(j) => j,
            Err(e) => {
                self.show_status(format!("Save failed: {}", e));
                return;
            }
        };

        // Create a blob and trigger download
        let window = match web_sys::window() {
            Some(w) => w,
            None => return,
        };
        let document = match window.document() {
            Some(d) => d,
            None => return,
        };

        let blob_parts = js_sys::Array::new();
        blob_parts.push(&json.into());

        let options = web_sys::BlobPropertyBag::new();
        options.set_type("application/json");

        if let Ok(blob) = web_sys::Blob::new_with_str_sequence_and_options(&blob_parts, &options) {
            if let Ok(url) = web_sys::Url::create_object_url_with_blob(&blob) {
                if let Ok(anchor) = document.create_element("a") {
                    let anchor: web_sys::HtmlAnchorElement = anchor.unchecked_into();
                    anchor.set_href(&url);
                    anchor.set_download(&format!("{}.json", self.config.name));
                    anchor.click();
                    let _ = web_sys::Url::revoke_object_url(&url);
                    self.show_status("Downloaded config file");
                }
            }
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn load_config(&mut self) {
        // For WASM, we need async file reading. We'll trigger a file input click
        // and handle the result via a callback. For simplicity, show a message.
        self.show_status("Use drag-and-drop or paste JSON in Custom tab");
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

        // Reset tracking state so configs get re-applied to new SimulationResources
        self.last_glyph_config = GlyphsConfig::default();
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

        // Reset tracking state so configs get re-applied to new SimulationResources
        self.last_glyph_config = GlyphsConfig::default();
    }
}

/// Debounce delay for auto-rebuild in seconds
const REBUILD_DEBOUNCE: f32 = 0.4;

impl eframe::App for EditorApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // Get wgpu render state for the viewport
        let wgpu_render_state = frame.wgpu_render_state();
        let delta_time = ctx.input(|i| i.stable_dt);

        // Apply UI scale
        ctx.set_pixels_per_point(self.editor_settings.ui_scale);

        // Apply theme settings to egui visuals
        {
            let settings = &self.editor_settings;

            // Start with base dark or light theme
            let mut visuals = if settings.dark_mode {
                egui::Visuals::dark()
            } else {
                egui::Visuals::light()
            };

            // Apply background tint and/or brightness
            let brightness = settings.brightness;
            let base_value = if settings.dark_mode { 0.12 } else { 0.97 };
            let adjusted_base = (base_value * brightness).clamp(0.05, 0.98);

            if settings.background_tint > 0.0 {
                let tinted_sat = 0.25 * settings.background_tint;

                // Create tinted background colors with brightness
                let bg_color = egui::Color32::from(egui::ecolor::Hsva::new(
                    settings.background_hue, tinted_sat, adjusted_base, 1.0
                ));
                let bg_darker = egui::Color32::from(egui::ecolor::Hsva::new(
                    settings.background_hue, tinted_sat, (adjusted_base * 0.85).clamp(0.02, 0.95), 1.0
                ));
                let bg_lighter = egui::Color32::from(egui::ecolor::Hsva::new(
                    settings.background_hue, tinted_sat * 0.7, (adjusted_base * 1.15).clamp(0.05, 0.98), 1.0
                ));

                // Apply to various background elements
                visuals.panel_fill = bg_color;
                visuals.window_fill = bg_color;
                visuals.extreme_bg_color = bg_darker;
                visuals.faint_bg_color = bg_lighter;
                visuals.widgets.noninteractive.bg_fill = bg_darker;
                visuals.widgets.inactive.bg_fill = bg_darker;
            } else if (brightness - 1.0).abs() > 0.01 {
                // No tint but brightness is adjusted - apply brightness to default colors
                let adjust_color = |color: egui::Color32| -> egui::Color32 {
                    let hsva = egui::ecolor::Hsva::from(color);
                    let new_value = (hsva.v * brightness).clamp(0.02, 0.98);
                    egui::Color32::from(egui::ecolor::Hsva::new(hsva.h, hsva.s, new_value, hsva.a))
                };

                visuals.panel_fill = adjust_color(visuals.panel_fill);
                visuals.window_fill = adjust_color(visuals.window_fill);
                visuals.extreme_bg_color = adjust_color(visuals.extreme_bg_color);
                visuals.faint_bg_color = adjust_color(visuals.faint_bg_color);
                visuals.widgets.noninteractive.bg_fill = adjust_color(visuals.widgets.noninteractive.bg_fill);
                visuals.widgets.inactive.bg_fill = adjust_color(visuals.widgets.inactive.bg_fill);
            }

            // Apply panel opacity
            if settings.panel_opacity < 1.0 {
                let alpha = (settings.panel_opacity * 255.0) as u8;
                let [r, g, b, _] = visuals.panel_fill.to_array();
                visuals.panel_fill = egui::Color32::from_rgba_unmultiplied(r, g, b, alpha);
                let [r, g, b, _] = visuals.window_fill.to_array();
                visuals.window_fill = egui::Color32::from_rgba_unmultiplied(r, g, b, alpha);
            }

            // Apply accent color
            let accent = egui::ecolor::Hsva::new(settings.accent_hue, 0.7, 0.8, 1.0);
            let accent_color = egui::Color32::from(accent);
            let accent_dim = egui::Color32::from(egui::ecolor::Hsva::new(
                settings.accent_hue, 0.5, if settings.dark_mode { 0.5 } else { 0.6 }, 1.0
            ));

            visuals.selection.bg_fill = accent_color;
            visuals.selection.stroke.color = accent_color;
            visuals.widgets.hovered.bg_stroke.color = accent_color;
            visuals.widgets.active.bg_stroke.color = accent_color;
            visuals.hyperlink_color = accent_color;
            visuals.widgets.noninteractive.fg_stroke.color = accent_dim;

            ctx.set_visuals(visuals);
        }

        // Fullscreen mode toggle (F11 to toggle, Escape to exit)
        ctx.input(|i| {
            if i.key_pressed(egui::Key::F11) {
                self.fullscreen_mode = !self.fullscreen_mode;
            }
            if i.key_pressed(egui::Key::Escape) && self.fullscreen_mode {
                self.fullscreen_mode = false;
            }
        });

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
                // Note: spatial_grid_opacity is hot-swappable, not here
                || self.config.visuals.wireframe != self.previous_config.visuals.wireframe
                || self.config.visuals.wireframe_thickness != self.previous_config.visuals.wireframe_thickness
                || self.config.custom_shaders != self.previous_config.custom_shaders
                || self.config.fields != self.previous_config.fields
                || self.config.particle_fields != self.previous_config.particle_fields
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
            if let Some(state) = wgpu_render_state {
                self.rebuild_simulation(state);
            }
        }

        // Check if full reset needed
        if self.needs_reset {
            if let Some(state) = wgpu_render_state {
                self.reset_simulation(state);
            }
        }

        // Live update: background color (hot-swappable)
        if self.config.visuals.background_color != self.last_background_color {
            if let Some(state) = wgpu_render_state {
                if let Some(sim) = state.renderer.write().callback_resources.get_mut::<rdpe_editor::embedded::SimulationResources>() {
                    sim.set_background_color(Vec3::from_array(self.config.visuals.background_color));
                }
            }
            self.last_background_color = self.config.visuals.background_color;
        }

        // Live update: grid opacity (hot-swappable)
        if self.config.visuals.spatial_grid_opacity != self.last_grid_opacity {
            if let Some(state) = wgpu_render_state {
                if let Some(sim) = state.renderer.write().callback_resources.get_mut::<rdpe_editor::embedded::SimulationResources>() {
                    sim.set_grid_opacity(&state.queue, self.config.visuals.spatial_grid_opacity);
                }
            }
            self.last_grid_opacity = self.config.visuals.spatial_grid_opacity;
        }

        // Live update: glyph config (hot-swappable)
        // The actual glyph data updates happen in SimulationResources::prepare()
        if self.config.visuals.glyphs != self.last_glyph_config {
            if let Some(state) = wgpu_render_state {
                if let Some(sim) = state.renderer.write().callback_resources.get_mut::<rdpe_editor::embedded::SimulationResources>() {
                    sim.set_glyph_config(self.config.visuals.glyphs.to_glyph_config());
                }
            }
            self.last_glyph_config = self.config.visuals.glyphs.clone();
        }

        // Live update: custom uniform values (hot-swappable)
        if let Some(state) = wgpu_render_state {
            if let Some(sim) = state.renderer.write().callback_resources.get_mut::<rdpe_editor::embedded::SimulationResources>() {
                sim.sync_custom_uniforms(&self.config.custom_uniforms);
            }
        }

        // Export window (floating) - only show when not fullscreen
        if !self.fullscreen_mode {
            render_export_window(ctx, &mut self.export_panel_state, &self.config, delta_time);
        }

        // Menu bar - only show when not fullscreen
        if !self.fullscreen_mode {
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
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        ui.separator();
                        if ui.button("Quit").clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
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

                // Options button with popup
                let options_btn = ui.button("⚙ Options");
                if options_btn.clicked() {
                    self.editor_settings.show_options = !self.editor_settings.show_options;
                }

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

                    // Pause/Play and Camera controls
                    if let Some(state) = wgpu_render_state {
                        let (is_paused, auto_orbit, orbit_speed) = {
                            let resources = state.renderer.read();
                            resources.callback_resources
                                .get::<rdpe_editor::embedded::SimulationResources>()
                                .map(|s| (s.is_paused(), s.auto_orbit, s.auto_orbit_speed))
                                .unwrap_or((false, false, 0.3))
                        };

                        let btn_text = if is_paused { "▶ Play" } else { "⏸ Pause" };
                        if ui.button(btn_text).clicked() {
                            if let Some(sim) = state.renderer.write()
                                .callback_resources
                                .get_mut::<rdpe_editor::embedded::SimulationResources>()
                            {
                                sim.set_paused(!is_paused);
                            }
                        }

                        ui.separator();

                        // Auto-orbit toggle
                        let orbit_text = if auto_orbit { "◐ Orbit" } else { "○ Orbit" };
                        if ui.button(orbit_text).on_hover_text("Toggle camera auto-rotation").clicked() {
                            if let Some(sim) = state.renderer.write()
                                .callback_resources
                                .get_mut::<rdpe_editor::embedded::SimulationResources>()
                            {
                                sim.auto_orbit = !auto_orbit;
                            }
                        }

                        // Orbit speed slider (only show when orbiting)
                        if auto_orbit {
                            let mut speed = orbit_speed;
                            if ui.add(egui::Slider::new(&mut speed, -1.0..=1.0).text("Speed").max_decimals(2)).changed() {
                                if let Some(sim) = state.renderer.write()
                                    .callback_resources
                                    .get_mut::<rdpe_editor::embedded::SimulationResources>()
                                {
                                    sim.auto_orbit_speed = speed;
                                }
                            }
                        }
                    }
                });
            });
        });

        // Options popup window
        if self.editor_settings.show_options {
            egui::Window::new("Editor Options")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.heading("Appearance");
                    ui.add_space(8.0);

                    // Dark/Light mode toggle
                    ui.horizontal(|ui| {
                        ui.label("Theme:");
                        ui.selectable_value(&mut self.editor_settings.dark_mode, true, "Dark");
                        ui.selectable_value(&mut self.editor_settings.dark_mode, false, "Light");
                    });

                    ui.add_space(8.0);

                    // Panel side toggle
                    ui.horizontal(|ui| {
                        ui.label("Panel Side:");
                        ui.selectable_value(&mut self.editor_settings.panel_side, PanelSide::Left, "Left");
                        ui.selectable_value(&mut self.editor_settings.panel_side, PanelSide::Right, "Right");
                    });

                    ui.add_space(8.0);

                    // UI Scale (drag value to avoid feedback loop with slider)
                    ui.horizontal(|ui| {
                        ui.label("UI Scale:");
                        ui.add(egui::DragValue::new(&mut self.editor_settings.ui_scale)
                            .range(0.5..=2.0)
                            .speed(0.01)
                            .max_decimals(2));
                    });

                    ui.add_space(8.0);

                    // Panel opacity slider
                    ui.add(egui::Slider::new(&mut self.editor_settings.panel_opacity, 0.3..=1.0)
                        .text("Panel Opacity"));

                    ui.add_space(4.0);

                    // Brightness slider
                    ui.add(egui::Slider::new(&mut self.editor_settings.brightness, 0.5..=1.5)
                        .text("Brightness"));

                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(8.0);

                    // Accent color
                    ui.label("Accent Color:");
                    let accent_hue = &mut self.editor_settings.accent_hue;
                    let accent_preview = egui::ecolor::Hsva::new(*accent_hue, 0.7, 0.8, 1.0);
                    let (rect, _) = ui.allocate_exact_size(egui::vec2(200.0, 16.0), egui::Sense::hover());
                    ui.painter().rect_filled(rect, 4.0, accent_preview);
                    ui.add(egui::Slider::new(accent_hue, 0.0..=1.0).text("Hue").show_value(false));

                    ui.add_space(8.0);

                    // Background tint
                    ui.label("Background Tint:");
                    let bg_hue = &mut self.editor_settings.background_hue;
                    let bg_tint = &mut self.editor_settings.background_tint;

                    // Preview with current tint strength applied
                    let base_value = if self.editor_settings.dark_mode { 0.15 } else { 0.95 };
                    let tinted_sat = 0.3 * *bg_tint;
                    let bg_preview = egui::ecolor::Hsva::new(*bg_hue, tinted_sat, base_value, 1.0);
                    let (rect, _) = ui.allocate_exact_size(egui::vec2(200.0, 16.0), egui::Sense::hover());
                    ui.painter().rect_filled(rect, 4.0, bg_preview);

                    ui.add(egui::Slider::new(bg_hue, 0.0..=1.0).text("Hue").show_value(false));
                    ui.add(egui::Slider::new(bg_tint, 0.0..=1.0).text("Strength").show_value(false));

                    ui.add_space(12.0);

                    // Close button
                    ui.horizontal(|ui| {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("Close").clicked() {
                                self.editor_settings.show_options = false;
                            }
                        });
                    });
                });
        }
        } // end if !fullscreen_mode for menu bar

        // Status bar - only show when not fullscreen
        if !self.fullscreen_mode {
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
        } // end if !fullscreen_mode for status bar

        // Particle Inspector panel (shows when a particle is selected)
        // Get currently selected particle info from GPU
        let selected_info = wgpu_render_state.as_ref().and_then(|state| {
            state.renderer.read().callback_resources.get::<SimulationResources>()
                .and_then(|sim| {
                    let idx = sim.selected_particle()?;
                    let data = sim.selected_particle_data()?;
                    let layout = self.config.particle_layout();
                    let parsed = ParsedParticle::from_bytes_with_layout(data, &layout)?;
                    Some((idx, parsed))
                })
        });

        // Sync editing_particle with selection - continuously update from GPU
        match (&mut self.editing_particle, &selected_info) {
            (Some((edit_idx, edit_particle)), Some((sel_idx, sel_particle))) if *edit_idx == *sel_idx => {
                // Same particle selected - update with fresh GPU data
                *edit_particle = sel_particle.clone();
            }
            (Some((edit_idx, _)), Some((sel_idx, sel_particle))) if *edit_idx != *sel_idx => {
                // Selection changed, update to new particle
                self.editing_particle = Some((*sel_idx, sel_particle.clone()));
            }
            (None, Some((sel_idx, sel_particle))) => {
                // New selection
                self.editing_particle = Some((*sel_idx, sel_particle.clone()));
            }
            (Some(_), None) => {
                // Selection cleared
                self.editing_particle = None;
            }
            _ => {}
        }

        let mut should_clear_selection = false;
        // Only show particle inspector when not in fullscreen mode
        if !self.fullscreen_mode {
        if let Some((idx, ref mut particle)) = self.editing_particle {
            let mut particle_changed = false;
            let mut clear_clicked = false;

            egui::TopBottomPanel::bottom("particle_inspector")
                .resizable(true)
                .min_height(80.0)
                .max_height(250.0)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.heading(format!("Particle #{}", idx));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.small_button("Clear Selection").clicked() {
                                clear_clicked = true;
                            }
                        });
                    });
                    ui.separator();

                    egui::ScrollArea::horizontal().show(ui, |ui| {
                        ui.horizontal(|ui| {
                            // Position
                            ui.vertical(|ui| {
                                ui.label("Position");
                                ui.horizontal(|ui| {
                                    particle_changed |= ui.add(egui::DragValue::new(&mut particle.position[0]).speed(0.01).prefix("x: ")).changed();
                                    particle_changed |= ui.add(egui::DragValue::new(&mut particle.position[1]).speed(0.01).prefix("y: ")).changed();
                                    particle_changed |= ui.add(egui::DragValue::new(&mut particle.position[2]).speed(0.01).prefix("z: ")).changed();
                                });
                            });

                            ui.separator();

                            // Velocity
                            ui.vertical(|ui| {
                                ui.label("Velocity");
                                ui.horizontal(|ui| {
                                    particle_changed |= ui.add(egui::DragValue::new(&mut particle.velocity[0]).speed(0.01).prefix("x: ")).changed();
                                    particle_changed |= ui.add(egui::DragValue::new(&mut particle.velocity[1]).speed(0.01).prefix("y: ")).changed();
                                    particle_changed |= ui.add(egui::DragValue::new(&mut particle.velocity[2]).speed(0.01).prefix("z: ")).changed();
                                });
                            });

                            ui.separator();

                            // Color
                            ui.vertical(|ui| {
                                ui.label("Color");
                                let mut color = [particle.color[0], particle.color[1], particle.color[2]];
                                if ui.color_edit_button_rgb(&mut color).changed() {
                                    particle.color = color;
                                    particle_changed = true;
                                }
                            });

                            ui.separator();

                            // Scale, Age, Type
                            ui.vertical(|ui| {
                                ui.horizontal(|ui| {
                                    ui.label("Scale:");
                                    particle_changed |= ui.add(egui::DragValue::new(&mut particle.scale).speed(0.01).range(0.01..=10.0)).changed();
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Age:");
                                    particle_changed |= ui.add(egui::DragValue::new(&mut particle.age).speed(0.1)).changed();
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Type:");
                                    particle_changed |= ui.add(egui::DragValue::new(&mut particle.particle_type).range(0..=255)).changed();
                                });
                            });

                            ui.separator();

                            // Custom fields
                            if !particle.custom_fields.is_empty() {
                                ui.vertical(|ui| {
                                    ui.label("Custom");
                                    for (name, value) in &mut particle.custom_fields {
                                        ui.horizontal(|ui| {
                                            ui.label(format!("{}:", name));
                                            match value {
                                                rdpe_editor::spawn::FieldValue::F32(v) => {
                                                    particle_changed |= ui.add(egui::DragValue::new(v).speed(0.01)).changed();
                                                }
                                                rdpe_editor::spawn::FieldValue::U32(v) => {
                                                    particle_changed |= ui.add(egui::DragValue::new(v)).changed();
                                                }
                                                rdpe_editor::spawn::FieldValue::I32(v) => {
                                                    particle_changed |= ui.add(egui::DragValue::new(v)).changed();
                                                }
                                                rdpe_editor::spawn::FieldValue::Vec2(v) => {
                                                    particle_changed |= ui.add(egui::DragValue::new(&mut v[0]).speed(0.01).prefix("x: ")).changed();
                                                    particle_changed |= ui.add(egui::DragValue::new(&mut v[1]).speed(0.01).prefix("y: ")).changed();
                                                }
                                                rdpe_editor::spawn::FieldValue::Vec3(v) => {
                                                    particle_changed |= ui.add(egui::DragValue::new(&mut v[0]).speed(0.01).prefix("x: ")).changed();
                                                    particle_changed |= ui.add(egui::DragValue::new(&mut v[1]).speed(0.01).prefix("y: ")).changed();
                                                    particle_changed |= ui.add(egui::DragValue::new(&mut v[2]).speed(0.01).prefix("z: ")).changed();
                                                }
                                                rdpe_editor::spawn::FieldValue::Vec4(v) => {
                                                    particle_changed |= ui.add(egui::DragValue::new(&mut v[0]).speed(0.01)).changed();
                                                    particle_changed |= ui.add(egui::DragValue::new(&mut v[1]).speed(0.01)).changed();
                                                    particle_changed |= ui.add(egui::DragValue::new(&mut v[2]).speed(0.01)).changed();
                                                    particle_changed |= ui.add(egui::DragValue::new(&mut v[3]).speed(0.01)).changed();
                                                }
                                            }
                                        });
                                    }
                                });
                            }
                        });
                    });
                });

            // Write changes back to GPU if particle was modified
            if particle_changed {
                if let Some(state) = wgpu_render_state {
                    let layout = self.config.particle_layout();
                    let bytes = particle.to_bytes(&layout);
                    if let Some(sim) = state.renderer.read().callback_resources.get::<SimulationResources>() {
                        sim.write_particle_at(&state.queue, idx, &bytes);
                    }
                }
            }

            // Handle clear selection after the closure
            if clear_clicked {
                if let Some(state) = wgpu_render_state {
                    if let Some(sim) = state.renderer.write().callback_resources.get_mut::<SimulationResources>() {
                        sim.clear_selection();
                    }
                }
                should_clear_selection = true;
            }
        }
        } // end if !fullscreen_mode for particle inspector
        if should_clear_selection {
            self.editing_particle = None;
        }

        // Settings panel (left or right based on settings) - only show when not fullscreen
        if !self.fullscreen_mode {
        let settings_panel = match self.editor_settings.panel_side {
            PanelSide::Left => egui::SidePanel::left("settings"),
            PanelSide::Right => egui::SidePanel::right("settings"),
        };
        settings_panel
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
                    ui.selectable_value(&mut self.selected_tab, SidebarTab::Interactions, "Types");
                    ui.selectable_value(&mut self.selected_tab, SidebarTab::Emitters, "Emitters");
                    ui.selectable_value(&mut self.selected_tab, SidebarTab::Particle, "Particle");
                    ui.selectable_value(&mut self.selected_tab, SidebarTab::Fields, "Fields");
                    ui.selectable_value(&mut self.selected_tab, SidebarTab::Visuals, "Visuals");
                    ui.selectable_value(&mut self.selected_tab, SidebarTab::PostProcess, "Post FX");
                    ui.selectable_value(&mut self.selected_tab, SidebarTab::Mouse, "Mouse");
                    ui.selectable_value(&mut self.selected_tab, SidebarTab::Custom, "Custom");
                    ui.selectable_value(&mut self.selected_tab, SidebarTab::Stats, "Stats");
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
                            let has_neighbor_rules = self.config.rules.iter().any(|r| r.requires_neighbors());
                            let ctx = ShaderContext {
                                particle_fields: &self.config.particle_fields,
                                fields: &self.config.fields,
                                adjacency_enabled: self.config.adjacency_enabled,
                                has_neighbor_rules,
                            };
                            render_rules_panel(ui, &mut self.config.rules, &ctx);
                        }
                        SidebarTab::Interactions => {
                            // Sync num_types with spawn type_weights
                            let spawn_types = self.config.spawn.type_weights.len().max(1);
                            if self.config.interactions.num_types != spawn_types {
                                self.config.interactions.resize(spawn_types);
                            }

                            if render_interactions_panel(ui, &mut self.config.interactions, &mut self.interactions_panel_state) {
                                self.needs_rebuild = true;
                            }
                        }
                        SidebarTab::Emitters => {
                            if render_emitters_panel(ui, &mut self.config.emitters) {
                                self.needs_rebuild = true;
                            }
                        }
                        SidebarTab::Particle => {
                            render_particle_fields_panel(ui, &mut self.config);
                        }
                        SidebarTab::Fields => {
                            render_fields_panel(ui, &mut self.config.fields, &mut self.config.visuals.glyphs);

                            ui.separator();

                            // Volume rendering panel
                            let num_fields = self.config.fields.len();
                            render_volume_panel(ui, &mut self.config.volume_render, num_fields);
                        }
                        SidebarTab::Visuals => {
                            let _visuals_changed = render_visuals_panel(ui, &mut self.config);

                            ui.separator();

                            // Vertex effects
                            render_effects_panel(ui, &mut self.config.vertex_effects);
                        }
                        SidebarTab::PostProcess => {
                            let pp_changed =
                                render_post_process_panel(ui, &mut self.config.post_process);
                            if pp_changed {
                                self.needs_rebuild = true;
                            }
                        }
                        SidebarTab::Mouse => {
                            let old_power = self.config.mouse.power;
                            let mouse_changed = render_mouse_panel(ui, &mut self.config.mouse);
                            if mouse_changed {
                                // Check if power changed - this requires shader rebuild
                                if self.config.mouse.power != old_power {
                                    self.needs_rebuild = true;
                                } else {
                                    // Just config change (radius/strength/color) - update immediately
                                    if let Some(wgpu_render_state) = frame.wgpu_render_state() {
                                        if let Some(sim) = wgpu_render_state
                                            .renderer
                                            .write()
                                            .callback_resources
                                            .get_mut::<SimulationResources>()
                                        {
                                            sim.set_mouse_config(self.config.mouse.clone());
                                        }
                                    }
                                }
                            }
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
                        SidebarTab::Stats => {
                            // Get stats from simulation resources
                            if let Some(state) = wgpu_render_state {
                                if let Some(sim) = state.renderer.read().callback_resources.get::<rdpe_editor::embedded::SimulationResources>() {
                                    render_stats_panel(ui, sim.stats());
                                }
                            }
                        }
                    }
                });
            });
        } // end if !fullscreen_mode for right panel

        // Central panel: Simulation viewport
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(egui::Color32::from_rgb(
                (self.config.visuals.background_color[0] * 255.0) as u8,
                (self.config.visuals.background_color[1] * 255.0) as u8,
                (self.config.visuals.background_color[2] * 255.0) as u8,
            )))
            .show(ctx, |ui| {
                // Show the simulation viewport
                if let Some(state) = wgpu_render_state {
                    self.simulation.show(ui, state, self.config.speed);
                } else {
                    ui.centered_and_justified(|ui| {
                        ui.label("wgpu not available - simulation requires GPU");
                    });
                }

                // Fullscreen mode hint (fades away after a few seconds)
                if self.fullscreen_mode {
                    egui::Area::new(egui::Id::new("fullscreen_hint"))
                        .anchor(egui::Align2::CENTER_BOTTOM, [0.0, -20.0])
                        .show(ui.ctx(), |ui| {
                            ui.label(
                                egui::RichText::new("Press F11 or Escape to exit fullscreen")
                                    .color(egui::Color32::from_rgba_unmultiplied(255, 255, 255, 180))
                                    .small()
                            );
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
