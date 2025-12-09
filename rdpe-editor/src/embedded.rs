//! Embedded simulation for the rdpe editor.
//!
//! This module provides a way to run the particle simulation directly inside
//! the eframe window using egui_wgpu's custom painting system.
//!
//! The architecture follows egui_wgpu's callback pattern:
//! - `SimulationResources` holds persistent GPU resources (stored in CallbackResources)
//! - `SimulationCallback` is a lightweight struct passed to each paint call
//! - `prepare()` runs compute passes and updates uniforms
//! - `paint()` issues draw commands

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};
use std::collections::HashMap;
use wgpu::util::DeviceExt;
use crate::config::{SimConfig, UniformValueConfig, ParticleLayout};
use crate::shader_gen;
use crate::shader_validate;
use crate::spawn;
use rdpe::{FieldSystemGpu, VolumeRenderState, create_particle_field_bind_group_layout, SpatialGpu, SpatialConfig};
use crate::config::VolumeRenderConfig;

const WORKGROUP_SIZE: u32 = 256;

/// Base uniforms passed to shaders (fixed layout).
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct BaseUniforms {
    view_proj: [[f32; 4]; 4],
    time: f32,
    delta_time: f32,
    _padding: [f32; 2],
}

const BASE_UNIFORMS_SIZE: usize = std::mem::size_of::<BaseUniforms>();

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
    fn new(
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

    fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
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

    fn request_pick(&mut self, x: u32, y: u32) {
        let x = x.min(self.width.saturating_sub(1));
        let y = y.min(self.height.saturating_sub(1));
        self.pending_pick = Some((x, y));
    }

    fn render_and_pick(
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
            wgpu::ImageCopyTexture {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d { x: pick_x, y: pick_y, z: 0 },
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::ImageCopyBuffer {
                buffer: &self.staging_buffer,
                layout: wgpu::ImageDataLayout {
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

    fn clear_selection(&mut self) {
        self.selected_particle = None;
        self.selected_particle_data = None;
    }
}

/// Build uniform buffer data with base uniforms and custom values.
fn build_uniform_data(
    view_proj: Mat4,
    time: f32,
    delta_time: f32,
    custom_uniforms: &[(String, UniformValueConfig)],
) -> Vec<u8> {
    let mut data = Vec::with_capacity(BASE_UNIFORMS_SIZE + 256); // Reserve extra for custom

    // Write base uniforms
    let base = BaseUniforms {
        view_proj: view_proj.to_cols_array_2d(),
        time,
        delta_time,
        _padding: [0.0; 2],
    };
    data.extend_from_slice(bytemuck::bytes_of(&base));

    // Write custom uniforms with proper std140 alignment
    for (_name, value) in custom_uniforms {
        // Align to value's alignment requirement
        let alignment = value.alignment();
        let current_offset = data.len();
        let aligned_offset = (current_offset + alignment - 1) / alignment * alignment;
        data.resize(aligned_offset, 0u8); // Pad to alignment

        // Write value bytes
        data.extend_from_slice(&value.to_bytes());
    }

    // Ensure minimum buffer size and 16-byte alignment for the total buffer
    let final_size = ((data.len() + 15) / 16) * 16;
    data.resize(final_size, 0u8);

    data
}

/// Persistent GPU resources for the simulation.
///
/// This is stored in egui_wgpu's CallbackResources and persists across frames.
pub struct SimulationResources {
    // Pipelines
    compute_pipeline: wgpu::ComputePipeline,
    render_pipeline: wgpu::RenderPipeline,

    // Buffers
    particle_buffer: wgpu::Buffer,
    uniform_buffer: wgpu::Buffer,
    uniform_buffer_size: usize,

    // Bind groups
    compute_bind_group: wgpu::BindGroup,
    render_bind_group: wgpu::BindGroup,

    // Configuration
    pub num_particles: u32,
    pub particle_stride: usize,
    background_color: Vec3,

    // Custom uniforms (sorted by name for deterministic order)
    custom_uniforms: Vec<(String, UniformValueConfig)>,

    // State
    time: f32,
    paused: bool,

    // Camera (simple orbit camera)
    camera_distance: f32,
    camera_yaw: f32,
    camera_pitch: f32,

    // Cached camera info for volume rendering
    last_inv_view_proj: Mat4,
    last_camera_pos: Vec3,

    // Particle picking
    picking: PickingState,

    // Field system (optional)
    field_system: Option<FieldSystemGpu>,
    empty_bind_group: Option<wgpu::BindGroup>,
    field_bind_group: Option<wgpu::BindGroup>,

    // Volume rendering (optional)
    volume_render_state: Option<VolumeRenderState>,
    volume_config: Option<VolumeRenderConfig>,

    // Spatial hashing (optional, for neighbor queries)
    spatial: Option<SpatialGpu>,
}

impl SimulationResources {
    /// Create new simulation resources.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        device: &wgpu::Device,
        _queue: &wgpu::Queue,
        target_format: wgpu::TextureFormat,
        particle_data: &[u8],
        num_particles: u32,
        layout: &ParticleLayout,
        compute_shader_src: &str,
        render_shader_src: &str,
        background_color: Vec3,
        custom_uniforms_map: &HashMap<String, UniformValueConfig>,
        field_registry: &rdpe::FieldRegistry,
        volume_config: &VolumeRenderConfig,
        needs_spatial: bool,
        spatial_cell_size: f32,
        spatial_resolution: u32,
        particle_wgsl_struct: &str,
    ) -> Self {
        let particle_stride = layout.stride;
        // Create particle buffer
        let particle_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Particle Buffer"),
            contents: particle_data,
            usage: wgpu::BufferUsages::VERTEX
                | wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
        });

        // Create spatial hashing system if needed
        let spatial = if needs_spatial {
            let spatial_config = SpatialConfig {
                cell_size: spatial_cell_size,
                grid_resolution: spatial_resolution,
                max_neighbors: 0, // unlimited
            };
            Some(SpatialGpu::new(
                device,
                &particle_buffer,
                num_particles,
                spatial_config,
                particle_wgsl_struct,
            ))
        } else {
            None
        };

        // Sort custom uniforms by name for deterministic order (must match shader generation)
        let mut custom_uniforms: Vec<_> = custom_uniforms_map.iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        custom_uniforms.sort_by(|a, b| a.0.cmp(&b.0));

        // Create uniform buffer with initial data
        let uniform_data = build_uniform_data(
            Mat4::IDENTITY,
            0.0,
            0.016,
            &custom_uniforms,
        );
        let uniform_buffer_size = uniform_data.len();
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniform Buffer"),
            contents: &uniform_data,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Create field system if fields are defined
        let (field_system, field_bind_group_layout) = if !field_registry.is_empty() {
            let fs = FieldSystemGpu::new(device, field_registry);
            let layout = create_particle_field_bind_group_layout(device, field_registry.len());
            (Some(fs), Some(layout))
        } else {
            (None, None)
        };

        // Create compute bind group layout (with optional spatial bindings)
        let mut compute_layout_entries = vec![
            // Particles (storage, read-write)
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            // Uniforms
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ];

        // Add spatial bindings if needed (bindings 2-5)
        if spatial.is_some() {
            compute_layout_entries.extend([
                // Sorted particle indices
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Cell start
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Cell end
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
                // Spatial params
                wgpu::BindGroupLayoutEntry {
                    binding: 5,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ]);
        }

        let compute_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Compute Bind Group Layout"),
            entries: &compute_layout_entries,
        });

        // Create compute bind group entries
        let mut compute_bind_entries = vec![
            wgpu::BindGroupEntry {
                binding: 0,
                resource: particle_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: uniform_buffer.as_entire_binding(),
            },
        ];

        // Add spatial bind entries if needed
        if let Some(ref sp) = spatial {
            compute_bind_entries.extend([
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: sp.particle_indices_a.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: sp.cell_start.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: sp.cell_end.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: sp.spatial_params_buffer.as_entire_binding(),
                },
            ]);
        }

        let compute_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Compute Bind Group"),
            layout: &compute_bind_group_layout,
            entries: &compute_bind_entries,
        });

        // Create field bind group if fields exist
        let field_bind_group: Option<wgpu::BindGroup> = if let (Some(ref fs), Some(ref layout)) = (&field_system, &field_bind_group_layout) {
            fs.create_particle_bind_group(device, layout)
        } else {
            None
        };

        // Create compute pipeline
        let compute_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Compute Shader"),
            source: wgpu::ShaderSource::Wgsl(compute_shader_src.into()),
        });

        // Create empty bind group layout for group 1 placeholder (fields are at group 2)
        let empty_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Empty Bind Group Layout"),
            entries: &[],
        });

        // Build compute pipeline layout with optional field bind group
        // Group 0: compute, Group 1: empty placeholder, Group 2: fields
        let bind_group_layouts: Vec<&wgpu::BindGroupLayout> = if let Some(ref layout) = field_bind_group_layout {
            vec![&compute_bind_group_layout, &empty_bind_group_layout, layout]
        } else {
            vec![&compute_bind_group_layout]
        };

        let compute_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Compute Pipeline Layout"),
            bind_group_layouts: &bind_group_layouts,
            push_constant_ranges: &[],
        });

        // Create empty bind group for group 1
        let empty_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Empty Bind Group"),
            layout: &empty_bind_group_layout,
            entries: &[],
        });

        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Compute Pipeline"),
            layout: Some(&compute_pipeline_layout),
            module: &compute_shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });

        // Create render bind group layout
        let render_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Render Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
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

        let render_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Render Bind Group"),
            layout: &render_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
            ],
        });

        // Create render pipeline
        let render_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Render Shader"),
            source: wgpu::ShaderSource::Wgsl(render_shader_src.into()),
        });

        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[&render_bind_group_layout],
            push_constant_ranges: &[],
        });

        // Get particle offsets from layout
        let velocity_offset = layout.velocity_offset;
        let color_offset = layout.color_offset;
        let age_offset = layout.age_offset;
        let alive_offset = layout.alive_offset;
        let scale_offset = layout.scale_offset;

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &render_shader,
                entry_point: Some("vs_main"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: particle_stride as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &[
                        // Position at offset 0
                        wgpu::VertexAttribute {
                            offset: 0,
                            shader_location: 0,
                            format: wgpu::VertexFormat::Float32x3,
                        },
                        // Velocity
                        wgpu::VertexAttribute {
                            offset: velocity_offset as wgpu::BufferAddress,
                            shader_location: 1,
                            format: wgpu::VertexFormat::Float32x3,
                        },
                        // Color
                        wgpu::VertexAttribute {
                            offset: color_offset as wgpu::BufferAddress,
                            shader_location: 2,
                            format: wgpu::VertexFormat::Float32x3,
                        },
                        // Age
                        wgpu::VertexAttribute {
                            offset: age_offset as wgpu::BufferAddress,
                            shader_location: 3,
                            format: wgpu::VertexFormat::Float32,
                        },
                        // Alive flag
                        wgpu::VertexAttribute {
                            offset: alive_offset as wgpu::BufferAddress,
                            shader_location: 4,
                            format: wgpu::VertexFormat::Uint32,
                        },
                        // Scale
                        wgpu::VertexAttribute {
                            offset: scale_offset as wgpu::BufferAddress,
                            shader_location: 5,
                            format: wgpu::VertexFormat::Float32,
                        },
                    ],
                }],
                compilation_options: Default::default(),
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
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
            multiview: None,
            cache: None,
        });

        // Create picking state with a default size (will be resized on first frame)
        let picking = PickingState::new(
            device,
            800,  // Default width, will resize
            600,  // Default height, will resize
            layout,
            &uniform_buffer,
        );

        // Create volume render state if enabled and field system exists
        let (volume_render_state, stored_volume_config) = if volume_config.enabled {
            if let Some(ref fs) = field_system {
                let rdpe_config = volume_config.to_volume_config();
                let state = VolumeRenderState::new(device, fs, &rdpe_config, target_format);
                (Some(state), Some(volume_config.clone()))
            } else {
                (None, None)
            }
        } else {
            (None, None)
        };

        Self {
            compute_pipeline,
            render_pipeline,
            particle_buffer,
            uniform_buffer,
            uniform_buffer_size,
            compute_bind_group,
            render_bind_group,
            num_particles,
            particle_stride,
            background_color,
            custom_uniforms,
            time: 0.0,
            paused: false,
            camera_distance: 3.0,
            camera_yaw: 0.0,
            camera_pitch: 0.3,
            last_inv_view_proj: Mat4::IDENTITY,
            last_camera_pos: Vec3::new(0.0, 0.0, 3.0),
            picking,
            field_system,
            empty_bind_group: if field_bind_group.is_some() { Some(empty_bind_group) } else { None },
            field_bind_group,
            volume_render_state,
            volume_config: stored_volume_config,
            spatial,
        }
    }

    /// Update uniforms and optionally run compute.
    pub fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        delta_time: f32,
        aspect_ratio: f32,
    ) -> Vec<wgpu::CommandBuffer> {
        // Update time
        if !self.paused {
            self.time += delta_time;
        }

        // Calculate view-projection matrix
        let eye = Vec3::new(
            self.camera_distance * self.camera_yaw.cos() * self.camera_pitch.cos(),
            self.camera_distance * self.camera_pitch.sin(),
            self.camera_distance * self.camera_yaw.sin() * self.camera_pitch.cos(),
        );
        let view = Mat4::look_at_rh(eye, Vec3::ZERO, Vec3::Y);
        let proj = Mat4::perspective_rh(45.0_f32.to_radians(), aspect_ratio, 0.1, 100.0);
        let view_proj = proj * view;

        // Cache camera info for volume rendering
        self.last_inv_view_proj = view_proj.inverse();
        self.last_camera_pos = eye;

        // Build uniform data including custom uniforms
        let uniform_data = build_uniform_data(
            view_proj,
            self.time,
            delta_time,
            &self.custom_uniforms,
        );
        queue.write_buffer(&self.uniform_buffer, 0, &uniform_data);

        // Run compute pass if not paused
        let result = if !self.paused {
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Compute Encoder"),
            });

            // Run spatial hashing passes (if enabled) before particle compute
            if let Some(ref spatial) = self.spatial {
                spatial.execute(&mut encoder, queue);
            }

            // Run particle compute pass
            {
                let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("Particle Compute"),
                    timestamp_writes: None,
                });
                compute_pass.set_pipeline(&self.compute_pipeline);
                compute_pass.set_bind_group(0, &self.compute_bind_group, &[]);

                // Bind empty at group 1 and field bind group at group 2
                if let (Some(ref empty_bg), Some(ref field_bg)) = (&self.empty_bind_group, &self.field_bind_group) {
                    compute_pass.set_bind_group(1, empty_bg, &[]);
                    compute_pass.set_bind_group(2, field_bg, &[]);
                }

                let workgroups = self.num_particles.div_ceil(WORKGROUP_SIZE);
                compute_pass.dispatch_workgroups(workgroups, 1, 1);
            }

            // Run field processing passes (merge, blur, decay, clear)
            if let Some(ref mut field_system) = self.field_system {
                field_system.process(device, &mut encoder, queue);

                // Update volume render bind group after field buffer swap
                if let Some(ref mut volume_state) = self.volume_render_state {
                    volume_state.update_bind_group(device, field_system);
                }
            }

            vec![encoder.finish()]
        } else {
            vec![]
        };

        // Update volume render params (always, even when paused, for camera movement)
        if let (Some(ref volume_state), Some(ref field_system)) = (&self.volume_render_state, &self.field_system) {
            if volume_state.field_index < field_system.fields.len() {
                let field = &field_system.fields[volume_state.field_index];
                volume_state.update_params_with_field(
                    queue,
                    self.last_inv_view_proj,
                    self.last_camera_pos,
                    field.config.world_extent,
                    field.config.resolution,
                );
            }
        }

        result
    }

    /// Issue draw commands.
    pub fn paint(&self, render_pass: &mut wgpu::RenderPass<'static>) {
        // Render volume first (behind particles) - uses additive blending
        if let Some(ref volume_state) = self.volume_render_state {
            render_pass.set_pipeline(&volume_state.pipeline);
            render_pass.set_bind_group(0, &volume_state.bind_group, &[]);
            render_pass.draw(0..3, 0..1); // Fullscreen triangle
        }

        // Render particles
        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, &self.render_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.particle_buffer.slice(..));
        // Draw 4 vertices (quad) per particle instance
        render_pass.draw(0..4, 0..self.num_particles);
    }

    /// Check if volume rendering is enabled.
    pub fn has_volume_render(&self) -> bool {
        self.volume_render_state.is_some()
    }

    /// Set pause state.
    pub fn set_paused(&mut self, paused: bool) {
        self.paused = paused;
    }

    /// Is the simulation paused?
    pub fn is_paused(&self) -> bool {
        self.paused
    }

    /// Set background color.
    pub fn set_background_color(&mut self, color: Vec3) {
        self.background_color = color;
    }

    /// Get background color.
    pub fn background_color(&self) -> Vec3 {
        self.background_color
    }

    /// Rotate camera.
    pub fn rotate_camera(&mut self, delta_yaw: f32, delta_pitch: f32) {
        self.camera_yaw += delta_yaw;
        self.camera_pitch = (self.camera_pitch + delta_pitch).clamp(-1.4, 1.4);
    }

    /// Zoom camera.
    pub fn zoom_camera(&mut self, delta: f32) {
        self.camera_distance = (self.camera_distance - delta).clamp(1.0, 20.0);
    }

    /// Request picking at viewport coordinates.
    pub fn request_pick(&mut self, x: u32, y: u32) {
        self.picking.request_pick(x, y);
    }

    /// Get the currently selected particle index.
    pub fn selected_particle(&self) -> Option<u32> {
        self.picking.selected_particle
    }

    /// Get the raw data of the selected particle.
    pub fn selected_particle_data(&self) -> Option<&[u8]> {
        self.picking.selected_particle_data.as_deref()
    }

    /// Clear particle selection.
    pub fn clear_selection(&mut self) {
        self.picking.clear_selection();
    }

    /// Resize picking texture to match viewport.
    pub fn resize_picking(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        self.picking.resize(device, width, height);
    }

    /// Run picking pass and update selected particle data.
    pub fn update_picking(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        self.picking.render_and_pick(device, queue, &self.particle_buffer, self.num_particles);
    }

    /// Read particle data from GPU.
    pub fn read_particles(&self, device: &wgpu::Device, queue: &wgpu::Queue) -> Vec<u8> {
        let buffer_size = (self.num_particles as usize) * self.particle_stride;

        let staging = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Readback Staging"),
            size: buffer_size as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Readback Encoder"),
        });
        encoder.copy_buffer_to_buffer(&self.particle_buffer, 0, &staging, 0, buffer_size as u64);
        queue.submit(std::iter::once(encoder.finish()));

        let buffer_slice = staging.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = tx.send(result);
        });

        device.poll(wgpu::Maintain::Wait);
        rx.recv().unwrap().expect("Failed to map buffer");

        let data = buffer_slice.get_mapped_range();
        let result = data.to_vec();
        drop(data);
        staging.unmap();

        result
    }

    /// Write particle data to GPU.
    pub fn write_particles(&self, queue: &wgpu::Queue, data: &[u8]) {
        queue.write_buffer(&self.particle_buffer, 0, data);
    }

    /// Write a single particle's data at the given index.
    pub fn write_particle_at(&self, queue: &wgpu::Queue, index: u32, data: &[u8]) {
        if data.len() != self.particle_stride {
            return; // Data size mismatch
        }
        if index >= self.num_particles {
            return; // Out of bounds
        }
        let offset = index as u64 * self.particle_stride as u64;
        queue.write_buffer(&self.particle_buffer, offset, data);
    }

    /// Sync custom uniform values from config (hot-swap without rebuild).
    ///
    /// This updates the values of existing uniforms. Adding/removing uniforms
    /// still requires a rebuild since the shader struct changes.
    pub fn sync_custom_uniforms(&mut self, uniforms: &HashMap<String, UniformValueConfig>) {
        for (name, value) in &mut self.custom_uniforms {
            if let Some(new_value) = uniforms.get(name) {
                // Only update if types match (can't change type without rebuild)
                let types_match = matches!(
                    (&*value, new_value),
                    (UniformValueConfig::F32(_), UniformValueConfig::F32(_))
                    | (UniformValueConfig::Vec2(_), UniformValueConfig::Vec2(_))
                    | (UniformValueConfig::Vec3(_), UniformValueConfig::Vec3(_))
                    | (UniformValueConfig::Vec4(_), UniformValueConfig::Vec4(_))
                );
                if types_match {
                    *value = new_value.clone();
                }
            }
        }
    }

    /// Check if uniform structure matches (same names and types).
    pub fn uniforms_match(&self, uniforms: &HashMap<String, UniformValueConfig>) -> bool {
        if self.custom_uniforms.len() != uniforms.len() {
            return false;
        }
        for (name, value) in &self.custom_uniforms {
            match uniforms.get(name) {
                Some(other) => {
                    let types_match = matches!(
                        (value, other),
                        (UniformValueConfig::F32(_), UniformValueConfig::F32(_))
                        | (UniformValueConfig::Vec2(_), UniformValueConfig::Vec2(_))
                        | (UniformValueConfig::Vec3(_), UniformValueConfig::Vec3(_))
                        | (UniformValueConfig::Vec4(_), UniformValueConfig::Vec4(_))
                    );
                    if !types_match {
                        return false;
                    }
                }
                None => return false,
            }
        }
        true
    }
}

