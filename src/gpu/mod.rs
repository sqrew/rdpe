mod camera;
mod connections;
mod field_gpu;
mod post_process;
mod spatial_gpu;
mod spatial_grid_viz;
pub mod sub_emitter_gpu;
mod trails;
mod volume_render;

#[cfg(feature = "egui")]
mod egui_integration;

// Re-export submodule types
pub use camera::Camera;
pub use connections::ConnectionState;
pub use field_gpu::{FieldSystemGpu, create_particle_field_bind_group_layout};
pub use post_process::PostProcessState;
pub use spatial_grid_viz::SpatialGridViz;
pub use sub_emitter_gpu::SubEmitterGpu;
pub use trails::TrailState;
pub use volume_render::{VolumeConfig, VolumeRenderState};

use crate::field::FieldRegistry;

#[cfg(feature = "egui")]
pub use egui_integration::EguiIntegration;

use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};
use wgpu::util::DeviceExt;
use winit::window::Window;

pub use spatial_gpu::SpatialGpu;
use crate::spatial::SpatialConfig;
use crate::visuals::BlendMode;

const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
const WORKGROUP_SIZE: u32 = 256;

/// Convert BlendMode to wgpu BlendState
fn blend_mode_to_state(mode: BlendMode) -> wgpu::BlendState {
    match mode {
        BlendMode::Alpha => wgpu::BlendState::ALPHA_BLENDING,
        BlendMode::Additive => wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::SrcAlpha,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            },
        },
        BlendMode::Multiply => wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::Dst,
                dst_factor: wgpu::BlendFactor::Zero,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent::OVER,
        },
    }
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct Uniforms {
    view_proj: [[f32; 4]; 4],
    time: f32,
    delta_time: f32,
}

/// GPU state for particle simulation and rendering.
///
/// Some buffers are stored but not directly read - they must remain alive
/// because bind groups hold references to them.
#[allow(dead_code)]
pub struct GpuState {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    render_pipeline: wgpu::RenderPipeline,
    compute_pipeline: wgpu::ComputePipeline,
    particle_buffer: wgpu::Buffer,
    uniform_buffer: wgpu::Buffer,
    uniform_buffer_size: usize,
    uniform_bind_group: wgpu::BindGroup,
    compute_bind_group: wgpu::BindGroup,
    depth_texture: wgpu::TextureView,
    num_particles: u32,
    pub camera: Camera,
    // Optional spatial hashing
    spatial: Option<SpatialGpu>,
    // Trail rendering
    trail_state: Option<TrailState>,
    // Connection rendering
    connection_state: Option<ConnectionState>,
    // Particle communication inbox
    inbox_buffer: Option<wgpu::Buffer>,
    inbox_bind_group: Option<wgpu::BindGroup>,
    inbox_enabled: bool,
    // 3D spatial fields
    field_system: Option<FieldSystemGpu>,
    field_bind_group: Option<wgpu::BindGroup>,
    field_bind_group_layout: Option<wgpu::BindGroupLayout>,
    // Empty bind group for when fields are enabled but inbox is not
    empty_bind_group: Option<wgpu::BindGroup>,
    // Volume rendering for fields
    volume_render: Option<VolumeRenderState>,
    volume_config: Option<VolumeConfig>,
    // Background clear color
    background_color: Vec3,
    // Post-processing
    post_process: Option<PostProcessState>,
    // Custom textures for shaders
    custom_textures: Vec<wgpu::Texture>,
    custom_texture_views: Vec<wgpu::TextureView>,
    custom_samplers: Vec<wgpu::Sampler>,
    texture_bind_group: Option<wgpu::BindGroup>,
    texture_bind_group_layout: Option<wgpu::BindGroupLayout>,
    // Egui integration (when feature enabled)
    #[cfg(feature = "egui")]
    egui: Option<EguiIntegration>,
    #[cfg(feature = "egui")]
    window: Arc<Window>,
    // Sub-emitter system for spawning particles on death
    sub_emitter: Option<SubEmitterGpu>,
    // Spatial grid visualization
    spatial_grid_viz: Option<SpatialGridViz>,
    // CPU readback support
    particle_stride: usize,
    readback_staging: Option<wgpu::Buffer>,
}

impl GpuState {
    /// Create new GPU state for particle simulation.
    ///
    /// Takes many parameters because GPU initialization genuinely requires
    /// configuring multiple subsystems (particles, spatial hashing, trails, connections).
    #[allow(clippy::too_many_arguments)]
    pub async fn new(
        window: Arc<Window>,
        particle_data: &[u8],
        num_particles: u32,
        particle_stride: usize,
        compute_shader_src: &str,
        render_shader_src: &str,
        has_neighbors: bool,
        spatial_config: SpatialConfig,
        color_offset: Option<u32>,
        alive_offset: u32,
        scale_offset: u32,
        custom_uniform_size: usize,
        blend_mode: BlendMode,
        trail_length: u32,
        particle_size: f32,
        connections_enabled: bool,
        connections_radius: f32,
        inbox_enabled: bool,
        background_color: Vec3,
        post_process_shader: Option<&str>,
        custom_uniform_fields: &str,
        texture_registry: &crate::textures::TextureRegistry,
        _texture_declarations: &str,
        field_registry: &FieldRegistry,
        volume_config: Option<&VolumeConfig>,
        sub_emitters: &[crate::sub_emitter::SubEmitter],
        spatial_grid_opacity: f32,
        particle_wgsl_struct: &str,
        #[cfg(feature = "egui")] egui_enabled: bool,
    ) -> Self {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        let surface = instance.create_surface(window.clone()).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("Device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    memory_hints: Default::default(),
                },
                None, // trace path
            )
            .await
            .unwrap();

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::AutoNoVsync, // Uncapped FPS for benchmarking
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let depth_texture = create_depth_texture(&device, &config);

