//! GPU spatial hashing infrastructure
//!
//! Handles Morton code computation, radix sort, and cell table building.

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

use crate::spatial::{SpatialConfig, MORTON_WGSL};

const WORKGROUP_SIZE: u32 = 256;
const RADIX_BITS: u32 = 4;
const RADIX_SIZE: u32 = 16; // 2^4

/// Calculate number of sort passes needed based on grid resolution.
/// Morton codes use 3 * log2(grid_resolution) bits.
/// Always returns an even number so final result is in buffer A (for build_cells bind group).
fn calculate_sort_passes(grid_resolution: u32) -> u32 {
    let bits_per_axis = (grid_resolution as f32).log2().ceil() as u32;
    let total_bits = bits_per_axis * 3; // Morton code interleaves 3 axes
    let passes = total_bits.div_ceil(RADIX_BITS);
    // Round up to even number so result ends in buffer A
    if passes % 2 == 1 { passes + 1 } else { passes }
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct SpatialParams {
    pub cell_size: f32,
    pub grid_resolution: u32,
    pub num_particles: u32,
    pub max_neighbors: u32,
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct SortParams {
    num_elements: u32,
    bit_offset: u32,
    _pad0: u32,
    _pad1: u32,
}

/// GPU resources for spatial hashing
#[allow(dead_code)] // Fields used indirectly via bind groups
pub struct SpatialGpu {
    // Buffers
    morton_codes_a: wgpu::Buffer,
    morton_codes_b: wgpu::Buffer,
    pub particle_indices_a: wgpu::Buffer,
    particle_indices_b: wgpu::Buffer,
    histogram: wgpu::Buffer,
    pub cell_start: wgpu::Buffer,
    pub cell_end: wgpu::Buffer,
    pub spatial_params_buffer: wgpu::Buffer,
    pub sort_params_buffer: wgpu::Buffer,

    // Pipelines
    compute_morton_pipeline: wgpu::ComputePipeline,
    histogram_pipeline: wgpu::ComputePipeline,
    prefix_sum_pipeline: wgpu::ComputePipeline,
    scatter_pipeline: wgpu::ComputePipeline,
    build_cells_pipeline: wgpu::ComputePipeline,
    clear_histogram_pipeline: wgpu::ComputePipeline,
    clear_cells_pipeline: wgpu::ComputePipeline,

    // Bind groups (we'll need to swap for ping-pong)
    morton_bind_group: wgpu::BindGroup,
    histogram_bind_group_a: wgpu::BindGroup,
    histogram_bind_group_b: wgpu::BindGroup,
    prefix_sum_bind_group: wgpu::BindGroup,
    scatter_bind_group_a_to_b: wgpu::BindGroup,
    scatter_bind_group_b_to_a: wgpu::BindGroup,
    build_cells_bind_group: wgpu::BindGroup,
    clear_histogram_bind_group: wgpu::BindGroup,
    clear_cells_bind_group: wgpu::BindGroup,

    pub config: SpatialConfig,
    num_particles: u32,
    sort_passes: u32,
}

impl SpatialGpu {
    pub fn new(
        device: &wgpu::Device,
        particle_buffer: &wgpu::Buffer,
        num_particles: u32,
        config: SpatialConfig,
        particle_wgsl_struct: &str,
    ) -> Self {
        // Create buffers
        let buffer_size = (num_particles as usize * std::mem::size_of::<u32>()) as u64;

        let morton_codes_a = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Morton Codes A"),
            size: buffer_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let morton_codes_b = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Morton Codes B"),
            size: buffer_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let particle_indices_a = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Particle Indices A"),
            size: buffer_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let particle_indices_b = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Particle Indices B"),
            size: buffer_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let histogram = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Radix Histogram"),
            size: (RADIX_SIZE as usize * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let total_cells = config.total_cells();
        let cell_table_size = (total_cells as usize * std::mem::size_of::<u32>()) as u64;

        let cell_start = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Cell Start"),
            size: cell_table_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let cell_end = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Cell End"),
            size: cell_table_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let spatial_params = SpatialParams {
            cell_size: config.cell_size,
            grid_resolution: config.grid_resolution,
            num_particles,
            max_neighbors: config.max_neighbors,
        };

        let spatial_params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Spatial Params"),
            contents: bytemuck::cast_slice(&[spatial_params]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let sort_params_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Sort Params"),
            size: std::mem::size_of::<SortParams>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create shaders and pipelines
        let (
            compute_morton_pipeline,
            histogram_pipeline,
            prefix_sum_pipeline,
            scatter_pipeline,
            build_cells_pipeline,
            clear_histogram_pipeline,
            clear_cells_pipeline,
        ) = create_pipelines(device, particle_wgsl_struct);

        // Create bind groups
        let morton_bind_group = create_morton_bind_group(
            device,
            &compute_morton_pipeline,
            particle_buffer,
            &morton_codes_a,
            &particle_indices_a,
            &spatial_params_buffer,
        );

        let histogram_bind_group_a = create_histogram_bind_group(
            device,
            &histogram_pipeline,
            &morton_codes_a,
            &histogram,
            &sort_params_buffer,
        );

        let histogram_bind_group_b = create_histogram_bind_group(
            device,
            &histogram_pipeline,
            &morton_codes_b,
            &histogram,
            &sort_params_buffer,
        );

        let prefix_sum_bind_group = create_prefix_sum_bind_group(
            device,
            &prefix_sum_pipeline,
            &histogram,
        );

        let scatter_bind_group_a_to_b = create_scatter_bind_group(
            device,
            &scatter_pipeline,
            &morton_codes_a,
            &particle_indices_a,
            &morton_codes_b,
            &particle_indices_b,
            &histogram,
            &sort_params_buffer,
        );

        let scatter_bind_group_b_to_a = create_scatter_bind_group(
            device,
            &scatter_pipeline,
            &morton_codes_b,
            &particle_indices_b,
            &morton_codes_a,
            &particle_indices_a,
            &histogram,
            &sort_params_buffer,
        );

        let build_cells_bind_group = create_build_cells_bind_group(
            device,
            &build_cells_pipeline,
            &morton_codes_a, // After even number of passes, result is in A
            &cell_start,
            &cell_end,
            &spatial_params_buffer,
        );

        let clear_histogram_bind_group = create_clear_bind_group(
            device,
            &clear_histogram_pipeline,
            &histogram,
            RADIX_SIZE,
        );

        let clear_cells_bind_group = create_clear_bind_group(
            device,
            &clear_cells_pipeline,
            &cell_start,
            total_cells,
        );

        let sort_passes = calculate_sort_passes(config.grid_resolution);

        Self {
            morton_codes_a,
            morton_codes_b,
            particle_indices_a,
            particle_indices_b,
            histogram,
            cell_start,
            cell_end,
            spatial_params_buffer,
            sort_params_buffer,
            compute_morton_pipeline,
            histogram_pipeline,
            prefix_sum_pipeline,
            scatter_pipeline,
            build_cells_pipeline,
            clear_histogram_pipeline,
            clear_cells_pipeline,
            morton_bind_group,
            histogram_bind_group_a,
            histogram_bind_group_b,
            prefix_sum_bind_group,
            scatter_bind_group_a_to_b,
            scatter_bind_group_b_to_a,
            build_cells_bind_group,
            clear_histogram_bind_group,
            clear_cells_bind_group,
            config,
            num_particles,
            sort_passes,
        }
    }

    /// Execute spatial hashing passes
    pub fn execute(&self, encoder: &mut wgpu::CommandEncoder, queue: &wgpu::Queue) {
        let workgroups = self.num_particles.div_ceil(WORKGROUP_SIZE);

        // Step 1: Compute Morton codes
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Compute Morton"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.compute_morton_pipeline);
            pass.set_bind_group(0, &self.morton_bind_group, &[]);
            pass.dispatch_workgroups(workgroups, 1, 1);
        }

        // Step 2: Radix sort (dynamic passes based on grid resolution)
        let mut source_is_a = true;

        for pass_idx in 0..self.sort_passes {
            let bit_offset = pass_idx * RADIX_BITS;

            // Update sort params
            let sort_params = SortParams {
                num_elements: self.num_particles,
                bit_offset,
                _pad0: 0,
                _pad1: 0,
            };
            queue.write_buffer(&self.sort_params_buffer, 0, bytemuck::cast_slice(&[sort_params]));

            // Clear histogram
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("Clear Histogram"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&self.clear_histogram_pipeline);
                pass.set_bind_group(0, &self.clear_histogram_bind_group, &[]);
                pass.dispatch_workgroups(1, 1, 1);
            }

            // Histogram pass
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("Radix Histogram"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&self.histogram_pipeline);
                pass.set_bind_group(
                    0,
                    if source_is_a { &self.histogram_bind_group_a } else { &self.histogram_bind_group_b },
                    &[],
                );
                pass.dispatch_workgroups(workgroups, 1, 1);
            }

            // Prefix sum
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("Prefix Sum"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&self.prefix_sum_pipeline);
                pass.set_bind_group(0, &self.prefix_sum_bind_group, &[]);
                pass.dispatch_workgroups(1, 1, 1);
            }

            // Scatter pass
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("Radix Scatter"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&self.scatter_pipeline);
                pass.set_bind_group(
                    0,
                    if source_is_a { &self.scatter_bind_group_a_to_b } else { &self.scatter_bind_group_b_to_a },
                    &[],
                );
                pass.dispatch_workgroups(workgroups, 1, 1);
            }

            source_is_a = !source_is_a;
        }

        // Step 3: Build cell table
        // Clear cell tables first
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Clear Cells"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.clear_cells_pipeline);
            pass.set_bind_group(0, &self.clear_cells_bind_group, &[]);
            let cell_workgroups = self.config.total_cells().div_ceil(WORKGROUP_SIZE);
            pass.dispatch_workgroups(cell_workgroups, 1, 1);
        }

        // Build cell start/end
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Build Cell Table"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.build_cells_pipeline);
            pass.set_bind_group(0, &self.build_cells_bind_group, &[]);
            pass.dispatch_workgroups(workgroups, 1, 1);
        }
    }
}