/// Lightweight callback struct for each frame.
///
/// This is passed to `Callback::new_paint_callback()` and contains
/// per-frame parameters. The heavy resources are in `SimulationResources`.
pub struct SimulationCallback {
    pub delta_time: f32,
    pub clear_color: [f32; 3],
}

impl egui_wgpu::CallbackTrait for SimulationCallback {
    fn prepare(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        screen_descriptor: &egui_wgpu::ScreenDescriptor,
        _egui_encoder: &mut wgpu::CommandEncoder,
        resources: &mut egui_wgpu::CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        let aspect = screen_descriptor.size_in_pixels[0] as f32
            / screen_descriptor.size_in_pixels[1] as f32;

        if let Some(sim) = resources.get_mut::<SimulationResources>() {
            sim.prepare(device, queue, self.delta_time, aspect)
        } else {
            vec![]
        }
    }

    fn paint(
        &self,
        _info: egui::PaintCallbackInfo,
        render_pass: &mut wgpu::RenderPass<'static>,
        resources: &egui_wgpu::CallbackResources,
    ) {
        if let Some(sim) = resources.get::<SimulationResources>() {
            sim.paint(render_pass);
        }
    }
}

/// High-level handle for managing the embedded simulation in the editor.
pub struct EmbeddedSimulation {
    /// Whether the simulation has been initialized in CallbackResources.
    initialized: bool,
    /// Current delta time.
    delta_time: f32,
    /// Last frame instant for delta time calculation.
    last_frame: std::time::Instant,
    /// Shader compilation error message (if any).
    shader_error: Option<String>,
}

