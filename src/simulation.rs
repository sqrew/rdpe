//! Simulation builder and runner

use crate::gpu::GpuState;
use crate::rules::Rule;
use crate::spatial::{SpatialConfig, MORTON_WGSL, NEIGHBOR_UTILS_WGSL};
use crate::ParticleTrait;
use std::marker::PhantomData;
use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};

/// A particle simulation builder.
///
/// Use method chaining to configure, then call `.run()` to start.
pub struct Simulation<P: ParticleTrait> {
    particle_count: u32,
    bounds: f32,
    spawner: Option<Box<dyn Fn(u32, u32) -> P + Send + Sync>>,
    rules: Vec<Rule>,
    spatial_config: SpatialConfig,
    _phantom: PhantomData<P>,
}

impl<P: ParticleTrait + 'static> Simulation<P> {
    /// Create a new simulation with default settings.
    pub fn new() -> Self {
        Self {
            particle_count: 10_000,
            bounds: 1.0,
            spawner: None,
            rules: Vec::new(),
            spatial_config: SpatialConfig::default(),
            _phantom: PhantomData,
        }
    }

    /// Set the number of particles.
    pub fn with_particle_count(mut self, count: u32) -> Self {
        self.particle_count = count;
        self
    }

    /// Set the bounding box half-size (cube from -bounds to +bounds).
    pub fn with_bounds(mut self, bounds: f32) -> Self {
        self.bounds = bounds;
        self
    }

    /// Set the particle spawner function.
    /// Called with (particle_index, total_count) for each particle.
    pub fn with_spawner<F>(mut self, spawner: F) -> Self
    where
        F: Fn(u32, u32) -> P + Send + Sync + 'static,
    {
        self.spawner = Some(Box::new(spawner));
        self
    }

    /// Add a rule to the simulation.
    pub fn with_rule(mut self, rule: Rule) -> Self {
        self.rules.push(rule);
        self
    }

    /// Configure spatial hashing (cell size and grid resolution).
    /// Only needed if using neighbor-based rules.
    pub fn with_spatial_config(mut self, cell_size: f32, grid_resolution: u32) -> Self {
        self.spatial_config = SpatialConfig::new(cell_size, grid_resolution);
        self
    }

    /// Check if any rules require neighbor queries
    fn has_neighbor_rules(&self) -> bool {
        self.rules.iter().any(|r| r.requires_neighbors())
    }

    /// Generate the compute shader WGSL code.
    fn generate_compute_shader(&self) -> String {
        let particle_struct = P::WGSL_STRUCT;
        let has_neighbors = self.has_neighbor_rules();

        // Generate non-neighbor rules
        let simple_rules_code: String = self
            .rules
            .iter()
            .filter(|r| !r.requires_neighbors())
            .map(|r| r.to_wgsl(self.bounds))
            .collect::<Vec<_>>()
            .join("\n\n");

        if !has_neighbors {
            // Simple shader without neighbor queries
            format!(
                r#"{particle_struct}

struct Uniforms {{
    view_proj: mat4x4<f32>,
    time: f32,
    delta_time: f32,
}};

@group(0) @binding(0)
var<storage, read_write> particles: array<Particle>;

@group(0) @binding(1)
var<uniform> uniforms: Uniforms;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {{
    let index = global_id.x;
    let num_particles = arrayLength(&particles);

    if index >= num_particles {{
        return;
    }}

    var p = particles[index];

{simple_rules_code}

    // Integrate velocity
    p.position += p.velocity * uniforms.delta_time;

    particles[index] = p;
}}
"#
            )
        } else {
            // Complex shader with neighbor queries
            let neighbor_rules_code: String = self
                .rules
                .iter()
                .filter(|r| r.requires_neighbors())
                .map(|r| r.to_neighbor_wgsl())
                .collect::<Vec<_>>()
                .join("\n");

            let post_neighbor_code: String = self
                .rules
                .iter()
                .filter(|r| r.requires_neighbors())
                .map(|r| r.to_post_neighbor_wgsl())
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join("\n\n");

            // Check if we need cohesion/alignment accumulators
            let needs_cohesion = self.rules.iter().any(|r| matches!(r, Rule::Cohere { .. }));
            let needs_alignment = self.rules.iter().any(|r| matches!(r, Rule::Align { .. }));

            let accumulator_vars = {
                let mut vars = String::new();
                if needs_cohesion {
                    vars.push_str("    var cohesion_sum = vec3<f32>(0.0);\n    var cohesion_count = 0.0;\n");
                }
                if needs_alignment {
                    vars.push_str("    var alignment_sum = vec3<f32>(0.0);\n    var alignment_count = 0.0;\n");
                }
                vars
            };

            format!(
                r#"{particle_struct}

{MORTON_WGSL}

{NEIGHBOR_UTILS_WGSL}

struct Uniforms {{
    view_proj: mat4x4<f32>,
    time: f32,
    delta_time: f32,
}};

struct SpatialParams {{
    cell_size: f32,
    grid_resolution: u32,
    num_particles: u32,
    _pad: u32,
}};

@group(0) @binding(0)
var<storage, read_write> particles: array<Particle>;

@group(0) @binding(1)
var<uniform> uniforms: Uniforms;

@group(0) @binding(2)
var<storage, read> sorted_indices: array<u32>;

@group(0) @binding(3)
var<storage, read> cell_start: array<u32>;

@group(0) @binding(4)
var<storage, read> cell_end: array<u32>;

@group(0) @binding(5)
var<uniform> spatial: SpatialParams;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {{
    let index = global_id.x;
    let num_particles = arrayLength(&particles);

    if index >= num_particles {{
        return;
    }}

    var p = particles[index];
    let my_pos = p.position;
    let my_cell = pos_to_cell(my_pos, spatial.cell_size, spatial.grid_resolution);

{accumulator_vars}
    // Neighbor iteration
    for (var offset_idx = 0u; offset_idx < 27u; offset_idx++) {{
        let neighbor_morton = neighbor_cell_morton(my_cell, offset_idx, spatial.grid_resolution);

        if neighbor_morton == 0xFFFFFFFFu {{
            continue; // Out of bounds
        }}

        let start = cell_start[neighbor_morton];
        let end = cell_end[neighbor_morton];

        if start == 0xFFFFFFFFu {{
            continue; // Empty cell
        }}

        for (var j = start; j < end; j++) {{
            let other_idx = sorted_indices[j];

            if other_idx == index {{
                continue; // Skip self
            }}

            let other = particles[other_idx];
            let neighbor_pos = other.position;
            let neighbor_vel = other.velocity;
            let diff = my_pos - neighbor_pos;
            let neighbor_dist = length(diff);
            let neighbor_dir = select(vec3<f32>(0.0), diff / neighbor_dist, neighbor_dist > 0.0001);

{neighbor_rules_code}
        }}
    }}

{post_neighbor_code}

{simple_rules_code}

    // Integrate velocity
    p.position += p.velocity * uniforms.delta_time;

    particles[index] = p;
}}
"#
            )
        }
    }

    /// Generate the render shader WGSL code.
    fn generate_render_shader(&self) -> String {
        let color_expr = match P::COLOR_FIELD {
            Some(field) => format!("particle_{}", field),
            None => "normalize(particle_pos) * 0.5 + 0.5".to_string(),
        };

        format!(
            r#"struct Uniforms {{
    view_proj: mat4x4<f32>,
    time: f32,
    delta_time: f32,
}};

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

