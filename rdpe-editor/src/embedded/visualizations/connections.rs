//! Connection visualization between particles.
//!
//! This module provides GPU-accelerated visualization of connections between particles
//! within a specified radius. It uses spatial hashing for efficient neighbor finding
//! and renders connections as thin lines with distance-based alpha blending.

use rdpe::SpatialGpu;
use wgpu::util::DeviceExt;

/// Visualizes connections between nearby particles using GPU compute and rendering.
///
/// This struct manages the GPU resources needed to:
/// - Find connections between particles within a specified radius using spatial hashing
/// - Store connection data as line segments
/// - Render connections with distance-based alpha blending
pub(crate) struct ConnectionVisualization {
    /// Buffer storing connection line segments.
    _connection_buffer: wgpu::Buffer,
    /// Atomic counter for connections found.
    count_buffer: wgpu::Buffer,
    /// Compute pipeline to find connections.
    compute_pipeline: wgpu::ComputePipeline,
    /// Compute bind group.
    compute_bind_group: wgpu::BindGroup,
    /// Render pipeline for drawing connections.
    render_pipeline: wgpu::RenderPipeline,
    /// Render bind group.
    render_bind_group: wgpu::BindGroup,
    /// Maximum connections.
    max_connections: u32,
    /// Connection radius.
    _radius: f32,
    /// Number of particles.
    num_particles: u32,
}

impl ConnectionVisualization {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        device: &wgpu::Device,
        particle_buffer: &wgpu::Buffer,
        uniform_buffer: &wgpu::Buffer,
        spatial: &SpatialGpu,
        num_particles: u32,
        radius: f32,
        color: [f32; 3],
        thickness: f32,
        particle_stride: usize,
        target_format: wgpu::TextureFormat,
    ) -> Self {
        // Each particle gets a fixed number of connection slots for deterministic results
        let max_connections_per_particle: u32 = 8;
        let max_connections = num_particles * max_connections_per_particle;
        let particle_stride_vec4 = particle_stride / 16;

        // Connection buffer: stores line segments as vec4 pairs
        // Initialize with zeros to ensure no undefined behavior on first frame
        let buffer_size = (max_connections as usize) * 32; // 2 vec4s per connection
        let connection_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Connection Buffer"),
            contents: &vec![0u8; buffer_size],
            usage: wgpu::BufferUsages::STORAGE,
        });

        // Atomic counter (kept for compatibility but not used in deterministic mode)
        let count_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Connection Count Buffer"),
            contents: &[0u8; 4],
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        // Params buffer: [radius, max_connections, num_particles, max_per_particle]
        let params_data: [u32; 4] = [
            radius.to_bits(),
            max_connections,
            num_particles,
            max_connections_per_particle,
        ];
        let params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Connection Params Buffer"),
            contents: bytemuck::cast_slice(&params_data),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        // Create compute shader with deterministic slot allocation
        let compute_shader_src = Self::generate_compute_shader(particle_stride_vec4, max_connections_per_particle);
        let compute_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Connection Compute Shader"),
            source: wgpu::ShaderSource::Wgsl(compute_shader_src.into()),
        });

        // Compute bind group layout
        let compute_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Connection Compute Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: false }, has_dynamic_offset: false, min_binding_size: None }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 2, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: false }, has_dynamic_offset: false, min_binding_size: None }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 3, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 4, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 5, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 6, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 7, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None }, count: None },
            ],
        });

        let compute_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Connection Compute Bind Group"),
            layout: &compute_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: particle_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: connection_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: count_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 3, resource: params_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 4, resource: spatial.particle_indices_a.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 5, resource: spatial.cell_start.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 6, resource: spatial.cell_end.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 7, resource: spatial.spatial_params_buffer.as_entire_binding() },
            ],
        });

        let compute_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Connection Compute Pipeline Layout"),
            bind_group_layouts: &[&compute_bind_group_layout],
            push_constant_ranges: &[],
        });

        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Connection Compute Pipeline"),
            layout: Some(&compute_pipeline_layout),
            module: &compute_shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });

        // Create render shader with color and thickness
        let render_shader_src = generate_connection_render_shader(color, thickness);
        let render_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Connection Render Shader"),
            source: wgpu::ShaderSource::Wgsl(render_shader_src.into()),
        });

        let render_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Connection Render Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::VERTEX, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::VERTEX, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None }, count: None },
            ],
        });

        let render_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Connection Render Bind Group"),
            layout: &render_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: uniform_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: connection_buffer.as_entire_binding() },
            ],
        });

        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Connection Render Pipeline Layout"),
            bind_group_layouts: &[&render_bind_group_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Connection Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &render_shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &render_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: target_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        Self {
            _connection_buffer: connection_buffer,
            count_buffer,
            compute_pipeline,
            compute_bind_group,
            render_pipeline,
            render_bind_group,
            max_connections,
            _radius: radius,
            num_particles,
        }
    }

    fn generate_compute_shader(particle_stride_vec4: usize, max_connections_per_particle: u32) -> String {
        // BRUTE FORCE VERSION: O(n²) but completely deterministic
        // This bypasses the spatial hash entirely to test if it's the cause of flickering
        format!(r#"
struct ConnectionParams {{
    radius: f32,
    max_connections: u32,
    num_particles: u32,
    max_per_particle: u32,
}};

struct SpatialParams {{
    cell_size: f32,
    grid_resolution: u32,
    num_particles: u32,
    max_neighbors: u32,
}};

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
    // Add small epsilon to radius to prevent flickering at boundary due to float precision
    let radius_with_epsilon = params.radius * 1.001;
    let radius_sq = radius_with_epsilon * radius_with_epsilon;

    // Each particle has dedicated slots for its connections (deterministic)
    let my_slot_base = idx * {max_per_particle}u;

    // BRUTE FORCE: Iterate through ALL particles in index order
    // This is O(n²) but completely deterministic - no spatial hash involved
    // We iterate from idx+1 to num_particles to only create connections from lower to higher index
    var neighbor_data: array<vec4<f32>, {max_per_particle}>;  // xyz = other_pos, w = dist_sq
    var neighbor_count = 0u;

    for (var other_idx = idx + 1u; other_idx < params.num_particles; other_idx++) {{
        // Early exit if we've found enough neighbors
        if neighbor_count >= {max_per_particle}u {{
            break;
        }}

        let other_pos = particles[other_idx * {particle_stride_vec4}u].xyz;
        let diff = other_pos - my_pos;
        let dist_sq = dot(diff, diff);

        if dist_sq < radius_sq && dist_sq > 0.0001 {{
            // Since we iterate in index order, first N found are the lowest-indexed neighbors
            neighbor_data[neighbor_count] = vec4<f32>(other_pos, dist_sq);
            neighbor_count += 1u;
        }}
    }}

    // Write to deterministic slots
    for (var i = 0u; i < neighbor_count; i++) {{
        let slot = my_slot_base + i;
        if slot * 2u + 1u < params.max_connections * 2u {{
            let other_pos = neighbor_data[i].xyz;
            let dist = sqrt(neighbor_data[i].w);
            let alpha = 1.0 - dist / params.radius;
            connections[slot * 2u] = vec4<f32>(my_pos, alpha);
            connections[slot * 2u + 1u] = vec4<f32>(other_pos, 0.0);
        }}
    }}

    // Clear unused slots for this particle (both vec4s)
    for (var i = neighbor_count; i < {max_per_particle}u; i++) {{
        let slot = my_slot_base + i;
        if slot * 2u + 1u < params.max_connections * 2u {{
            connections[slot * 2u] = vec4<f32>(0.0, 0.0, 0.0, 0.0);
            connections[slot * 2u + 1u] = vec4<f32>(0.0, 0.0, 0.0, 0.0);
        }}
    }}
}}
"#, particle_stride_vec4 = particle_stride_vec4, max_per_particle = max_connections_per_particle)
    }

    pub(crate) fn compute(&self, encoder: &mut wgpu::CommandEncoder, queue: &wgpu::Queue) {
        // Reset connection count
        queue.write_buffer(&self.count_buffer, 0, &[0u8; 4]);

        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Connection Compute Pass"),
            timestamp_writes: None,
        });
        compute_pass.set_pipeline(&self.compute_pipeline);
        compute_pass.set_bind_group(0, &self.compute_bind_group, &[]);
        compute_pass.dispatch_workgroups(self.num_particles.div_ceil(256), 1, 1);
    }

    pub(crate) fn render(&self, render_pass: &mut wgpu::RenderPass<'_>) {
        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, &self.render_bind_group, &[]);
        // Draw max_connections instances (empty ones will be culled by alpha check)
        render_pass.draw(0..6, 0..self.max_connections);
    }
}

