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

mod picking;
mod visualizations;
mod widget;

pub use widget::EmbeddedSimulation;
pub use picking::{PickingState, PickingRequest};

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};
use std::collections::HashMap;
use wgpu::util::DeviceExt;
use crate::config::{BlendModeConfig, UniformValueConfig, ParticleLayout, MouseConfig, VolumeRenderConfig};
use rdpe::{FieldSystemGpu, VolumeRenderState, create_particle_field_bind_group_layout, SpatialGpu, SpatialConfig};
use visualizations::{GridVisualization, ConnectionVisualization, WireframeVisualization, TrailVisualization};

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

/// Mouse uniforms passed to shaders for mouse interaction.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct MouseUniforms {
    /// Ray origin (camera position) in world space (xyz) + padding
    pub ray_origin: [f32; 4],
    /// Ray direction (normalized) in world space (xyz) + padding
    pub ray_dir: [f32; 4],
    /// Mouse button down (1.0) or up (0.0) + radius + strength + padding
    pub down_radius_strength_pad: [f32; 4],
    /// Mouse color (rgb) + padding
    pub color: [f32; 4],
}

const MOUSE_UNIFORMS_SIZE: usize = std::mem::size_of::<MouseUniforms>();

/// Current mouse state for the simulation.
#[derive(Clone, Debug, Default)]
pub struct MouseState {
    /// Ray origin (camera position)
    pub ray_origin: Vec3,
    /// Ray direction (normalized)
    pub ray_dir: Vec3,
    /// Whether the primary mouse button is held
    pub is_down: bool,
}

/// Build uniform buffer data with base uniforms, mouse uniforms, and custom values.
fn build_uniform_data(
    view_proj: Mat4,
    time: f32,
    delta_time: f32,
    mouse_state: &MouseState,
    mouse_config: &MouseConfig,
    custom_uniforms: &[(String, UniformValueConfig)],
) -> Vec<u8> {
    let mut data = Vec::with_capacity(BASE_UNIFORMS_SIZE + MOUSE_UNIFORMS_SIZE + 256); // Reserve extra for custom

    // Write base uniforms
    let base = BaseUniforms {
        view_proj: view_proj.to_cols_array_2d(),
        time,
        delta_time,
        _padding: [0.0; 2],
    };
    data.extend_from_slice(bytemuck::bytes_of(&base));

    // Write mouse uniforms
    let mouse = MouseUniforms {
        ray_origin: [mouse_state.ray_origin.x, mouse_state.ray_origin.y, mouse_state.ray_origin.z, 0.0],
        ray_dir: [mouse_state.ray_dir.x, mouse_state.ray_dir.y, mouse_state.ray_dir.z, 0.0],
        down_radius_strength_pad: [
            if mouse_state.is_down { 1.0 } else { 0.0 },
            mouse_config.radius,
            mouse_config.strength,
            0.0,
        ],
        color: [mouse_config.color[0], mouse_config.color[1], mouse_config.color[2], 0.0],
    };
    data.extend_from_slice(bytemuck::bytes_of(&mouse));

    // Write custom uniforms with proper std140 alignment
    for (_name, value) in custom_uniforms {
        // Align to value's alignment requirement
        let alignment = value.alignment();
        let current_offset = data.len();
        let aligned_offset = current_offset.div_ceil(alignment) * alignment;
        data.resize(aligned_offset, 0u8); // Pad to alignment

        // Write value bytes
        data.extend_from_slice(&value.to_bytes());
    }

    // Ensure minimum buffer size and 16-byte alignment for the total buffer
    let final_size = data.len().div_ceil(16) * 16;
    data.resize(final_size, 0u8);

    data
}

pub struct SimulationResources {
    // Pipelines
    compute_pipeline: wgpu::ComputePipeline,
    render_pipeline: wgpu::RenderPipeline,

    // Buffers
    particle_buffer: wgpu::Buffer,
    uniform_buffer: wgpu::Buffer,
    _uniform_buffer_size: usize,

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
    _volume_config: Option<VolumeRenderConfig>,

    // Spatial hashing (optional, for neighbor queries)
    spatial: Option<SpatialGpu>,

    // Spatial grid visualization (debug overlay)
    grid_viz: Option<GridVisualization>,

    // Connection visualization
    connections: Option<ConnectionVisualization>,

    // Wireframe mesh visualization
    wireframe: Option<WireframeVisualization>,

    // Trail visualization
    trails: Option<TrailVisualization>,

