//! Grid visualization for the 3D editor.
//!
//! This module provides a 3D grid overlay that can be rendered in the scene
//! to help users understand spatial relationships and scale. The grid consists
//! of lines parallel to each axis, forming a cubic lattice.

use bytemuck;
use wgpu;
use wgpu::util::DeviceExt;

const GRID_SHADER: &str = r#"
struct Uniforms {
    view_proj: mat4x4<f32>,
    time: f32,
    delta_time: f32,
};

struct GridParams {
    opacity: f32,
};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var<storage, read> lines: array<vec4<f32>>;
@group(0) @binding(2) var<uniform> grid_params: GridParams;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
};

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32,
) -> VertexOutput {
    var out: VertexOutput;

    let pos_a = lines[instance_index * 2u].xyz;
    let pos_b = lines[instance_index * 2u + 1u].xyz;

    let line_dir = pos_b - pos_a;
    let line_len = length(line_dir);

    if line_len < 0.0001 {
        out.clip_position = vec4<f32>(0.0, 0.0, -1000.0, 1.0);
        return out;
    }

    let dir = line_dir / line_len;

    var perp = cross(dir, vec3<f32>(0.0, 1.0, 0.0));
    if length(perp) < 0.001 {
        perp = cross(dir, vec3<f32>(1.0, 0.0, 0.0));
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
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(0.4, 0.6, 0.8, grid_params.opacity);
}
"#;

pub(crate) struct GridVisualization {
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    _line_buffer: wgpu::Buffer,
    params_buffer: wgpu::Buffer,
    line_count: u32,
    opacity: f32,
}

impl GridVisualization {
    pub(crate) fn new(
        device: &wgpu::Device,
        uniform_buffer: &wgpu::Buffer,
        cell_size: f32,
        resolution: u32,
        opacity: f32,
        target_format: wgpu::TextureFormat,
    ) -> Self {
        // Generate grid lines
        let lines = Self::generate_lines(cell_size, resolution);
        let line_count = lines.len() as u32 / 2;

        let line_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Grid Line Buffer"),
            contents: bytemuck::cast_slice(&lines),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Grid Params Buffer"),
            contents: bytemuck::bytes_of(&[opacity, 0.0_f32, 0.0, 0.0]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Grid Shader"),
            source: wgpu::ShaderSource::Wgsl(GRID_SHADER.into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Grid Bind Group Layout"),
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
            label: Some("Grid Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: line_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Grid Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Grid Pipeline"),
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
            pipeline,
            bind_group,
            _line_buffer: line_buffer,
            params_buffer,
            line_count,
            opacity,
        }
    }

    fn generate_lines(cell_size: f32, resolution: u32) -> Vec<[f32; 4]> {
        let res = resolution as i32;
        let half_extent = (res as f32 * cell_size) / 2.0;
        let mut lines = Vec::new();

        // Lines parallel to X axis
        for y in 0..=res {
            for z in 0..=res {
                let y_pos = -half_extent + y as f32 * cell_size;
                let z_pos = -half_extent + z as f32 * cell_size;
                lines.push([-half_extent, y_pos, z_pos, 1.0]);
                lines.push([half_extent, y_pos, z_pos, 1.0]);
            }
        }

        // Lines parallel to Y axis
        for x in 0..=res {
            for z in 0..=res {
                let x_pos = -half_extent + x as f32 * cell_size;
                let z_pos = -half_extent + z as f32 * cell_size;
                lines.push([x_pos, -half_extent, z_pos, 1.0]);
                lines.push([x_pos, half_extent, z_pos, 1.0]);
            }
        }

        // Lines parallel to Z axis
        for x in 0..=res {
            for y in 0..=res {
                let x_pos = -half_extent + x as f32 * cell_size;
                let y_pos = -half_extent + y as f32 * cell_size;
                lines.push([x_pos, y_pos, -half_extent, 1.0]);
                lines.push([x_pos, y_pos, half_extent, 1.0]);
            }
        }

        lines
    }

    pub(crate) fn set_opacity(&mut self, queue: &wgpu::Queue, opacity: f32) {
        self.opacity = opacity;
        queue.write_buffer(&self.params_buffer, 0, bytemuck::bytes_of(&[opacity, 0.0_f32, 0.0, 0.0]));
    }

    pub(crate) fn render(&self, render_pass: &mut wgpu::RenderPass<'static>) {
        if self.opacity > 0.0 {
            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_bind_group(0, &self.bind_group, &[]);
            render_pass.draw(0..6, 0..self.line_count);
        }
    }
}
