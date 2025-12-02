mod spatial_gpu;

#[cfg(feature = "egui")]
mod egui_integration;

#[cfg(feature = "egui")]
pub use egui_integration::EguiIntegration;

use std::sync::Arc;
use std::time::Instant;

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

pub struct Camera {
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
    pub target: Vec3,
}

impl Camera {
    fn new() -> Self {
        Self {
            yaw: 0.0,
            pitch: 0.3,
            distance: 3.0,
            target: Vec3::ZERO,
        }
    }

    fn position(&self) -> Vec3 {
        let x = self.distance * self.pitch.cos() * self.yaw.sin();
        let y = self.distance * self.pitch.sin();
        let z = self.distance * self.pitch.cos() * self.yaw.cos();
        self.target + Vec3::new(x, y, z)
    }

    fn view_matrix(&self) -> Mat4 {
        Mat4::look_at_rh(self.position(), self.target, Vec3::Y)
    }
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
    start_time: Instant,
    last_frame_time: Instant,
    // Optional spatial hashing
    spatial: Option<SpatialGpu>,
    // Trail rendering (buffer kept alive for bind group references)
    trail_buffer: Option<wgpu::Buffer>,
    trail_compute_pipeline: Option<wgpu::ComputePipeline>,
    trail_compute_bind_group: Option<wgpu::BindGroup>,
    trail_render_pipeline: Option<wgpu::RenderPipeline>,
    trail_render_bind_group: Option<wgpu::BindGroup>,
    trail_length: u32,
    // Connection rendering (buffers kept alive for bind group references)
    connections_enabled: bool,
    connections_radius: f32,
    connection_buffer: Option<wgpu::Buffer>,
    connection_count_buffer: Option<wgpu::Buffer>,
    connection_compute_pipeline: Option<wgpu::ComputePipeline>,
    connection_compute_bind_group: Option<wgpu::BindGroup>,
    connection_render_pipeline: Option<wgpu::RenderPipeline>,
    connection_render_bind_group: Option<wgpu::BindGroup>,
    max_connections: u32,
    // Particle communication inbox
    inbox_buffer: Option<wgpu::Buffer>,
    inbox_bind_group: Option<wgpu::BindGroup>,
    inbox_enabled: bool,
    // Background clear color
    background_color: Vec3,
    // Post-processing
    post_process_enabled: bool,
    offscreen_texture: Option<wgpu::Texture>,
    offscreen_view: Option<wgpu::TextureView>,
    offscreen_depth_texture: Option<wgpu::Texture>,
    offscreen_depth_view: Option<wgpu::TextureView>,
    post_process_pipeline: Option<wgpu::RenderPipeline>,
    post_process_bind_group: Option<wgpu::BindGroup>,
    post_process_bind_group_layout: Option<wgpu::BindGroupLayout>,
    scene_sampler: Option<wgpu::Sampler>,
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
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::STORAGE,
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
        let base_size = std::mem::size_of::<Uniforms>();
        let total_size = ((base_size + custom_uniform_size) + 15) & !15;

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
            Some(SpatialGpu::new(&device, &particle_buffer, num_particles, spatial_config))
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