fn create_pipelines(
    device: &wgpu::Device,
    particle_wgsl_struct: &str,
) -> (
    wgpu::ComputePipeline,
    wgpu::ComputePipeline,
    wgpu::ComputePipeline,
    wgpu::ComputePipeline,
    wgpu::ComputePipeline,
    wgpu::ComputePipeline,
    wgpu::ComputePipeline,
) {
    // Morton code computation shader - uses actual particle struct for correct stride
    let morton_shader_src = format!(
        r#"{}

{}

struct SpatialParams {{
    cell_size: f32,
    grid_resolution: u32,
    num_particles: u32,
    max_neighbors: u32,
}};

@group(0) @binding(0) var<storage, read> particles: array<Particle>;
@group(0) @binding(1) var<storage, read_write> morton_codes: array<u32>;
@group(0) @binding(2) var<storage, read_write> particle_indices: array<u32>;
@group(0) @binding(3) var<uniform> params: SpatialParams;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {{
    let idx = global_id.x;
    if idx >= params.num_particles {{
        return;
    }}

    let pos = particles[idx].position;
    morton_codes[idx] = pos_to_morton(pos, params.cell_size, params.grid_resolution);
    particle_indices[idx] = idx;
}}
"#,
        MORTON_WGSL,
        particle_wgsl_struct
    );

    let morton_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Morton Shader"),
        source: wgpu::ShaderSource::Wgsl(morton_shader_src.into()),
    });

    // Histogram shader
    let histogram_shader_src = r#"
