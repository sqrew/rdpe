//! Trail visualization for particle systems.
//!
//! This module provides GPU-accelerated trail rendering for particles. It maintains
//! a history of past particle positions and renders them as connected line segments
//! with alpha blending to create smooth, fading trails.
//!
//! The trail system uses two pipelines:
//! - A compute pipeline to update the trail history buffer each frame
//! - A render pipeline to draw the trail segments as textured quads

use bytemuck;
use wgpu;
use wgpu::util::DeviceExt;

const TRAIL_COMPUTE_SHADER: &str = r#"
struct TrailParams {
    num_particles: u32,
    trail_length: u32,
    particle_stride: u32,
    alive_offset: u32,
};

@group(0) @binding(0) var<storage, read> particles: array<u32>;
@group(0) @binding(1) var<storage, read_write> trails: array<vec4<f32>>;
@group(0) @binding(2) var<uniform> params: TrailParams;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let particle_idx = global_id.x;
    if particle_idx >= params.num_particles {
        return;
    }

    // Read particle data
    let base = particle_idx * params.particle_stride;
    let alive = particles[base + params.alive_offset];

    // Read position
    let pos = vec3<f32>(
        bitcast<f32>(particles[base]),
        bitcast<f32>(particles[base + 1u]),
        bitcast<f32>(particles[base + 2u])
    );

    // Trail base index for this particle
    let trail_base = particle_idx * params.trail_length;

    // Shift trail positions (from end to start, so we don't overwrite)
    for (var i = params.trail_length - 1u; i > 0u; i = i - 1u) {
        trails[trail_base + i] = trails[trail_base + i - 1u];
    }

    // Write current position to front of trail
    // Alpha is 1.0 for alive particles, 0.0 for dead
    let alpha = select(0.0, 1.0, alive != 0u);
    trails[trail_base] = vec4<f32>(pos, alpha);
}
"#;

const TRAIL_RENDER_SHADER: &str = r#"
struct Uniforms {
    view_proj: mat4x4<f32>,
    time: f32,
    delta_time: f32,
};

struct TrailParams {
    num_particles: u32,
    trail_length: u32,
    particle_stride: u32,
    alive_offset: u32,
};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var<storage, read> trails: array<vec4<f32>>;
@group(0) @binding(2) var<uniform> params: TrailParams;

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

    // Decode particle index and segment index from instance
    let segments_per_particle = params.trail_length - 1u;
    let particle_idx = instance_index / segments_per_particle;
    let segment_idx = instance_index % segments_per_particle;

    // Trail base for this particle
    let trail_base = particle_idx * params.trail_length;

    // Get the two positions for this segment
    let pos_a = trails[trail_base + segment_idx];
    let pos_b = trails[trail_base + segment_idx + 1u];

    // Check if segment is valid (both ends have alpha > 0)
    if pos_a.w < 0.001 || pos_b.w < 0.001 {
        out.clip_position = vec4<f32>(0.0, 0.0, -1000.0, 1.0);
        out.alpha = 0.0;
        return out;
    }

    // Calculate alpha based on segment position (fade toward end)
    let segment_t = f32(segment_idx) / f32(segments_per_particle);
    let base_alpha = 1.0 - segment_t * 0.9; // Fade from 1.0 to 0.1

    // Build line segment quad
    let line_dir = pos_b.xyz - pos_a.xyz;
    let line_len = length(line_dir);

    if line_len < 0.0001 {
        out.clip_position = vec4<f32>(0.0, 0.0, -1000.0, 1.0);
        out.alpha = 0.0;
        return out;
    }

    let dir = line_dir / line_len;

    // Perpendicular for line thickness (thinner for older segments)
    let thickness = 0.003 * (1.0 - segment_t * 0.7);
    var perp = cross(dir, vec3<f32>(0.0, 1.0, 0.0));
    if length(perp) < 0.001 {
        perp = cross(dir, vec3<f32>(1.0, 0.0, 0.0));
    }
    perp = normalize(perp) * thickness;

    // Build quad vertices
    var pos: vec3<f32>;
    switch vertex_index {
        case 0u: { pos = pos_a.xyz - perp; }
        case 1u: { pos = pos_a.xyz + perp; }
        case 2u: { pos = pos_b.xyz - perp; }
        case 3u: { pos = pos_a.xyz + perp; }
        case 4u: { pos = pos_b.xyz - perp; }
        default: { pos = pos_b.xyz + perp; }
    }

    out.clip_position = uniforms.view_proj * vec4<f32>(pos, 1.0);
    out.alpha = base_alpha * min(pos_a.w, pos_b.w);

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(0.7, 0.85, 1.0, in.alpha * 0.6);
}
"#;

