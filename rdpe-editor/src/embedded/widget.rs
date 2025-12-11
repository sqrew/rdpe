//! Embedded simulation widget for the rdpe editor.
//!
//! This module provides the `EmbeddedSimulation` widget that can be embedded
//! in an egui UI to display and interact with a particle simulation.
//!
//! # Usage
//!
//! ```ignore
//! let mut sim = EmbeddedSimulation::new();
//!
//! // Initialize once when wgpu is available
//! if let Some(wgpu_state) = frame.wgpu_render_state() {
//!     sim.initialize(wgpu_state, &config);
//! }
//!
//! // Show in your UI
//! sim.show(ui, wgpu_state, speed);
//! ```

use glam::{Mat4, Vec3};
use crate::config::SimConfig;
use crate::shader_gen;
use crate::shader_validate;
use crate::spawn;
use super::{SimulationResources, SimulationCallback};

/// Embedded simulation widget that manages the simulation lifecycle and UI.
///
/// This struct is the main entry point for embedding a particle simulation
/// in an egui interface. It handles:
/// - Initialization and reinitialization of GPU resources
/// - Simulation state management
/// - Camera controls (orbit, zoom)
/// - Mouse interaction for particle selection and powers
/// - Error handling for shader compilation
///
/// # Example
///
/// ```ignore
/// let mut sim = EmbeddedSimulation::new();
///
/// // In your UI update code:
/// if let Some(wgpu_state) = frame.wgpu_render_state() {
///     if !sim.is_initialized() {
///         sim.initialize(wgpu_state, &config);
///     }
///     sim.show(ui, wgpu_state, 1.0);
/// }
/// ```
pub struct EmbeddedSimulation {
    /// Whether the simulation has been initialized in CallbackResources.
    initialized: bool,
    /// Current delta time.
    delta_time: f32,
    /// Last frame instant for delta time calculation.
    last_frame: std::time::Instant,
    /// Shader compilation error message (if any).
    shader_error: Option<String>,
}

impl EmbeddedSimulation {
    /// Create a new embedded simulation handle (resources not yet created).
    pub fn new() -> Self {
        Self {
            initialized: false,
            delta_time: 0.016,
            last_frame: std::time::Instant::now(),
            shader_error: None,
        }
    }

    /// Get the current shader error, if any.
    pub fn shader_error(&self) -> Option<&str> {
        self.shader_error.as_deref()
    }

    /// Clear the shader error.
    pub fn clear_error(&mut self) {
        self.shader_error = None;
    }

    /// Initialize the simulation resources in egui's callback resources.
    ///
    /// Call this once when the wgpu render state is available.
    pub fn initialize(
        &mut self,
        wgpu_render_state: &egui_wgpu::RenderState,
        config: &SimConfig,
    ) {
        if self.initialized {
            return;
        }

        // Get particle layout from config
        let layout = config.particle_layout();

        // Generate particle data using proper spawn config
        let particle_data = spawn::generate_particles(config);

        // Generate shaders using the actual rule system
        let compute_shader = shader_gen::generate_compute_shader(config);
        let render_shader = shader_gen::generate_render_shader(config);

        // Validate shaders before compiling
        if let Err(errors) = shader_validate::validate_shaders(&compute_shader, &render_shader) {
            let error_msg = errors.iter()
                .map(|e| e.to_string())
                .collect::<Vec<_>>()
                .join("\n\n");
            self.shader_error = Some(error_msg);
            return; // Don't crash, just store error
        }

        // Clear any previous error
        self.shader_error = None;

        let field_registry = config.to_field_registry();
        let particle_wgsl_struct = config.particle_wgsl_struct();
        let wireframe_mesh = config.visuals.wireframe.to_mesh();
        let resources = SimulationResources::new(
            &wgpu_render_state.device,
            &wgpu_render_state.queue,
            wgpu_render_state.target_format,
            &particle_data,
            config.particle_count,
            &layout,
            &compute_shader,
            &render_shader,
            Vec3::from_array(config.visuals.background_color),
            &config.custom_uniforms,
            &field_registry,
            &config.volume_render,
            config.needs_spatial(),
            config.spatial_cell_size,
            config.spatial_resolution,
            &particle_wgsl_struct,
            &config.visuals.blend_mode,
            config.visuals.spatial_grid_opacity,
            config.visuals.connections_enabled,
            config.visuals.connections_radius,
            config.visuals.connections_color,
            wireframe_mesh.as_ref(),
            config.visuals.wireframe_thickness,
            config.particle_size,
            config.visuals.trail_length,
            config.mouse.clone(),
        );

        wgpu_render_state
            .renderer
            .write()
            .callback_resources
            .insert(resources);

        self.initialized = true;
    }

