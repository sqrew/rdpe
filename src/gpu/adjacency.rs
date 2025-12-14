//! Adjacency buffer for storing neighbor relationships between particles.
//!
//! This module provides a GPU buffer that stores, for each particle, a list of
//! its neighboring particle indices. This enables graph-based operations like:
//! - Information propagation along edges
//! - Constraint-based physics (springs between connected particles)
//! - Network analysis (degree, clustering, etc.)

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

use super::SpatialGpu;

/// Default maximum neighbors per particle.
pub const DEFAULT_MAX_ADJACENCY: u32 = 32;

const WORKGROUP_SIZE: u32 = 256;

/// Parameters for adjacency computation.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct AdjacencyParams {
    num_particles: u32,
    max_neighbors: u32,
    radius: f32,
    _pad: u32,
}

/// GPU resources for adjacency buffer management.
#[allow(dead_code)]
pub struct AdjacencyGpu {
    /// The adjacency buffer.
    /// Layout per particle: [count: u32, neighbor_indices: array<u32, max_neighbors>]
    /// Total stride: (1 + max_neighbors) * 4 bytes
    pub buffer: wgpu::Buffer,
    /// Compute pipeline for populating the adjacency buffer.
    compute_pipeline: wgpu::ComputePipeline,
    /// Bind group for compute shader.
    compute_bind_group: wgpu::BindGroup,
    /// Parameters buffer.
    params_buffer: wgpu::Buffer,
    /// Maximum neighbors per particle.
    pub max_neighbors: u32,
    /// Number of particles.
    num_particles: u32,
    /// Radius for neighbor detection.
    pub radius: f32,
}

impl AdjacencyGpu {
    /// Create a new adjacency buffer system.
    ///
    /// # Arguments
    /// * `device` - The wgpu device
    /// * `particle_buffer` - Buffer containing particle data
    /// * `spatial` - Spatial hashing system for neighbor queries
    /// * `num_particles` - Number of particles
    /// * `max_neighbors` - Maximum neighbors to store per particle
    /// * `radius` - Radius for neighbor detection
    /// * `particle_stride` - Byte stride of each particle in the buffer
    pub fn new(
        device: &wgpu::Device,
        particle_buffer: &wgpu::Buffer,
        spatial: &SpatialGpu,
        num_particles: u32,
        max_neighbors: u32,
        radius: f32,
        particle_stride: usize,
    ) -> Self {
        // Each particle stores: count + max_neighbors indices
        let stride_u32 = 1 + max_neighbors;
        let buffer_size = (num_particles as usize) * (stride_u32 as usize) * std::mem::size_of::<u32>();

        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Adjacency Buffer"),
            size: buffer_size as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let params = AdjacencyParams {
            num_particles,
            max_neighbors,
            radius,
            _pad: 0,
        };

        let params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Adjacency Params Buffer"),
            contents: bytemuck::bytes_of(&params),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        let (compute_pipeline, compute_bind_group) = create_compute_pipeline(
            device,
            particle_buffer,
            &buffer,
            &params_buffer,
            spatial,
            particle_stride,
            max_neighbors,
        );

        Self {
            buffer,
            compute_pipeline,
            compute_bind_group,
            params_buffer,
            max_neighbors,
            num_particles,
            radius,
        }
    }

    /// Execute the adjacency computation pass.
    ///
    /// This should be called after spatial hashing is executed but before
    /// the main particle compute shader.
    pub fn execute(&self, encoder: &mut wgpu::CommandEncoder) {
        let workgroups = self.num_particles.div_ceil(WORKGROUP_SIZE);

        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Compute Adjacency"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&self.compute_pipeline);
        pass.set_bind_group(0, &self.compute_bind_group, &[]);
        pass.dispatch_workgroups(workgroups, 1, 1);
    }

    /// Get the stride (in u32s) for each particle's adjacency data.
    pub fn stride_u32(&self) -> u32 {
        1 + self.max_neighbors
    }
}

fn create_compute_pipeline(
    device: &wgpu::Device,
    particle_buffer: &wgpu::Buffer,
    adjacency_buffer: &wgpu::Buffer,
    params_buffer: &wgpu::Buffer,
    spatial: &SpatialGpu,
    particle_stride: usize,
    max_neighbors: u32,
) -> (wgpu::ComputePipeline, wgpu::BindGroup) {
    let shader_src = generate_compute_shader(particle_stride, max_neighbors);
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Adjacency Compute Shader"),
        source: wgpu::ShaderSource::Wgsl(shader_src.into()),
    });

    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Adjacency Compute Bind Group Layout"),
        entries: &[
            // particles (read)
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
            // adjacency (write)
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
            // params
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
            // sorted_indices
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
            // cell_start
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
            // cell_end
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
            // spatial params
            wgpu::BindGroupLayoutEntry {
                binding: 6,
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
        label: Some("Adjacency Compute Bind Group"),
        layout: &bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: particle_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: adjacency_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: params_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: spatial.particle_indices_a.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 4,
                resource: spatial.cell_start.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 5,
                resource: spatial.cell_end.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 6,
                resource: spatial.spatial_params_buffer.as_entire_binding(),
            },
        ],
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Adjacency Compute Pipeline Layout"),
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });

    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("Adjacency Compute Pipeline"),
        layout: Some(&pipeline_layout),
        module: &shader,
        entry_point: Some("main"),
        compilation_options: Default::default(),
        cache: None,
    });

    (pipeline, bind_group)
}

