//! Wireframe mesh visualization for particles.
//!
//! This module provides a visualization system that renders wireframe meshes
//! at each particle position. Each particle can have its own position, scale,
//! color, and alive state, with the wireframe mesh geometry being shared across
//! all particles.

use wgpu::util::DeviceExt;

use crate::config::BlendModeConfig;

/// Wireframe mesh visualization renderer.
///
/// Renders a wireframe mesh at each particle position, with per-particle
/// transformations and colors. The mesh is defined as a set of line segments,
/// and each line is rendered as a thin quad to ensure visibility.
pub(crate) struct WireframeVisualization {
    /// Buffer storing mesh line segments (6 floats per line: x0,y0,z0,x1,y1,z1).
    mesh_buffer: wgpu::Buffer,
    /// Number of lines per mesh.
    lines_per_mesh: u32,
    /// Render pipeline.
    pipeline: wgpu::RenderPipeline,
    /// Bind group.
    bind_group: wgpu::BindGroup,
    /// Params buffer.
    params_buffer: wgpu::Buffer,
    /// Number of particles.
    num_particles: u32,
    /// Base particle size.
    base_size: f32,
}

impl WireframeVisualization {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        device: &wgpu::Device,
        particle_buffer: &wgpu::Buffer,
        uniform_buffer: &wgpu::Buffer,
        mesh: &rdpe::WireframeMesh,
        line_thickness: f32,
        particle_size: f32,
        num_particles: u32,
        particle_stride: usize,
        color_offset: Option<u32>,
        alive_offset: u32,
        scale_offset: u32,
        target_format: wgpu::TextureFormat,
        blend_mode: &BlendModeConfig,
    ) -> Self {
        // Convert mesh lines to flat f32 array
        let mesh_data = mesh.to_vertices();
        let lines_per_mesh = mesh.line_count();

        let mesh_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Wireframe Mesh Buffer"),
            contents: bytemuck::cast_slice(&mesh_data),
            usage: wgpu::BufferUsages::STORAGE,
        });

        // Wireframe params: [line_thickness, lines_per_mesh, base_size, _pad]
        let params: [f32; 4] = [
            line_thickness,
            f32::from_bits(lines_per_mesh),
            particle_size,
            0.0,
        ];
        let params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Wireframe Params Buffer"),
            contents: bytemuck::cast_slice(&params),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Generate shader
        let shader_src = Self::generate_shader(
            particle_stride,
            color_offset,
            alive_offset,
            scale_offset,
        );

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Wireframe Shader"),
            source: wgpu::ShaderSource::Wgsl(shader_src.into()),
        });

        // Bind group layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Wireframe Bind Group Layout"),
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
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
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
            label: Some("Wireframe Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: particle_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: mesh_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Wireframe Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        // Convert blend mode
        let blend_state = blend_mode.to_wgpu_blend_state();

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Wireframe Pipeline"),
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
                    format: target_format,
                    blend: Some(blend_state),
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
            depth_stencil: None, // No depth buffer in egui_wgpu
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        Self {
            mesh_buffer,
            lines_per_mesh,
            pipeline,
            bind_group,
            params_buffer,
            num_particles,
            base_size: particle_size,
        }
    }

    fn generate_shader(
        particle_stride: usize,
        color_offset: Option<u32>,
        alive_offset: u32,
        scale_offset: u32,
    ) -> String {
        let stride_u32 = particle_stride / 4;
        let alive_idx = alive_offset / 4;
        let scale_idx = scale_offset / 4;

        let color_code = if let Some(offset) = color_offset {
            let color_idx = offset / 4;
            format!(
                r#"
    // Read particle color (3 floats)
    let color = vec3<f32>(
        bitcast<f32>(particle_data[base + {color_idx}u]),
        bitcast<f32>(particle_data[base + {color_idx}u + 1u]),
        bitcast<f32>(particle_data[base + {color_idx}u + 2u])
    );"#,
                color_idx = color_idx
            )
        } else {
            r#"
    let color = normalize(particle_pos) * 0.5 + 0.5;"#.to_string()
        };

        format!(
            r#"struct Uniforms {{
    view_proj: mat4x4<f32>,
    time: f32,
    delta_time: f32,
}};

