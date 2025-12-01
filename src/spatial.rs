//! Spatial hashing infrastructure using Morton encoding (Z-order curve)
//!
//! This module provides GPU-accelerated spatial hashing for efficient
//! neighbor queries in particle simulations.

/// Configuration for spatial hashing grid
#[derive(Clone, Copy, Debug)]
pub struct SpatialConfig {
    /// Size of each cell in world units
    pub cell_size: f32,
    /// Number of cells per dimension (grid is cell_count^3)
    pub grid_resolution: u32,
}

impl Default for SpatialConfig {
    fn default() -> Self {
        Self {
            cell_size: 0.1,
            grid_resolution: 64, // 64^3 = 262144 cells, fits in 18-bit Morton code
        }
    }
}

impl SpatialConfig {
    pub fn new(cell_size: f32, grid_resolution: u32) -> Self {
        assert!(grid_resolution.is_power_of_two(), "Grid resolution must be power of 2");
        assert!(grid_resolution <= 1024, "Grid resolution must be <= 1024 for 30-bit Morton codes");
        Self { cell_size, grid_resolution }
    }

    /// Total number of cells in the grid
    pub fn total_cells(&self) -> u32 {
        self.grid_resolution * self.grid_resolution * self.grid_resolution
    }
}

/// WGSL code for Morton encoding utilities
pub const MORTON_WGSL: &str = r#"
// Expand 10-bit integer to 30 bits by inserting 2 zeros between each bit
fn expand_bits(v: u32) -> u32 {
    var x = v & 0x000003FFu; // 10 bits
    x = (x | (x << 16u)) & 0x030000FFu;
    x = (x | (x <<  8u)) & 0x0300F00Fu;
    x = (x | (x <<  4u)) & 0x030C30C3u;
    x = (x | (x <<  2u)) & 0x09249249u;
    return x;
}

// Compute 30-bit Morton code for 3D point (each coord 0-1023)
fn morton_encode(x: u32, y: u32, z: u32) -> u32 {
    return expand_bits(x) | (expand_bits(y) << 1u) | (expand_bits(z) << 2u);
}

// Convert world position to cell coordinates
fn pos_to_cell(pos: vec3<f32>, cell_size: f32, grid_res: u32) -> vec3<u32> {
    // Offset by half grid to center around origin
    let half_grid = f32(grid_res) * cell_size * 0.5;
    let normalized = (pos + vec3<f32>(half_grid)) / cell_size;
    let clamped = clamp(normalized, vec3<f32>(0.0), vec3<f32>(f32(grid_res - 1u)));
    return vec3<u32>(clamped);
}

// Get Morton code for a world position
fn pos_to_morton(pos: vec3<f32>, cell_size: f32, grid_res: u32) -> u32 {
    let cell = pos_to_cell(pos, cell_size, grid_res);
    return morton_encode(cell.x, cell.y, cell.z);
}

// Compact 30 bits to 10 bits by extracting every third bit
fn compact_bits(v: u32) -> u32 {
    var x = v & 0x09249249u;
    x = (x | (x >>  2u)) & 0x030C30C3u;
    x = (x | (x >>  4u)) & 0x0300F00Fu;
    x = (x | (x >>  8u)) & 0x030000FFu;
    x = (x | (x >> 16u)) & 0x000003FFu;
    return x;
}

// Decode Morton code back to cell coordinates
fn morton_decode(code: u32) -> vec3<u32> {
    return vec3<u32>(
        compact_bits(code),
        compact_bits(code >> 1u),
        compact_bits(code >> 2u)
    );
}
"#;

/// WGSL code for computing Morton codes for all particles
pub const COMPUTE_MORTON_WGSL: &str = r#"
struct Particle {
    position: vec3<f32>,
    // ... rest of particle fields handled by generated code
};

struct SpatialParams {
    cell_size: f32,
    grid_resolution: u32,
    num_particles: u32,
    _pad: u32,
};

@group(0) @binding(0) var<storage, read> particles: array<Particle>;
@group(0) @binding(1) var<storage, read_write> morton_codes: array<u32>;
@group(0) @binding(2) var<storage, read_write> particle_indices: array<u32>;
@group(0) @binding(3) var<uniform> params: SpatialParams;

@compute @workgroup_size(256)
fn compute_morton(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    if idx >= params.num_particles {
        return;
    }

    let pos = particles[idx].position;
    morton_codes[idx] = pos_to_morton(pos, params.cell_size, params.grid_resolution);
    particle_indices[idx] = idx;
}
"#;

/// WGSL code for radix sort histogram pass
pub const RADIX_HISTOGRAM_WGSL: &str = r#"
struct SortParams {
    num_elements: u32,
    bit_offset: u32,
    _pad0: u32,
    _pad1: u32,
};

@group(0) @binding(0) var<storage, read> keys: array<u32>;
@group(0) @binding(1) var<storage, read_write> histogram: array<atomic<u32>>;
@group(0) @binding(2) var<uniform> params: SortParams;

const RADIX_BITS: u32 = 4u;
const RADIX_SIZE: u32 = 16u; // 2^4

@compute @workgroup_size(256)
fn radix_histogram(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    if idx >= params.num_elements {
        return;
    }

    let key = keys[idx];
    let digit = (key >> params.bit_offset) & (RADIX_SIZE - 1u);
    atomicAdd(&histogram[digit], 1u);
}
"#;

/// WGSL code for prefix sum (scan) - simple single-pass for small arrays
pub const PREFIX_SUM_WGSL: &str = r#"
@group(0) @binding(0) var<storage, read_write> data: array<u32>;
@group(0) @binding(1) var<uniform> count: u32;