    /// Check if initialized.
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Reinitialize the simulation with a new config, preserving particle state if possible.
    ///
    /// This is used when config changes require a rebuild but we want to keep particles.
    /// Note: If particle count changes, state cannot be preserved and particles are regenerated.
    pub fn reinitialize(
        &mut self,
        wgpu_render_state: &egui_wgpu::RenderState,
        config: &SimConfig,
    ) {
        // Generate new shaders first to validate before any state changes
        let compute_shader = shader_gen::generate_compute_shader(config);
        let render_shader = shader_gen::generate_render_shader(config);

        // Validate shaders before compiling
        if let Err(errors) = shader_validate::validate_shaders(&compute_shader, &render_shader) {
            let error_msg = errors.iter()
                .map(|e| e.to_string())
                .collect::<Vec<_>>()
                .join("\n\n");
            self.shader_error = Some(error_msg);
            return; // Don't crash, keep old resources running
        }

        // Clear any previous error
        self.shader_error = None;

        // Get particle layout from config
        let layout = config.particle_layout();

        // Read existing particles and camera state if we're already initialized
        // If stride changed (due to adding/removing custom fields), we can't preserve existing particle data
        let (existing_particles, old_camera) = if self.initialized {
            let resources = wgpu_render_state.renderer.read();
            if let Some(sim) = resources.callback_resources.get::<SimulationResources>() {
                let particles = if sim.num_particles == config.particle_count && sim.particle_stride == layout.stride {
                    Some(sim.read_particles(&wgpu_render_state.device, &wgpu_render_state.queue))
                } else {
                    None // Particle count or stride changed, can't preserve
                };
                let camera = Some((sim.camera_distance, sim.camera_yaw, sim.camera_pitch));
                (particles, camera)
            } else {
                (None, None)
            }
        } else {
            (None, None)
        };

        // Generate particle data (use existing or new)
        let particle_data = if let Some(data) = existing_particles {
            data
        } else {
            spawn::generate_particles(config)
        };

        // Create new resources
        let field_registry = config.to_field_registry();
        let particle_wgsl_struct = config.particle_wgsl_struct();
        let wireframe_mesh = config.visuals.wireframe.to_mesh();
        let resources = SimulationResources::new(
            &wgpu_render_state.device,
            &wgpu_render_state.queue,
            wgpu_render_state.target_format,
            &particle_data,
            config.particle_count,
            &layout,
            &compute_shader,
            &render_shader,
            Vec3::from_array(config.visuals.background_color),
            &config.custom_uniforms,
            &field_registry,
            &config.volume_render,
            config.needs_spatial(),
            config.spatial_cell_size,
            config.spatial_resolution,
            &particle_wgsl_struct,
            &config.visuals.blend_mode,
            config.visuals.spatial_grid_opacity,
            config.visuals.connections_enabled,
            config.visuals.connections_radius,
            config.visuals.connections_color,
            wireframe_mesh.as_ref(),
            config.visuals.wireframe_thickness,
            config.particle_size,
            config.visuals.trail_length,
            config.mouse.clone(),
        );

        // Replace resources
        wgpu_render_state
            .renderer
            .write()
            .callback_resources
            .insert(resources);

        // Restore camera state if we had one
        if let Some((distance, yaw, pitch)) = old_camera {
            if let Some(sim) = wgpu_render_state.renderer.write().callback_resources.get_mut::<SimulationResources>() {
                sim.camera_distance = distance;
                sim.camera_yaw = yaw;
                sim.camera_pitch = pitch;
            }
        }

        self.initialized = true;
    }

