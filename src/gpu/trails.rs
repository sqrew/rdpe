//! Trail rendering system for particle motion history.
//!
//! Stores position history for each particle and renders fading trails
//! behind moving particles.

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

use super::{blend_mode_to_state, DEPTH_FORMAT};
use crate::visuals::BlendMode;

/// Parameters for trail rendering.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct TrailParams {
    num_particles: u32,
    trail_length: u32,
    _pad: [u32; 2],
}

/// GPU resources for trail rendering.
#[allow(dead_code)]
pub struct TrailState {
    /// Buffer storing trail position history.
    pub buffer: wgpu::Buffer,
    /// Compute pipeline for updating trails.
    pub compute_pipeline: wgpu::ComputePipeline,
    /// Bind group for compute shader.
    pub compute_bind_group: wgpu::BindGroup,
    /// Render pipeline for drawing trails.
    pub render_pipeline: wgpu::RenderPipeline,
    /// Bind group for render shader.
    pub render_bind_group: wgpu::BindGroup,
    /// Number of particles.
    pub num_particles: u32,
    /// Trail length per particle.
    pub trail_length: u32,
    /// Params buffer (kept alive for bind group).
    params_buffer: wgpu::Buffer,
}

impl TrailState {
    /// Create a new trail system.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        device: &wgpu::Device,
        particle_buffer: &wgpu::Buffer,
        uniform_buffer: &wgpu::Buffer,
        num_particles: u32,
        trail_length: u32,
        particle_stride: usize,
        particle_size: f32,
        blend_mode: BlendMode,
        surface_format: wgpu::TextureFormat,
    ) -> Self {
        // Trail buffer: stores position history for each particle
        // Each entry is vec4<f32> (xyz = position, w = alpha/validity)
        let buffer_size = (num_particles as usize) * (trail_length as usize) * 16;
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Trail Buffer"),
            size: buffer_size as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::VERTEX,
            mapped_at_creation: false,
        });

        // Trail params uniform
        let trail_params = TrailParams {
            num_particles,
            trail_length,
            _pad: [0; 2],
        };
        let params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Trail Params Buffer"),
            contents: bytemuck::bytes_of(&trail_params),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        // Create compute pipeline
        let (compute_pipeline, compute_bind_group) = create_compute_pipeline(
            device,
            particle_buffer,
            &buffer,
            &params_buffer,
            particle_stride,
        );

        // Create render pipeline
        let (render_pipeline, render_bind_group) = create_render_pipeline(
            device,
            uniform_buffer,
            &buffer,
            &params_buffer,
            particle_size,
            blend_mode,
            surface_format,
        );

        Self {
            buffer,
            compute_pipeline,
            compute_bind_group,
            render_pipeline,
            render_bind_group,
            num_particles,
            trail_length,
            params_buffer,
        }
    }
}

fn create_compute_pipeline(
    device: &wgpu::Device,
    particle_buffer: &wgpu::Buffer,
    trail_buffer: &wgpu::Buffer,
    params_buffer: &wgpu::Buffer,
    particle_stride: usize,
) -> (wgpu::ComputePipeline, wgpu::BindGroup) {
    let shader_src = generate_compute_shader(particle_stride);
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Trail Compute Shader"),
        source: wgpu::ShaderSource::Wgsl(shader_src.into()),
    });

    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Trail Compute Bind Group Layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    });

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Trail Compute Bind Group"),
        layout: &bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: particle_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: trail_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: params_buffer.as_entire_binding(),
            },
        ],
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Trail Compute Pipeline Layout"),
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });

    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("Trail Compute Pipeline"),
        layout: Some(&pipeline_layout),
        module: &shader,
        entry_point: Some("main"),
        compilation_options: Default::default(),
        cache: None,
    });

    (pipeline, bind_group)
}