fn generate_compute_shader(particle_stride: usize, max_neighbors: u32) -> String {
    let particle_stride_vec4 = particle_stride / 16;
    let adjacency_stride = 1 + max_neighbors;

    format!(
        r#"
struct AdjacencyParams {{
    num_particles: u32,
    max_neighbors: u32,
    radius: f32,
}};

struct SpatialParams {{
    cell_size: f32,
    grid_resolution: u32,
    num_particles: u32,
    max_neighbors: u32,
}};

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

@group(0) @binding(0) var<storage, read> particles: array<vec4<f32>>;
@group(0) @binding(1) var<storage, read_write> adjacency: array<u32>;
@group(0) @binding(2) var<uniform> params: AdjacencyParams;
@group(0) @binding(3) var<storage, read> sorted_indices: array<u32>;
@group(0) @binding(4) var<storage, read> cell_start: array<u32>;
@group(0) @binding(5) var<storage, read> cell_end: array<u32>;
@group(0) @binding(6) var<uniform> spatial: SpatialParams;

const ADJACENCY_STRIDE: u32 = {adjacency_stride}u;
const MAX_NEIGHBORS: u32 = {max_neighbors}u;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {{
    let idx = global_id.x;
    if idx >= params.num_particles {{
        return;
    }}

    let my_pos = particles[idx * {particle_stride_vec4}u].xyz;
    let my_cell = pos_to_cell(my_pos, spatial.cell_size, spatial.grid_resolution);
    let radius_sq = params.radius * params.radius;

    // Base offset in adjacency buffer for this particle
    let adj_base = idx * ADJACENCY_STRIDE;
    var neighbor_count = 0u;

    // Iterate through 27 neighboring cells
    for (var dx = -1; dx <= 1; dx++) {{
        for (var dy = -1; dy <= 1; dy++) {{
            for (var dz = -1; dz <= 1; dz++) {{
                if neighbor_count >= MAX_NEIGHBORS {{
                    break;
                }}

                let neighbor_cell = my_cell + vec3<i32>(dx, dy, dz);

                // Bounds check
                if neighbor_cell.x < 0 || neighbor_cell.x >= i32(spatial.grid_resolution) ||
                   neighbor_cell.y < 0 || neighbor_cell.y >= i32(spatial.grid_resolution) ||
                   neighbor_cell.z < 0 || neighbor_cell.z >= i32(spatial.grid_resolution) {{
                    continue;
                }}

                let morton = morton_encode_10bit(u32(neighbor_cell.x), u32(neighbor_cell.y), u32(neighbor_cell.z));
                let start = cell_start[morton];
                let end = cell_end[morton];

                // Skip empty cells
                if start == 0xFFFFFFFFu {{
                    continue;
                }}

                // Iterate particles in this cell
                for (var j = start; j < end; j++) {{
                    if neighbor_count >= MAX_NEIGHBORS {{
                        break;
                    }}

                    let other_idx = sorted_indices[j];

                    // Skip self
                    if other_idx == idx {{
                        continue;
                    }}

                    let other_pos = particles[other_idx * {particle_stride_vec4}u].xyz;
                    let diff = other_pos - my_pos;
                    let dist_sq = dot(diff, diff);

                    // Within radius?
                    if dist_sq < radius_sq && dist_sq > 0.0001 {{
                        // Store neighbor index (offset by 1 for count slot)
                        adjacency[adj_base + 1u + neighbor_count] = other_idx;
                        neighbor_count += 1u;
                    }}
                }}
            }}
        }}
    }}

    // Store neighbor count at the start
    adjacency[adj_base] = neighbor_count;
}}
"#
    )
}

/// WGSL code for adjacency buffer access utilities.
///
/// Provides helper functions for querying the adjacency buffer in compute shaders.
pub fn adjacency_wgsl(max_neighbors: u32) -> String {
    let adjacency_stride = 1 + max_neighbors;
    format!(
        r#"
// Adjacency buffer layout constants
const ADJACENCY_STRIDE: u32 = {adjacency_stride}u;
const ADJACENCY_MAX_NEIGHBORS: u32 = {max_neighbors}u;

// Get the number of neighbors for a particle
fn adjacency_count(particle_idx: u32) -> u32 {{
    return adjacency[particle_idx * ADJACENCY_STRIDE];
}}

// Get the index of the nth neighbor of a particle
fn adjacency_neighbor(particle_idx: u32, neighbor_n: u32) -> u32 {{
    return adjacency[particle_idx * ADJACENCY_STRIDE + 1u + neighbor_n];
}}
"#
    )
}