    // Mouse interaction
    mouse_state: MouseState,
    mouse_config: MouseConfig,
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
        blend_mode: &BlendModeConfig,
        spatial_grid_opacity: f32,
        connections_enabled: bool,
        connections_radius: f32,
        connections_color: [f32; 3],
        wireframe_mesh: Option<&rdpe::WireframeMesh>,
        wireframe_thickness: f32,
        particle_size: f32,
        trail_length: u32,
        mouse_config: MouseConfig,
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
            &MouseState::default(),
            &mouse_config,
            &custom_uniforms,
        );
        let uniform_buffer_size = uniform_data.len();
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniform Buffer"),
            contents: &uniform_data,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Create spatial grid visualization (for debug overlay)
        let grid_viz = if needs_spatial {
            Some(GridVisualization::new(
                device,
                &uniform_buffer,
                spatial_cell_size,
                spatial_resolution,
                spatial_grid_opacity,
                target_format,
            ))
        } else {
            None
        };

        // Create connection visualization if enabled (requires spatial)
        let connections = if connections_enabled {
            spatial.as_ref().map(|s| ConnectionVisualization::new(
                device,
                &particle_buffer,
                &uniform_buffer,
                s,
                num_particles,
                connections_radius,
                connections_color,
                particle_stride,
                target_format,
            ))
        } else {
            None
        };

        // Create wireframe visualization if mesh is provided
        let wireframe = wireframe_mesh.map(|mesh| WireframeVisualization::new(
            device,
            &particle_buffer,
            &uniform_buffer,
            mesh,
            wireframe_thickness,
            particle_size,
            num_particles,
            particle_stride,
            Some(layout.color_offset as u32),
            layout.alive_offset as u32,
            layout.scale_offset as u32,
            target_format,
            blend_mode,
        ));

        // Create trail visualization if trail_length > 0
        let trails = if trail_length > 1 {
            Some(TrailVisualization::new(
                device,
                &particle_buffer,
                &uniform_buffer,
                num_particles,
                trail_length,
                particle_stride,
                layout.alive_offset as u32,
                target_format,
            ))
        } else {
            None
        };

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
                    blend: Some(blend_mode.to_wgpu_blend_state()),
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
            _uniform_buffer_size: uniform_buffer_size,
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
            _volume_config: stored_volume_config,
            spatial,
            grid_viz,
            connections,
            wireframe,
            trails,
            mouse_state: MouseState::default(),
            mouse_config,
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

        // Build uniform data including custom uniforms and mouse
        let uniform_data = build_uniform_data(
            view_proj,
            self.time,
            delta_time,
            &self.mouse_state,
            &self.mouse_config,
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

            // Run connection finding compute pass (after spatial update)
            if let Some(ref connections) = self.connections {
                connections.compute(&mut encoder, queue);
            }

            // Run trail update compute pass
            if let Some(ref trails) = self.trails {
                trails.compute(&mut encoder);
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

        // Render spatial grid (debug overlay) - before particles so it's behind
        if let Some(ref grid) = self.grid_viz {
            grid.render(render_pass);
        }

        // Render trails (before particles so they're behind)
        if let Some(ref trails) = self.trails {
            trails.render(render_pass);
        }

        // Render particles (wireframe or billboard)
        if let Some(ref wireframe) = self.wireframe {
            wireframe.render(render_pass);
        } else {
            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.render_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.particle_buffer.slice(..));
            // Draw 4 vertices (quad) per particle instance
            render_pass.draw(0..4, 0..self.num_particles);
        }

        // Render connections (after particles so they overlay)
        if let Some(ref connections) = self.connections {
            connections.render(render_pass);
        }
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

    /// Set grid visualization opacity.
    pub fn set_grid_opacity(&mut self, queue: &wgpu::Queue, opacity: f32) {
        if let Some(ref mut grid) = self.grid_viz {
            grid.set_opacity(queue, opacity);
        }
    }

    /// Update mouse state (ray and button).
    pub fn set_mouse_state(&mut self, ray_origin: Vec3, ray_dir: Vec3, is_down: bool) {
        self.mouse_state.ray_origin = ray_origin;
        self.mouse_state.ray_dir = ray_dir;
        self.mouse_state.is_down = is_down;
    }

    /// Update mouse configuration.
    pub fn set_mouse_config(&mut self, config: MouseConfig) {
        self.mouse_config = config;
    }

    /// Get current mouse config.
    pub fn mouse_config(&self) -> &MouseConfig {
        &self.mouse_config
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
    ///
    /// Returns `None` if the buffer cannot be mapped for reading.
    pub fn read_particles(&self, device: &wgpu::Device, queue: &wgpu::Queue) -> Option<Vec<u8>> {
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
            // Ignore send errors - receiver may have been dropped
            let _ = tx.send(result);
        });

        device.poll(wgpu::Maintain::Wait);

        // Handle channel receive and buffer mapping errors gracefully
        let map_result = rx.recv().ok()?.ok()?;
        drop(map_result); // We just needed to confirm success

        let data = buffer_slice.get_mapped_range();
        let result = data.to_vec();
        drop(data);
        staging.unmap();

        Some(result)
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
    pub viewport_width: f32,
    pub viewport_height: f32,
}

impl egui_wgpu::CallbackTrait for SimulationCallback {
    fn prepare(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        _screen_descriptor: &egui_wgpu::ScreenDescriptor,
        _egui_encoder: &mut wgpu::CommandEncoder,
        resources: &mut egui_wgpu::CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        // Use viewport dimensions for aspect ratio, not screen size
        let aspect = self.viewport_width / self.viewport_height.max(1.0);

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
