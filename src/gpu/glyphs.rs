//! Vector glyph rendering for field and velocity visualization.
//!
//! Renders arrows/glyphs showing vector data in 3D space.
//! Can visualize vector fields at sample points or particle velocities.

use bytemuck::{Pod, Zeroable};
use glam::Vec3;

use crate::visuals::{GlyphColorMode, GlyphConfig, GlyphMode};

/// Vertex data for a single glyph line segment.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct GlyphVertex {
    position: [f32; 3],
    color: [f32; 3],
}

/// GPU resources for glyph rendering.
pub struct GlyphRenderer {
    /// Vertex buffer for glyph geometry.
    vertex_buffer: wgpu::Buffer,
    /// Render pipeline.
    pipeline: wgpu::RenderPipeline,
    /// Bind group for uniforms.
    bind_group: wgpu::BindGroup,
    /// Number of vertices to render.
    vertex_count: u32,
    /// Maximum number of glyphs.
    max_glyphs: u32,
    /// Current configuration.
    config: GlyphConfig,
}

impl GlyphRenderer {
    /// Create a new glyph renderer.
    pub fn new(
        device: &wgpu::Device,
        uniform_buffer: &wgpu::Buffer,
        surface_format: wgpu::TextureFormat,
        max_glyphs: u32,
    ) -> Self {
        // 6 vertices per glyph (3 line segments * 2 endpoints)
        let max_vertices = max_glyphs * 6;
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Glyph Vertex Buffer"),
            size: (max_vertices as usize * std::mem::size_of::<GlyphVertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let (pipeline, bind_group) =
            create_render_pipeline(device, uniform_buffer, surface_format);

        Self {
            vertex_buffer,
            pipeline,
            bind_group,
            vertex_count: 0,
            max_glyphs,
            config: GlyphConfig::default(),
        }
    }

    /// Update glyph configuration.
    pub fn set_config(&mut self, config: GlyphConfig) {
        self.config = config;
    }

    /// Update glyphs from vector field data.
    ///
    /// `sample_field` is called with (x, y, z) position and should return the vector at that point.
    pub fn update_from_field(
        &mut self,
        queue: &wgpu::Queue,
        bounds: f32,
        sample_field: impl Fn(Vec3) -> Vec3,
    ) {
        if matches!(self.config.mode, GlyphMode::None) {
            self.vertex_count = 0;
            return;
        }

        let res = self.config.grid_resolution;
        let mut vertices = Vec::new();

        // Sample field at grid points
        let step = (bounds * 2.0) / res as f32;
        let start = -bounds + step * 0.5;

        for ix in 0..res {
            for iy in 0..res {
                for iz in 0..res {
                    if vertices.len() / 6 >= self.max_glyphs as usize {
                        break;
                    }

                    let pos = Vec3::new(
                        start + ix as f32 * step,
                        start + iy as f32 * step,
                        start + iz as f32 * step,
                    );

                    let vec = sample_field(pos);
                    let magnitude = vec.length();

                    if magnitude < 0.0001 {
                        continue;
                    }

                    let dir = vec / magnitude;
                    let arrow_len = self.config.scale * magnitude.min(1.0);

                    // Determine color
                    let color = match self.config.color_mode {
                        GlyphColorMode::Uniform => self.config.color,
                        GlyphColorMode::ByMagnitude => {
                            // Map magnitude to color (green to red)
                            let t = (magnitude / 2.0).min(1.0);
                            Vec3::new(t, 1.0 - t, 0.0)
                        }
                        GlyphColorMode::ByDirection => {
                            // Map direction to RGB
                            Vec3::new(
                                dir.x.abs(),
                                dir.y.abs(),
                                dir.z.abs(),
                            )
                        }
                    };

                    // Generate arrow vertices
                    self.add_arrow_vertices(&mut vertices, pos, dir, arrow_len, color);
                }
            }
        }

        self.vertex_count = vertices.len() as u32;

        if !vertices.is_empty() {
            queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&vertices));
        }
    }

    /// Update glyphs from particle velocities.
    pub fn update_from_particles(
        &mut self,
        queue: &wgpu::Queue,
        positions: &[Vec3],
        velocities: &[Vec3],
        sample_rate: u32,
    ) {
        if matches!(self.config.mode, GlyphMode::None) || !matches!(self.config.mode, GlyphMode::ParticleVelocity) {
            self.vertex_count = 0;
            return;
        }

        let mut vertices = Vec::new();

        for (i, (pos, vel)) in positions.iter().zip(velocities.iter()).enumerate() {
            if i % sample_rate as usize != 0 {
                continue;
            }
            if vertices.len() / 6 >= self.max_glyphs as usize {
                break;
            }

            let magnitude = vel.length();
            if magnitude < 0.0001 {
                continue;
            }

            let dir = *vel / magnitude;
            let arrow_len = self.config.scale * magnitude.min(1.0);

            let color = match self.config.color_mode {
                GlyphColorMode::Uniform => self.config.color,
                GlyphColorMode::ByMagnitude => {
                    let t = (magnitude / 2.0).min(1.0);
                    Vec3::new(t, 1.0 - t, 0.0)
                }
                GlyphColorMode::ByDirection => {
                    Vec3::new(dir.x.abs(), dir.y.abs(), dir.z.abs())
                }
            };

            self.add_arrow_vertices(&mut vertices, *pos, dir, arrow_len, color);
        }

        self.vertex_count = vertices.len() as u32;

        if !vertices.is_empty() {
            queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&vertices));
        }
    }

    /// Add arrow vertices for a single glyph.
    fn add_arrow_vertices(
        &self,
        vertices: &mut Vec<GlyphVertex>,
        base: Vec3,
        dir: Vec3,
        length: f32,
        color: Vec3,
    ) {
        let tip = base + dir * length;
        let head_size = length * 0.25;

        // Get perpendicular vector for arrowhead
        let perp = get_perpendicular(dir);

        // Arrowhead points
        let head_back = tip - dir * head_size;
        let head_left = head_back + perp * head_size * 0.4;
        let head_right = head_back - perp * head_size * 0.4;

        let color_arr = color.to_array();

        // Shaft (line segment)
        vertices.push(GlyphVertex {
            position: base.to_array(),
            color: color_arr,
        });
        vertices.push(GlyphVertex {
            position: tip.to_array(),
            color: color_arr,
        });

        // Arrowhead left
        vertices.push(GlyphVertex {
            position: tip.to_array(),
            color: color_arr,
        });
        vertices.push(GlyphVertex {
            position: head_left.to_array(),
            color: color_arr,
        });

        // Arrowhead right
        vertices.push(GlyphVertex {
            position: tip.to_array(),
            color: color_arr,
        });
        vertices.push(GlyphVertex {
            position: head_right.to_array(),
            color: color_arr,
        });
    }

    /// Render the glyphs.
    pub fn render(&self, render_pass: &mut wgpu::RenderPass<'_>) {
        if self.vertex_count == 0 || matches!(self.config.mode, GlyphMode::None) {
            return;
        }

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.draw(0..self.vertex_count, 0..1);
    }

    /// Check if glyphs are enabled.
    pub fn is_enabled(&self) -> bool {
        !matches!(self.config.mode, GlyphMode::None)
    }

    /// Get current config.
    pub fn config(&self) -> &GlyphConfig {
        &self.config
    }
}