    /// Full reset: regenerate all particles from spawn config.
    ///
    /// Use this when you want fresh particles (after changing spawn settings, or to clear chaos).
    pub fn reset(
        &mut self,
        wgpu_render_state: &egui_wgpu::RenderState,
        config: &SimConfig,
    ) {
        // Generate new shaders first to validate
        let compute_shader = shader_gen::generate_compute_shader(config);
        let render_shader = shader_gen::generate_render_shader(config);

        // Validate shaders before compiling
        if let Err(errors) = shader_validate::validate_shaders(&compute_shader, &render_shader) {
            let error_msg = errors.iter()
                .map(|e| e.to_string())
                .collect::<Vec<_>>()
                .join("\n\n");
            self.shader_error = Some(error_msg);
            return; // Don't crash, keep old resources running
        }

        // Clear any previous error
        self.shader_error = None;

        // Get particle layout from config
        let layout = config.particle_layout();

        // Save camera state before replacing resources
        let old_camera = {
            let resources = wgpu_render_state.renderer.read();
            resources.callback_resources.get::<SimulationResources>()
                .map(|sim| (sim.camera_distance, sim.camera_yaw, sim.camera_pitch))
        };

        // Always generate fresh particles
        let particle_data = spawn::generate_particles(config);

        // Create new resources
        let field_registry = config.to_field_registry();
        let particle_wgsl_struct = config.particle_wgsl_struct();
        let wireframe_mesh = config.visuals.wireframe.to_mesh();
        let resources = SimulationResources::new(
            &wgpu_render_state.device,
            &wgpu_render_state.queue,
            wgpu_render_state.target_format,
            &particle_data,
            config.particle_count,
            &layout,
            &compute_shader,
            &render_shader,
            Vec3::from_array(config.visuals.background_color),
            &config.custom_uniforms,
            &field_registry,
            &config.volume_render,
            config.needs_spatial(),
            config.spatial_cell_size,
            config.spatial_resolution,
            &particle_wgsl_struct,
            &config.visuals.blend_mode,
            config.visuals.spatial_grid_opacity,
            config.visuals.connections_enabled,
            config.visuals.connections_radius,
            config.visuals.connections_color,
            wireframe_mesh.as_ref(),
            config.visuals.wireframe_thickness,
            config.particle_size,
            config.visuals.trail_length,
            config.mouse.clone(),
        );

        // Replace resources
        wgpu_render_state
            .renderer
            .write()
            .callback_resources
            .insert(resources);

        // Restore camera state if we had one
        if let Some((distance, yaw, pitch)) = old_camera {
            if let Some(sim) = wgpu_render_state.renderer.write().callback_resources.get_mut::<SimulationResources>() {
                sim.camera_distance = distance;
                sim.camera_yaw = yaw;
                sim.camera_pitch = pitch;
            }
        }

        self.initialized = true;
    }