        let particle_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Particle Buffer"),
            contents: particle_data,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        });

        let camera = Camera::new();
        let aspect = config.width as f32 / config.height as f32;
        let view = camera.view_matrix();
        let proj = Mat4::perspective_rh(45.0_f32.to_radians(), aspect, 0.1, 100.0);
        let view_proj = proj * view;

        let uniforms = Uniforms {
            view_proj: view_proj.to_cols_array_2d(),
            time: 0.0,
            delta_time: 0.0,
        };

        // Base uniform size + custom uniforms (aligned to 16 bytes for uniform buffer)
        // Note: We pad base_size to 16-byte alignment before adding custom uniforms
        // to ensure vec3/vec4 custom uniforms are properly aligned
        let base_size = std::mem::size_of::<Uniforms>();
        let padded_base_size = (base_size + 15) & !15;
        let total_size = ((padded_base_size + custom_uniform_size) + 15) & !15;

        // Create buffer with base uniforms + space for custom uniforms
        let mut uniform_data = bytemuck::bytes_of(&uniforms).to_vec();
        uniform_data.resize(total_size, 0);

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniform Buffer"),
            contents: &uniform_data,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let uniform_buffer_size = total_size;

        // Create spatial hashing if needed
        let spatial = if has_neighbors {
            Some(SpatialGpu::new(
                &device,
                &particle_buffer,
                num_particles,
                spatial_config,
                particle_wgsl_struct,
            ))
        } else {
            None
        };

        // Create inbox buffer for particle communication (4 atomic i32 channels per particle)
        let inbox_buffer = if inbox_enabled {
            // 4 i32 values per particle = 16 bytes per particle
            let inbox_size = (num_particles as usize) * 16;
            Some(device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Inbox Buffer"),
                size: inbox_size as u64,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }))
        } else {
            None
        };

        // Render bind group layout (visible to both vertex and fragment for custom shaders)
        let uniform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Uniform Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Uniform Bind Group"),
            layout: &uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        // Compute bind group layout - different depending on whether we have neighbors
        let (compute_bind_group_layout, compute_bind_group) = if let Some(ref spatial) = spatial {
            // With neighbors: particles, uniforms, sorted_indices, cell_start, cell_end, spatial_params
            let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Compute Bind Group Layout (with neighbors)"),
                entries: &[
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
                ],
            });

            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Compute Bind Group (with neighbors)"),
                layout: &layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: particle_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: uniform_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: spatial.particle_indices_a.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: spatial.cell_start.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: spatial.cell_end.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 5,
                        resource: spatial.spatial_params_buffer.as_entire_binding(),
                    },
                ],
            });

            (layout, bind_group)
        } else {
            // Without neighbors: just particles and uniforms
            let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Compute Bind Group Layout"),
                entries: &[
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
                ],
            });

            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Compute Bind Group"),
                layout: &layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: particle_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: uniform_buffer.as_entire_binding(),
                    },
                ],
            });

            (layout, bind_group)
        };

        // Create texture bind group layout early (needed for render pipeline layout)
        let texture_bind_group_layout = if !texture_registry.textures.is_empty() {
            let mut layout_entries = Vec::new();
            for i in 0..texture_registry.textures.len() {
                layout_entries.push(wgpu::BindGroupLayoutEntry {
                    binding: (i * 2) as u32,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                });
                layout_entries.push(wgpu::BindGroupLayoutEntry {
                    binding: (i * 2 + 1) as u32,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                });
            }
            Some(device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Texture Bind Group Layout"),
                entries: &layout_entries,
            }))
        } else {
            None
        };

        // Render pipeline
        let render_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Render Shader"),
            source: wgpu::ShaderSource::Wgsl(render_shader_src.into()),
        });

        // Build bind group layouts vec, including texture layout if present
        let mut bind_group_layouts_vec: Vec<&wgpu::BindGroupLayout> = vec![&uniform_bind_group_layout];
        if let Some(ref tex_layout) = texture_bind_group_layout {
            bind_group_layouts_vec.push(tex_layout);
        }

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &bind_group_layouts_vec,
                push_constant_ranges: &[],
            });

        // Build vertex attributes: position, optional color, alive, scale
        let vertex_attributes: Vec<wgpu::VertexAttribute> = if let Some(offset) = color_offset {
            vec![
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3, // position
                },
                wgpu::VertexAttribute {
                    offset: offset as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3, // color
                },
                wgpu::VertexAttribute {
                    offset: alive_offset as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Uint32, // alive
                },
                wgpu::VertexAttribute {
                    offset: scale_offset as wgpu::BufferAddress,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float32, // scale
                },
            ]
        } else {
            vec![
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3, // position
                },
                wgpu::VertexAttribute {
                    offset: alive_offset as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Uint32, // alive
                },
                wgpu::VertexAttribute {
                    offset: scale_offset as wgpu::BufferAddress,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float32, // scale
                },
            ]
        };

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &render_shader,
                entry_point: Some("vs_main"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: particle_stride as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &vertex_attributes,
                }],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &render_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(blend_mode_to_state(blend_mode)),
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
                // Disable depth writes for additive blending so particles can blend through each other
                depth_write_enabled: !matches!(blend_mode, BlendMode::Additive),
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // Compute pipeline
        let compute_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Compute Shader"),
            source: wgpu::ShaderSource::Wgsl(compute_shader_src.into()),
        });

        // Create inbox bind group layout and bind group if enabled
        let (inbox_bind_group_layout, inbox_bind_group) = if let Some(ref inbox_buf) = inbox_buffer {
            let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Inbox Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Inbox Bind Group"),
                layout: &layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: inbox_buf.as_entire_binding(),
                }],
            });

            (Some(layout), Some(bind_group))
        } else {
            (None, None)
        };

        // Create field system if fields are registered
        let (field_system, field_bind_group_layout, field_bind_group) = if !field_registry.is_empty() {
            let system = FieldSystemGpu::new(&device, field_registry);
            let layout = create_particle_field_bind_group_layout(&device, system.field_count);
            let bind_group = system.create_particle_bind_group(&device, &layout);
            (Some(system), Some(layout), bind_group)
        } else {
            (None, None, None)
        };

        // Create volume render state if configured and fields exist
        let volume_render = if let (Some(config), Some(ref fs)) = (&volume_config, &field_system) {
            Some(VolumeRenderState::new(&device, fs, config, surface_format))
        } else {
            None
        };

        // Create sub-emitter system early so we can use its bind group layout
        let sub_emitter = if !sub_emitters.is_empty() {
            Some(SubEmitterGpu::new(
                &device,
                &particle_buffer,
                num_particles,
                sub_emitters,
                particle_wgsl_struct,
            ))
        } else {
            None
        };

        // Build compute pipeline layout with optional inbox, field, and sub-emitter bind groups
        // Group 0: particles/uniforms/spatial
        // Group 1: inbox (if enabled)
        // Group 2: fields (if enabled)
        // Group 3: sub-emitter death buffers (if enabled)
        let (compute_pipeline_layout, empty_bind_group) = {
            // Create empty layout/bind group for gaps
            let empty_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Empty Bind Group Layout"),
                entries: &[],
            });
            let empty_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Empty Bind Group"),
                layout: &empty_layout,
                entries: &[],
            });

            // Build layouts vec dynamically
            let mut layouts: Vec<&wgpu::BindGroupLayout> = vec![&compute_bind_group_layout];

            // Group 1: inbox or empty
            if let Some(ref inbox_layout) = inbox_bind_group_layout {
                layouts.push(inbox_layout);
            } else if field_bind_group_layout.is_some() || sub_emitter.is_some() {
                // Need placeholder at group 1 if we have group 2 or 3
                layouts.push(&empty_layout);
            }

            // Group 2: fields or empty
            if let Some(ref field_layout) = field_bind_group_layout {
                layouts.push(field_layout);
            } else if sub_emitter.is_some() {
                // Need placeholder at group 2 if we have group 3
                layouts.push(&empty_layout);
            }

            // Group 3: sub-emitter death buffers
            if let Some(ref se) = sub_emitter {
                layouts.push(&se.death_bind_group_layout);
            }

            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Compute Pipeline Layout"),
                bind_group_layouts: &layouts,
                push_constant_ranges: &[],
            });

            // Only keep empty_bg if we need it
            let keep_empty = (inbox_bind_group_layout.is_none() && (field_bind_group_layout.is_some() || sub_emitter.is_some()))
                || (field_bind_group_layout.is_none() && sub_emitter.is_some());
            (layout, if keep_empty { Some(empty_bg) } else { None })
        };

        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Compute Pipeline"),
            layout: Some(&compute_pipeline_layout),
            module: &compute_shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });

        // Trail system (if trail_length > 0)
        let trail_state = if trail_length > 0 {
            Some(TrailState::new(
                &device,
                &particle_buffer,
                &uniform_buffer,
                num_particles,
                trail_length,
                particle_stride,
                color_offset,
                particle_size,
                blend_mode,
                config.format,
            ))
        } else {
            None
        };

        // Connection system (requires spatial hashing)
        let connection_state = if let (true, Some(spatial_ref)) = (connections_enabled, spatial.as_ref()) {
            Some(ConnectionState::new(
                &device,
                &particle_buffer,
                &uniform_buffer,
                spatial_ref,
                num_particles,
                connections_radius,
                particle_stride,
                blend_mode,
                config.format,
            ))
        } else {
            None
        };

        // Post-processing setup
        let post_process = if let Some(shader_code) = post_process_shader {
            Some(PostProcessState::new(
                &device,
                &uniform_buffer,
                shader_code,
                custom_uniform_fields,
                config.width,
                config.height,
                config.format,
            ))
        } else {
            None
        };

        // Create custom textures
        let mut custom_textures = Vec::new();
        let mut custom_texture_views = Vec::new();
        let mut custom_samplers = Vec::new();

        for (_name, config) in &texture_registry.textures {
            // Create texture
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Custom Texture"),
                size: wgpu::Extent3d {
                    width: config.width,
                    height: config.height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });

            // Upload texture data
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                &config.data,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(4 * config.width),
                    rows_per_image: Some(config.height),
                },
                wgpu::Extent3d {
                    width: config.width,
                    height: config.height,
                    depth_or_array_layers: 1,
                },
            );

            // Create view
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

            // Create sampler
            let filter = match config.filter {
                crate::textures::FilterMode::Linear => wgpu::FilterMode::Linear,
                crate::textures::FilterMode::Nearest => wgpu::FilterMode::Nearest,
            };
            let address_mode = match config.address_mode {
                crate::textures::AddressMode::ClampToEdge => wgpu::AddressMode::ClampToEdge,
                crate::textures::AddressMode::Repeat => wgpu::AddressMode::Repeat,
                crate::textures::AddressMode::MirrorRepeat => wgpu::AddressMode::MirrorRepeat,
            };
            let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
                address_mode_u: address_mode,
                address_mode_v: address_mode,
                address_mode_w: address_mode,
                mag_filter: filter,
                min_filter: filter,
                mipmap_filter: wgpu::FilterMode::Nearest,
                ..Default::default()
            });

            custom_textures.push(texture);
            custom_texture_views.push(view);
            custom_samplers.push(sampler);
        }

        // Create texture bind group using the layout we created earlier
        let texture_bind_group = if let Some(ref layout) = texture_bind_group_layout {
            let mut bind_group_entries = Vec::new();
            for i in 0..custom_texture_views.len() {
                bind_group_entries.push(wgpu::BindGroupEntry {
                    binding: (i * 2) as u32,
                    resource: wgpu::BindingResource::TextureView(&custom_texture_views[i]),
                });
                bind_group_entries.push(wgpu::BindGroupEntry {
                    binding: (i * 2 + 1) as u32,
                    resource: wgpu::BindingResource::Sampler(&custom_samplers[i]),
                });
            }

            Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Texture Bind Group"),
                layout,
                entries: &bind_group_entries,
            }))
        } else {
            None
        };

        // Initialize egui if feature enabled
        #[cfg(feature = "egui")]
        let egui = if egui_enabled {
            Some(EguiIntegration::new(&device, config.format, &window))
        } else {
            None
        };

        // Spatial grid visualization (always created for runtime toggling)
        let spatial_grid_viz = Some(SpatialGridViz::new(
            &device,
            &uniform_buffer,
            &spatial_config,
            spatial_grid_opacity,
            surface_format,
        ));

        Self {
            surface,
            device,
            queue,
            config,
            render_pipeline,
            compute_pipeline,
            particle_buffer,
            uniform_buffer,
            uniform_buffer_size,
            uniform_bind_group,
            compute_bind_group,
            depth_texture,
            num_particles,
            camera,
            spatial,
            trail_state,
            connection_state,
            inbox_buffer,
            inbox_bind_group,
            inbox_enabled,
            field_system,
            field_bind_group,
            field_bind_group_layout,
            empty_bind_group,
            volume_render,
            volume_config: volume_config.cloned(),
            background_color,
            post_process,
            custom_textures,
            custom_texture_views,
            custom_samplers,
            texture_bind_group,
            texture_bind_group_layout,
            #[cfg(feature = "egui")]
            egui,
            #[cfg(feature = "egui")]
            window,
            sub_emitter,
            spatial_grid_viz,
            particle_stride,
            readback_staging: None,
        }
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
            self.depth_texture = create_depth_texture(&self.device, &self.config);

            // Resize post-processing if enabled
            if let Some(ref mut pp) = self.post_process {
                pp.resize(
                    &self.device,
                    &self.uniform_buffer,
                    self.config.width,
                    self.config.height,
                    self.config.format,
                );
            }
        }
    }

    /// Set the spatial grid visualization opacity.
    ///
    /// Use 0.0 to hide the grid, 1.0 for full visibility.
    pub fn set_grid_opacity(&mut self, opacity: f32) {
        if let Some(ref mut grid) = self.spatial_grid_viz {
            grid.set_opacity(&self.queue, opacity);
        }
    }

    /// Read particle data from GPU to CPU synchronously.
    ///
    /// This is an expensive operation that stalls the GPU pipeline.
    /// Use sparingly (e.g., once per second, or on user request).
    ///
    /// Returns raw bytes that can be cast to your particle's GPU type:
    /// ```ignore
    /// let bytes = gpu_state.read_particles_sync();
    /// let particles: &[MyParticleGpu] = bytemuck::cast_slice(&bytes);
    /// ```
    pub fn read_particles_sync(&mut self) -> Vec<u8> {
        let buffer_size = (self.num_particles as usize) * self.particle_stride;

        // Create or reuse staging buffer
        if self.readback_staging.is_none() {
            self.readback_staging = Some(self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Readback Staging Buffer"),
                size: buffer_size as u64,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            }));
        }

        let staging = self.readback_staging.as_ref().unwrap();

        // Copy particle buffer to staging
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Readback Encoder"),
        });
        encoder.copy_buffer_to_buffer(
            &self.particle_buffer,
            0,
            staging,
            0,
            buffer_size as u64,
        );
        self.queue.submit(std::iter::once(encoder.finish()));

        // Map and read
        let buffer_slice = staging.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            tx.send(result).unwrap();
        });

        // Wait for mapping to complete
        self.device.poll(wgpu::Maintain::Wait);
        rx.recv().unwrap().expect("Failed to map readback buffer");

        // Copy data
        let data = buffer_slice.get_mapped_range();
        let result = data.to_vec();
        drop(data);
        staging.unmap();

        result
    }

    /// Get the number of particles.
    pub fn num_particles(&self) -> u32 {
        self.num_particles
    }

    /// Get the particle stride (bytes per particle).
    pub fn particle_stride(&self) -> usize {
        self.particle_stride
    }

    /// Process a winit event through egui.
    ///
    /// Returns true if egui consumed the event (don't pass to camera controls).
    #[cfg(feature = "egui")]
    pub fn on_window_event(&mut self, event: &winit::event::WindowEvent) -> bool {
        if let Some(ref mut egui) = self.egui {
            egui.on_window_event(&self.window, event)
        } else {
            false
        }
    }

    /// Check if egui is enabled.
    #[cfg(feature = "egui")]
    #[allow(dead_code)]
    pub fn egui_enabled(&self) -> bool {
        self.egui.is_some()
    }

    /// Get access to egui context for running UI.
    #[cfg(feature = "egui")]
    #[allow(dead_code)]
    pub fn egui_ctx(&self) -> Option<&egui::Context> {
        self.egui.as_ref().map(|e| &e.ctx)
    }

    fn update_uniforms(&mut self, time: f32, delta_time: f32, custom_uniform_bytes: Option<&[u8]>) {
        let aspect = self.config.width as f32 / self.config.height as f32;
        let view = self.camera.view_matrix();
        let proj = Mat4::perspective_rh(45.0_f32.to_radians(), aspect, 0.1, 100.0);
        let view_proj = proj * view;

        let uniforms = Uniforms {
            view_proj: view_proj.to_cols_array_2d(),
            time,
            delta_time,
        };

        // Write base uniforms
        let base_bytes = bytemuck::bytes_of(&uniforms);

        if let Some(custom_bytes) = custom_uniform_bytes {
            // Combine base and custom uniforms
            let mut combined = base_bytes.to_vec();
            // Pad to 16-byte alignment before appending custom uniforms
            // This ensures vec3/vec4 custom uniforms are properly aligned
            while !combined.len().is_multiple_of(16) {
                combined.push(0);
            }
            combined.extend_from_slice(custom_bytes);
            // Pad to buffer size
            combined.resize(self.uniform_buffer_size, 0);
            self.queue.write_buffer(&self.uniform_buffer, 0, &combined);
        } else {
            self.queue.write_buffer(&self.uniform_buffer, 0, base_bytes);
        }
    }

    /// Render without UI (original method for backwards compatibility).
    pub fn render(&mut self, time: f32, delta_time: f32, custom_uniform_bytes: Option<&[u8]>) -> Result<(), wgpu::SurfaceError> {
        #[cfg(feature = "egui")]
        {
            self.render_with_ui(time, delta_time, custom_uniform_bytes, |_| {})
        }
        #[cfg(not(feature = "egui"))]
        {
            self.render_internal(time, delta_time, custom_uniform_bytes)
        }
    }

    /// Render with egui UI callback.
    #[cfg(feature = "egui")]
    pub fn render_with_ui<F>(
        &mut self,
        time: f32,
        delta_time: f32,
        custom_uniform_bytes: Option<&[u8]>,
        ui_callback: F,
    ) -> Result<(), wgpu::SurfaceError>
    where
        F: FnOnce(&egui::Context),
    {
        use egui_integration::EguiFrameOutput;

        self.update_uniforms(time, delta_time, custom_uniform_bytes);

        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // Process egui frame before creating encoder
        let egui_output: Option<EguiFrameOutput> = if let Some(ref mut egui) = self.egui {
            egui.begin_frame(&self.window);
            ui_callback(&egui.ctx);
            Some(egui.end_frame(&self.window))
        } else {
            None
        };

        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [self.config.width, self.config.height],
            pixels_per_point: self.window.scale_factor() as f32,
        };

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        // Prepare egui textures and buffers (before render pass)
        if let (Some(ref mut egui), Some(ref egui_out)) = (&mut self.egui, &egui_output) {
            egui.prepare(&self.device, &self.queue, &mut encoder, egui_out, &screen_descriptor);
        }

        // Spatial hashing pass (if enabled)
        if let Some(ref spatial) = self.spatial {
            spatial.execute(&mut encoder, &self.queue);
        }

        // Clear inbox buffer before compute pass
        if let Some(ref inbox_buf) = self.inbox_buffer {
            let inbox_size = (self.num_particles as usize) * 16;
            let zeros = vec![0u8; inbox_size];
            self.queue.write_buffer(inbox_buf, 0, &zeros);
        }

        // Clear sub-emitter death buffers before compute pass
        if let Some(ref se) = self.sub_emitter {
            se.clear_buffers(&self.queue);
        }

        // Recreate field bind group each frame (buffers may have been swapped during blur)
        let field_bind_group = if let (Some(ref field_sys), Some(ref layout)) =
            (&self.field_system, &self.field_bind_group_layout)
        {
            field_sys.create_particle_bind_group(&self.device, layout)
        } else {
            None
        };

        // Compute pass
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Compute Pass"),
                timestamp_writes: None,
            });

            compute_pass.set_pipeline(&self.compute_pipeline);
            compute_pass.set_bind_group(0, &self.compute_bind_group, &[]);

            // Set inbox bind group if enabled (group 1)
            if let Some(ref inbox_bg) = self.inbox_bind_group {
                compute_pass.set_bind_group(1, inbox_bg, &[]);
            } else if self.field_system.is_some() || self.sub_emitter.is_some() {
                // Need placeholder at group 1 if we have group 2 or 3
                if let Some(ref empty_bg) = self.empty_bind_group {
                    compute_pass.set_bind_group(1, empty_bg, &[]);
                }
            }

            // Set field bind group if enabled (group 2)
            if let Some(ref field_bg) = field_bind_group {
                compute_pass.set_bind_group(2, field_bg, &[]);
            } else if self.sub_emitter.is_some() {
                // Need placeholder at group 2 if we have group 3
                if let Some(ref empty_bg) = self.empty_bind_group {
                    compute_pass.set_bind_group(2, empty_bg, &[]);
                }
            }

            // Set sub-emitter death buffer bind group if enabled (group 3)
            if let Some(ref se) = self.sub_emitter {
                compute_pass.set_bind_group(3, &se.death_bind_group, &[]);
            }

            let workgroups = self.num_particles.div_ceil(WORKGROUP_SIZE);
            compute_pass.dispatch_workgroups(workgroups, 1, 1);
        }

        // Sub-emitter spawn pass (spawn children from death events)
        if let Some(ref se) = self.sub_emitter {
            se.spawn_children(&mut encoder);
        }

        // Field processing pass (merge deposits, blur/decay, clear write buffer)
        if let Some(ref mut field_sys) = self.field_system {
            field_sys.process(&self.device, &mut encoder, &self.queue);

            // Update volume render bind group after field processing (buffers may have swapped)
            if let Some(ref mut vol) = self.volume_render {
                vol.update_bind_group(&self.device, field_sys);

                // Get camera matrices for ray reconstruction
                let aspect = self.config.width as f32 / self.config.height as f32;
                let proj = Mat4::perspective_rh(std::f32::consts::FRAC_PI_4, aspect, 0.1, 100.0);
                let view = self.camera.view_matrix();
                let view_proj = proj * view;
                let inv_view_proj = view_proj.inverse();
                let camera_pos = self.camera.position();

                // Get field extent and resolution for the rendered field
                let field_idx = vol.field_index;
                let field_extent = field_sys.fields[field_idx].config.world_extent;
                let field_resolution = field_sys.fields[field_idx].config.resolution;

                vol.update_params_with_field(
                    &self.queue,
                    inv_view_proj,
                    camera_pos,
                    field_extent,
                    field_resolution,
                );
            }
        }

        // Trail compute pass (after particles are updated)
        if let Some(ref trail) = self.trail_state {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Trail Compute Pass"),
                timestamp_writes: None,
            });

            compute_pass.set_pipeline(&trail.compute_pipeline);
            compute_pass.set_bind_group(0, &trail.compute_bind_group, &[]);

            let workgroups = self.num_particles.div_ceil(WORKGROUP_SIZE);
            compute_pass.dispatch_workgroups(workgroups, 1, 1);
        }

        // Connection compute pass (find pairs within radius)
        if let Some(ref conn) = self.connection_state {
            // Reset connection count to 0
            self.queue.write_buffer(&conn.count_buffer, 0, &[0u8; 4]);

            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Connection Compute Pass"),
                timestamp_writes: None,
            });

            compute_pass.set_pipeline(&conn.compute_pipeline);
            compute_pass.set_bind_group(0, &conn.compute_bind_group, &[]);

            let workgroups = self.num_particles.div_ceil(WORKGROUP_SIZE);
            compute_pass.dispatch_workgroups(workgroups, 1, 1);
        }

        // Render pass - render to offscreen texture if post-processing, otherwise to screen
        let render_target = if let Some(ref pp) = self.post_process {
            &pp.view
        } else {
            &view
        };
        let depth_target = if let Some(ref pp) = self.post_process {
            &pp.depth_view
        } else {
            &self.depth_texture
        };

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: render_target,
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
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: depth_target,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            // Draw connections first (behind everything)
            if let Some(ref conn) = self.connection_state {
                render_pass.set_pipeline(&conn.render_pipeline);
                render_pass.set_bind_group(0, &conn.render_bind_group, &[]);
                // Draw up to max_connections line quads (6 vertices each)
                render_pass.draw(0..6, 0..conn.max_connections);
            }

            // Draw spatial grid (debug visualization) if opacity > 0
            if let Some(ref grid) = self.spatial_grid_viz {
                if grid.opacity > 0.0 {
                    render_pass.set_pipeline(grid.pipeline());
                    render_pass.set_bind_group(0, grid.bind_group(), &[]);
                    // Draw all grid lines (6 vertices per line quad)
                    render_pass.draw(0..6, 0..grid.line_count());
                }
            }

            // Draw trails (behind particles)
            if let Some(ref trail) = self.trail_state {
                render_pass.set_pipeline(&trail.render_pipeline);
                render_pass.set_bind_group(0, &trail.render_bind_group, &[]);
                // Draw all trail points: num_particles * trail_length instances, 6 vertices each
                let total_trail_instances = self.num_particles * trail.trail_length;
                render_pass.draw(0..6, 0..total_trail_instances);
            }

            // Draw particles on top
            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
            // Bind textures if available
            if let Some(ref tex_bind_group) = self.texture_bind_group {
                render_pass.set_bind_group(1, tex_bind_group, &[]);
            }
            render_pass.set_vertex_buffer(0, self.particle_buffer.slice(..));
            render_pass.draw(0..6, 0..self.num_particles);
        }

        // Volume render pass (if enabled) - renders field as volumetric fog/glow
        if let Some(ref vol) = self.volume_render {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Volume Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: render_target,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load, // Don't clear - blend on top
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None, // No depth for fullscreen volume
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(&vol.pipeline);
            render_pass.set_bind_group(0, &vol.bind_group, &[]);
            render_pass.draw(0..3, 0..1); // Fullscreen triangle
        }

        // Post-processing pass (if enabled)
        if let Some(ref pp) = self.post_process {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Post-Process Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(&pp.pipeline);
            render_pass.set_bind_group(0, &pp.bind_group, &[]);
            render_pass.draw(0..3, 0..1); // Fullscreen triangle
        }

        // Render egui on top of everything (separate render pass for proper blending)
        if let (Some(ref egui), Some(ref egui_out)) = (&self.egui, &egui_output) {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Egui Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load, // Don't clear - draw over particles
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            // SAFETY: egui-wgpu requires RenderPass<'static> but the pass is used synchronously
            // within this scope and dropped before encoder.finish(). This transmute is safe
            // because the render pass doesn't escape this block.
            let render_pass: &mut wgpu::RenderPass<'static> = unsafe {
                std::mem::transmute(&mut render_pass)
            };
            egui.renderer().render(render_pass, &egui_out.paint_jobs, &screen_descriptor);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        // Cleanup egui textures
        if let (Some(ref mut egui), Some(ref egui_out)) = (&mut self.egui, &egui_output) {
            egui.cleanup(egui_out);
        }

        Ok(())
    }

    /// Internal render without egui (used when feature disabled).
    #[cfg(not(feature = "egui"))]
    fn render_internal(&mut self, time: f32, delta_time: f32, custom_uniform_bytes: Option<&[u8]>) -> Result<(), wgpu::SurfaceError> {
        self.update_uniforms(time, delta_time, custom_uniform_bytes);

        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        // Spatial hashing pass (if enabled)
        if let Some(ref spatial) = self.spatial {
            spatial.execute(&mut encoder, &self.queue);
        }

        // Clear inbox buffer before compute pass
        if let Some(ref inbox_buf) = self.inbox_buffer {
            let inbox_size = (self.num_particles as usize) * 16;
            let zeros = vec![0u8; inbox_size];
            self.queue.write_buffer(inbox_buf, 0, &zeros);
        }

        // Clear sub-emitter death buffers before compute pass
        if let Some(ref se) = self.sub_emitter {
            se.clear_buffers(&self.queue);
        }

        // Recreate field bind group each frame (buffers may have been swapped during blur)
        let field_bind_group = if let (Some(ref field_sys), Some(ref layout)) =
            (&self.field_system, &self.field_bind_group_layout)
        {
            field_sys.create_particle_bind_group(&self.device, layout)
        } else {
            None
        };

        // Compute pass
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Compute Pass"),
                timestamp_writes: None,
            });

            compute_pass.set_pipeline(&self.compute_pipeline);
            compute_pass.set_bind_group(0, &self.compute_bind_group, &[]);

            // Set inbox bind group if enabled (group 1)
            if let Some(ref inbox_bg) = self.inbox_bind_group {
                compute_pass.set_bind_group(1, inbox_bg, &[]);
            } else if self.field_system.is_some() || self.sub_emitter.is_some() {
                // Need placeholder at group 1 if we have group 2 or 3
                if let Some(ref empty_bg) = self.empty_bind_group {
                    compute_pass.set_bind_group(1, empty_bg, &[]);
                }
            }

            // Set field bind group if enabled (group 2)
            if let Some(ref field_bg) = field_bind_group {
                compute_pass.set_bind_group(2, field_bg, &[]);
            } else if self.sub_emitter.is_some() {
                // Need placeholder at group 2 if we have group 3
                if let Some(ref empty_bg) = self.empty_bind_group {
                    compute_pass.set_bind_group(2, empty_bg, &[]);
                }
            }

            // Set sub-emitter death buffer bind group if enabled (group 3)
            if let Some(ref se) = self.sub_emitter {
                compute_pass.set_bind_group(3, &se.death_bind_group, &[]);
            }

            let workgroups = self.num_particles.div_ceil(WORKGROUP_SIZE);
            compute_pass.dispatch_workgroups(workgroups, 1, 1);
        }

        // Sub-emitter spawn pass (spawn children from death events)
        if let Some(ref se) = self.sub_emitter {
            se.spawn_children(&mut encoder);
        }

        // Field processing pass (merge deposits, blur/decay, clear write buffer)
        if let Some(ref mut field_sys) = self.field_system {
            field_sys.process(&self.device, &mut encoder, &self.queue);

            // Update volume render bind group after field processing (buffers may have swapped)
            if let Some(ref mut vol) = self.volume_render {
                vol.update_bind_group(&self.device, field_sys);

                // Get camera matrices for ray reconstruction
                let aspect = self.config.width as f32 / self.config.height as f32;
                let proj = Mat4::perspective_rh(std::f32::consts::FRAC_PI_4, aspect, 0.1, 100.0);
                let view = self.camera.view_matrix();
                let view_proj = proj * view;
                let inv_view_proj = view_proj.inverse();
                let camera_pos = self.camera.position();

                // Get field extent and resolution for the rendered field
                let field_idx = vol.field_index;
                let field_extent = field_sys.fields[field_idx].config.world_extent;
                let field_resolution = field_sys.fields[field_idx].config.resolution;

                vol.update_params_with_field(
                    &self.queue,
                    inv_view_proj,
                    camera_pos,
                    field_extent,
                    field_resolution,
                );
            }
        }

        // Trail compute pass (after particles are updated)
        if let Some(ref trail) = self.trail_state {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Trail Compute Pass"),
                timestamp_writes: None,
            });

            compute_pass.set_pipeline(&trail.compute_pipeline);
            compute_pass.set_bind_group(0, &trail.compute_bind_group, &[]);

            let workgroups = self.num_particles.div_ceil(WORKGROUP_SIZE);
            compute_pass.dispatch_workgroups(workgroups, 1, 1);
        }

        // Connection compute pass (find pairs within radius)
        if let Some(ref conn) = self.connection_state {
            // Reset connection count to 0
            self.queue.write_buffer(&conn.count_buffer, 0, &[0u8; 4]);

            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Connection Compute Pass"),
                timestamp_writes: None,
            });

            compute_pass.set_pipeline(&conn.compute_pipeline);
            compute_pass.set_bind_group(0, &conn.compute_bind_group, &[]);

            let workgroups = self.num_particles.div_ceil(WORKGROUP_SIZE);
            compute_pass.dispatch_workgroups(workgroups, 1, 1);
        }

        // Render pass - render to offscreen texture if post-processing, otherwise to screen
        let render_target = if let Some(ref pp) = self.post_process {
            &pp.view
        } else {
            &view
        };
        let depth_target = if let Some(ref pp) = self.post_process {
            &pp.depth_view
        } else {
            &self.depth_texture
        };

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: render_target,
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
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: depth_target,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            // Draw connections first (behind everything)
            if let Some(ref conn) = self.connection_state {
                render_pass.set_pipeline(&conn.render_pipeline);
                render_pass.set_bind_group(0, &conn.render_bind_group, &[]);
                // Draw up to max_connections line quads (6 vertices each)
                render_pass.draw(0..6, 0..conn.max_connections);
            }

            // Draw spatial grid (debug visualization) if opacity > 0
            if let Some(ref grid) = self.spatial_grid_viz {
                if grid.opacity > 0.0 {
                    render_pass.set_pipeline(grid.pipeline());
                    render_pass.set_bind_group(0, grid.bind_group(), &[]);
                    // Draw all grid lines (6 vertices per line quad)
                    render_pass.draw(0..6, 0..grid.line_count());
                }
            }

            // Draw trails (behind particles)
            if let Some(ref trail) = self.trail_state {
                render_pass.set_pipeline(&trail.render_pipeline);
                render_pass.set_bind_group(0, &trail.render_bind_group, &[]);
                // Draw all trail points: num_particles * trail_length instances, 6 vertices each
                let total_trail_instances = self.num_particles * trail.trail_length;
                render_pass.draw(0..6, 0..total_trail_instances);
            }

            // Draw particles on top
            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
            // Bind textures if available
            if let Some(ref tex_bind_group) = self.texture_bind_group {
                render_pass.set_bind_group(1, tex_bind_group, &[]);
            }
            render_pass.set_vertex_buffer(0, self.particle_buffer.slice(..));
            render_pass.draw(0..6, 0..self.num_particles);
        }

        // Volume render pass (if enabled) - renders field as volumetric fog/glow
        if let Some(ref vol) = self.volume_render {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Volume Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: render_target,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load, // Don't clear - blend on top
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None, // No depth for fullscreen volume
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(&vol.pipeline);
            render_pass.set_bind_group(0, &vol.bind_group, &[]);
            render_pass.draw(0..3, 0..1); // Fullscreen triangle
        }

        // Post-processing pass (if enabled)
        if let Some(ref pp) = self.post_process {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Post-Process Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(&pp.pipeline);
            render_pass.set_bind_group(0, &pp.bind_group, &[]);
            render_pass.draw(0..3, 0..1); // Fullscreen triangle
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

fn create_depth_texture(
    device: &wgpu::Device,
    config: &wgpu::SurfaceConfiguration,
) -> wgpu::TextureView {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Depth Texture"),
        size: wgpu::Extent3d {
            width: config.width,
            height: config.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: DEPTH_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    texture.create_view(&wgpu::TextureViewDescriptor::default())
}
