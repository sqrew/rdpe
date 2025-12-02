# Spatial Hashing

Spatial hashing accelerates neighbor queries from O(n²) to approximately O(n). It's automatically enabled when you use neighbor-based rules.

## Why It's Needed

Without spatial hashing, checking every particle against every other particle is prohibitively slow:

| Particles | Naive Comparisons | With Spatial Hash |
|-----------|-------------------|-------------------|
| 1,000 | 1,000,000 | ~50,000 |
| 10,000 | 100,000,000 | ~500,000 |
| 100,000 | 10,000,000,000 | ~5,000,000 |

## How It Works

### 1. Morton Encoding (Z-Order Curve)

3D space is divided into a grid of cells. Each cell gets a unique ID using Morton encoding:

```
3D Position → Cell Coordinates → Morton Code (single u32)
```

Morton codes preserve spatial locality - nearby cells have similar codes.

### 2. Radix Sort

Particles are sorted by their Morton code using GPU radix sort:

- 8 passes (4 bits per pass for 32-bit keys)
- Each pass: histogram → prefix sum → scatter
- Result: particles ordered by spatial cell

### 3. Cell Table

After sorting, we build a lookup table:

```
cell_start[morton_code] = first particle index in this cell
cell_end[morton_code] = one past last particle index
```

### 4. Neighbor Iteration

To find neighbors, check the 27 adjacent cells (3×3×3 cube):

```wgsl
for offset in 0..27 {
    let neighbor_cell = get_neighbor_cell(my_cell, offset);
    for particle in cell_start[neighbor_cell]..cell_end[neighbor_cell] {
        // Check distance, apply rule
    }
}
```

## Configuration

Configure spatial hashing with:

```rust
.with_spatial_config(cell_size, grid_resolution)
```

### Cell Size

Should be **at least as large as your largest interaction radius**:

```rust
// If your largest rule has radius 0.15:
.with_spatial_config(0.15, 32)

// Or slightly larger for safety:
.with_spatial_config(0.2, 32)
```

If cell size is smaller than interaction radius, you might miss neighbors in non-adjacent cells.

### Grid Resolution

Must be a **power of 2** (16, 32, 64, 128, etc.):

```rust
.with_spatial_config(0.1, 32)  // 32³ = 32,768 cells
.with_spatial_config(0.1, 64)  // 64³ = 262,144 cells
```

The grid covers space from `-resolution * cell_size / 2` to `+resolution * cell_size / 2`:

| Resolution | Cell Size | Coverage |
|------------|-----------|----------|
| 32 | 0.1 | -1.6 to +1.6 |
| 64 | 0.1 | -3.2 to +3.2 |
| 32 | 0.05 | -0.8 to +0.8 |

Ensure your bounds fit within the grid coverage.

## When It's Used

Spatial hashing is automatically enabled when you use any of these rules:

- `Rule::Separate`
- `Rule::Cohere`
- `Rule::Align`
- `Rule::Collide`
- `Rule::Convert`
- `Rule::Typed` (wrapping a neighbor rule)

Non-neighbor rules (Gravity, Drag, BounceWalls, etc.) don't trigger spatial hashing.

## Memory Usage

The spatial hash requires additional GPU buffers:

| Buffer | Size |
|--------|------|
| Morton codes (×2) | 4 bytes × particles × 2 |
| Particle indices (×2) | 4 bytes × particles × 2 |
| Cell start | 4 bytes × grid_resolution³ |
| Cell end | 4 bytes × grid_resolution³ |
| Histogram | 64 bytes |

For 10,000 particles with 32³ grid:
- Morton/indices: 160 KB
- Cell tables: 256 KB
- Total: ~416 KB

## Performance Tips

1. **Match cell size to interaction radius** - Too small wastes work checking empty cells; too large checks too many particles per cell.

2. **Don't over-resolve** - 32³ is usually enough. 64³ only helps if particles are very spread out.

3. **Spatial hash runs every frame** - It's fast, but the cost is proportional to particle count.

4. **Combine interaction radii** - If possible, use similar radii for all neighbor rules to optimize cell size.