impl EmbeddedSimulation {
    /// Create a new embedded simulation handle (resources not yet created).
    pub fn new() -> Self {
        Self {
            initialized: false,
            delta_time: 0.016,
            last_frame: std::time::Instant::now(),
            shader_error: None,
        }
    }

    /// Get the current shader error, if any.
    pub fn shader_error(&self) -> Option<&str> {
        self.shader_error.as_deref()
    }

    /// Clear the shader error.
    pub fn clear_error(&mut self) {
        self.shader_error = None;
    }

    /// Initialize the simulation resources in egui's callback resources.
    ///
    /// Call this once when the wgpu render state is available.
    pub fn initialize(
        &mut self,
        wgpu_render_state: &egui_wgpu::RenderState,
        config: &SimConfig,
    ) {
        if self.initialized {
            return;
        }

        // Get particle layout from config
        let layout = config.particle_layout();

        // Generate particle data using proper spawn config
        let particle_data = spawn::generate_particles(config);

        // Generate shaders using the actual rule system
        let compute_shader = shader_gen::generate_compute_shader(config);
        let render_shader = shader_gen::generate_render_shader(config);

        // Validate shaders before compiling
        if let Err(errors) = shader_validate::validate_shaders(&compute_shader, &render_shader) {
            let error_msg = errors.iter()
                .map(|e| e.to_string())
                .collect::<Vec<_>>()
                .join("\n\n");
            self.shader_error = Some(error_msg);
            return; // Don't crash, just store error
        }

        // Clear any previous error
        self.shader_error = None;

        let field_registry = config.to_field_registry();
        let particle_wgsl_struct = config.particle_wgsl_struct();
        let resources = SimulationResources::new(
            &wgpu_render_state.device,
            &wgpu_render_state.queue,
            wgpu_render_state.target_format,
            &particle_data,
            config.particle_count,
            &layout,
            &compute_shader,
            &render_shader,
            Vec3::from_array(config.visuals.background_color),
            &config.custom_uniforms,
            &field_registry,
            &config.volume_render,
            config.needs_spatial(),
            config.spatial_cell_size,
            config.spatial_resolution,
            &particle_wgsl_struct,
        );

        wgpu_render_state
            .renderer
            .write()
            .callback_resources
            .insert(resources);

        self.initialized = true;
    }

