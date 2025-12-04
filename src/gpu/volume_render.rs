//! Volume rendering for 3D spatial fields.
//!
//! Renders fields as volumetric fog/clouds using ray marching.
//! This allows visualizing field data directly without particles.

use bytemuck::{Pod, Zeroable};
use glam::Vec3;
use wgpu::util::DeviceExt;

use super::FieldSystemGpu;
use crate::visuals::Palette;

/// Configuration for volume rendering.
#[derive(Clone, Debug)]
pub struct VolumeConfig {
    /// Which field index to render (default: 0).
    pub field_index: u32,
    /// Number of ray march steps (higher = better quality, slower).
    pub steps: u32,
    /// Density multiplier (higher = more opaque).
    pub density_scale: f32,
    /// Color palette for density mapping.
    pub palette: Palette,
    /// Minimum density threshold (values below are transparent).
    pub threshold: f32,
    /// Whether to use additive blending (glow effect).
    pub additive: bool,
}

impl Default for VolumeConfig {
    fn default() -> Self {
        Self {
            field_index: 0,
            steps: 64,
            density_scale: 5.0,
            palette: Palette::Inferno,
            threshold: 0.01,
            additive: true,
        }
    }
}

impl VolumeConfig {
    /// Create a new volume config with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the field index to render.
    pub fn with_field(mut self, index: u32) -> Self {
        self.field_index = index;
        self
    }

    /// Set the number of ray march steps.
    pub fn with_steps(mut self, steps: u32) -> Self {
        self.steps = steps.clamp(8, 256);
        self
    }

    /// Set the density scale multiplier.
    pub fn with_density_scale(mut self, scale: f32) -> Self {
        self.density_scale = scale;
        self
    }

    /// Set the color palette.
    pub fn with_palette(mut self, palette: Palette) -> Self {
        self.palette = palette;
        self
    }

    /// Set the minimum density threshold.
    pub fn with_threshold(mut self, threshold: f32) -> Self {
        self.threshold = threshold.max(0.0);
        self
    }

    /// Enable or disable additive blending.
    pub fn with_additive(mut self, additive: bool) -> Self {
        self.additive = additive;
        self
    }
}

/// GPU parameters for volume rendering.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct VolumeParams {
    /// Inverse view-projection matrix for ray reconstruction.
    inv_view_proj: [[f32; 4]; 4],
    /// Camera position in world space.
    camera_pos: [f32; 3],
    /// Number of ray march steps.
    steps: u32,
    /// Field extent (world space bounds).
    field_extent: f32,
    /// Field resolution.
    field_resolution: u32,
    /// Density scale multiplier.
    density_scale: f32,
    /// Minimum density threshold.
    threshold: f32,
    /// Palette colors (5 stops).
    palette: [[f32; 4]; 5],
}

/// GPU state for volume rendering.
#[allow(dead_code)]
pub struct VolumeRenderState {
    /// Render pipeline for the volume pass.
    pub pipeline: wgpu::RenderPipeline,
    /// Bind group layout for recreation.
    pub bind_group_layout: wgpu::BindGroupLayout,
    /// Current bind group.
    pub bind_group: wgpu::BindGroup,
    /// Volume parameters buffer.
    pub params_buffer: wgpu::Buffer,
    /// Configuration.
    pub config: VolumeConfig,
    /// Field index being rendered.
    pub field_index: usize,
}