struct SortParams {
    num_elements: u32,
    bit_offset: u32,
    _pad0: u32,
    _pad1: u32,
};

@group(0) @binding(0) var<storage, read> keys: array<u32>;
@group(0) @binding(1) var<storage, read_write> histogram: array<atomic<u32>>;
@group(0) @binding(2) var<uniform> params: SortParams;

const RADIX_SIZE: u32 = 16u;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    if idx >= params.num_elements {
        return;
    }

    let key = keys[idx];
    let digit = (key >> params.bit_offset) & (RADIX_SIZE - 1u);
    atomicAdd(&histogram[digit], 1u);
}
"#;

    let histogram_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Histogram Shader"),
        source: wgpu::ShaderSource::Wgsl(histogram_shader_src.into()),
    });

    // Prefix sum shader
    let prefix_sum_shader_src = r#"
@group(0) @binding(0) var<storage, read_write> data: array<u32>;

var<workgroup> temp: array<u32, 16>;

@compute @workgroup_size(16)
fn main(@builtin(local_invocation_id) local_id: vec3<u32>) {
    let tid = local_id.x;

    // Load into shared memory
    temp[tid] = data[tid];
    workgroupBarrier();

    // Inclusive scan using up-sweep and down-sweep
    // Up-sweep
    for (var stride = 1u; stride < 16u; stride *= 2u) {
        if tid >= stride {
            temp[tid] += temp[tid - stride];
        }
        workgroupBarrier();
    }

    // Convert to exclusive scan
    let inclusive = temp[tid];
    workgroupBarrier();

    if tid == 0u {
        data[tid] = 0u;
    } else {
        data[tid] = temp[tid - 1u];
    }
}
"#;

    let prefix_sum_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Prefix Sum Shader"),
        source: wgpu::ShaderSource::Wgsl(prefix_sum_shader_src.into()),
    });

    // Scatter shader
    let scatter_shader_src = r#"
