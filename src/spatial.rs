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
    /// Maximum neighbors to process per particle (0 = unlimited)
    pub max_neighbors: u32,
}

impl Default for SpatialConfig {
    fn default() -> Self {
        Self {
            cell_size: 0.1,
            grid_resolution: 64, // 64^3 = 262144 cells, fits in 18-bit Morton code
            max_neighbors: 0,    // 0 = unlimited
        }
    }
}

impl SpatialConfig {
    /// Create a new spatial configuration with the given cell size and grid resolution.
    ///
    /// # Panics
    /// Panics if `grid_resolution` is not a power of 2 or exceeds 1024.
    pub fn new(cell_size: f32, grid_resolution: u32) -> Self {
        assert!(grid_resolution.is_power_of_two(), "Grid resolution must be power of 2");
        assert!(grid_resolution <= 1024, "Grid resolution must be <= 1024 for 30-bit Morton codes");
        Self { cell_size, grid_resolution, max_neighbors: 0 }
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