impl VolumeRenderState {
    /// Create a new volume render system.
    pub fn new(
        device: &wgpu::Device,
        field_system: &FieldSystemGpu,
        config: &VolumeConfig,
        surface_format: wgpu::TextureFormat,
    ) -> Self {
        let field_index = config.field_index as usize;

        // Get field info
        let field = &field_system.fields[field_index];
        let field_extent = field.config.world_extent;
        let field_resolution = field.config.resolution;

        // Create params buffer with placeholder values (updated each frame)
        let palette_colors = config.palette.colors();
        let params = VolumeParams {
            inv_view_proj: glam::Mat4::IDENTITY.to_cols_array_2d(),
            camera_pos: [0.0, 0.0, 3.0],
            steps: config.steps,
            field_extent,
            field_resolution,
            density_scale: config.density_scale,
            threshold: config.threshold,
            palette: [
                [palette_colors[0].x, palette_colors[0].y, palette_colors[0].z, 1.0],
                [palette_colors[1].x, palette_colors[1].y, palette_colors[1].z, 1.0],
                [palette_colors[2].x, palette_colors[2].y, palette_colors[2].z, 1.0],
                [palette_colors[3].x, palette_colors[3].y, palette_colors[3].z, 1.0],
                [palette_colors[4].x, palette_colors[4].y, palette_colors[4].z, 1.0],
            ],
        };

        let params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Volume Params Buffer"),
            contents: bytemuck::bytes_of(&params),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Create bind group layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Volume Render Bind Group Layout"),
            entries: &[
                // Volume params
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Field read buffer
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        // Create bind group
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Volume Render Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: field.current_read_buffer().as_entire_binding(),
                },
            ],
        });

        // Create shader
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Volume Render Shader"),
            source: wgpu::ShaderSource::Wgsl(VOLUME_SHADER.into()),
        });

        // Create pipeline
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Volume Render Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let blend_state = if config.additive {
            wgpu::BlendState {
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
            }
        } else {
            wgpu::BlendState::ALPHA_BLENDING
        };

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Volume Render Pipeline"),
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
                    blend: Some(blend_state),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None, // Volume renders behind everything, no depth test
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        Self {
            pipeline,
            bind_group_layout,
            bind_group,
            params_buffer,
            config: config.clone(),
            field_index,
        }
    }

    /// Update the bind group when field buffers change (after blur swap).
    pub fn update_bind_group(
        &mut self,
        device: &wgpu::Device,
        field_system: &FieldSystemGpu,
    ) {
        let field = &field_system.fields[self.field_index];

        self.bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Volume Render Bind Group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: field.current_read_buffer().as_entire_binding(),
                },
            ],
        });
    }

    /// Update parameters with field info.
    pub fn update_params_with_field(
        &self,
        queue: &wgpu::Queue,
        inv_view_proj: glam::Mat4,
        camera_pos: Vec3,
        field_extent: f32,
        field_resolution: u32,
    ) {
        let palette_colors = self.config.palette.colors();

        let params = VolumeParams {
            inv_view_proj: inv_view_proj.to_cols_array_2d(),
            camera_pos: camera_pos.to_array(),
            steps: self.config.steps,
            field_extent,
            field_resolution,
            density_scale: self.config.density_scale,
            threshold: self.config.threshold,
            palette: [
                [palette_colors[0].x, palette_colors[0].y, palette_colors[0].z, 1.0],
                [palette_colors[1].x, palette_colors[1].y, palette_colors[1].z, 1.0],
                [palette_colors[2].x, palette_colors[2].y, palette_colors[2].z, 1.0],
                [palette_colors[3].x, palette_colors[3].y, palette_colors[3].z, 1.0],
                [palette_colors[4].x, palette_colors[4].y, palette_colors[4].z, 1.0],
            ],
        };

        queue.write_buffer(&self.params_buffer, 0, bytemuck::bytes_of(&params));
    }
}

/// Ray marching volume shader.
const VOLUME_SHADER: &str = r#"
struct VolumeParams {
    inv_view_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    steps: u32,
    field_extent: f32,
    field_resolution: u32,
    density_scale: f32,
    threshold: f32,
    palette: array<vec4<f32>, 5>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@group(0) @binding(0)
var<uniform> params: VolumeParams;

@group(0) @binding(1)
var<storage, read> field: array<f32>;

// Fullscreen triangle vertex shader
@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    // Fullscreen triangle that covers the screen
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
}

