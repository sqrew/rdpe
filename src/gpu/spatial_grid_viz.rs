//! Spatial hash grid visualization.
//!
//! Renders a wireframe overlay showing the spatial hash grid cells.
//! Useful for debugging spatial configuration and understanding neighbor queries.

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

use super::DEPTH_FORMAT;
use crate::spatial::SpatialConfig;

/// GPU parameters for grid rendering.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct GridParams {
    /// Grid line opacity.
    opacity: f32,
    /// Padding for alignment.
    _pad: [f32; 3],
}

/// GPU state for spatial grid visualization.
#[allow(dead_code)]
pub struct SpatialGridViz {
    /// Buffer storing line segment endpoints.
    line_buffer: wgpu::Buffer,
    /// Number of line segments.
    line_count: u32,
    /// Render pipeline.
    pipeline: wgpu::RenderPipeline,
    /// Bind group for rendering.
    bind_group: wgpu::BindGroup,
    /// Bind group layout (for reference).
    bind_group_layout: wgpu::BindGroupLayout,
    /// Grid params buffer.
    params_buffer: wgpu::Buffer,
    /// Current opacity.
    pub opacity: f32,
}

impl SpatialGridViz {
    /// Create a new spatial grid visualization.
    pub fn new(
        device: &wgpu::Device,
        uniform_buffer: &wgpu::Buffer,
        spatial_config: &SpatialConfig,
        opacity: f32,
        surface_format: wgpu::TextureFormat,
    ) -> Self {
        // Generate grid line segments
        let lines = generate_grid_lines(spatial_config);
        let line_count = lines.len() as u32 / 2; // 2 vec4s per line

        let line_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Spatial Grid Line Buffer"),
            contents: bytemuck::cast_slice(&lines),
            usage: wgpu::BufferUsages::STORAGE,
        });

        // Grid params
        let params = GridParams {
            opacity,
            _pad: [0.0; 3],
        };
        let params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Spatial Grid Params Buffer"),
            contents: bytemuck::bytes_of(&params),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Create shader
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Spatial Grid Shader"),
            source: wgpu::ShaderSource::Wgsl(GRID_SHADER.into()),
        });

        // Bind group layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Spatial Grid Bind Group Layout"),
            entries: &[
                // Uniforms (view_proj)
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
                // Line segments
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
                // Grid params
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
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
            label: Some("Spatial Grid Bind Group"),
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
            label: Some("Spatial Grid Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Spatial Grid Pipeline"),
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
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
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

        Self {
            line_buffer,
            line_count,
            pipeline,
            bind_group,
            bind_group_layout,
            params_buffer,
            opacity,
        }
    }

    /// Update the grid opacity.
    pub fn set_opacity(&mut self, queue: &wgpu::Queue, opacity: f32) {
        self.opacity = opacity;
        let params = GridParams {
            opacity,
            _pad: [0.0; 3],
        };
        queue.write_buffer(&self.params_buffer, 0, bytemuck::bytes_of(&params));
    }

    /// Get the render pipeline.
    pub fn pipeline(&self) -> &wgpu::RenderPipeline {
        &self.pipeline
    }

    /// Get the bind group.
    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }

    /// Get the number of line instances to draw.
    pub fn line_count(&self) -> u32 {
        self.line_count
    }
}

/// Generate grid line segments for the given spatial config.
fn generate_grid_lines(config: &SpatialConfig) -> Vec<[f32; 4]> {
    let res = config.grid_resolution as i32;
    let cell_size = config.cell_size;
    let half_extent = (res as f32 * cell_size) / 2.0;

    let mut lines = Vec::new();

    // Lines parallel to X axis (vary Y and Z)
    for y in 0..=res {
        for z in 0..=res {
            let y_pos = -half_extent + y as f32 * cell_size;
            let z_pos = -half_extent + z as f32 * cell_size;
            // Start point
            lines.push([-half_extent, y_pos, z_pos, 1.0]);
            // End point
            lines.push([half_extent, y_pos, z_pos, 1.0]);
        }
    }

    // Lines parallel to Y axis (vary X and Z)
    for x in 0..=res {
        for z in 0..=res {
            let x_pos = -half_extent + x as f32 * cell_size;
            let z_pos = -half_extent + z as f32 * cell_size;
            // Start point
            lines.push([x_pos, -half_extent, z_pos, 1.0]);
            // End point
            lines.push([x_pos, half_extent, z_pos, 1.0]);
        }
    }

    // Lines parallel to Z axis (vary X and Y)
    for x in 0..=res {
        for y in 0..=res {
            let x_pos = -half_extent + x as f32 * cell_size;
            let y_pos = -half_extent + y as f32 * cell_size;
            // Start point
            lines.push([x_pos, y_pos, -half_extent, 1.0]);
            // End point
            lines.push([x_pos, y_pos, half_extent, 1.0]);
        }
    }

    lines
}

/// Grid line rendering shader.
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

    // Get line endpoints
    let pos_a = lines[instance_index * 2u].xyz;
    let pos_b = lines[instance_index * 2u + 1u].xyz;

    // Create thin quad along the line
    let line_dir = pos_b - pos_a;
    let line_len = length(line_dir);

    if line_len < 0.0001 {
        out.clip_position = vec4<f32>(0.0, 0.0, -1000.0, 1.0);
        return out;
    }

    let dir = line_dir / line_len;

    // Find perpendicular direction for line width
    var perp = cross(dir, vec3<f32>(0.0, 1.0, 0.0));
    if length(perp) < 0.001 {
        perp = cross(dir, vec3<f32>(1.0, 0.0, 0.0));
    }
    perp = normalize(perp) * 0.001; // Very thin lines

    // Build quad vertices (2 triangles, 6 vertices)
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
    // Subtle grid color
    return vec4<f32>(0.4, 0.6, 0.8, grid_params.opacity);
}
"#;