        // Build compute pipeline layout with optional inbox bind group
        let compute_pipeline_layout = if let Some(ref inbox_layout) = inbox_bind_group_layout {
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Compute Pipeline Layout"),
                bind_group_layouts: &[&compute_bind_group_layout, inbox_layout],
                push_constant_ranges: &[],
            })
        } else {
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Compute Pipeline Layout"),
                bind_group_layouts: &[&compute_bind_group_layout],
                push_constant_ranges: &[],
            })
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
        let (trail_buffer, trail_compute_pipeline, trail_compute_bind_group, trail_render_pipeline, trail_render_bind_group) =
            if trail_length > 0 {
                // Trail buffer: stores position history for each particle
                // Each entry is vec4<f32> (xyz = position, w = alpha/validity)
                let trail_buffer_size = (num_particles as usize) * (trail_length as usize) * 16; // 16 bytes per vec4
                let trail_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("Trail Buffer"),
                    size: trail_buffer_size as u64,
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::VERTEX,
                    mapped_at_creation: false,
                });

                // Trail params uniform
                #[repr(C)]
                #[derive(Copy, Clone, Pod, Zeroable)]
                struct TrailParams {
                    num_particles: u32,
                    trail_length: u32,
                    _pad: [u32; 2],
                }
                let trail_params = TrailParams {
                    num_particles,
                    trail_length,
                    _pad: [0; 2],
                };
                let trail_params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Trail Params Buffer"),
                    contents: bytemuck::bytes_of(&trail_params),
                    usage: wgpu::BufferUsages::UNIFORM,
                });

                // Trail compute shader - shifts trail positions and stores new position
                let trail_compute_shader_src = format!(r#"
struct TrailParams {{
    num_particles: u32,
    trail_length: u32,
}};

@group(0) @binding(0)
var<storage, read> particles: array<vec4<f32>>; // Only need position (first vec4)

@group(0) @binding(1)
var<storage, read_write> trails: array<vec4<f32>>;

@group(0) @binding(2)
var<uniform> params: TrailParams;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {{
    let particle_idx = global_id.x;
    if particle_idx >= params.num_particles {{
        return;
    }}

    let trail_base = particle_idx * params.trail_length;

    // Shift trail positions back (from end to start)
    for (var i = params.trail_length - 1u; i > 0u; i--) {{
        trails[trail_base + i] = trails[trail_base + i - 1u];
    }}

    // Store current position at front with full alpha
    // Read position from particle buffer (assuming position is first field)
    let pos = particles[particle_idx * {particle_stride_vec4}u];
    trails[trail_base] = vec4<f32>(pos.xyz, 1.0);
}}
"#, particle_stride_vec4 = particle_stride / 16);

                let trail_compute_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some("Trail Compute Shader"),
                    source: wgpu::ShaderSource::Wgsl(trail_compute_shader_src.into()),
                });

                // Trail compute bind group layout
                let trail_compute_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Trail Compute Bind Group Layout"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: false },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 2,
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

                let trail_compute_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("Trail Compute Bind Group"),
                    layout: &trail_compute_bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: particle_buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: trail_buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: trail_params_buffer.as_entire_binding(),
                        },
                    ],
                });

                let trail_compute_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Trail Compute Pipeline Layout"),
                    bind_group_layouts: &[&trail_compute_bind_group_layout],
                    push_constant_ranges: &[],
                });

                let trail_compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("Trail Compute Pipeline"),
                    layout: Some(&trail_compute_pipeline_layout),
                    module: &trail_compute_shader,
                    entry_point: Some("main"),
                    compilation_options: Default::default(),
                    cache: None,
                });

                // Trail render shader - renders trail points with fading alpha
                let trail_render_shader_src = format!(r#"
struct Uniforms {{
    view_proj: mat4x4<f32>,
    time: f32,
    delta_time: f32,
}};

struct TrailParams {{
    num_particles: u32,
    trail_length: u32,
}};

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

@group(0) @binding(1)
var<storage, read> trails: array<vec4<f32>>;

@group(0) @binding(2)
var<uniform> params: TrailParams;

struct VertexOutput {{
    @builtin(position) clip_position: vec4<f32>,
    @location(0) alpha: f32,
    @location(1) uv: vec2<f32>,
}};

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32,
) -> VertexOutput {{
    var out: VertexOutput;

    // Decode particle and trail index from instance
    let particle_idx = instance_index / params.trail_length;
    let trail_idx = instance_index % params.trail_length;

    // Get trail position
    let trail_base = particle_idx * params.trail_length;
    let trail_data = trails[trail_base + trail_idx];
    let pos = trail_data.xyz;
    let valid = trail_data.w;

    // Skip invalid trail points
    if valid < 0.5 {{
        out.clip_position = vec4<f32>(0.0, 0.0, -1000.0, 1.0);
        out.alpha = 0.0;
        out.uv = vec2<f32>(0.0);
        return out;
    }}

    // Quad vertices
    var quad_vertices = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>( 1.0,  1.0),
    );

    let quad_pos = quad_vertices[vertex_index];

    // Size decreases along trail, alpha also fades
    let trail_progress = f32(trail_idx) / f32(params.trail_length);
    let size_factor = 1.0 - trail_progress * 0.7; // Size from 100% to 30%
    let alpha_factor = 1.0 - trail_progress; // Alpha from 100% to 0%

    let base_size = {particle_size};
    let trail_size = base_size * size_factor * 0.5; // Trail points are smaller

    let world_pos = vec4<f32>(pos, 1.0);
    var clip_pos = uniforms.view_proj * world_pos;

    clip_pos.x += quad_pos.x * trail_size * clip_pos.w;
    clip_pos.y += quad_pos.y * trail_size * clip_pos.w;

    out.clip_position = clip_pos;
    out.alpha = alpha_factor * 0.5; // Trail is semi-transparent
    out.uv = quad_pos;

    return out;
}}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {{
    let dist = length(in.uv);
    if dist > 1.0 {{
        discard;
    }}
    let circle_alpha = 1.0 - smoothstep(0.3, 1.0, dist);
    // Trail color: white/grey gradient
    let color = vec3<f32>(0.7, 0.8, 1.0);
    return vec4<f32>(color, circle_alpha * in.alpha);
}}
"#, particle_size = particle_size);

                let trail_render_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some("Trail Render Shader"),
                    source: wgpu::ShaderSource::Wgsl(trail_render_shader_src.into()),
                });

                // Trail render bind group layout
                let trail_render_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Trail Render Bind Group Layout"),
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
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                    ],
                });

                let trail_render_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("Trail Render Bind Group"),
                    layout: &trail_render_bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: uniform_buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: trail_buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: trail_params_buffer.as_entire_binding(),
                        },
                    ],
                });

                let trail_render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Trail Render Pipeline Layout"),
                    bind_group_layouts: &[&trail_render_bind_group_layout],
                    push_constant_ranges: &[],
                });

                let trail_render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("Trail Render Pipeline"),
                    layout: Some(&trail_render_pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &trail_render_shader,
                        entry_point: Some("vs_main"),
                        buffers: &[], // No vertex buffers, all data from storage
                        compilation_options: Default::default(),
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &trail_render_shader,
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
                        depth_write_enabled: false, // Trails don't write to depth
                        depth_compare: wgpu::CompareFunction::Less,
                        stencil: wgpu::StencilState::default(),
                        bias: wgpu::DepthBiasState::default(),
                    }),
                    multisample: wgpu::MultisampleState::default(),
                    multiview: None,
                    cache: None,
                });

                (Some(trail_buffer), Some(trail_compute_pipeline), Some(trail_compute_bind_group),
                 Some(trail_render_pipeline), Some(trail_render_bind_group))
            } else {
                (None, None, None, None, None)
            };

        // Connection system (requires spatial hashing)
        let max_connections = num_particles * 8; // Average 8 connections per particle max
        let (connection_buffer, connection_count_buffer, connection_compute_pipeline,
             connection_compute_bind_group, connection_render_pipeline, connection_render_bind_group) =
            if let (true, Some(spatial_ref)) = (connections_enabled, spatial.as_ref()) {

                // Connection buffer: stores line segments as vec4 pairs (posA.xyz + alpha, posB.xyz + unused)
                let connection_buffer_size = (max_connections as usize) * 32; // 2 vec4s per connection
                let connection_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("Connection Buffer"),
                    size: connection_buffer_size as u64,
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::VERTEX,
                    mapped_at_creation: false,
                });

                // Atomic counter for number of connections
                let connection_count_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Connection Count Buffer"),
                    contents: &[0u8; 4],
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                });

                // Connection params
                #[repr(C)]
                #[derive(Copy, Clone, Pod, Zeroable)]
                struct ConnectionParams {
                    radius: f32,
                    max_connections: u32,
                    num_particles: u32,
                    _pad: u32,
                }
                let conn_params = ConnectionParams {
                    radius: connections_radius,
                    max_connections,
                    num_particles,
                    _pad: 0,
                };
                let conn_params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Connection Params Buffer"),
                    contents: bytemuck::bytes_of(&conn_params),
                    usage: wgpu::BufferUsages::UNIFORM,
                });

                // Connection compute shader - finds pairs within radius
                let conn_compute_shader_src = format!(r#"
struct ConnectionParams {{
    radius: f32,
    max_connections: u32,
    num_particles: u32,
}};

struct SpatialParams {{
    cell_size: f32,
    grid_resolution: u32,
    num_particles: u32,
    _pad: u32,
}};

// Morton encoding for spatial hashing
fn morton_encode_10bit(x: u32, y: u32, z: u32) -> u32 {{
    var xx = x & 0x3FFu;
    var yy = y & 0x3FFu;
    var zz = z & 0x3FFu;

    xx = (xx | (xx << 16u)) & 0x030000FFu;
    xx = (xx | (xx << 8u)) & 0x0300F00Fu;
    xx = (xx | (xx << 4u)) & 0x030C30C3u;
    xx = (xx | (xx << 2u)) & 0x09249249u;

    yy = (yy | (yy << 16u)) & 0x030000FFu;
    yy = (yy | (yy << 8u)) & 0x0300F00Fu;
    yy = (yy | (yy << 4u)) & 0x030C30C3u;
    yy = (yy | (yy << 2u)) & 0x09249249u;

    zz = (zz | (zz << 16u)) & 0x030000FFu;
    zz = (zz | (zz << 8u)) & 0x0300F00Fu;
    zz = (zz | (zz << 4u)) & 0x030C30C3u;
    zz = (zz | (zz << 2u)) & 0x09249249u;

    return xx | (yy << 1u) | (zz << 2u);
}}

fn pos_to_cell(pos: vec3<f32>, cell_size: f32, grid_res: u32) -> vec3<i32> {{
    let half_grid = f32(grid_res) * 0.5;
    let grid_pos = (pos / cell_size) + half_grid;
    return vec3<i32>(
        clamp(i32(floor(grid_pos.x)), 0, i32(grid_res) - 1),
        clamp(i32(floor(grid_pos.y)), 0, i32(grid_res) - 1),
        clamp(i32(floor(grid_pos.z)), 0, i32(grid_res) - 1)
    );
}}

@group(0) @binding(0)
var<storage, read> particles: array<vec4<f32>>;

@group(0) @binding(1)
var<storage, read_write> connections: array<vec4<f32>>;

@group(0) @binding(2)
var<storage, read_write> connection_count: atomic<u32>;

@group(0) @binding(3)
var<uniform> params: ConnectionParams;

@group(0) @binding(4)
var<storage, read> sorted_indices: array<u32>;

@group(0) @binding(5)
var<storage, read> cell_start: array<u32>;

@group(0) @binding(6)
var<storage, read> cell_end: array<u32>;

@group(0) @binding(7)
var<uniform> spatial: SpatialParams;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {{
    let idx = global_id.x;
    if idx >= params.num_particles {{
        return;
    }}

    let my_pos = particles[idx * {particle_stride_vec4}u].xyz;
    let my_cell = pos_to_cell(my_pos, spatial.cell_size, spatial.grid_resolution);
    let radius_sq = params.radius * params.radius;

    // Only check particles with higher index to avoid duplicates
    for (var dx = -1; dx <= 1; dx++) {{
        for (var dy = -1; dy <= 1; dy++) {{
            for (var dz = -1; dz <= 1; dz++) {{
                let neighbor_cell = my_cell + vec3<i32>(dx, dy, dz);

                if neighbor_cell.x < 0 || neighbor_cell.x >= i32(spatial.grid_resolution) ||
                   neighbor_cell.y < 0 || neighbor_cell.y >= i32(spatial.grid_resolution) ||
                   neighbor_cell.z < 0 || neighbor_cell.z >= i32(spatial.grid_resolution) {{
                    continue;
                }}

                let morton = morton_encode_10bit(u32(neighbor_cell.x), u32(neighbor_cell.y), u32(neighbor_cell.z));
                let start = cell_start[morton];
                let end = cell_end[morton];

                if start == 0xFFFFFFFFu {{
                    continue;
                }}

                for (var j = start; j < end; j++) {{
                    let other_idx = sorted_indices[j];

                    // Only connect to particles with higher index (avoid duplicates)
                    if other_idx <= idx {{
                        continue;
                    }}

                    let other_pos = particles[other_idx * {particle_stride_vec4}u].xyz;
                    let diff = other_pos - my_pos;
                    let dist_sq = dot(diff, diff);

                    if dist_sq < radius_sq && dist_sq > 0.0001 {{
                        let conn_idx = atomicAdd(&connection_count, 1u);
                        if conn_idx < params.max_connections {{
                            let dist = sqrt(dist_sq);
                            let alpha = 1.0 - dist / params.radius;
                            connections[conn_idx * 2u] = vec4<f32>(my_pos, alpha);
                            connections[conn_idx * 2u + 1u] = vec4<f32>(other_pos, 0.0);
                        }}
                    }}
                }}
            }}
        }}
    }}
}}
"#, particle_stride_vec4 = particle_stride / 16);

                let conn_compute_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some("Connection Compute Shader"),
                    source: wgpu::ShaderSource::Wgsl(conn_compute_shader_src.into()),
                });

                let conn_compute_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Connection Compute Bind Group Layout"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: false },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 2,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: false },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 3,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
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
                                ty: wgpu::BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 6,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 7,
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

                let conn_compute_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("Connection Compute Bind Group"),
                    layout: &conn_compute_bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: particle_buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: connection_buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: connection_count_buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 3,
                            resource: conn_params_buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 4,
                            resource: spatial_ref.particle_indices_a.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 5,
                            resource: spatial_ref.cell_start.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 6,
                            resource: spatial_ref.cell_end.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 7,
                            resource: spatial_ref.spatial_params_buffer.as_entire_binding(),
                        },
                    ],
                });

                let conn_compute_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Connection Compute Pipeline Layout"),
                    bind_group_layouts: &[&conn_compute_bind_group_layout],
                    push_constant_ranges: &[],
                });

                let conn_compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("Connection Compute Pipeline"),
                    layout: Some(&conn_compute_pipeline_layout),
                    module: &conn_compute_shader,
                    entry_point: Some("main"),
                    compilation_options: Default::default(),
                    cache: None,
                });

                // Connection render shader - draws lines as thin quads
                let conn_render_shader_src = r#"