    /// Check if initialized.
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Reinitialize the simulation with a new config, preserving particle state if possible.
    ///
    /// This is used when config changes require a rebuild but we want to keep particles.
    /// Note: If particle count changes, state cannot be preserved and particles are regenerated.
    pub fn reinitialize(
        &mut self,
        wgpu_render_state: &egui_wgpu::RenderState,
        config: &SimConfig,
    ) {
        // Generate new shaders first to validate before any state changes
        let compute_shader = shader_gen::generate_compute_shader(config);
        let render_shader = shader_gen::generate_render_shader(config);

        // Validate shaders before compiling
        if let Err(errors) = shader_validate::validate_shaders(&compute_shader, &render_shader) {
            let error_msg = errors.iter()
                .map(|e| e.to_string())
                .collect::<Vec<_>>()
                .join("\n\n");
            self.shader_error = Some(error_msg);
            return; // Don't crash, keep old resources running
        }

        // Clear any previous error
        self.shader_error = None;

        // Get particle layout from config
        let layout = config.particle_layout();

        // Read existing particles and camera state if we're already initialized
        // If stride changed (due to adding/removing custom fields), we can't preserve existing particle data
        let (existing_particles, old_camera) = if self.initialized {
            let resources = wgpu_render_state.renderer.read();
            if let Some(sim) = resources.callback_resources.get::<SimulationResources>() {
                let particles = if sim.num_particles == config.particle_count && sim.particle_stride == layout.stride {
                    Some(sim.read_particles(&wgpu_render_state.device, &wgpu_render_state.queue))
                } else {
                    None // Particle count or stride changed, can't preserve
                };
                let camera = Some((sim.camera_distance, sim.camera_yaw, sim.camera_pitch));
                (particles, camera)
            } else {
                (None, None)
            }
        } else {
            (None, None)
        };

        // Generate particle data (use existing or new)
        let particle_data = if let Some(data) = existing_particles {
            data
        } else {
            spawn::generate_particles(config)
        };

        // Create new resources
        let field_registry = config.to_field_registry();
        let particle_wgsl_struct = config.particle_wgsl_struct();
        let resources = SimulationResources::new(
            &wgpu_render_state.device,
            &wgpu_render_state.queue,
            wgpu_render_state.target_format,
            &particle_data,
            config.particle_count,
            &layout,
            &compute_shader,
            &render_shader,
            Vec3::from_array(config.visuals.background_color),
            &config.custom_uniforms,
            &field_registry,
            &config.volume_render,
            config.needs_spatial(),
            config.spatial_cell_size,
            config.spatial_resolution,
            &particle_wgsl_struct,
        );

        // Replace resources
        wgpu_render_state
            .renderer
            .write()
            .callback_resources
            .insert(resources);

        // Restore camera state if we had one
        if let Some((distance, yaw, pitch)) = old_camera {
            if let Some(sim) = wgpu_render_state.renderer.write().callback_resources.get_mut::<SimulationResources>() {
                sim.camera_distance = distance;
                sim.camera_yaw = yaw;
                sim.camera_pitch = pitch;
            }
        }

        self.initialized = true;
    }

