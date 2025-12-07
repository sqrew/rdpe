//! GPU-based particle picking for selection.
//!
//! Renders particle indices to an offscreen texture and reads back
//! the pixel under the cursor to determine which particle was clicked.

/// Picking state for GPU-based particle selection.
pub struct PickingState {
    /// Texture storing particle indices (R32Uint format).
    texture: wgpu::Texture,
    texture_view: wgpu::TextureView,
    /// Depth texture for picking pass.
    depth_texture: wgpu::TextureView,
    /// Render pipeline for picking pass.
    pipeline: wgpu::RenderPipeline,
    /// Staging buffer for reading back a single pixel (particle index).
    staging_buffer: wgpu::Buffer,
    /// Staging buffer for reading back selected particle's data.
    particle_staging_buffer: wgpu::Buffer,
    /// Size of a single particle in bytes.
    particle_stride: usize,
    /// Current texture dimensions.
    width: u32,
    height: u32,
    /// Pending pick request (pixel coordinates).
    pending_pick: Option<(u32, u32)>,
    /// Whether we need to copy particle data on next frame.
    pending_particle_copy: bool,
    /// Whether particle data was copied this frame and is ready to read.
    particle_copy_done: bool,
    /// Result of last pick (particle index, or None if no particle).
    pub selected_particle: Option<u32>,
    /// Raw bytes of the selected particle's data.
    pub selected_particle_data: Option<Vec<u8>>,
}

impl PickingState {
    /// Create a new picking state.
    pub fn new(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        particle_stride: usize,
        color_offset: Option<u32>,
        alive_offset: u32,
        scale_offset: u32,
    ) -> Self {
        let (texture, texture_view) = Self::create_picking_texture(device, width, height);
        let depth_texture = Self::create_depth_texture(device, width, height);

        let pipeline = Self::create_pipeline(device, particle_stride, color_offset, alive_offset, scale_offset);

        // Staging buffer for single pixel readback
        // Must be at least 256 bytes due to COPY_BYTES_PER_ROW_ALIGNMENT requirement
        let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Picking Staging Buffer"),
            size: 256,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        // Staging buffer for reading back selected particle's data
        // Round up to 256 bytes for alignment
        let particle_buffer_size = ((particle_stride + 255) / 256) * 256;
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
            staging_buffer,
            particle_staging_buffer,
            particle_stride,
            width,
            height,
            pending_pick: None,
            pending_particle_copy: false,
            particle_copy_done: false,
            selected_particle: None,
            selected_particle_data: None,
        }
    }

    fn create_picking_texture(
        device: &wgpu::Device,
        width: u32,
        height: u32,
    ) -> (wgpu::Texture, wgpu::TextureView) {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Picking Texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
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
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
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
        particle_stride: usize,
        color_offset: Option<u32>,
        alive_offset: u32,
        scale_offset: u32,
    ) -> wgpu::RenderPipeline {
        // Choose shader based on whether color attribute is present
        let shader_src = if color_offset.is_some() {
            PICKING_SHADER_WITH_COLOR
        } else {
            PICKING_SHADER_NO_COLOR
        };
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Picking Shader"),
            source: wgpu::ShaderSource::Wgsl(shader_src.into()),
        });

        // Create uniform bind group layout (must match the main uniform buffer's bind group layout)
        let uniform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Picking Uniform Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    // Must match main pipeline's layout visibility for bind group compatibility
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Picking Pipeline Layout"),
            bind_group_layouts: &[&uniform_bind_group_layout],
            push_constant_ranges: &[],
        });

        // Build vertex attributes matching the particle struct layout
        let vertex_attributes: Vec<wgpu::VertexAttribute> = if let Some(color_off) = color_offset {
            vec![
                // Position at location 0
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x3,
                    offset: 0,
                    shader_location: 0,
                },
                // Color at location 1
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x3,
                    offset: color_off as wgpu::BufferAddress,
                    shader_location: 1,
                },
                // Alive at location 2
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Uint32,
                    offset: alive_offset as wgpu::BufferAddress,
                    shader_location: 2,
                },
                // Scale at location 3
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32,
                    offset: scale_offset as wgpu::BufferAddress,
                    shader_location: 3,
                },
            ]
        } else {
            vec![
                // Position at location 0
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x3,
                    offset: 0,
                    shader_location: 0,
                },
                // Alive at location 2
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Uint32,
                    offset: alive_offset as wgpu::BufferAddress,
                    shader_location: 2,
                },
                // Scale at location 3
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32,
                    offset: scale_offset as wgpu::BufferAddress,
                    shader_location: 3,
                },
            ]
        };

        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Picking Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: particle_stride as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &vertex_attributes,
                }],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::R32Uint,
                    blend: None, // No blending for picking
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

    /// Resize the picking texture.
    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        if width == self.width && height == self.height {
            return;
        }
        self.width = width;
        self.height = height;
        let (texture, view) = Self::create_picking_texture(device, width, height);
        self.texture = texture;
        self.texture_view = view;
        self.depth_texture = Self::create_depth_texture(device, width, height);
    }

    /// Request a pick at the given screen coordinates.
    pub fn request_pick(&mut self, x: u32, y: u32) {
        // Clamp to texture bounds
        let x = x.min(self.width.saturating_sub(1));
        let y = y.min(self.height.saturating_sub(1));
        self.pending_pick = Some((x, y));
    }

    /// Check if there's a pending pick request.
    pub fn has_pending_pick(&self) -> bool {
        self.pending_pick.is_some()
    }

    /// Render the picking pass.
    pub fn render(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        particle_buffer: &wgpu::Buffer,
        uniform_bind_group: &wgpu::BindGroup,
        num_particles: u32,
    ) {
        // Only render if there's a pending pick
        if self.pending_pick.is_none() {
            return;
        }

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Picking Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.texture_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.0,
                        g: 0.0,
                        b: 0.0,
                        a: 0.0,
                    }),
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
        render_pass.set_bind_group(0, uniform_bind_group, &[]);
        render_pass.set_vertex_buffer(0, particle_buffer.slice(..));
        render_pass.draw(0..6, 0..num_particles);
    }

    /// Copy the picked pixel to the staging buffer.
    pub fn copy_pixel(&mut self, encoder: &mut wgpu::CommandEncoder) {
        if let Some((x, y)) = self.pending_pick {
            encoder.copy_texture_to_buffer(
                wgpu::ImageCopyTexture {
                    texture: &self.texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d { x, y, z: 0 },
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::ImageCopyBuffer {
                    buffer: &self.staging_buffer,
                    layout: wgpu::ImageDataLayout {
                        offset: 0,
                        // Must be aligned to COPY_BYTES_PER_ROW_ALIGNMENT (256)
                        bytes_per_row: Some(256),
                        rows_per_image: Some(1),
                    },
                },
                wgpu::Extent3d {
                    width: 1,
                    height: 1,
                    depth_or_array_layers: 1,
                },
            );
        }
    }

    /// Read back the picked pixel (call after submit).
    pub fn read_result(&mut self, device: &wgpu::Device) {
        if self.pending_pick.is_none() {
            return;
        }

        let buffer_slice = self.staging_buffer.slice(..);
        buffer_slice.map_async(wgpu::MapMode::Read, |_| {});
        device.poll(wgpu::Maintain::Wait);

        {
            let data = buffer_slice.get_mapped_range();
            let value = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
            // Index 0 means no particle (we store index + 1)
            let new_selection = if value > 0 { Some(value - 1) } else { None };

            // If selection changed and we have a valid selection, request particle data copy
            if new_selection != self.selected_particle {
                self.selected_particle = new_selection;
                if new_selection.is_some() {
                    // Request copy on NEXT frame
                    self.pending_particle_copy = true;
                    self.selected_particle_data = None;
                }
            }
        }

        self.staging_buffer.unmap();
        self.pending_pick = None;
    }

    /// Check if we need to copy particle data (every frame if selected).
    pub fn needs_particle_data_copy(&self) -> bool {
        self.selected_particle.is_some()
    }

    /// Copy the selected particle's data to staging buffer.
    pub fn copy_particle_data(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        particle_buffer: &wgpu::Buffer,
    ) {
        if let Some(idx) = self.selected_particle {
            let offset = idx as u64 * self.particle_stride as u64;
            encoder.copy_buffer_to_buffer(
                particle_buffer,
                offset,
                &self.particle_staging_buffer,
                0,
                self.particle_stride as u64,
            );
            self.particle_copy_done = true;
        }
    }

    /// Read back the particle data (call after submit).
    pub fn read_particle_data(&mut self, device: &wgpu::Device) {
        // Only read if we actually copied data this frame
        if !self.particle_copy_done {
            return;
        }

        if self.selected_particle.is_none() {
            self.particle_copy_done = false;
            return;
        }

        let buffer_slice = self.particle_staging_buffer.slice(..self.particle_stride as u64);
        buffer_slice.map_async(wgpu::MapMode::Read, |_| {});
        device.poll(wgpu::Maintain::Wait);

        {
            let data = buffer_slice.get_mapped_range();
            self.selected_particle_data = Some(data.to_vec());
        }

        self.particle_staging_buffer.unmap();
        self.particle_copy_done = false;
    }

    /// Clear the selection.
    pub fn clear_selection(&mut self) {
        self.selected_particle = None;
        self.selected_particle_data = None;
        self.pending_particle_copy = false;
        self.particle_copy_done = false;
    }
}