var<workgroup> temp: array<u32, 256>;

@compute @workgroup_size(256)
fn prefix_sum(
    @builtin(local_invocation_id) local_id: vec3<u32>,
    @builtin(workgroup_id) group_id: vec3<u32>
) {
    let tid = local_id.x;

    // Load into shared memory
    if tid < count {
        temp[tid] = data[tid];
    } else {
        temp[tid] = 0u;
    }
    workgroupBarrier();

    // Up-sweep (reduce)
    for (var stride = 1u; stride < 256u; stride *= 2u) {
        let idx = (tid + 1u) * stride * 2u - 1u;
        if idx < 256u {
            temp[idx] += temp[idx - stride];
        }
        workgroupBarrier();
    }

    // Set last element to 0 for exclusive scan
    if tid == 0u {
        temp[255] = 0u;
    }
    workgroupBarrier();

    // Down-sweep
    for (var stride = 128u; stride > 0u; stride /= 2u) {
        let idx = (tid + 1u) * stride * 2u - 1u;
        if idx < 256u {
            let t = temp[idx - stride];
            temp[idx - stride] = temp[idx];
            temp[idx] += t;
        }
        workgroupBarrier();
    }

    // Write back
    if tid < count {
        data[tid] = temp[tid];
    }
}
"#;

/// WGSL code for radix sort scatter pass
pub const RADIX_SCATTER_WGSL: &str = r#"
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
@group(0) @binding(4) var<storage, read_write> histogram: array<atomic<u32>>;
@group(0) @binding(5) var<uniform> params: SortParams;

const RADIX_BITS: u32 = 4u;
const RADIX_SIZE: u32 = 16u;

@compute @workgroup_size(256)
fn radix_scatter(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    if idx >= params.num_elements {
        return;
    }

    let key = keys_in[idx];
    let val = vals_in[idx];
    let digit = (key >> params.bit_offset) & (RADIX_SIZE - 1u);

    // Get destination index via atomic increment
    let dest = atomicAdd(&histogram[digit], 1u);

    keys_out[dest] = key;
    vals_out[dest] = val;
}
"#;

/// WGSL code for building cell offset table
pub const BUILD_CELL_TABLE_WGSL: &str = r#"
struct SpatialParams {
    cell_size: f32,
    grid_resolution: u32,
    num_particles: u32,
    _pad: u32,
};

@group(0) @binding(0) var<storage, read> sorted_morton: array<u32>;
@group(0) @binding(1) var<storage, read_write> cell_start: array<u32>;
@group(0) @binding(2) var<storage, read_write> cell_end: array<u32>;
@group(0) @binding(3) var<uniform> params: SpatialParams;

@compute @workgroup_size(256)
fn build_cell_table(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    if idx >= params.num_particles {
        return;
    }

    let code = sorted_morton[idx];

    // Check if this is the start of a new cell
    if idx == 0u {
        cell_start[code] = 0u;
    } else {
        let prev_code = sorted_morton[idx - 1u];
        if code != prev_code {
            cell_start[code] = idx;
            cell_end[prev_code] = idx;
        }
    }

    // Handle last particle
    if idx == params.num_particles - 1u {
        cell_end[code] = params.num_particles;
    }
}
"#;

/// WGSL code for neighbor iteration utilities
pub const NEIGHBOR_UTILS_WGSL: &str = r#"
// Offsets for 27 neighboring cells (including self)
const NEIGHBOR_OFFSETS: array<vec3<i32>, 27> = array<vec3<i32>, 27>(
    vec3<i32>(-1, -1, -1), vec3<i32>(0, -1, -1), vec3<i32>(1, -1, -1),
    vec3<i32>(-1,  0, -1), vec3<i32>(0,  0, -1), vec3<i32>(1,  0, -1),
    vec3<i32>(-1,  1, -1), vec3<i32>(0,  1, -1), vec3<i32>(1,  1, -1),
    vec3<i32>(-1, -1,  0), vec3<i32>(0, -1,  0), vec3<i32>(1, -1,  0),
    vec3<i32>(-1,  0,  0), vec3<i32>(0,  0,  0), vec3<i32>(1,  0,  0),
    vec3<i32>(-1,  1,  0), vec3<i32>(0,  1,  0), vec3<i32>(1,  1,  0),
    vec3<i32>(-1, -1,  1), vec3<i32>(0, -1,  1), vec3<i32>(1, -1,  1),
    vec3<i32>(-1,  0,  1), vec3<i32>(0,  0,  1), vec3<i32>(1,  0,  1),
    vec3<i32>(-1,  1,  1), vec3<i32>(0,  1,  1), vec3<i32>(1,  1,  1),
);

// Get Morton code for a neighboring cell (returns 0xFFFFFFFF if out of bounds)
fn neighbor_cell_morton(cell: vec3<u32>, offset_idx: u32, grid_res: u32) -> u32 {
    let offset = NEIGHBOR_OFFSETS[offset_idx];
    let neighbor = vec3<i32>(cell) + offset;

    if neighbor.x < 0 || neighbor.y < 0 || neighbor.z < 0 ||
       neighbor.x >= i32(grid_res) || neighbor.y >= i32(grid_res) || neighbor.z >= i32(grid_res) {
        return 0xFFFFFFFFu; // Invalid marker
    }

    return morton_encode(u32(neighbor.x), u32(neighbor.y), u32(neighbor.z));
}
"#;