fn create_render_pipeline(
    device: &wgpu::Device,
    uniform_buffer: &wgpu::Buffer,
    trail_buffer: &wgpu::Buffer,
    params_buffer: &wgpu::Buffer,
    particle_size: f32,
    blend_mode: BlendMode,
    surface_format: wgpu::TextureFormat,
) -> (wgpu::RenderPipeline, wgpu::BindGroup) {
    let shader_src = generate_render_shader(particle_size);
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Trail Render Shader"),
        source: wgpu::ShaderSource::Wgsl(shader_src.into()),
    });

    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Trail Render Bind Group Layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    });

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Trail Render Bind Group"),
        layout: &bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: trail_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: params_buffer.as_entire_binding(),
            },
        ],
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Trail Render Pipeline Layout"),
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });

    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Trail Render Pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format: surface_format,
                blend: Some(blend_mode_to_state(blend_mode)),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            polygon_mode: wgpu::PolygonMode::Fill,
            unclipped_depth: false,
            conservative: false,
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: false,
            depth_compare: wgpu::CompareFunction::Less,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
        cache: None,
    });

    (pipeline, bind_group)
}

fn generate_compute_shader(particle_stride: usize) -> String {
    let particle_stride_vec4 = particle_stride / 16;
    format!(
        r#"
struct TrailParams {{
    num_particles: u32,
    trail_length: u32,
}};

@group(0) @binding(0)
var<storage, read> particles: array<vec4<f32>>;

@group(0) @binding(1)
var<storage, read_write> trails: array<vec4<f32>>;

@group(0) @binding(2)
var<uniform> params: TrailParams;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {{
    let particle_idx = global_id.x;
    if particle_idx >= params.num_particles {{
        return;
    }}

    let trail_base = particle_idx * params.trail_length;

    // Shift trail positions back (from end to start)
    for (var i = params.trail_length - 1u; i > 0u; i--) {{
        trails[trail_base + i] = trails[trail_base + i - 1u];
    }}

    // Store current position at front with full alpha
    let pos = particles[particle_idx * {particle_stride_vec4}u];
    trails[trail_base] = vec4<f32>(pos.xyz, 1.0);
}}
"#
    )
}

fn generate_render_shader(particle_size: f32) -> String {
    format!(
        r#"
struct Uniforms {{
    view_proj: mat4x4<f32>,
    time: f32,
    delta_time: f32,
}};

struct TrailParams {{
    num_particles: u32,
    trail_length: u32,
}};

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

@group(0) @binding(1)
var<storage, read> trails: array<vec4<f32>>;

@group(0) @binding(2)
var<uniform> params: TrailParams;

struct VertexOutput {{
    @builtin(position) clip_position: vec4<f32>,
    @location(0) alpha: f32,
    @location(1) uv: vec2<f32>,
}};

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32,
) -> VertexOutput {{
    var out: VertexOutput;

    // Decode particle and trail index from instance
    let particle_idx = instance_index / params.trail_length;
    let trail_idx = instance_index % params.trail_length;

    // Get trail position
    let trail_base = particle_idx * params.trail_length;
    let trail_data = trails[trail_base + trail_idx];
    let pos = trail_data.xyz;
    let valid = trail_data.w;

    // Skip invalid trail points
    if valid < 0.5 {{
        out.clip_position = vec4<f32>(0.0, 0.0, -1000.0, 1.0);
        out.alpha = 0.0;
        out.uv = vec2<f32>(0.0);
        return out;
    }}

    // Quad vertices
    var quad_vertices = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>( 1.0,  1.0),
    );

    let quad_pos = quad_vertices[vertex_index];

    // Size decreases along trail, alpha also fades
    let trail_progress = f32(trail_idx) / f32(params.trail_length);
    let size_factor = 1.0 - trail_progress * 0.7;
    let alpha_factor = 1.0 - trail_progress;

    let base_size = {particle_size};
    let trail_size = base_size * size_factor * 0.5;

    let world_pos = vec4<f32>(pos, 1.0);
    var clip_pos = uniforms.view_proj * world_pos;

    clip_pos.x += quad_pos.x * trail_size * clip_pos.w;
    clip_pos.y += quad_pos.y * trail_size * clip_pos.w;

    out.clip_position = clip_pos;
    out.alpha = alpha_factor * 0.5;
    out.uv = quad_pos;

    return out;
}}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {{
    let dist = length(in.uv);
    if dist > 1.0 {{
        discard;
    }}
    let circle_alpha = 1.0 - smoothstep(0.3, 1.0, dist);
    let color = vec3<f32>(0.7, 0.8, 1.0);
    return vec4<f32>(color, circle_alpha * in.alpha);
}}
"#
    )
}
