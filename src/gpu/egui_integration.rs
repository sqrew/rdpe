//! Egui integration for RDPE.
//!
//! This module provides optional egui UI support when the `egui` feature is enabled.

use std::sync::Arc;
use winit::window::Window;

/// Egui integration state.
///
/// Wraps egui context, winit state, and wgpu renderer.
pub struct EguiIntegration {
    pub ctx: egui::Context,
    state: egui_winit::State,
    renderer: egui_wgpu::Renderer,
}

/// Output from egui frame processing.
pub struct EguiFrameOutput {
    pub paint_jobs: Vec<egui::ClippedPrimitive>,
    pub textures_delta: egui::TexturesDelta,
    pub pixels_per_point: f32,
}

impl EguiIntegration {
    /// Create new egui integration.
    pub fn new(
        device: &wgpu::Device,
        output_format: wgpu::TextureFormat,
        window: &Arc<Window>,
    ) -> Self {
        let ctx = egui::Context::default();

        // Configure default style - dark theme that fits particle sims
        let mut style = egui::Style::default();
        style.visuals = egui::Visuals::dark();
        style.visuals.window_shadow = egui::Shadow::NONE;
        style.visuals.popup_shadow = egui::Shadow::NONE;
        ctx.set_style(style);

        let state = egui_winit::State::new(
            ctx.clone(),
            egui::ViewportId::ROOT,
            window.as_ref(),
            Some(window.scale_factor() as f32),
            None,
            None,
        );

        let renderer = egui_wgpu::Renderer::new(
            device,
            output_format,
            None,  // depth format
            1,     // msaa samples
            false, // dithering
        );

        Self { ctx, state, renderer }
    }

    /// Process a winit event.
    ///
    /// Returns true if egui consumed the event (don't pass to camera controls).
    pub fn on_window_event(
        &mut self,
        window: &Window,
        event: &winit::event::WindowEvent,
    ) -> bool {
        let response = self.state.on_window_event(window, event);
        response.consumed
    }

    /// Begin a new frame. Call before your UI code.
    pub fn begin_frame(&mut self, window: &Window) {
        let raw_input = self.state.take_egui_input(window);
        self.ctx.begin_frame(raw_input);
    }

    /// End the frame and get the output for rendering.
    pub fn end_frame(&mut self, window: &Window) -> EguiFrameOutput {
        let full_output = self.ctx.end_frame();

        // Handle platform output (clipboard, cursor, etc.)
        self.state.handle_platform_output(window, full_output.platform_output);

        // Tessellate shapes into paint jobs
        let paint_jobs = self.ctx.tessellate(full_output.shapes, full_output.pixels_per_point);

        EguiFrameOutput {
            paint_jobs,
            textures_delta: full_output.textures_delta,
            pixels_per_point: full_output.pixels_per_point,
        }
    }

    /// Prepare textures and buffers for rendering. Call before creating render pass.
    pub fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        output: &EguiFrameOutput,
        screen_descriptor: &egui_wgpu::ScreenDescriptor,
    ) {
        // Update textures
        for (id, image_delta) in &output.textures_delta.set {
            self.renderer.update_texture(device, queue, *id, image_delta);
        }

        // Update buffers
        self.renderer.update_buffers(
            device,
            queue,
            encoder,
            &output.paint_jobs,
            screen_descriptor,
        );
    }

    /// Get a reference to the renderer for direct rendering.
    pub fn renderer(&self) -> &egui_wgpu::Renderer {
        &self.renderer
    }

    /// Free textures after frame is done.
    pub fn cleanup(&mut self, output: &EguiFrameOutput) {
        for id in &output.textures_delta.free {
            self.renderer.free_texture(id);
        }
    }
}