    /// Full reset: regenerate all particles from spawn config.
    ///
    /// Use this when you want fresh particles (after changing spawn settings, or to clear chaos).
    pub fn reset(
        &mut self,
        wgpu_render_state: &egui_wgpu::RenderState,
        config: &SimConfig,
    ) {
        // Generate new shaders first to validate
        let compute_shader = shader_gen::generate_compute_shader(config);
        let render_shader = shader_gen::generate_render_shader(config);

        // Validate shaders before compiling
        if let Err(errors) = shader_validate::validate_shaders(&compute_shader, &render_shader) {
            let error_msg = errors.iter()
                .map(|e| e.to_string())
                .collect::<Vec<_>>()
                .join("\n\n");
            self.shader_error = Some(error_msg);
            return; // Don't crash, keep old resources running
        }

        // Clear any previous error
        self.shader_error = None;

        // Get particle layout from config
        let layout = config.particle_layout();

        // Save camera state before replacing resources
        let old_camera = {
            let resources = wgpu_render_state.renderer.read();
            resources.callback_resources.get::<SimulationResources>()
                .map(|sim| (sim.camera_distance, sim.camera_yaw, sim.camera_pitch))
        };

        // Always generate fresh particles
        let particle_data = spawn::generate_particles(config);

        // Create new resources
        let field_registry = config.to_field_registry();
        let particle_wgsl_struct = config.particle_wgsl_struct();
        let resources = SimulationResources::new(
            &wgpu_render_state.device,
            &wgpu_render_state.queue,
            wgpu_render_state.target_format,
            &particle_data,
            config.particle_count,
            &layout,
            &compute_shader,
            &render_shader,
            Vec3::from_array(config.visuals.background_color),
            &config.custom_uniforms,
            &field_registry,
            &config.volume_render,
            config.needs_spatial(),
            config.spatial_cell_size,
            config.spatial_resolution,
            &particle_wgsl_struct,
        );

        // Replace resources
        wgpu_render_state
            .renderer
            .write()
            .callback_resources
            .insert(resources);

        // Restore camera state if we had one
        if let Some((distance, yaw, pitch)) = old_camera {
            if let Some(sim) = wgpu_render_state.renderer.write().callback_resources.get_mut::<SimulationResources>() {
                sim.camera_distance = distance;
                sim.camera_yaw = yaw;
                sim.camera_pitch = pitch;
            }
        }

        self.initialized = true;
    }