struct SortParams {
    num_elements: u32,
    bit_offset: u32,
    _pad0: u32,
    _pad1: u32,
};

@group(0) @binding(0) var<storage, read> keys_in: array<u32>;
@group(0) @binding(1) var<storage, read> vals_in: array<u32>;
@group(0) @binding(2) var<storage, read_write> keys_out: array<u32>;
@group(0) @binding(3) var<storage, read_write> vals_out: array<u32>;
@group(0) @binding(4) var<storage, read_write> offsets: array<atomic<u32>>;
@group(0) @binding(5) var<uniform> params: SortParams;

const RADIX_SIZE: u32 = 16u;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    if idx >= params.num_elements {
        return;
    }

    let key = keys_in[idx];
    let val = vals_in[idx];
    let digit = (key >> params.bit_offset) & (RADIX_SIZE - 1u);

    let dest = atomicAdd(&offsets[digit], 1u);

    keys_out[dest] = key;
    vals_out[dest] = val;
}
"#;

    let scatter_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Scatter Shader"),
        source: wgpu::ShaderSource::Wgsl(scatter_shader_src.into()),
    });

    // Build cells shader
    let build_cells_shader_src = r#"
struct SpatialParams {
    cell_size: f32,
    grid_resolution: u32,
    num_particles: u32,
    max_neighbors: u32,
};

@group(0) @binding(0) var<storage, read> sorted_morton: array<u32>;
@group(0) @binding(1) var<storage, read_write> cell_start: array<u32>;
@group(0) @binding(2) var<storage, read_write> cell_end: array<u32>;
@group(0) @binding(3) var<uniform> params: SpatialParams;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    if idx >= params.num_particles {
        return;
    }

    let code = sorted_morton[idx];

    if idx == 0u {
        cell_start[code] = 0u;
    } else {
        let prev_code = sorted_morton[idx - 1u];
        if code != prev_code {
            cell_start[code] = idx;
            cell_end[prev_code] = idx;
        }
    }

    if idx == params.num_particles - 1u {
        cell_end[code] = params.num_particles;
    }
}
"#;

    let build_cells_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Build Cells Shader"),
        source: wgpu::ShaderSource::Wgsl(build_cells_shader_src.into()),
    });

    // Clear buffer shader
    let clear_shader_src = r#"
@group(0) @binding(0) var<storage, read_write> data: array<u32>;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    if idx < arrayLength(&data) {
        data[idx] = 0xFFFFFFFFu;
    }
}
"#;

    let clear_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Clear Shader"),
        source: wgpu::ShaderSource::Wgsl(clear_shader_src.into()),
    });

    // Create pipeline layouts and pipelines
    let morton_pipeline = create_compute_pipeline(device, &morton_shader, "main", "Morton Pipeline");
    let histogram_pipeline = create_compute_pipeline(device, &histogram_shader, "main", "Histogram Pipeline");
    let prefix_sum_pipeline = create_compute_pipeline(device, &prefix_sum_shader, "main", "Prefix Sum Pipeline");
    let scatter_pipeline = create_compute_pipeline(device, &scatter_shader, "main", "Scatter Pipeline");
    let build_cells_pipeline = create_compute_pipeline(device, &build_cells_shader, "main", "Build Cells Pipeline");
    let clear_histogram_pipeline = create_compute_pipeline(device, &clear_shader, "main", "Clear Histogram Pipeline");
    let clear_cells_pipeline = create_compute_pipeline(device, &clear_shader, "main", "Clear Cells Pipeline");

    (
        morton_pipeline,
        histogram_pipeline,
        prefix_sum_pipeline,
        scatter_pipeline,
        build_cells_pipeline,
        clear_histogram_pipeline,
        clear_cells_pipeline,
    )
}

