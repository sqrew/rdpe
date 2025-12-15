//! Post-processing rendering system
//!
//! Renders the scene to an intermediate texture, then applies post-processing
//! effects via a customizable shader.

use wgpu::util::DeviceExt;

use crate::config::PostProcessConfig;

/// Post-processing uniforms passed to the shader
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PostProcessUniforms {
    pub time: f32,
    pub intensity: f32,
    pub resolution: [f32; 2],
}

/// Post-processing render state
pub struct PostProcessState {
    /// Intermediate texture to render scene to
    pub intermediate_texture: wgpu::Texture,
    pub intermediate_view: wgpu::TextureView,
    /// Sampler for the intermediate texture
    pub sampler: wgpu::Sampler,
    /// Uniform buffer for post-process params
    pub uniform_buffer: wgpu::Buffer,
    /// Bind group layout
    pub bind_group_layout: wgpu::BindGroupLayout,
    /// Bind group
    pub bind_group: wgpu::BindGroup,
    /// Render pipeline
    pub pipeline: wgpu::RenderPipeline,
    /// Current dimensions
    pub width: u32,
    pub height: u32,
}

impl PostProcessState {
    /// Create new post-processing state
    pub fn new(
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
        width: u32,
        height: u32,
        config: &PostProcessConfig,
    ) -> Self {
        // Create intermediate texture
        let (intermediate_texture, intermediate_view) =
            create_intermediate_texture(device, surface_format, width, height);

        // Create sampler
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        // Create uniform buffer
        let uniforms = PostProcessUniforms {
            time: 0.0,
            intensity: config.intensity,
            resolution: [width as f32, height as f32],
        };
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Post Process Uniform Buffer"),
            contents: bytemuck::cast_slice(&[uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Create bind group layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Post Process Bind Group Layout"),
            entries: &[
                // Texture
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
                // Sampler
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                // Uniforms
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

        // Create bind group
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Post Process Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&intermediate_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: uniform_buffer.as_entire_binding(),
                },
            ],
        });

        // Create pipeline
        let pipeline =
            create_post_process_pipeline(device, surface_format, &bind_group_layout, config);

        Self {
            intermediate_texture,
            intermediate_view,
            sampler,
            uniform_buffer,
            bind_group_layout,
            bind_group,
            pipeline,
            width,
            height,
        }
    }

    /// Resize the intermediate texture
    pub fn resize(
        &mut self,
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
        width: u32,
        height: u32,
        config: &PostProcessConfig,
    ) {
        if self.width == width && self.height == height {
            return;
        }

        self.width = width;
        self.height = height;

        // Recreate texture
        let (intermediate_texture, intermediate_view) =
            create_intermediate_texture(device, surface_format, width, height);
        self.intermediate_texture = intermediate_texture;
        self.intermediate_view = intermediate_view;

        // Recreate bind group with new texture view
        self.bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Post Process Bind Group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&self.intermediate_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.uniform_buffer.as_entire_binding(),
                },
            ],
        });

        // Rebuild pipeline in case shader changed
        self.pipeline = create_post_process_pipeline(
            device,
            surface_format,
            &self.bind_group_layout,
            config,
        );
    }

    /// Update uniforms
    pub fn update_uniforms(&self, queue: &wgpu::Queue, time: f32, intensity: f32) {
        let uniforms = PostProcessUniforms {
            time,
            intensity,
            resolution: [self.width as f32, self.height as f32],
        };
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));
    }

    /// Rebuild the pipeline with new shader code
    pub fn rebuild_pipeline(
        &mut self,
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
        config: &PostProcessConfig,
    ) {
        self.pipeline = create_post_process_pipeline(
            device,
            surface_format,
            &self.bind_group_layout,
            config,
        );
    }

    /// Get the intermediate texture view for rendering the scene to
    pub fn get_intermediate_view(&self) -> &wgpu::TextureView {
        &self.intermediate_view
    }

    /// Render the post-process pass
    pub fn render(&self, render_pass: &mut wgpu::RenderPass<'static>) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.draw(0..3, 0..1); // Fullscreen triangle
    }
}

fn create_intermediate_texture(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    width: u32,
    height: u32,
) -> (wgpu::Texture, wgpu::TextureView) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Post Process Intermediate Texture"),
        size: wgpu::Extent3d {
            width: width.max(1),
            height: height.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });

    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    (texture, view)
}

fn create_post_process_pipeline(
    device: &wgpu::Device,
    surface_format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
    config: &PostProcessConfig,
) -> wgpu::RenderPipeline {
    let shader_code = generate_post_process_shader(config);

    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Post Process Shader"),
        source: wgpu::ShaderSource::Wgsl(shader_code.into()),
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Post Process Pipeline Layout"),
        bind_group_layouts: &[bind_group_layout],
        push_constant_ranges: &[],
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Post Process Pipeline"),
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
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            unclipped_depth: false,
            polygon_mode: wgpu::PolygonMode::Fill,
            conservative: false,
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
        cache: None,
    })
}

fn generate_post_process_shader(config: &PostProcessConfig) -> String {
    let user_code = config.shader_code();

    format!(
        r#"
// Post-process shader

struct PostProcessUniforms {{
    pp_time: f32,
    pp_intensity: f32,
    pp_resolution: vec2<f32>,
}}

@group(0) @binding(0) var input_texture: texture_2d<f32>;
@group(0) @binding(1) var input_sampler: sampler;
@group(0) @binding(2) var<uniform> pp_uniforms: PostProcessUniforms;

struct VertexOutput {{
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {{
    // Fullscreen triangle
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0)
    );

    var uvs = array<vec2<f32>, 3>(
        vec2<f32>(0.0, 1.0),
        vec2<f32>(2.0, 1.0),
        vec2<f32>(0.0, -1.0)
    );

    var out: VertexOutput;
    out.position = vec4<f32>(positions[vertex_index], 0.0, 1.0);
    out.uv = uvs[vertex_index];
    return out;
}}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {{
    let uv = in.uv;
    let pp_time = pp_uniforms.pp_time;
    let pp_intensity = pp_uniforms.pp_intensity;
    let pp_resolution = pp_uniforms.pp_resolution;

    var output_color: vec4<f32>;

    // User post-process code
    {user_code}

    return output_color;
}}
"#
    )
}