    /// Render the simulation viewport in egui.
    ///
    /// Call this in your UI code where you want the viewport to appear.
    pub fn show(&mut self, ui: &mut egui::Ui, wgpu_render_state: &egui_wgpu::RenderState) {
        // Calculate delta time
        let now = std::time::Instant::now();
        self.delta_time = now.duration_since(self.last_frame).as_secs_f32();
        self.last_frame = now;

        // Get available rect
        let rect = ui.available_rect_before_wrap();
        let viewport_width = rect.width() as u32;
        let viewport_height = rect.height() as u32;

        // Handle input for camera control
        let response = ui.allocate_rect(rect, egui::Sense::click_and_drag());

        // Resize picking texture if needed and handle picking
        {
            let mut renderer = wgpu_render_state.renderer.write();
            if let Some(sim) = renderer.callback_resources.get_mut::<SimulationResources>() {
                // Resize picking texture to match viewport
                sim.resize_picking(&wgpu_render_state.device, viewport_width.max(1), viewport_height.max(1));

                // Handle click for particle picking (only on click, not drag)
                if response.clicked() {
                    if let Some(pos) = response.interact_pointer_pos() {
                        let x = (pos.x - rect.left()) as u32;
                        let y = (pos.y - rect.top()) as u32;
                        sim.request_pick(x, y);
                    }
                }

                // Camera rotation via drag
                if response.dragged() {
                    let delta = response.drag_delta();
                    sim.rotate_camera(-delta.x * 0.01, -delta.y * 0.01);
                }

                // Run picking pass to update selection
                sim.update_picking(&wgpu_render_state.device, &wgpu_render_state.queue);
            }
        }

        // Camera zoom via scroll
        let scroll_delta = ui.input(|i| i.raw_scroll_delta.y);
        if scroll_delta.abs() > 0.1 {
            if let Some(sim) = wgpu_render_state.renderer.write().callback_resources.get_mut::<SimulationResources>() {
                sim.zoom_camera(scroll_delta * 0.01);
            }
        }

        // Get background color from resources
        let clear_color = if let Some(sim) = wgpu_render_state.renderer.read().callback_resources.get::<SimulationResources>() {
            let bg = sim.background_color();
            [bg.x, bg.y, bg.z]
        } else {
            [0.0, 0.0, 0.0]
        };

        // Add the paint callback
        let callback = SimulationCallback {
            delta_time: self.delta_time,
            clear_color,
        };

        ui.painter().add(egui_wgpu::Callback::new_paint_callback(
            rect,
            callback,
        ));

        // Request repaint for continuous animation
        ui.ctx().request_repaint();
    }
}