struct Uniforms {
    view_proj: mat4x4<f32>,
    time: f32,
    delta_time: f32,
};

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

@group(0) @binding(1)
var<storage, read> connections: array<vec4<f32>>;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) alpha: f32,
};

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32,
) -> VertexOutput {
    var out: VertexOutput;

    // Each connection is 2 vec4s: posA with alpha, posB
    let conn_data_a = connections[instance_index * 2u];
    let conn_data_b = connections[instance_index * 2u + 1u];

    let pos_a = conn_data_a.xyz;
    let pos_b = conn_data_b.xyz;
    let alpha = conn_data_a.w;

    // Skip invalid connections (alpha = 0 means not set)
    if alpha < 0.001 {
        out.clip_position = vec4<f32>(0.0, 0.0, -1000.0, 1.0);
        out.alpha = 0.0;
        return out;
    }

    // Create a thin quad for the line
    // Vertices: 0-1 at pos_a, 2-3 at pos_b (as a quad)
    let line_dir = normalize(pos_b - pos_a);

    // Get perpendicular vector for line width
    var perp = cross(line_dir, vec3<f32>(0.0, 1.0, 0.0));
    if length(perp) < 0.001 {
        perp = cross(line_dir, vec3<f32>(1.0, 0.0, 0.0));
    }
    perp = normalize(perp) * 0.002; // Line width

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
    out.alpha = alpha * 0.5;

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(0.5, 0.7, 1.0, in.alpha);
}
"#;

                let conn_render_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some("Connection Render Shader"),
                    source: wgpu::ShaderSource::Wgsl(conn_render_shader_src.into()),
                });

                let conn_render_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Connection Render Bind Group Layout"),
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
                    ],
                });

                let conn_render_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("Connection Render Bind Group"),
                    layout: &conn_render_bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: uniform_buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: connection_buffer.as_entire_binding(),
                        },
                    ],
                });

                let conn_render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Connection Render Pipeline Layout"),
                    bind_group_layouts: &[&conn_render_bind_group_layout],
                    push_constant_ranges: &[],
                });

                let conn_render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("Connection Render Pipeline"),
                    layout: Some(&conn_render_pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &conn_render_shader,
                        entry_point: Some("vs_main"),
                        buffers: &[],
                        compilation_options: Default::default(),
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &conn_render_shader,
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
                        depth_write_enabled: false,
                        depth_compare: wgpu::CompareFunction::Less,
                        stencil: wgpu::StencilState::default(),
                        bias: wgpu::DepthBiasState::default(),
                    }),
                    multisample: wgpu::MultisampleState::default(),
                    multiview: None,
                    cache: None,
                });

                (Some(connection_buffer), Some(connection_count_buffer), Some(conn_compute_pipeline),
                 Some(conn_compute_bind_group), Some(conn_render_pipeline), Some(conn_render_bind_group))
            } else {
                (None, None, None, None, None, None)
            };

        let now = Instant::now();

        // Post-processing setup
        let post_process_enabled = post_process_shader.is_some();
        let (offscreen_texture, offscreen_view, offscreen_depth_texture, offscreen_depth_view,
             post_process_pipeline, post_process_bind_group, post_process_bind_group_layout, scene_sampler) =
            if let Some(shader_code) = post_process_shader {
                // Create offscreen render target
                let offscreen_tex = device.create_texture(&wgpu::TextureDescriptor {
                    label: Some("Offscreen Texture"),
                    size: wgpu::Extent3d {
                        width: config.width,
                        height: config.height,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: config.format,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                });
                let offscreen_v = offscreen_tex.create_view(&wgpu::TextureViewDescriptor::default());

                // Offscreen depth buffer
                let offscreen_depth_tex = device.create_texture(&wgpu::TextureDescriptor {
                    label: Some("Offscreen Depth Texture"),
                    size: wgpu::Extent3d {
                        width: config.width,
                        height: config.height,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: DEPTH_FORMAT,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                    view_formats: &[],
                });
                let offscreen_depth_v = offscreen_depth_tex.create_view(&wgpu::TextureViewDescriptor::default());

                // Sampler for the scene texture
                let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
                    label: Some("Scene Sampler"),
                    address_mode_u: wgpu::AddressMode::ClampToEdge,
                    address_mode_v: wgpu::AddressMode::ClampToEdge,
                    address_mode_w: wgpu::AddressMode::ClampToEdge,
                    mag_filter: wgpu::FilterMode::Linear,
                    min_filter: wgpu::FilterMode::Linear,
                    mipmap_filter: wgpu::FilterMode::Nearest,
                    ..Default::default()
                });

                // Post-process shader - uses same Uniforms layout as main render shader
                let post_shader_src = format!(
                    r#"
struct Uniforms {{
    view_proj: mat4x4<f32>,
    time: f32,
    delta_time: f32,
{custom_uniform_fields}
}};

struct VertexOutput {{
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}};

@group(0) @binding(0)
var scene: texture_2d<f32>;
@group(0) @binding(1)
var scene_sampler: sampler;
@group(0) @binding(2)
var<uniform> uniforms: Uniforms;

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {{
    // Fullscreen triangle
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );
    var uvs = array<vec2<f32>, 3>(
        vec2<f32>(0.0, 1.0),
        vec2<f32>(2.0, 1.0),
        vec2<f32>(0.0, -1.0),
    );

    var out: VertexOutput;
    out.clip_position = vec4<f32>(positions[vertex_index], 0.0, 1.0);
    out.uv = uvs[vertex_index];
    return out;
}}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {{
{shader_code}
}}
"#,
                    shader_code = shader_code,
                    custom_uniform_fields = custom_uniform_fields
                );

                let post_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some("Post-Process Shader"),
                    source: wgpu::ShaderSource::Wgsl(post_shader_src.into()),
                });

                // Bind group layout
                let pp_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Post-Process Bind Group Layout"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled: false,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
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

                // We need a small uniform buffer for time/resolution
                // This will be created separately and updated in render

                let pp_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("Post-Process Bind Group"),
                    layout: &pp_bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&offscreen_v),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(&sampler),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: uniform_buffer.as_entire_binding(), // Reuse main uniform buffer (time is at offset 64)
                        },
                    ],
                });

                let pp_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Post-Process Pipeline Layout"),
                    bind_group_layouts: &[&pp_bind_group_layout],
                    push_constant_ranges: &[],
                });

                let pp_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("Post-Process Pipeline"),
                    layout: Some(&pp_pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &post_shader,
                        entry_point: Some("vs_main"),
                        buffers: &[],
                        compilation_options: Default::default(),
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &post_shader,
                        entry_point: Some("fs_main"),
                        targets: &[Some(wgpu::ColorTargetState {
                            format: config.format,
                            blend: None,
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

                (Some(offscreen_tex), Some(offscreen_v), Some(offscreen_depth_tex), Some(offscreen_depth_v),
                 Some(pp_pipeline), Some(pp_bind_group), Some(pp_bind_group_layout), Some(sampler))
            } else {
                (None, None, None, None, None, None, None, None)
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
            start_time: now,
            last_frame_time: now,
            spatial,
            trail_buffer,
            trail_compute_pipeline,
            trail_compute_bind_group,
            trail_render_pipeline,
            trail_render_bind_group,
            trail_length,
            connections_enabled,
            connections_radius,
            connection_buffer,
            connection_count_buffer,
            connection_compute_pipeline,
            connection_compute_bind_group,
            connection_render_pipeline,
            connection_render_bind_group,
            max_connections,
            inbox_buffer,
            inbox_bind_group,
            inbox_enabled,
            background_color,
            post_process_enabled,
            offscreen_texture,
            offscreen_view,
            offscreen_depth_texture,
            offscreen_depth_view,
            post_process_pipeline,
            post_process_bind_group,
            post_process_bind_group_layout,
            scene_sampler,
            custom_textures,
            custom_texture_views,
            custom_samplers,
            texture_bind_group,
            texture_bind_group_layout,
            #[cfg(feature = "egui")]
            egui,
            #[cfg(feature = "egui")]
            window,
        }
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
            self.depth_texture = create_depth_texture(&self.device, &self.config);

            // Recreate offscreen textures if post-processing is enabled
            if self.post_process_enabled {
                // Recreate offscreen color texture
                let offscreen_texture = self.device.create_texture(&wgpu::TextureDescriptor {
                    label: Some("Offscreen Texture"),
                    size: wgpu::Extent3d {
                        width: self.config.width,
                        height: self.config.height,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: self.config.format,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                });
                let offscreen_view = offscreen_texture.create_view(&wgpu::TextureViewDescriptor::default());

                // Recreate offscreen depth texture
                let offscreen_depth_texture = self.device.create_texture(&wgpu::TextureDescriptor {
                    label: Some("Offscreen Depth Texture"),
                    size: wgpu::Extent3d {
                        width: self.config.width,
                        height: self.config.height,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Depth32Float,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                    view_formats: &[],
                });
                let offscreen_depth_view = offscreen_depth_texture.create_view(&wgpu::TextureViewDescriptor::default());

                // Recreate post-process bind group with new texture view
                if let Some(ref layout) = self.post_process_bind_group_layout {
                    let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("Post-Process Bind Group"),
                        layout,
                        entries: &[
                            wgpu::BindGroupEntry {
                                binding: 0,
                                resource: wgpu::BindingResource::TextureView(&offscreen_view),
                            },
                            wgpu::BindGroupEntry {
                                binding: 1,
                                resource: wgpu::BindingResource::Sampler(self.scene_sampler.as_ref().unwrap()),
                            },
                            wgpu::BindGroupEntry {
                                binding: 2,
                                resource: self.uniform_buffer.as_entire_binding(),
                            },
                        ],
                    });
                    self.post_process_bind_group = Some(bind_group);
                }

                self.offscreen_texture = Some(offscreen_texture);
                self.offscreen_view = Some(offscreen_view);
                self.offscreen_depth_texture = Some(offscreen_depth_texture);
                self.offscreen_depth_view = Some(offscreen_depth_view);
            }
        }
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
    pub fn egui_enabled(&self) -> bool {
        self.egui.is_some()
    }

    /// Get mutable access to egui context for running UI.
    #[cfg(feature = "egui")]
    pub fn egui_ctx(&self) -> Option<&egui::Context> {
        self.egui.as_ref().map(|e| &e.ctx)
    }

    /// Get current time and delta time, updating internal state.
    pub fn get_time_info(&mut self) -> (f32, f32) {
        let now = Instant::now();
        let delta_time = now.duration_since(self.last_frame_time).as_secs_f32();
        self.last_frame_time = now;
        let time = self.start_time.elapsed().as_secs_f32();
        (time, delta_time)
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

        // Compute pass
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Compute Pass"),
                timestamp_writes: None,
            });

            compute_pass.set_pipeline(&self.compute_pipeline);
            compute_pass.set_bind_group(0, &self.compute_bind_group, &[]);

            // Set inbox bind group if enabled
            if let Some(ref inbox_bg) = self.inbox_bind_group {
                compute_pass.set_bind_group(1, inbox_bg, &[]);
            }

            let workgroups = self.num_particles.div_ceil(WORKGROUP_SIZE);
            compute_pass.dispatch_workgroups(workgroups, 1, 1);
        }

        // Trail compute pass (after particles are updated)
        if let (Some(ref pipeline), Some(ref bind_group)) = (&self.trail_compute_pipeline, &self.trail_compute_bind_group) {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Trail Compute Pass"),
                timestamp_writes: None,
            });

            compute_pass.set_pipeline(pipeline);
            compute_pass.set_bind_group(0, bind_group, &[]);

            let workgroups = self.num_particles.div_ceil(WORKGROUP_SIZE);
            compute_pass.dispatch_workgroups(workgroups, 1, 1);
        }

        // Connection compute pass (find pairs within radius)
        if let (Some(ref count_buffer), Some(ref pipeline), Some(ref bind_group)) =
            (&self.connection_count_buffer, &self.connection_compute_pipeline, &self.connection_compute_bind_group) {
            // Reset connection count to 0
            self.queue.write_buffer(count_buffer, 0, &[0u8; 4]);

            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Connection Compute Pass"),
                timestamp_writes: None,
            });

            compute_pass.set_pipeline(pipeline);
            compute_pass.set_bind_group(0, bind_group, &[]);

            let workgroups = self.num_particles.div_ceil(WORKGROUP_SIZE);
            compute_pass.dispatch_workgroups(workgroups, 1, 1);
        }

        // Render pass - render to offscreen texture if post-processing, otherwise to screen
        let render_target = if self.post_process_enabled {
            self.offscreen_view.as_ref().unwrap()
        } else {
            &view
        };
        let depth_target = if self.post_process_enabled {
            self.offscreen_depth_view.as_ref().unwrap()
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
            if let (Some(ref pipeline), Some(ref bind_group)) = (&self.connection_render_pipeline, &self.connection_render_bind_group) {
                render_pass.set_pipeline(pipeline);
                render_pass.set_bind_group(0, bind_group, &[]);
                // Draw up to max_connections line quads (6 vertices each)
                render_pass.draw(0..6, 0..self.max_connections);
            }

            // Draw trails (behind particles)
            if let (Some(ref pipeline), Some(ref bind_group)) = (&self.trail_render_pipeline, &self.trail_render_bind_group) {
                render_pass.set_pipeline(pipeline);
                render_pass.set_bind_group(0, bind_group, &[]);
                // Draw all trail points: num_particles * trail_length instances, 6 vertices each
                let total_trail_instances = self.num_particles * self.trail_length;
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

        // Post-processing pass (if enabled)
        if let (Some(ref pipeline), Some(ref bind_group)) = (&self.post_process_pipeline, &self.post_process_bind_group) {
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

            render_pass.set_pipeline(pipeline);
            render_pass.set_bind_group(0, bind_group, &[]);
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

        // Compute pass
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Compute Pass"),
                timestamp_writes: None,
            });

            compute_pass.set_pipeline(&self.compute_pipeline);
            compute_pass.set_bind_group(0, &self.compute_bind_group, &[]);

            // Set inbox bind group if enabled
            if let Some(ref inbox_bg) = self.inbox_bind_group {
                compute_pass.set_bind_group(1, inbox_bg, &[]);
            }

            let workgroups = self.num_particles.div_ceil(WORKGROUP_SIZE);
            compute_pass.dispatch_workgroups(workgroups, 1, 1);
        }

        // Trail compute pass (after particles are updated)
        if let (Some(ref pipeline), Some(ref bind_group)) = (&self.trail_compute_pipeline, &self.trail_compute_bind_group) {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Trail Compute Pass"),
                timestamp_writes: None,
            });

            compute_pass.set_pipeline(pipeline);
            compute_pass.set_bind_group(0, bind_group, &[]);

            let workgroups = self.num_particles.div_ceil(WORKGROUP_SIZE);
            compute_pass.dispatch_workgroups(workgroups, 1, 1);
        }

        // Connection compute pass (find pairs within radius)
        if let (Some(ref count_buffer), Some(ref pipeline), Some(ref bind_group)) =
            (&self.connection_count_buffer, &self.connection_compute_pipeline, &self.connection_compute_bind_group) {
            // Reset connection count to 0
            self.queue.write_buffer(count_buffer, 0, &[0u8; 4]);

            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Connection Compute Pass"),
                timestamp_writes: None,
            });

            compute_pass.set_pipeline(pipeline);
            compute_pass.set_bind_group(0, bind_group, &[]);

            let workgroups = self.num_particles.div_ceil(WORKGROUP_SIZE);
            compute_pass.dispatch_workgroups(workgroups, 1, 1);
        }

        // Render pass - render to offscreen texture if post-processing, otherwise to screen
        let render_target = if self.post_process_enabled {
            self.offscreen_view.as_ref().unwrap()
        } else {
            &view
        };
        let depth_target = if self.post_process_enabled {
            self.offscreen_depth_view.as_ref().unwrap()
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
            if let (Some(ref pipeline), Some(ref bind_group)) = (&self.connection_render_pipeline, &self.connection_render_bind_group) {
                render_pass.set_pipeline(pipeline);
                render_pass.set_bind_group(0, bind_group, &[]);
                // Draw up to max_connections line quads (6 vertices each)
                render_pass.draw(0..6, 0..self.max_connections);
            }

            // Draw trails (behind particles)
            if let (Some(ref pipeline), Some(ref bind_group)) = (&self.trail_render_pipeline, &self.trail_render_bind_group) {
                render_pass.set_pipeline(pipeline);
                render_pass.set_bind_group(0, bind_group, &[]);
                // Draw all trail points: num_particles * trail_length instances, 6 vertices each
                let total_trail_instances = self.num_particles * self.trail_length;
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

        // Post-processing pass (if enabled)
        if let (Some(ref pipeline), Some(ref bind_group)) = (&self.post_process_pipeline, &self.post_process_bind_group) {
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

            render_pass.set_pipeline(pipeline);
            render_pass.set_bind_group(0, bind_group, &[]);
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
