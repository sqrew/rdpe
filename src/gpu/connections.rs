//! Connection rendering between nearby particles.
//!
//! Draws lines between particles that are within a specified radius,
//! using spatial hashing for efficient neighbor queries.

use bytemuck::{Pod, Zeroable};
use glam::Vec3;
use wgpu::util::DeviceExt;

use super::{blend_mode_to_state, SpatialGpu, DEPTH_FORMAT};
use crate::visuals::BlendMode;

/// Parameters for connection rendering (compute shader).
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct ConnectionParams {
    radius: f32,
    max_connections: u32,
    num_particles: u32,
    _pad: u32,
}

/// Parameters for connection rendering (render shader).
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct RenderParams {
    color: [f32; 3],
    _pad: f32,
}

/// GPU resources for connection rendering.
#[allow(dead_code)]
pub struct ConnectionState {
    /// Buffer storing connection line segments.
    pub buffer: wgpu::Buffer,
    /// Atomic counter for number of connections found.
    pub count_buffer: wgpu::Buffer,
    /// Compute pipeline for finding connections.
    pub compute_pipeline: wgpu::ComputePipeline,
    /// Bind group for compute shader.
    pub compute_bind_group: wgpu::BindGroup,
    /// Render pipeline for drawing connections.
    pub render_pipeline: wgpu::RenderPipeline,
    /// Bind group for render shader.
    pub render_bind_group: wgpu::BindGroup,
    /// Maximum number of connections.
    pub max_connections: u32,
    /// Connection radius.
    pub radius: f32,
    /// Params buffer (kept alive for bind group).
    params_buffer: wgpu::Buffer,
    /// Render params buffer (kept alive for bind group).
    render_params_buffer: wgpu::Buffer,
}

impl ConnectionState {
    /// Create a new connection system.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        device: &wgpu::Device,
        particle_buffer: &wgpu::Buffer,
        uniform_buffer: &wgpu::Buffer,
        spatial: &SpatialGpu,
        num_particles: u32,
        radius: f32,
        color: Vec3,
        particle_stride: usize,
        blend_mode: BlendMode,
        surface_format: wgpu::TextureFormat,
    ) -> Self {
        let max_connections = num_particles * 8;

        // Connection buffer: stores line segments as vec4 pairs
        let buffer_size = (max_connections as usize) * 32; // 2 vec4s per connection
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Connection Buffer"),
            size: buffer_size as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::VERTEX,
            mapped_at_creation: false,
        });

        // Atomic counter for number of connections
        let count_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Connection Count Buffer"),
            contents: &[0u8; 4],
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        // Connection params (compute shader)
        let conn_params = ConnectionParams {
            radius,
            max_connections,
            num_particles,
            _pad: 0,
        };
        let params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Connection Params Buffer"),
            contents: bytemuck::bytes_of(&conn_params),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        // Render params (render shader)
        let render_params = RenderParams {
            color: color.to_array(),
            _pad: 0.0,
        };
        let render_params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Connection Render Params Buffer"),
            contents: bytemuck::bytes_of(&render_params),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        // Create compute pipeline
        let (compute_pipeline, compute_bind_group) = create_compute_pipeline(
            device,
            particle_buffer,
            &buffer,
            &count_buffer,
            &params_buffer,
            spatial,
            particle_stride,
        );

        // Create render pipeline
        let (render_pipeline, render_bind_group) = create_render_pipeline(
            device,
            uniform_buffer,
            &buffer,
            &render_params_buffer,
            blend_mode,
            surface_format,
        );

        Self {
            buffer,
            count_buffer,
            compute_pipeline,
            compute_bind_group,
            render_pipeline,
            render_bind_group,
            max_connections,
            radius,
            params_buffer,
            render_params_buffer,
        }
    }
}