/// Picking shader - outputs particle index + 1 (0 = no particle).
/// We don't need color for picking, only position, alive, and scale.
const PICKING_SHADER_NO_COLOR: &str = r#"
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
    @location(2) alive: u32,
    @location(3) scale: f32,
) -> VertexOutput {
    var out: VertexOutput;

    // Cull dead particles
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

    // Billboard transform (same as main render)
    let view_proj = uniforms.view_proj;
    let right = vec3<f32>(view_proj[0][0], view_proj[1][0], view_proj[2][0]);
    let up = vec3<f32>(view_proj[0][1], view_proj[1][1], view_proj[2][1]);

    // Use a slightly larger size for easier picking
    let particle_size = 0.03 * scale * 1.5;
    let world_pos = particle_pos + right * quad_pos.x * particle_size + up * quad_pos.y * particle_size;

    out.clip_position = view_proj * vec4<f32>(world_pos, 1.0);
    out.particle_index = instance_index + 1u; // +1 so 0 means "no particle"

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) u32 {
    return in.particle_index;
}
"#;

/// Picking shader with color attribute (needed when particle has color field).
const PICKING_SHADER_WITH_COLOR: &str = r#"
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

    // Cull dead particles
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

    // Billboard transform (same as main render)
    let view_proj = uniforms.view_proj;
    let right = vec3<f32>(view_proj[0][0], view_proj[1][0], view_proj[2][0]);
    let up = vec3<f32>(view_proj[0][1], view_proj[1][1], view_proj[2][1]);

    // Use a slightly larger size for easier picking
    let particle_size = 0.03 * scale * 1.5;
    let world_pos = particle_pos + right * quad_pos.x * particle_size + up * quad_pos.y * particle_size;

    out.clip_position = view_proj * vec4<f32>(world_pos, 1.0);
    out.particle_index = instance_index + 1u; // +1 so 0 means "no particle"

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) u32 {
    return in.particle_index;
}
"#;