    /// Render the simulation viewport in egui.
    ///
    /// Call this in your UI code where you want the viewport to appear.
    /// The `speed` parameter controls simulation speed (1.0 = normal, 0.5 = half, 2.0 = double).
    pub fn show(&mut self, ui: &mut egui::Ui, wgpu_render_state: &egui_wgpu::RenderState, speed: f32) {
        // Calculate delta time with speed multiplier
        let now = std::time::Instant::now();
        self.delta_time = now.duration_since(self.last_frame).as_secs_f32() * speed;
        self.last_frame = now;

        // Get available rect
        let rect = ui.available_rect_before_wrap();
        let viewport_width = rect.width() as u32;
        let viewport_height = rect.height() as u32;

        // Handle input for camera control
        let response = ui.allocate_rect(rect, egui::Sense::click_and_drag());

        // Resize picking texture if needed and handle picking
        {
            let mut renderer = wgpu_render_state.renderer.write();
            if let Some(sim) = renderer.callback_resources.get_mut::<SimulationResources>() {
                // Resize picking texture to match viewport
                sim.resize_picking(&wgpu_render_state.device, viewport_width.max(1), viewport_height.max(1));

                // Handle click for particle picking (only on click, not drag)
                if response.clicked() {
                    if let Some(pos) = response.interact_pointer_pos() {
                        let x = (pos.x - rect.left()) as u32;
                        let y = (pos.y - rect.top()) as u32;
                        sim.request_pick(x, y);
                    }
                }

                // Camera rotation via secondary (right) mouse button drag
                if response.dragged_by(egui::PointerButton::Secondary) {
                    let delta = response.drag_delta();
                    sim.rotate_camera(-delta.x * 0.01, -delta.y * 0.01);
                }

                // Track mouse state for mouse powers (Shift + primary button)
                let shift_held = ui.input(|i| i.modifiers.shift);
                let primary_down = ui.input(|i| i.pointer.primary_down());
                let power_active = shift_held && primary_down;
                if let Some(pos) = response.hover_pos().or(response.interact_pointer_pos()) {
                    // Convert screen position to normalized device coords (-1 to 1)
                    let ndc_x = (pos.x - rect.left()) / rect.width() * 2.0 - 1.0;
                    let ndc_y = 1.0 - (pos.y - rect.top()) / rect.height() * 2.0;

                    // Compute view-projection matrix fresh to match current viewport
                    let aspect_ratio = rect.width() / rect.height().max(1.0);
                    let eye = Vec3::new(
                        sim.camera_distance * sim.camera_yaw.cos() * sim.camera_pitch.cos(),
                        sim.camera_distance * sim.camera_pitch.sin(),
                        sim.camera_distance * sim.camera_yaw.sin() * sim.camera_pitch.cos(),
                    );
                    let view = Mat4::look_at_rh(eye, Vec3::ZERO, Vec3::Y);
                    let proj = Mat4::perspective_rh(45.0_f32.to_radians(), aspect_ratio, 0.1, 100.0);
                    let view_proj = proj * view;
                    let inv_vp = view_proj.inverse();

                    let near_clip = glam::Vec4::new(ndc_x, ndc_y, -1.0, 1.0);
                    let near_world = inv_vp * near_clip;
                    let near_point = Vec3::new(
                        near_world.x / near_world.w,
                        near_world.y / near_world.w,
                        near_world.z / near_world.w,
                    );

                    let far_clip = glam::Vec4::new(ndc_x, ndc_y, 1.0, 1.0);
                    let far_world = inv_vp * far_clip;
                    let far_point = Vec3::new(
                        far_world.x / far_world.w,
                        far_world.y / far_world.w,
                        far_world.z / far_world.w,
                    );

                    let ray_dir = (far_point - near_point).normalize();

                    // Pass the ray to the shader - it will check distance from each particle to the ray
                    sim.set_mouse_state(eye, ray_dir, power_active);
                } else {
                    // Mouse not over viewport
                    sim.set_mouse_state(Vec3::ZERO, Vec3::Z, false);
                }

                // Run picking pass to update selection
                sim.update_picking(&wgpu_render_state.device, &wgpu_render_state.queue);
            }
        }

        // Camera zoom via scroll
        let scroll_delta = ui.input(|i| i.raw_scroll_delta.y);
        if scroll_delta.abs() > 0.1 {
            if let Some(sim) = wgpu_render_state.renderer.write().callback_resources.get_mut::<SimulationResources>() {
                sim.zoom_camera(scroll_delta * 0.01);
            }
        }

        // Get background color from resources
        let clear_color = if let Some(sim) = wgpu_render_state.renderer.read().callback_resources.get::<SimulationResources>() {
            let bg = sim.background_color();
            [bg.x, bg.y, bg.z]
        } else {
            [0.0, 0.0, 0.0]
        };

        // Add the paint callback
        let callback = SimulationCallback {
            delta_time: self.delta_time,
            clear_color,
            viewport_width: rect.width(),
            viewport_height: rect.height(),
        };

        ui.painter().add(egui_wgpu::Callback::new_paint_callback(
            rect,
            callback,
        ));

        // Request repaint for continuous animation
        ui.ctx().request_repaint();
    }
}

impl Default for EmbeddedSimulation {
    fn default() -> Self {
        Self::new()
    }
}
