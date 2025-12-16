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
mod post_process;
mod visualizations;
mod widget;

pub use picking::{PickingRequest, PickingState};
pub use widget::EmbeddedSimulation;

use crate::config::{
    BlendModeConfig, MouseConfig, ParticleLayout, PostProcessConfig, UniformValueConfig,
    VolumeRenderConfig,
};
use post_process::PostProcessState;
use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};
use rdpe::{
    FieldSystemGpu, GlyphConfig, GlyphRenderer, SpatialConfig, SpatialGpu, VolumeRenderState,
    create_particle_field_bind_group_layout,
};
use std::collections::HashMap;
use visualizations::{
    ConnectionVisualization, GridVisualization, TrailVisualization, WireframeVisualization,
};
use wgpu::util::DeviceExt;

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

/// Statistics for a single particle field (scalar or vector).
#[derive(Clone, Debug, Default)]
pub struct FieldStats {
    pub name: String,
    pub components: u32,  // 1 for scalar, 3 for vec3
    pub min: Vec3,        // For scalars, only .x is used
    pub max: Vec3,
    pub avg: Vec3,
}

/// Computed statistics for the simulation.
#[derive(Clone, Debug, Default)]
pub struct SimulationStats {
    // General particle info
    pub total_particles: u32,
    pub alive_particles: u32,

    // Spatial metrics
    pub center_of_mass: Vec3,
    pub bounding_min: Vec3,
    pub bounding_max: Vec3,

    // Velocity metrics
    pub avg_velocity: Vec3,
    pub avg_speed: f32,
    pub max_speed: f32,