/// GPU-accelerated trail visualization for particle systems.
///
/// Maintains a circular buffer of past positions for each particle and renders
/// them as connected line segments with smooth alpha blending.
pub(crate) struct TrailVisualization {
    /// Buffer storing past positions for each particle.
    /// Layout: [p0_t0, p0_t1, ..., p0_tN, p1_t0, p1_t1, ..., p1_tN, ...]
    /// Each position is vec4 (xyz + alpha).
    _trail_buffer: wgpu::Buffer,
    /// Compute pipeline to update trail history.
    compute_pipeline: wgpu::ComputePipeline,
    /// Compute bind group.
    compute_bind_group: wgpu::BindGroup,
    /// Render pipeline for drawing trails.
    render_pipeline: wgpu::RenderPipeline,
    /// Render bind group.
    render_bind_group: wgpu::BindGroup,
    /// Number of particles.
    num_particles: u32,
    /// Trail length (number of past positions stored).
    trail_length: u32,
}

impl TrailVisualization {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        device: &wgpu::Device,
        particle_buffer: &wgpu::Buffer,
        uniform_buffer: &wgpu::Buffer,
        num_particles: u32,
        trail_length: u32,
        particle_stride: usize,
        alive_offset: u32,
        target_format: wgpu::TextureFormat,
    ) -> Self {
        let particle_stride_u32 = particle_stride / 4;

        // Trail buffer: num_particles * trail_length * vec4 (16 bytes each)
        let buffer_size = (num_particles as usize) * (trail_length as usize) * 16;
        let trail_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Trail Buffer"),
            size: buffer_size as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Params buffer: [num_particles, trail_length, particle_stride, alive_offset]
        let params: [u32; 4] = [num_particles, trail_length, particle_stride_u32 as u32, alive_offset];
        let params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Trail Params Buffer"),
            contents: bytemuck::cast_slice(&params),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        // Create compute shader
        let compute_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Trail Compute Shader"),
            source: wgpu::ShaderSource::Wgsl(TRAIL_COMPUTE_SHADER.into()),
        });

        // Compute bind group layout
        let compute_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Trail Compute Bind Group Layout"),
            entries: &[
                // Particle buffer (read)
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
                // Trail buffer (read/write)
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
                // Params
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

        let compute_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Trail Compute Bind Group"),
            layout: &compute_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: particle_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: trail_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: params_buffer.as_entire_binding() },
            ],
        });

        let compute_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Trail Compute Pipeline Layout"),
            bind_group_layouts: &[&compute_bind_group_layout],
            push_constant_ranges: &[],
        });

        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Trail Compute Pipeline"),
            layout: Some(&compute_pipeline_layout),
            module: &compute_shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });

        // Create render shader
        let render_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Trail Render Shader"),
            source: wgpu::ShaderSource::Wgsl(TRAIL_RENDER_SHADER.into()),
        });

        // Render bind group layout
        let render_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Trail Render Bind Group Layout"),
            entries: &[
                // Uniforms
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
                // Trail buffer
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
                // Params
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

        let render_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Trail Render Bind Group"),
            layout: &render_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: uniform_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: trail_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: params_buffer.as_entire_binding() },
            ],
        });

        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Trail Render Pipeline Layout"),
            bind_group_layouts: &[&render_bind_group_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Trail Render Pipeline"),
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
            _trail_buffer: trail_buffer,
            compute_pipeline,
            compute_bind_group,
            render_pipeline,
            render_bind_group,
            num_particles,
            trail_length,
        }
    }

    pub(crate) fn compute(&self, encoder: &mut wgpu::CommandEncoder) {
        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Trail Compute Pass"),
            timestamp_writes: None,
        });
        compute_pass.set_pipeline(&self.compute_pipeline);
        compute_pass.set_bind_group(0, &self.compute_bind_group, &[]);
        compute_pass.dispatch_workgroups(self.num_particles.div_ceil(256), 1, 1);
    }

    pub(crate) fn render(&self, render_pass: &mut wgpu::RenderPass<'static>) {
        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, &self.render_bind_group, &[]);
        // Draw 6 vertices per line segment, (trail_length - 1) segments per particle
        let segments_per_particle = self.trail_length.saturating_sub(1);
        let total_segments = self.num_particles * segments_per_particle;
        render_pass.draw(0..6, 0..total_segments);
    }
}