fn create_compute_pipeline(
    device: &wgpu::Device,
    particle_buffer: &wgpu::Buffer,
    connection_buffer: &wgpu::Buffer,
    count_buffer: &wgpu::Buffer,
    params_buffer: &wgpu::Buffer,
    spatial: &SpatialGpu,
    particle_stride: usize,
) -> (wgpu::ComputePipeline, wgpu::BindGroup) {
    let shader_src = generate_compute_shader(particle_stride);
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Connection Compute Shader"),
        source: wgpu::ShaderSource::Wgsl(shader_src.into()),
    });

    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Connection Compute Bind Group Layout"),
        entries: &[
            // particles
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
            // connections
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
            // connection_count
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            // params
            wgpu::BindGroupLayoutEntry {
                binding: 3,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            // sorted_indices
            wgpu::BindGroupLayoutEntry {
                binding: 4,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            // cell_start
            wgpu::BindGroupLayoutEntry {
                binding: 5,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            // cell_end
            wgpu::BindGroupLayoutEntry {
                binding: 6,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            // spatial params
            wgpu::BindGroupLayoutEntry {
                binding: 7,
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
        label: Some("Connection Compute Bind Group"),
        layout: &bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: particle_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: connection_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: count_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: params_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 4,
                resource: spatial.particle_indices_a.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 5,
                resource: spatial.cell_start.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 6,
                resource: spatial.cell_end.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 7,
                resource: spatial.spatial_params_buffer.as_entire_binding(),
            },
        ],
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Connection Compute Pipeline Layout"),
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });

    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("Connection Compute Pipeline"),
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
    connection_buffer: &wgpu::Buffer,
    render_params_buffer: &wgpu::Buffer,
    blend_mode: BlendMode,
    surface_format: wgpu::TextureFormat,
) -> (wgpu::RenderPipeline, wgpu::BindGroup) {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Connection Render Shader"),
        source: wgpu::ShaderSource::Wgsl(RENDER_SHADER.into()),
    });

    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Connection Render Bind Group Layout"),
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
                visibility: wgpu::ShaderStages::FRAGMENT,
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
        label: Some("Connection Render Bind Group"),
        layout: &bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: connection_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: render_params_buffer.as_entire_binding(),
            },
        ],
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Connection Render Pipeline Layout"),
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });

    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Connection Render Pipeline"),
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
struct ConnectionParams {{
    radius: f32,
    max_connections: u32,
    num_particles: u32,
}};

struct SpatialParams {{
    cell_size: f32,
    grid_resolution: u32,
    num_particles: u32,
    max_neighbors: u32,
}};

fn morton_encode_10bit(x: u32, y: u32, z: u32) -> u32 {{
    var xx = x & 0x3FFu;
    var yy = y & 0x3FFu;
    var zz = z & 0x3FFu;

    xx = (xx | (xx << 16u)) & 0x030000FFu;
    xx = (xx | (xx << 8u)) & 0x0300F00Fu;
    xx = (xx | (xx << 4u)) & 0x030C30C3u;
    xx = (xx | (xx << 2u)) & 0x09249249u;

    yy = (yy | (yy << 16u)) & 0x030000FFu;
    yy = (yy | (yy << 8u)) & 0x0300F00Fu;
    yy = (yy | (yy << 4u)) & 0x030C30C3u;
    yy = (yy | (yy << 2u)) & 0x09249249u;

    zz = (zz | (zz << 16u)) & 0x030000FFu;
    zz = (zz | (zz << 8u)) & 0x0300F00Fu;
    zz = (zz | (zz << 4u)) & 0x030C30C3u;
    zz = (zz | (zz << 2u)) & 0x09249249u;

    return xx | (yy << 1u) | (zz << 2u);
}}

fn pos_to_cell(pos: vec3<f32>, cell_size: f32, grid_res: u32) -> vec3<i32> {{
    let half_grid = f32(grid_res) * 0.5;
    let grid_pos = (pos / cell_size) + half_grid;
    return vec3<i32>(
        clamp(i32(floor(grid_pos.x)), 0, i32(grid_res) - 1),
        clamp(i32(floor(grid_pos.y)), 0, i32(grid_res) - 1),
        clamp(i32(floor(grid_pos.z)), 0, i32(grid_res) - 1)
    );
}}