/// Generates the WGSL shader code for rendering connections.
///
/// Creates a shader that renders connections as thin lines between particles,
/// with distance-based alpha blending for fade-out effects.
///
/// # Arguments
///
/// * `color` - RGB color values for the connection lines (range 0.0-1.0)
/// * `thickness` - Line thickness multiplier (1.0 = default 0.003 world units)
pub(crate) fn generate_connection_render_shader(color: [f32; 3], thickness: f32) -> String {
    let line_thickness = 0.003 * thickness;
    format!(r#"
struct Uniforms {{
    view_proj: mat4x4<f32>,
    time: f32,
    delta_time: f32,
}};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var<storage, read> connections: array<vec4<f32>>;

struct VertexOutput {{
    @builtin(position) clip_position: vec4<f32>,
    @location(0) alpha: f32,
}};

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32,
) -> VertexOutput {{
    var out: VertexOutput;

    let conn_data_a = connections[instance_index * 2u];
    let conn_data_b = connections[instance_index * 2u + 1u];

    let pos_a = conn_data_a.xyz;
    let pos_b = conn_data_b.xyz;
    let alpha = conn_data_a.w;

    if alpha < 0.001 {{
        out.clip_position = vec4<f32>(0.0, 0.0, -1000.0, 1.0);
        out.alpha = 0.0;
        return out;
    }}

    let line_dir = normalize(pos_b - pos_a);

    var perp = cross(line_dir, vec3<f32>(0.0, 1.0, 0.0));
    if length(perp) < 0.001 {{
        perp = cross(line_dir, vec3<f32>(1.0, 0.0, 0.0));
    }}
    perp = normalize(perp) * {};

    var pos: vec3<f32>;
    switch vertex_index {{
        case 0u: {{ pos = pos_a - perp; }}
        case 1u: {{ pos = pos_a + perp; }}
        case 2u: {{ pos = pos_b - perp; }}
        case 3u: {{ pos = pos_a + perp; }}
        case 4u: {{ pos = pos_b - perp; }}
        default: {{ pos = pos_b + perp; }}
    }}

    out.clip_position = uniforms.view_proj * vec4<f32>(pos, 1.0);
    out.alpha = alpha * 0.6;

    return out;
}}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {{
    return vec4<f32>({}, {}, {}, in.alpha);
}}
"#, line_thickness, color[0], color[1], color[2])
}
