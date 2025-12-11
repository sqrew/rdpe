//! GPU-based particle picking for selection.

use crate::config::ParticleLayout;

/// Picking shader - outputs particle index + 1 (0 = no particle).
const PICKING_SHADER: &str = r#"
struct Uniforms {
    view_proj: mat4x4<f32>,
    time: f32,
    delta_time: f32,
};

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) @interpolate(flat) particle_index: u32,
};

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32,
    @location(0) particle_pos: vec3<f32>,
    @location(1) particle_color: vec3<f32>,
    @location(2) alive: u32,
    @location(3) scale: f32,
) -> VertexOutput {
    var out: VertexOutput;

    if alive == 0u {
        out.clip_position = vec4<f32>(0.0, 0.0, -1000.0, 1.0);
        out.particle_index = 0u;
        return out;
    }

    var quad_vertices = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>( 1.0,  1.0),
    );

    let quad_pos = quad_vertices[vertex_index];
    let view_proj = uniforms.view_proj;
    let right = vec3<f32>(view_proj[0][0], view_proj[1][0], view_proj[2][0]);
    let up = vec3<f32>(view_proj[0][1], view_proj[1][1], view_proj[2][1]);

    // Slightly larger for easier picking
    let particle_size = 0.03 * scale * 1.5;
    let world_pos = particle_pos + right * quad_pos.x * particle_size + up * quad_pos.y * particle_size;

    out.clip_position = view_proj * vec4<f32>(world_pos, 1.0);
    out.particle_index = instance_index + 1u;

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) u32 {
    return in.particle_index;
}
"#;

/// Shared state for picking requests between UI and render callback.
#[derive(Default)]
pub struct PickingRequest {
    /// Pending pick coordinates (viewport-relative pixels)
    pub pending: Option<(u32, u32)>,
    /// Current viewport dimensions
    pub viewport_size: (u32, u32),
}

/// State for GPU-based particle picking.
pub struct PickingState {
    texture: wgpu::Texture,
    texture_view: wgpu::TextureView,
    depth_texture: wgpu::TextureView,
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    staging_buffer: wgpu::Buffer,
    particle_staging_buffer: wgpu::Buffer,
    width: u32,
    height: u32,
    particle_stride: usize,
    pending_pick: Option<(u32, u32)>,
    /// Selected particle index (None = no selection)
    pub selected_particle: Option<u32>,
    /// Raw bytes of selected particle data
    pub selected_particle_data: Option<Vec<u8>>,
}