impl Default for EmbeddedSimulation {
    fn default() -> Self {
        Self::new()
    }
}

/// Parsed particle data for display in the inspector.
///
/// This struct holds parsed values for all fields (base and custom)
/// using dynamic layout information.
#[derive(Debug, Clone)]
pub struct ParsedParticle {
    /// Base fields (always present)
    pub position: [f32; 3],
    pub velocity: [f32; 3],
    pub color: [f32; 3],
    pub age: f32,
    pub alive: u32,
    pub scale: f32,
    pub particle_type: u32,
    /// Custom field values (name -> value)
    pub custom_fields: Vec<(String, crate::spawn::FieldValue)>,
}

impl ParsedParticle {
    /// Parse raw particle bytes using the given layout.
    ///
    /// This dynamically parses based on the layout, supporting any particle configuration.
    pub fn from_bytes_with_layout(data: &[u8], layout: &ParticleLayout) -> Option<Self> {
        if data.len() < layout.stride {
            return None;
        }

        use crate::spawn::{read_vec3, read_f32, read_u32, read_field_value};

        // Read base fields
        let position = read_vec3(data, layout.position_offset);
        let velocity = read_vec3(data, layout.velocity_offset);
        let color = read_vec3(data, layout.color_offset);
        let age = read_f32(data, layout.age_offset);
        let alive = read_u32(data, layout.alive_offset);
        let scale = read_f32(data, layout.scale_offset);
        let particle_type = read_u32(data, layout.particle_type_offset);

        // Read custom fields
        let custom_fields: Vec<_> = layout
            .custom_fields()
            .map(|f| {
                let value = read_field_value(data, f.offset, f.field_type);
                (f.name.clone(), value)
            })
            .collect();

        Some(Self {
            position: [position.x, position.y, position.z],
            velocity: [velocity.x, velocity.y, velocity.z],
            color: [color.x, color.y, color.z],
            age,
            alive,
            scale,
            particle_type,
            custom_fields,
        })
    }