struct VertexOutput {{
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
    @location(1) uv: vec2<f32>,
}};

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    @location(0) particle_pos: vec3<f32>,
) -> VertexOutput {{
    var quad_vertices = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>( 1.0,  1.0),
    );

    let quad_pos = quad_vertices[vertex_index];
    let particle_size = 0.015;

    let world_pos = vec4<f32>(particle_pos, 1.0);
    var clip_pos = uniforms.view_proj * world_pos;

    clip_pos.x += quad_pos.x * particle_size * clip_pos.w;
    clip_pos.y += quad_pos.y * particle_size * clip_pos.w;

    var out: VertexOutput;
    out.clip_position = clip_pos;
    out.color = {color_expr};
    out.uv = quad_pos;

    return out;
}}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {{
    let dist = length(in.uv);
    if dist > 1.0 {{
        discard;
    }}
    let alpha = 1.0 - smoothstep(0.5, 1.0, dist);
    return vec4<f32>(in.color, alpha);
}}
"#
        )
    }

    /// Run the simulation. This blocks until the window is closed.
    pub fn run(mut self) {
        let spawner = self
            .spawner
            .take()
            .expect("Must provide a spawner with .with_spawner()");

        let has_neighbors = self.has_neighbor_rules();

        // Generate shaders before moving self
        let compute_shader = self.generate_compute_shader();
        let render_shader = self.generate_render_shader();

        // Generate particles
        let particles: Vec<P> = (0..self.particle_count)
            .map(|i| spawner(i, self.particle_count))
            .collect();

        let config = SimConfig {
            particle_count: self.particle_count,
            compute_shader,
            render_shader,
            has_neighbors,
            spatial_config: self.spatial_config,
        };

        let event_loop = EventLoop::new().unwrap();
        event_loop.set_control_flow(ControlFlow::Poll);

        let mut app = App::<P>::new(particles, config);
        event_loop.run_app(&mut app).unwrap();
    }
}