impl PickingState {
    pub fn new(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        layout: &ParticleLayout,
        uniform_buffer: &wgpu::Buffer,
    ) -> Self {
        let (texture, texture_view) = Self::create_texture(device, width, height);
        let depth_texture = Self::create_depth_texture(device, width, height);

        // Create bind group layout matching our uniform buffer
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Picking Bind Group Layout"),
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
            label: Some("Picking Bind Group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let particle_stride = layout.stride;
        let pipeline = Self::create_pipeline(device, &bind_group_layout, layout);

        // Staging buffers for readback
        let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Picking Staging Buffer"),
            size: 256, // Minimum for alignment
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let particle_buffer_size = particle_stride.div_ceil(256) * 256;
        let particle_staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Particle Data Staging Buffer"),
            size: particle_buffer_size as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        Self {
            texture,
            texture_view,
            depth_texture,
            pipeline,
            bind_group,
            staging_buffer,
            particle_staging_buffer,
            width,
            height,
            particle_stride,
            pending_pick: None,
            selected_particle: None,
            selected_particle_data: None,
        }
    }

    fn create_texture(device: &wgpu::Device, width: u32, height: u32) -> (wgpu::Texture, wgpu::TextureView) {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Picking Texture"),
            size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R32Uint,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        (texture, view)
    }

    fn create_depth_texture(device: &wgpu::Device, width: u32, height: u32) -> wgpu::TextureView {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Picking Depth Texture"),
            size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        texture.create_view(&wgpu::TextureViewDescriptor::default())
    }

    fn create_pipeline(
        device: &wgpu::Device,
        bind_group_layout: &wgpu::BindGroupLayout,
        layout: &ParticleLayout,
    ) -> wgpu::RenderPipeline {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Picking Shader"),
            source: wgpu::ShaderSource::Wgsl(PICKING_SHADER.into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Picking Pipeline Layout"),
            bind_group_layouts: &[bind_group_layout],
            push_constant_ranges: &[],
        });

        let particle_stride = layout.stride;
        let color_offset = layout.color_offset;
        let alive_offset = layout.alive_offset;
        let scale_offset = layout.scale_offset;

        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Picking Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: particle_stride as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &[
                        wgpu::VertexAttribute {
                            offset: 0,
                            shader_location: 0,
                            format: wgpu::VertexFormat::Float32x3,
                        },
                        wgpu::VertexAttribute {
                            offset: color_offset as wgpu::BufferAddress,
                            shader_location: 1,
                            format: wgpu::VertexFormat::Float32x3,
                        },
                        wgpu::VertexAttribute {
                            offset: alive_offset as wgpu::BufferAddress,
                            shader_location: 2,
                            format: wgpu::VertexFormat::Uint32,
                        },
                        wgpu::VertexAttribute {
                            offset: scale_offset as wgpu::BufferAddress,
                            shader_location: 3,
                            format: wgpu::VertexFormat::Float32,
                        },
                    ],
                }],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::R32Uint,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        })
    }

    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        if width == self.width && height == self.height {
            return;
        }
        self.width = width;
        self.height = height;
        let (texture, view) = Self::create_texture(device, width, height);
        self.texture = texture;
        self.texture_view = view;
        self.depth_texture = Self::create_depth_texture(device, width, height);
    }

    pub fn request_pick(&mut self, x: u32, y: u32) {
        let x = x.min(self.width.saturating_sub(1));
        let y = y.min(self.height.saturating_sub(1));
        self.pending_pick = Some((x, y));
    }

    pub fn render_and_pick(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        particle_buffer: &wgpu::Buffer,
        num_particles: u32,
    ) {
        if self.pending_pick.is_none() {
            // Still update selected particle data if we have a selection
            if self.selected_particle.is_some() {
                self.copy_and_read_particle_data(device, queue, particle_buffer);
            }
            return;
        }

        let (pick_x, pick_y) = self.pending_pick.take().unwrap();

        // Create command encoder for picking
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Picking Encoder"),
        });

        // Render picking pass
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Picking Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.texture_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_bind_group(0, &self.bind_group, &[]);
            render_pass.set_vertex_buffer(0, particle_buffer.slice(..));
            render_pass.draw(0..6, 0..num_particles);
        }

        // Copy picked pixel to staging buffer
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d { x: pick_x, y: pick_y, z: 0 },
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &self.staging_buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(256),
                    rows_per_image: Some(1),
                },
            },
            wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
        );

        queue.submit(std::iter::once(encoder.finish()));

        // Read back the picked pixel
        let buffer_slice = self.staging_buffer.slice(..);
        buffer_slice.map_async(wgpu::MapMode::Read, |_| {});
        device.poll(wgpu::Maintain::Wait);

        {
            let data = buffer_slice.get_mapped_range();
            let value = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
            self.selected_particle = if value > 0 { Some(value - 1) } else { None };
        }
        self.staging_buffer.unmap();

        // If we have a selection, copy particle data
        if self.selected_particle.is_some() {
            self.copy_and_read_particle_data(device, queue, particle_buffer);
        } else {
            self.selected_particle_data = None;
        }
    }

    fn copy_and_read_particle_data(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        particle_buffer: &wgpu::Buffer,
    ) {
        let idx = match self.selected_particle {
            Some(idx) => idx,
            None => return,
        };

        let offset = idx as u64 * self.particle_stride as u64;

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Particle Copy Encoder"),
        });

        encoder.copy_buffer_to_buffer(
            particle_buffer,
            offset,
            &self.particle_staging_buffer,
            0,
            self.particle_stride as u64,
        );

        queue.submit(std::iter::once(encoder.finish()));

        let buffer_slice = self.particle_staging_buffer.slice(..self.particle_stride as u64);
        buffer_slice.map_async(wgpu::MapMode::Read, |_| {});
        device.poll(wgpu::Maintain::Wait);

        {
            let data = buffer_slice.get_mapped_range();
            self.selected_particle_data = Some(data.to_vec());
        }
        self.particle_staging_buffer.unmap();
    }

    pub fn clear_selection(&mut self) {
        self.selected_particle = None;
        self.selected_particle_data = None;
    }
}