    // Per-field statistics
    pub field_stats: Vec<FieldStats>,
}

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
        ray_origin: [
            mouse_state.ray_origin.x,
            mouse_state.ray_origin.y,
            mouse_state.ray_origin.z,
            0.0,
        ],
        ray_dir: [
            mouse_state.ray_dir.x,
            mouse_state.ray_dir.y,
            mouse_state.ray_dir.z,
            0.0,
        ],
        down_radius_strength_pad: [
            if mouse_state.is_down { 1.0 } else { 0.0 },
            mouse_config.radius,
            mouse_config.strength,
            0.0,
        ],
        color: [
            mouse_config.color[0],
            mouse_config.color[1],
            mouse_config.color[2],
            0.0,
        ],
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
    camera_center: Vec3,
    pub auto_orbit: bool,
    pub auto_orbit_speed: f32,

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

    // Adjacency buffer (optional, for graph-based operations)
    adjacency: Option<rdpe::AdjacencyGpu>,
    adjacency_bind_group: Option<wgpu::BindGroup>,

    // Spatial grid visualization (debug overlay)
    grid_viz: Option<GridVisualization>,

    // Connection visualization
    connections: Option<ConnectionVisualization>,

    // Wireframe mesh visualization
    wireframe: Option<WireframeVisualization>,

    // Trail visualization
    trails: Option<TrailVisualization>,

    // Vector glyph visualization
    glyph_renderer: Option<GlyphRenderer>,
    /// Cached field data for glyph sampling (updated periodically)
    glyph_field_cache: Option<Vec<f32>>,
    /// Which field index the cache is for
    glyph_field_index: Option<usize>,
    /// Frame counter for periodic glyph updates
    glyph_update_counter: u32,

    // Statistics
    /// Cached simulation statistics
    stats: SimulationStats,
    /// Frame counter for periodic stats updates
    stats_update_counter: u32,
    /// Particle layout info for parsing readback data
    particle_layout: ParticleLayout,

    // Mouse interaction
    mouse_state: MouseState,
    mouse_config: MouseConfig,

    // Post-processing (optional)
    post_process: Option<PostProcessState>,
    post_process_enabled: bool,
    post_process_config: PostProcessConfig,
    surface_format: wgpu::TextureFormat,
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
        adjacency_enabled: bool,
        adjacency_max_neighbors: u32,
        adjacency_radius: f32,
        post_process_config: &PostProcessConfig,
        viewport_width: u32,
        viewport_height: u32,
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

        // Create adjacency buffer if enabled (requires spatial)
        let adjacency = if adjacency_enabled && spatial.is_some() {
            spatial.as_ref().map(|sp| {
                rdpe::AdjacencyGpu::new(
                    device,
                    &particle_buffer,
                    sp,
                    num_particles,
                    adjacency_max_neighbors,
                    adjacency_radius,
                    particle_stride,
                )
            })
        } else {
            None
        };

        // Sort custom uniforms by name for deterministic order (must match shader generation)
        let mut custom_uniforms: Vec<_> = custom_uniforms_map
            .iter()
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
            spatial.as_ref().map(|s| {
                ConnectionVisualization::new(
                    device,
                    &particle_buffer,
                    &uniform_buffer,
                    s,
                    num_particles,
                    connections_radius,
                    connections_color,
                    particle_stride,
                    target_format,
                )
            })
        } else {
            None
        };

        // Create wireframe visualization if mesh is provided
        let wireframe = wireframe_mesh.map(|mesh| {
            WireframeVisualization::new(
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
            )
        });

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

        // Create glyph renderer for vector field visualization
        let glyph_renderer = Some(GlyphRenderer::new(
            device,
            &uniform_buffer,
            target_format,
            10000, // max glyphs
        ));

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

        let compute_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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

        // Create adjacency bind group layout and bind group (group 3)
        let (adjacency_bind_group_layout, adjacency_bind_group) = if let Some(ref adj) = adjacency {
            let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Adjacency Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });
            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Adjacency Bind Group"),
                layout: &layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: adj.buffer.as_entire_binding(),
                }],
            });
            (Some(layout), Some(bind_group))
        } else {
            (None, None)
        };

        // Create field bind group if fields exist
        let field_bind_group: Option<wgpu::BindGroup> =
            if let (Some(ref fs), Some(ref layout)) = (&field_system, &field_bind_group_layout) {
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
        let empty_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Empty Bind Group Layout"),
                entries: &[],
            });

        // Build compute pipeline layout with optional field and adjacency bind groups
        // Group 0: compute, Group 1: empty placeholder, Group 2: fields, Group 3: adjacency
        let bind_group_layouts: Vec<&wgpu::BindGroupLayout> =
            match (&field_bind_group_layout, &adjacency_bind_group_layout) {
                (Some(field_layout), Some(adj_layout)) => {
                    vec![
                        &compute_bind_group_layout,
                        &empty_bind_group_layout,
                        field_layout,
                        adj_layout,
                    ]
                }
                (Some(field_layout), None) => {
                    vec![
                        &compute_bind_group_layout,
                        &empty_bind_group_layout,
                        field_layout,
                    ]
                }
                (None, Some(adj_layout)) => {
                    // Need empty groups for 1 and 2 before adjacency at group 3
                    vec![
                        &compute_bind_group_layout,
                        &empty_bind_group_layout,
                        &empty_bind_group_layout,
                        adj_layout,
                    ]
                }
                (None, None) => {
                    vec![&compute_bind_group_layout]
                }
            };

        let compute_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
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
        let render_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Render Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let render_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Render Bind Group"),
            layout: &render_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        // Create render pipeline
        let render_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Render Shader"),
            source: wgpu::ShaderSource::Wgsl(render_shader_src.into()),
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
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
            800, // Default width, will resize
            600, // Default height, will resize
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

        // Store empty bind group if fields or adjacency need it
        let needs_empty_bind_group = field_bind_group.is_some() || adjacency_bind_group.is_some();

        // Create post-processing state if enabled
        let post_process = if post_process_config.enabled && viewport_width > 0 && viewport_height > 0 {
            Some(PostProcessState::new(
                device,
                target_format,
                viewport_width,
                viewport_height,
                post_process_config,
            ))
        } else {
            None
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
            camera_center: Vec3::ZERO,
            auto_orbit: false,
            auto_orbit_speed: 0.3,
            last_inv_view_proj: Mat4::IDENTITY,
            last_camera_pos: Vec3::new(0.0, 0.0, 3.0),
            picking,
            field_system,
            empty_bind_group: if needs_empty_bind_group {
                Some(empty_bind_group)
            } else {
                None
            },
            field_bind_group,
            volume_render_state,
            _volume_config: stored_volume_config,
            spatial,
            adjacency,
            adjacency_bind_group,
            grid_viz,
            connections,
            wireframe,
            trails,
            glyph_renderer,
            glyph_field_cache: None,
            glyph_field_index: None,
            glyph_update_counter: 0,
            stats: SimulationStats::default(),
            stats_update_counter: 0,
            particle_layout: layout.clone(),
            mouse_state: MouseState::default(),
            mouse_config,
            post_process,
            post_process_enabled: post_process_config.enabled,
            post_process_config: post_process_config.clone(),
            surface_format: target_format,
        }
    }

    /// Update uniforms and optionally run compute.
    pub fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        delta_time: f32,
        aspect_ratio: f32,
        viewport_width: u32,
        viewport_height: u32,
    ) -> Vec<wgpu::CommandBuffer> {
        // Update time
        if !self.paused {
            self.time += delta_time;
        }

        // Auto-orbit camera (runs even when paused for nice viewing)
        if self.auto_orbit {
            self.camera_yaw += self.auto_orbit_speed * delta_time;
        }

        // Calculate view-projection matrix
        let eye = self.camera_center + Vec3::new(
            self.camera_distance * self.camera_yaw.cos() * self.camera_pitch.cos(),
            self.camera_distance * self.camera_pitch.sin(),
            self.camera_distance * self.camera_yaw.sin() * self.camera_pitch.cos(),
        );
        let view = Mat4::look_at_rh(eye, self.camera_center, Vec3::Y);
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

        // Update post-process state (resize, uniforms)
        if let Some(ref mut pp) = self.post_process {
            if self.post_process_enabled && viewport_width > 0 && viewport_height > 0 {
                // Resize checks internally if dimensions changed
                pp.resize(device, self.surface_format, viewport_width, viewport_height, &self.post_process_config);
                pp.update_uniforms(queue, self.time);
            }
        } else if self.post_process_enabled && viewport_width > 0 && viewport_height > 0 {
            // Create post-process state if newly enabled
            self.post_process = Some(PostProcessState::new(
                device,
                self.surface_format,
                viewport_width,
                viewport_height,
                &self.post_process_config,
            ));
        }

        // Run compute pass if not paused
        let mut command_buffers = if !self.paused {
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Compute Encoder"),
            });

            // Run spatial hashing passes (if enabled) before particle compute
            if let Some(ref spatial) = self.spatial {
                spatial.execute(&mut encoder, queue);
            }

            // Run adjacency pass (after spatial, before particle compute)
            if let Some(ref adjacency) = self.adjacency {
                adjacency.execute(&mut encoder);
            }

            // Run particle compute pass
            {
                let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("Particle Compute"),
                    timestamp_writes: None,
                });
                compute_pass.set_pipeline(&self.compute_pipeline);
                compute_pass.set_bind_group(0, &self.compute_bind_group, &[]);

                // Bind optional groups based on what's enabled
                if let Some(ref empty_bg) = self.empty_bind_group {
                    // Always bind empty at group 1 if we have optional groups
                    compute_pass.set_bind_group(1, empty_bg, &[]);

                    // Bind field at group 2 if available, otherwise empty for adjacency
                    if let Some(ref field_bg) = self.field_bind_group {
                        compute_pass.set_bind_group(2, field_bg, &[]);
                    } else if self.adjacency_bind_group.is_some() {
                        // Adjacency without fields - bind empty at group 2
                        compute_pass.set_bind_group(2, empty_bg, &[]);
                    }
                }

                // Bind adjacency at group 3
                if let Some(ref adj_bg) = self.adjacency_bind_group {
                    compute_pass.set_bind_group(3, adj_bg, &[]);
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
        if let (Some(ref volume_state), Some(ref field_system)) =
            (&self.volume_render_state, &self.field_system)
        {
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

        // Update vector glyphs (periodically to avoid per-frame GPU readback cost)
        self.glyph_update_counter += 1;
        if self.glyph_update_counter >= 10 {
            self.glyph_update_counter = 0;
            self.update_glyphs_internal(device, queue);
        }

        // Update statistics (less frequently than glyphs since it reads all particles)
        self.stats_update_counter += 1;
        if self.stats_update_counter >= 30 {
            self.stats_update_counter = 0;
            self.update_stats_internal(device, queue);
        }

        // If post-processing is enabled, render the scene to the intermediate texture
        if self.has_post_process() {
            if let Some(ref pp) = self.post_process {
                let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Scene Render Encoder"),
                });

                {
                    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("Scene Render to Intermediate"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: pp.get_intermediate_view(),
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color {
                                    r: self.background_color.x as f64,
                                    g: self.background_color.y as f64,
                                    b: self.background_color.z as f64,
                                    a: 1.0,
                                }),
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                    });

                    // Render the scene to the intermediate texture
                    self.paint_scene(&mut render_pass);
                }

                command_buffers.push(encoder.finish());
            }
        }

        command_buffers
    }

    /// Internal method to paint the scene (used by both direct rendering and post-process)
    fn paint_scene(&self, render_pass: &mut wgpu::RenderPass<'_>) {
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

        // Render vector glyphs (on top of everything)
        if let Some(ref glyphs) = self.glyph_renderer {
            glyphs.render(render_pass);
        }
    }

    /// Issue draw commands.
    pub fn paint(&self, render_pass: &mut wgpu::RenderPass<'static>) {
        // If post-processing is enabled, the scene was already rendered to the intermediate
        // texture in prepare(). Now we just need to render the post-process pass.
        if self.has_post_process() {
            if let Some(ref pp) = self.post_process {
                pp.render(render_pass);
            }
        } else {
            // No post-processing - render scene directly
            self.paint_scene(render_pass);
        }
    }

    /// Check if volume rendering is enabled.
    pub fn has_volume_render(&self) -> bool {
        self.volume_render_state.is_some()
    }

    /// Check if post-processing is enabled and active.
    pub fn has_post_process(&self) -> bool {
        self.post_process.is_some() && self.post_process_enabled
    }

    /// Get the intermediate texture view for post-process rendering.
    /// When post-processing is enabled, the scene should render to this texture.
    pub fn get_post_process_intermediate_view(&self) -> Option<&wgpu::TextureView> {
        self.post_process.as_ref().map(|pp| pp.get_intermediate_view())
    }

    /// Render the post-process pass to the final output.
    pub fn render_post_process(&self, render_pass: &mut wgpu::RenderPass<'static>) {
        if let Some(ref pp) = self.post_process {
            pp.render(render_pass);
        }
    }

    /// Update post-process state (time, resize).
    pub fn update_post_process(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        width: u32,
        height: u32,
        config: &PostProcessConfig,
    ) {
        if let Some(ref mut pp) = self.post_process {
            // Resize if needed
            pp.resize(device, self.surface_format, width, height, config);
            // Update uniforms
            pp.update_uniforms(queue, self.time);
        } else if config.enabled && width > 0 && height > 0 {
            // Create post-process state if newly enabled
            self.post_process = Some(PostProcessState::new(
                device,
                self.surface_format,
                width,
                height,
                config,
            ));
            self.post_process_enabled = true;
        }

        // Update enabled state
        self.post_process_enabled = config.enabled;
    }

    /// Rebuild post-process pipeline (for shader changes).
    pub fn rebuild_post_process_pipeline(
        &mut self,
        device: &wgpu::Device,
        config: &PostProcessConfig,
    ) {
        if let Some(ref mut pp) = self.post_process {
            pp.rebuild_pipeline(device, self.surface_format, config);
        }
    }

    /// Update post-process configuration.
    pub fn set_post_process_config(&mut self, config: PostProcessConfig) {
        self.post_process_config = config;
        self.post_process_enabled = self.post_process_config.enabled;
    }

    /// Get current post-process config.
    pub fn post_process_config(&self) -> &PostProcessConfig {
        &self.post_process_config
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

    /// Set glyph renderer configuration.
    pub fn set_glyph_config(&mut self, config: GlyphConfig) {
        if let Some(ref mut glyphs) = self.glyph_renderer {
            glyphs.set_config(config);
        }
    }

    /// Update glyphs from field data.
    ///
    /// `bounds` is the simulation bounds (for sampling the field at grid points).
    /// `sample_field` is called with (x, y, z) and should return the vector at that point.
    pub fn update_glyphs_from_field(
        &mut self,
        queue: &wgpu::Queue,
        bounds: f32,
        sample_field: impl Fn(Vec3) -> Vec3,
    ) {
        if let Some(ref mut glyphs) = self.glyph_renderer {
            glyphs.update_from_field(queue, bounds, sample_field);
        }
    }

    /// Update glyphs from particle velocity data.
    pub fn update_glyphs_from_particles(
        &mut self,
        queue: &wgpu::Queue,
        positions: &[Vec3],
        velocities: &[Vec3],
        sample_rate: u32,
    ) {
        if let Some(ref mut glyphs) = self.glyph_renderer {
            glyphs.update_from_particles(queue, positions, velocities, sample_rate);
        }
    }

    /// Internal method to update glyphs based on current configuration.
    fn update_glyphs_internal(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        let glyph_renderer = match self.glyph_renderer.as_mut() {
            Some(r) => r,
            None => return,
        };

        let config = glyph_renderer.config().clone();

        match config.mode {
            rdpe::GlyphMode::None => {
                // Nothing to do
            }
            rdpe::GlyphMode::VectorField { field_index } => {
                // Get field data from the field system
                if let Some(ref field_system) = self.field_system {
                    if field_index < field_system.fields.len() {
                        let field = &field_system.fields[field_index];
                        let bounds = field.config.world_extent;

                        // Always refresh field data for continuous updates
                        // (the update rate is already limited by glyph_update_counter)
                        let data = field.read_field_data(device, queue);
                        self.glyph_field_cache = Some(data);
                        self.glyph_field_index = Some(field_index);

                        // Update glyphs using the field data
                        if let Some(ref data) = self.glyph_field_cache {
                            let field_ref = &field_system.fields[field_index];
                            glyph_renderer.update_from_field(queue, bounds, |pos| {
                                field_ref.sample_at(data, pos)
                            });
                        }
                    }
                }
            }
            rdpe::GlyphMode::ParticleVelocity => {
                // Particle velocity mode would need particle readback
                // For now, skip (could be implemented later)
            }
        }
    }

    /// Read particle data from GPU.
    fn read_particle_data(&self, device: &wgpu::Device, queue: &wgpu::Queue) -> Vec<u8> {
        let buffer_size = self.particle_buffer.size();

        // Create staging buffer for readback
        let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Particle Readback Staging"),
            size: buffer_size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Copy from particle buffer to staging
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Particle Readback Encoder"),
        });
        encoder.copy_buffer_to_buffer(&self.particle_buffer, 0, &staging_buffer, 0, buffer_size);
        queue.submit(std::iter::once(encoder.finish()));

        // Map and read the staging buffer
        let buffer_slice = staging_buffer.slice(..);
        let (sender, receiver) = std::sync::mpsc::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            sender.send(result).unwrap();
        });
        device.poll(wgpu::Maintain::Wait);
        receiver.recv().unwrap().unwrap();

        let data = buffer_slice.get_mapped_range();
        let result = data.to_vec();
        drop(data);
        staging_buffer.unmap();

        result
    }

    /// Internal method to update simulation statistics.
    fn update_stats_internal(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        let data = self.read_particle_data(device, queue);
        let layout = &self.particle_layout;
        let stride = layout.stride as usize;

        let mut stats = SimulationStats {
            total_particles: self.num_particles,
            ..Default::default()
        };

        let mut alive_count = 0u32;
        let mut pos_sum = Vec3::ZERO;
        let mut vel_sum = Vec3::ZERO;
        let mut speed_sum = 0.0f32;
        let mut max_speed = 0.0f32;
        let mut pos_min = Vec3::splat(f32::MAX);
        let mut pos_max = Vec3::splat(f32::MIN);

        // Initialize per-field stats
        let mut field_stats: Vec<(Vec3, Vec3, Vec3)> = Vec::new(); // (min, max, sum)
        for info in &layout.fields {
            let components = (info.field_type.byte_size() / 4) as u32;
            field_stats.push((
                Vec3::splat(f32::MAX),
                Vec3::splat(f32::MIN),
                Vec3::ZERO,
            ));
            stats.field_stats.push(FieldStats {
                name: info.name.clone(),
                components,
                ..Default::default()
            });
        }

        // Process each particle
        for i in 0..self.num_particles as usize {
            let base = i * stride;
            if base + stride > data.len() {
                break;
            }

            // Check if alive (alive is at a known offset in base fields)
            let alive_offset = layout.alive_offset as usize;
            let alive = if base + alive_offset + 4 <= data.len() {
                u32::from_le_bytes([
                    data[base + alive_offset],
                    data[base + alive_offset + 1],
                    data[base + alive_offset + 2],
                    data[base + alive_offset + 3],
                ])
            } else {
                0
            };

            if alive == 0 {
                continue;
            }
            alive_count += 1;

            // Read position (offset 0)
            let pos = Vec3::new(
                f32::from_le_bytes([data[base], data[base + 1], data[base + 2], data[base + 3]]),
                f32::from_le_bytes([data[base + 4], data[base + 5], data[base + 6], data[base + 7]]),
                f32::from_le_bytes([data[base + 8], data[base + 9], data[base + 10], data[base + 11]]),
            );

            // Read velocity (offset 16, after position + padding)
            let vel_offset = 16;
            let vel = Vec3::new(
                f32::from_le_bytes([data[base + vel_offset], data[base + vel_offset + 1], data[base + vel_offset + 2], data[base + vel_offset + 3]]),
                f32::from_le_bytes([data[base + vel_offset + 4], data[base + vel_offset + 5], data[base + vel_offset + 6], data[base + vel_offset + 7]]),
                f32::from_le_bytes([data[base + vel_offset + 8], data[base + vel_offset + 9], data[base + vel_offset + 10], data[base + vel_offset + 11]]),
            );

            let speed = vel.length();

            pos_sum += pos;
            vel_sum += vel;
            speed_sum += speed;
            max_speed = max_speed.max(speed);
            pos_min = pos_min.min(pos);
            pos_max = pos_max.max(pos);

            // Process each field for per-field stats
            for (field_idx, info) in layout.fields.iter().enumerate() {
                let offset = base + info.offset;
                let components = info.field_type.byte_size() / 4;
                let value = if components >= 3 && offset + 12 <= data.len() {
                    Vec3::new(
                        f32::from_le_bytes([data[offset], data[offset + 1], data[offset + 2], data[offset + 3]]),
                        f32::from_le_bytes([data[offset + 4], data[offset + 5], data[offset + 6], data[offset + 7]]),
                        f32::from_le_bytes([data[offset + 8], data[offset + 9], data[offset + 10], data[offset + 11]]),
                    )
                } else if components == 1 && offset + 4 <= data.len() {
                    Vec3::new(
                        f32::from_le_bytes([data[offset], data[offset + 1], data[offset + 2], data[offset + 3]]),
                        0.0,
                        0.0,
                    )
                } else {
                    Vec3::ZERO
                };

                let (min, max, sum) = &mut field_stats[field_idx];
                *min = min.min(value);
                *max = max.max(value);
                *sum += value;
            }
        }

        // Compute averages
        if alive_count > 0 {
            let count = alive_count as f32;
            stats.alive_particles = alive_count;
            stats.center_of_mass = pos_sum / count;
            stats.bounding_min = pos_min;
            stats.bounding_max = pos_max;
            stats.avg_velocity = vel_sum / count;
            stats.avg_speed = speed_sum / count;
            stats.max_speed = max_speed;

            for (field_idx, (min, max, sum)) in field_stats.into_iter().enumerate() {
                stats.field_stats[field_idx].min = min;
                stats.field_stats[field_idx].max = max;
                stats.field_stats[field_idx].avg = sum / count;
            }
        }

        self.stats = stats;
    }

    /// Get current simulation statistics.
    pub fn stats(&self) -> &SimulationStats {
        &self.stats
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

    /// Move the camera orbit center based on current camera orientation.
    ///
    /// Movement is relative to the camera's view direction:
    /// - `forward`: Move in the direction the camera is facing (in XZ plane)
    /// - `right`: Move perpendicular to forward (strafe)
    /// - `up`: Move along the Y axis
    pub fn move_camera(&mut self, forward: f32, right: f32, up: f32) {
        // Calculate forward direction in XZ plane based on yaw
        let forward_dir = Vec3::new(
            self.camera_yaw.cos(),
            0.0,
            self.camera_yaw.sin(),
        );
        // Right is perpendicular to forward in XZ plane
        let right_dir = Vec3::new(
            -self.camera_yaw.sin(),
            0.0,
            self.camera_yaw.cos(),
        );
        let up_dir = Vec3::Y;

        // Apply movement (scale for comfortable speed)
        let speed = 0.05;
        self.camera_center += forward_dir * forward * speed;
        self.camera_center += right_dir * right * speed;
        self.camera_center += up_dir * up * speed;
    }

    /// Reset camera to default position (origin, default distance/angles).
    pub fn reset_camera(&mut self) {
        self.camera_center = Vec3::ZERO;
        self.camera_distance = 3.0;
        self.camera_yaw = 0.0;
        self.camera_pitch = 0.3;
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
        self.picking
            .render_and_pick(device, queue, &self.particle_buffer, self.num_particles);
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
        let _ = map_result; // We just needed to confirm success

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
            sim.prepare(
                device,
                queue,
                self.delta_time,
                aspect,
                self.viewport_width as u32,
                self.viewport_height as u32,
            )
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

        use crate::spawn::{read_f32, read_field_value, read_u32, read_vec3};

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
        use crate::spawn::{write_f32_pub, write_field_value_pub, write_u32_pub, write_vec3_pub};
        use glam::Vec3;

        let mut bytes = vec![0u8; layout.stride];

        // Write base fields
        write_vec3_pub(
            &mut bytes,
            layout.position_offset,
            Vec3::from_array(self.position),
        );
        write_vec3_pub(
            &mut bytes,
            layout.velocity_offset,
            Vec3::from_array(self.velocity),
        );
        write_vec3_pub(
            &mut bytes,
            layout.color_offset,
            Vec3::from_array(self.color),
        );
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