@group(0) @binding(0) var<storage, read> particles: array<vec4<f32>>;
@group(0) @binding(1) var<storage, read_write> connections: array<vec4<f32>>;
@group(0) @binding(2) var<storage, read_write> connection_count: atomic<u32>;
@group(0) @binding(3) var<uniform> params: ConnectionParams;
@group(0) @binding(4) var<storage, read> sorted_indices: array<u32>;
@group(0) @binding(5) var<storage, read> cell_start: array<u32>;
@group(0) @binding(6) var<storage, read> cell_end: array<u32>;
@group(0) @binding(7) var<uniform> spatial: SpatialParams;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {{
    let idx = global_id.x;
    if idx >= params.num_particles {{
        return;
    }}

    let my_pos = particles[idx * {particle_stride_vec4}u].xyz;
    let my_cell = pos_to_cell(my_pos, spatial.cell_size, spatial.grid_resolution);
    let radius_sq = params.radius * params.radius;

    for (var dx = -1; dx <= 1; dx++) {{
        for (var dy = -1; dy <= 1; dy++) {{
            for (var dz = -1; dz <= 1; dz++) {{
                let neighbor_cell = my_cell + vec3<i32>(dx, dy, dz);

                if neighbor_cell.x < 0 || neighbor_cell.x >= i32(spatial.grid_resolution) ||
                   neighbor_cell.y < 0 || neighbor_cell.y >= i32(spatial.grid_resolution) ||
                   neighbor_cell.z < 0 || neighbor_cell.z >= i32(spatial.grid_resolution) {{
                    continue;
                }}

                let morton = morton_encode_10bit(u32(neighbor_cell.x), u32(neighbor_cell.y), u32(neighbor_cell.z));
                let start = cell_start[morton];
                let end = cell_end[morton];

                if start == 0xFFFFFFFFu {{
                    continue;
                }}

                for (var j = start; j < end; j++) {{
                    let other_idx = sorted_indices[j];

                    if other_idx <= idx {{
                        continue;
                    }}

                    let other_pos = particles[other_idx * {particle_stride_vec4}u].xyz;
                    let diff = other_pos - my_pos;
                    let dist_sq = dot(diff, diff);

                    if dist_sq < radius_sq && dist_sq > 0.0001 {{
                        let conn_idx = atomicAdd(&connection_count, 1u);
                        if conn_idx < params.max_connections {{
                            let dist = sqrt(dist_sq);
                            let alpha = 1.0 - dist / params.radius;
                            connections[conn_idx * 2u] = vec4<f32>(my_pos, alpha);
                            connections[conn_idx * 2u + 1u] = vec4<f32>(other_pos, 0.0);
                        }}
                    }}
                }}
            }}
        }}
    }}
}}
"#
    )
}

const RENDER_SHADER: &str = r#"
struct Uniforms {
    view_proj: mat4x4<f32>,
    time: f32,
    delta_time: f32,
};

struct RenderParams {
    color: vec3<f32>,
};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var<storage, read> connections: array<vec4<f32>>;
@group(0) @binding(2) var<uniform> render_params: RenderParams;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) alpha: f32,
};

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32,
) -> VertexOutput {
    var out: VertexOutput;

    let conn_data_a = connections[instance_index * 2u];
    let conn_data_b = connections[instance_index * 2u + 1u];

    let pos_a = conn_data_a.xyz;
    let pos_b = conn_data_b.xyz;
    let alpha = conn_data_a.w;

    if alpha < 0.001 {
        out.clip_position = vec4<f32>(0.0, 0.0, -1000.0, 1.0);
        out.alpha = 0.0;
        return out;
    }

    let line_dir = normalize(pos_b - pos_a);

    var perp = cross(line_dir, vec3<f32>(0.0, 1.0, 0.0));
    if length(perp) < 0.001 {
        perp = cross(line_dir, vec3<f32>(1.0, 0.0, 0.0));
    }
    perp = normalize(perp) * 0.002;

    var pos: vec3<f32>;
    switch vertex_index {
        case 0u: { pos = pos_a - perp; }
        case 1u: { pos = pos_a + perp; }
        case 2u: { pos = pos_b - perp; }
        case 3u: { pos = pos_a + perp; }
        case 4u: { pos = pos_b - perp; }
        default: { pos = pos_b + perp; }
    }

    out.clip_position = uniforms.view_proj * vec4<f32>(pos, 1.0);
    out.alpha = alpha * 0.5;

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(render_params.color, in.alpha);
}
"#;