struct WireframeParams {{
    line_thickness: f32,
    lines_per_mesh: u32,
    base_size: f32,
}};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var<storage, read> particle_data: array<u32>;
@group(0) @binding(2) var<storage, read> mesh_lines: array<f32>;
@group(0) @binding(3) var<uniform> params: WireframeParams;

struct VertexOutput {{
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
}};

const PARTICLE_STRIDE: u32 = {stride_u32}u;

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32,
) -> VertexOutput {{
    var out: VertexOutput;

    // Decode particle index and line index from instance
    let particle_idx = instance_index / params.lines_per_mesh;
    let line_idx = instance_index % params.lines_per_mesh;

    // Read particle data
    let base = particle_idx * PARTICLE_STRIDE;

    // Read alive flag
    let alive = particle_data[base + {alive_idx}u];
    if alive == 0u {{
        out.clip_position = vec4<f32>(0.0, 0.0, -1000.0, 1.0);
        out.color = vec3<f32>(0.0);
        return out;
    }}

    // Read particle position (first 3 floats)
    let particle_pos = vec3<f32>(
        bitcast<f32>(particle_data[base]),
        bitcast<f32>(particle_data[base + 1u]),
        bitcast<f32>(particle_data[base + 2u])
    );

    // Read particle scale
    let scale = bitcast<f32>(particle_data[base + {scale_idx}u]);
{color_code}

    // Read line endpoints from mesh buffer (6 floats per line)
    let line_base = line_idx * 6u;
    let local_a = vec3<f32>(
        mesh_lines[line_base],
        mesh_lines[line_base + 1u],
        mesh_lines[line_base + 2u]
    );
    let local_b = vec3<f32>(
        mesh_lines[line_base + 3u],
        mesh_lines[line_base + 4u],
        mesh_lines[line_base + 5u]
    );

    // Transform to world space
    let mesh_scale = params.base_size * scale;
    let world_a = particle_pos + local_a * mesh_scale;
    let world_b = particle_pos + local_b * mesh_scale;

    // Create thin quad along the line
    let line_dir = world_b - world_a;
    let line_len = length(line_dir);

    if line_len < 0.0001 {{
        out.clip_position = vec4<f32>(0.0, 0.0, -1000.0, 1.0);
        out.color = vec3<f32>(0.0);
        return out;
    }}

    let dir = line_dir / line_len;

    // Find perpendicular direction for line width
    var perp = cross(dir, vec3<f32>(0.0, 1.0, 0.0));
    if length(perp) < 0.001 {{
        perp = cross(dir, vec3<f32>(1.0, 0.0, 0.0));
    }}
    perp = normalize(perp) * params.line_thickness;

    // Second perpendicular for camera-facing quads
    let perp2 = normalize(cross(dir, perp)) * params.line_thickness;

    // Build quad vertices (2 triangles, 6 vertices)
    var pos: vec3<f32>;
    switch vertex_index {{
        case 0u: {{ pos = world_a - perp - perp2; }}
        case 1u: {{ pos = world_a + perp + perp2; }}
        case 2u: {{ pos = world_b - perp - perp2; }}
        case 3u: {{ pos = world_a + perp + perp2; }}
        case 4u: {{ pos = world_b - perp - perp2; }}
        default: {{ pos = world_b + perp + perp2; }}
    }}

    out.clip_position = uniforms.view_proj * vec4<f32>(pos, 1.0);
    out.color = color;
    return out;
}}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {{
    return vec4<f32>(in.color, 1.0);
}}
"#,
            stride_u32 = stride_u32,
            alive_idx = alive_idx,
            scale_idx = scale_idx,
            color_code = color_code,
        )
    }

    pub(crate) fn render(&self, render_pass: &mut wgpu::RenderPass<'static>) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        // 6 vertices per line, lines_per_mesh * num_particles instances
        render_pass.draw(0..6, 0..(self.lines_per_mesh * self.num_particles));
    }

    pub(crate) fn set_line_thickness(&mut self, queue: &wgpu::Queue, thickness: f32) {
        let params: [f32; 4] = [
            thickness,
            f32::from_bits(self.lines_per_mesh),
            self.base_size,
            0.0,
        ];
        queue.write_buffer(&self.params_buffer, 0, bytemuck::cast_slice(&params));
    }
}