    /// Parse raw particle bytes using default base layout (for backwards compatibility).
    ///
    /// This uses a minimal layout with just base fields.
    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        let layout = ParticleLayout::compute(&[]);
        Self::from_bytes_with_layout(data, &layout)
    }

    /// Serialize this particle back to bytes using the given layout.
    pub fn to_bytes(&self, layout: &ParticleLayout) -> Vec<u8> {
        use crate::spawn::{write_vec3_pub, write_f32_pub, write_u32_pub, write_field_value_pub};
        use glam::Vec3;

        let mut bytes = vec![0u8; layout.stride];

        // Write base fields
        write_vec3_pub(&mut bytes, layout.position_offset, Vec3::from_array(self.position));
        write_vec3_pub(&mut bytes, layout.velocity_offset, Vec3::from_array(self.velocity));
        write_vec3_pub(&mut bytes, layout.color_offset, Vec3::from_array(self.color));
        write_f32_pub(&mut bytes, layout.age_offset, self.age);
        write_u32_pub(&mut bytes, layout.alive_offset, self.alive);
        write_f32_pub(&mut bytes, layout.scale_offset, self.scale);
        write_u32_pub(&mut bytes, layout.particle_type_offset, self.particle_type);

        // Write custom fields
        for (name, value) in &self.custom_fields {
            if let Some(offset) = layout.field_offset(name) {
                write_field_value_pub(&mut bytes, offset, value);
            }
        }

        bytes
    }
}