// Sample the field at a world position
fn sample_field(pos: vec3<f32>) -> f32 {
    let extent = params.field_extent;
    let res = params.field_resolution;

    // Check bounds
    if (pos.x < -extent || pos.x > extent ||
        pos.y < -extent || pos.y > extent ||
        pos.z < -extent || pos.z > extent) {
        return 0.0;
    }

    // Map world position to grid coordinates
    let normalized = (pos + vec3<f32>(extent)) / (2.0 * extent);
    let grid_pos = clamp(normalized, vec3<f32>(0.0), vec3<f32>(0.999)) * f32(res);

    // Get cell coordinates
    let cell = vec3<u32>(floor(grid_pos));
    let frac = fract(grid_pos);

    // Trilinear interpolation
    let idx000 = cell.x + cell.y * res + cell.z * res * res;
    let idx100 = min(cell.x + 1u, res - 1u) + cell.y * res + cell.z * res * res;
    let idx010 = cell.x + min(cell.y + 1u, res - 1u) * res + cell.z * res * res;
    let idx110 = min(cell.x + 1u, res - 1u) + min(cell.y + 1u, res - 1u) * res + cell.z * res * res;
    let idx001 = cell.x + cell.y * res + min(cell.z + 1u, res - 1u) * res * res;
    let idx101 = min(cell.x + 1u, res - 1u) + cell.y * res + min(cell.z + 1u, res - 1u) * res * res;
    let idx011 = cell.x + min(cell.y + 1u, res - 1u) * res + min(cell.z + 1u, res - 1u) * res * res;
    let idx111 = min(cell.x + 1u, res - 1u) + min(cell.y + 1u, res - 1u) * res + min(cell.z + 1u, res - 1u) * res * res;

    let v000 = field[idx000];
    let v100 = field[idx100];
    let v010 = field[idx010];
    let v110 = field[idx110];
    let v001 = field[idx001];
    let v101 = field[idx101];
    let v011 = field[idx011];
    let v111 = field[idx111];

    let v00 = mix(v000, v100, frac.x);
    let v10 = mix(v010, v110, frac.x);
    let v01 = mix(v001, v101, frac.x);
    let v11 = mix(v011, v111, frac.x);
    let v0 = mix(v00, v10, frac.y);
    let v1 = mix(v01, v11, frac.y);

    return mix(v0, v1, frac.z);
}

// Sample palette color from normalized value (0-1)
fn sample_palette(t: f32) -> vec3<f32> {
    let tc = clamp(t, 0.0, 1.0);
    let scaled = tc * 4.0;
    let idx = u32(floor(scaled));
    let frac = fract(scaled);

    let c0 = params.palette[min(idx, 4u)].rgb;
    let c1 = params.palette[min(idx + 1u, 4u)].rgb;

    return mix(c0, c1, frac);
}

// Ray-box intersection for AABB
fn intersect_box(ray_origin: vec3<f32>, ray_dir: vec3<f32>, box_min: vec3<f32>, box_max: vec3<f32>) -> vec2<f32> {
    let inv_dir = 1.0 / ray_dir;
    let t1 = (box_min - ray_origin) * inv_dir;
    let t2 = (box_max - ray_origin) * inv_dir;
    let tmin = min(t1, t2);
    let tmax = max(t1, t2);
    let t_enter = max(max(tmin.x, tmin.y), tmin.z);
    let t_exit = min(min(tmax.x, tmax.y), tmax.z);
    return vec2<f32>(max(t_enter, 0.0), t_exit);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Reconstruct ray from screen coordinates
    let ndc = vec4<f32>(in.uv.x * 2.0 - 1.0, (1.0 - in.uv.y) * 2.0 - 1.0, 1.0, 1.0);
    let world_pos = params.inv_view_proj * ndc;
    let ray_target = world_pos.xyz / world_pos.w;
    let ray_origin = params.camera_pos;
    let ray_dir = normalize(ray_target - ray_origin);

    // Intersect ray with field bounding box
    let extent = params.field_extent;
    let box_min = vec3<f32>(-extent);
    let box_max = vec3<f32>(extent);
    let t_range = intersect_box(ray_origin, ray_dir, box_min, box_max);

    // No intersection
    if (t_range.x > t_range.y) {
        return vec4<f32>(0.0);
    }

    // Ray march parameters
    let t_start = t_range.x;
    let t_end = t_range.y;
    let step_size = (t_end - t_start) / f32(params.steps);

    // Accumulate color and opacity
    var accumulated_color = vec3<f32>(0.0);
    var accumulated_alpha = 0.0;
    var t = t_start;

    for (var i = 0u; i < params.steps; i++) {
        if (accumulated_alpha >= 0.99) {
            break;
        }

        let pos = ray_origin + ray_dir * t;
        let density = sample_field(pos);

        if (density > params.threshold) {
            // Map density to color
            let normalized_density = clamp(density * params.density_scale, 0.0, 1.0);
            let color = sample_palette(normalized_density);

            // Accumulate with front-to-back compositing
            let sample_alpha = normalized_density * (1.0 - accumulated_alpha) * 0.5;
            accumulated_color += color * sample_alpha;
            accumulated_alpha += sample_alpha;
        }

        t += step_size;
    }

    return vec4<f32>(accumulated_color, accumulated_alpha);
}
"#;