fn create_compute_pipeline(
    device: &wgpu::Device,
    shader: &wgpu::ShaderModule,
    entry_point: &str,
    label: &str,
) -> wgpu::ComputePipeline {
    device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some(label),
        layout: None, // Auto layout
        module: shader,
        entry_point: Some(entry_point),
        compilation_options: Default::default(),
        cache: None,
    })
}

fn create_morton_bind_group(
    device: &wgpu::Device,
    pipeline: &wgpu::ComputePipeline,
    particles: &wgpu::Buffer,
    morton_codes: &wgpu::Buffer,
    particle_indices: &wgpu::Buffer,
    params: &wgpu::Buffer,
) -> wgpu::BindGroup {
    let layout = pipeline.get_bind_group_layout(0);
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Morton Bind Group"),
        layout: &layout,
        entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: particles.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 1, resource: morton_codes.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 2, resource: particle_indices.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 3, resource: params.as_entire_binding() },
        ],
    })
}

fn create_histogram_bind_group(
    device: &wgpu::Device,
    pipeline: &wgpu::ComputePipeline,
    keys: &wgpu::Buffer,
    histogram: &wgpu::Buffer,
    params: &wgpu::Buffer,
) -> wgpu::BindGroup {
    let layout = pipeline.get_bind_group_layout(0);
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Histogram Bind Group"),
        layout: &layout,
        entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: keys.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 1, resource: histogram.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 2, resource: params.as_entire_binding() },
        ],
    })
}

fn create_prefix_sum_bind_group(
    device: &wgpu::Device,
    pipeline: &wgpu::ComputePipeline,
    data: &wgpu::Buffer,
) -> wgpu::BindGroup {
    let layout = pipeline.get_bind_group_layout(0);
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Prefix Sum Bind Group"),
        layout: &layout,
        entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: data.as_entire_binding() },
        ],
    })
}

#[allow(clippy::too_many_arguments)]
fn create_scatter_bind_group(
    device: &wgpu::Device,
    pipeline: &wgpu::ComputePipeline,
    keys_in: &wgpu::Buffer,
    vals_in: &wgpu::Buffer,
    keys_out: &wgpu::Buffer,
    vals_out: &wgpu::Buffer,
    offsets: &wgpu::Buffer,
    params: &wgpu::Buffer,
) -> wgpu::BindGroup {
    let layout = pipeline.get_bind_group_layout(0);
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Scatter Bind Group"),
        layout: &layout,
        entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: keys_in.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 1, resource: vals_in.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 2, resource: keys_out.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 3, resource: vals_out.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 4, resource: offsets.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 5, resource: params.as_entire_binding() },
        ],
    })
}

fn create_build_cells_bind_group(
    device: &wgpu::Device,
    pipeline: &wgpu::ComputePipeline,
    sorted_morton: &wgpu::Buffer,
    cell_start: &wgpu::Buffer,
    cell_end: &wgpu::Buffer,
    params: &wgpu::Buffer,
) -> wgpu::BindGroup {
    let layout = pipeline.get_bind_group_layout(0);
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Build Cells Bind Group"),
        layout: &layout,
        entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: sorted_morton.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 1, resource: cell_start.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 2, resource: cell_end.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 3, resource: params.as_entire_binding() },
        ],
    })
}

fn create_clear_bind_group(
    device: &wgpu::Device,
    pipeline: &wgpu::ComputePipeline,
    buffer: &wgpu::Buffer,
    _count: u32,
) -> wgpu::BindGroup {
    let layout = pipeline.get_bind_group_layout(0);
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Clear Bind Group"),
        layout: &layout,
        entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: buffer.as_entire_binding() },
        ],
    })
}