impl<P: ParticleTrait + 'static> Default for Simulation<P> {
    fn default() -> Self {
        Self::new()
    }
}

pub(crate) struct SimConfig {
    pub particle_count: u32,
    pub compute_shader: String,
    pub render_shader: String,
    pub has_neighbors: bool,
    pub spatial_config: SpatialConfig,
}

struct App<P: ParticleTrait> {
    window: Option<Arc<Window>>,
    gpu_state: Option<GpuState>,
    gpu_particles: Vec<P::Gpu>,
    config: SimConfig,
    mouse_pressed: bool,
    last_mouse_pos: Option<(f64, f64)>,
}

impl<P: ParticleTrait + 'static> App<P> {
    fn new(particles: Vec<P>, config: SimConfig) -> Self {
        // Convert user particles to GPU format
        let gpu_particles: Vec<P::Gpu> = particles.iter().map(|p| p.to_gpu()).collect();

        Self {
            window: None,
            gpu_state: None,
            gpu_particles,
            config,
            mouse_pressed: false,
            last_mouse_pos: None,
        }
    }
}

impl<P: ParticleTrait + 'static> ApplicationHandler for App<P> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let window_attrs = Window::default_attributes()
                .with_title("RDPE - Reaction Diffusion Particle Engine")
                .with_inner_size(winit::dpi::LogicalSize::new(1280, 720));

            let window = Arc::new(event_loop.create_window(window_attrs).unwrap());
            self.window = Some(window.clone());

            let particle_bytes = bytemuck::cast_slice(&self.gpu_particles);
            self.gpu_state = Some(pollster::block_on(GpuState::new(
                window,
                particle_bytes,
                self.config.particle_count,
                std::mem::size_of::<P::Gpu>(),
                &self.config.compute_shader,
                &self.config.render_shader,
                self.config.has_neighbors,
                self.config.spatial_config,
            )));
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(physical_size) => {
                if let Some(gpu_state) = &mut self.gpu_state {
                    gpu_state.resize(physical_size);
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if button == MouseButton::Left {
                    self.mouse_pressed = state == ElementState::Pressed;
                    if !self.mouse_pressed {
                        self.last_mouse_pos = None;
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                if self.mouse_pressed {
                    if let Some((last_x, last_y)) = self.last_mouse_pos {
                        let dx = position.x - last_x;
                        let dy = position.y - last_y;

                        if let Some(gpu_state) = &mut self.gpu_state {
                            gpu_state.camera.yaw -= dx as f32 * 0.005;
                            gpu_state.camera.pitch += dy as f32 * 0.005;
                            gpu_state.camera.pitch = gpu_state.camera.pitch.clamp(-1.5, 1.5);
                        }
                    }
                    self.last_mouse_pos = Some((position.x, position.y));
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let scroll = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y,
                    MouseScrollDelta::PixelDelta(pos) => pos.y as f32 * 0.1,
                };
                if let Some(gpu_state) = &mut self.gpu_state {
                    gpu_state.camera.distance -= scroll * 0.3;
                    gpu_state.camera.distance = gpu_state.camera.distance.clamp(0.5, 20.0);
                }
            }
            WindowEvent::RedrawRequested => {
                if let Some(gpu_state) = &mut self.gpu_state {
                    match gpu_state.render() {
                        Ok(_) => {}
                        Err(wgpu::SurfaceError::Lost) => {
                            gpu_state.resize(winit::dpi::PhysicalSize {
                                width: gpu_state.config.width,
                                height: gpu_state.config.height,
                            })
                        }
                        Err(wgpu::SurfaceError::OutOfMemory) => event_loop.exit(),
                        Err(e) => eprintln!("Render error: {:?}", e),
                    }
                }
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            _ => {}
        }
    }
}