/// Get a vector perpendicular to the given direction.
fn get_perpendicular(dir: Vec3) -> Vec3 {
    let up = if dir.y.abs() < 0.9 {
        Vec3::Y
    } else {
        Vec3::X
    };
    dir.cross(up).normalize()
}

fn create_render_pipeline(
    device: &wgpu::Device,
    uniform_buffer: &wgpu::Buffer,
    surface_format: wgpu::TextureFormat,
) -> (wgpu::RenderPipeline, wgpu::BindGroup) {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Glyph Shader"),
        source: wgpu::ShaderSource::Wgsl(SHADER.into()),
    });

    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Glyph Bind Group Layout"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    });

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Glyph Bind Group"),
        layout: &bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: uniform_buffer.as_entire_binding(),
        }],
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Glyph Pipeline Layout"),
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });

    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Glyph Render Pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<GlyphVertex>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[
                    wgpu::VertexAttribute {
                        offset: 0,
                        shader_location: 0,
                        format: wgpu::VertexFormat::Float32x3,
                    },
                    wgpu::VertexAttribute {
                        offset: 12,
                        shader_location: 1,
                        format: wgpu::VertexFormat::Float32x3,
                    },
                ],
            }],
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
            topology: wgpu::PrimitiveTopology::LineList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            polygon_mode: wgpu::PolygonMode::Fill,
            unclipped_depth: false,
            conservative: false,
        },
        depth_stencil: None, // No depth - glyphs render on top of everything
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
        cache: None,
    });

    (pipeline, bind_group)
}

const SHADER: &str = r#"
struct Uniforms {
    view_proj: mat4x4<f32>,
    time: f32,
    delta_time: f32,
};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = uniforms.view_proj * vec4<f32>(in.position, 1.0);
    out.color = in.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, 1.0);
}
"#;
