# Performance Tips

RDPE runs on the GPU, but performance still varies based on configuration.

## Particle Count

The GPU handles particles in parallel, but performance depends on what you're simulating:

| Scenario | Particles | Typical FPS |
|----------|-----------|-------------|
| No neighbors (gravity, drag, etc.) | 500,000 | 60+ |
| Full boids (separate, cohere, align) | 50,000 | 20+ |
| Spatial fields | 100,000 | 30+ |

### Tips

- Start with fewer particles, increase until performance drops
- Integrated GPUs handle fewer particles than discrete GPUs
- Debug builds are slower; use `--release` for real performance

```bash
cargo run --example boids --release
```

## Spatial Hashing

Neighbor rules trigger spatial hashing every frame.

### Cell Size

**Match cell size to your largest interaction radius:**

```rust
// If largest radius is 0.15:
.with_spatial_config(0.15, 32)  // Good
.with_spatial_config(0.05, 32)  // Bad: checking 27 cells when 1 would do
.with_spatial_config(0.5, 32)   // Bad: too many particles per cell
```

### Grid Resolution

Higher resolution = more cells = more memory, but potentially fewer particles per cell:

```rust
.with_spatial_config(0.1, 32)   // 32,768 cells - usually enough
.with_spatial_config(0.1, 64)   // 262,144 cells - for very spread simulations
.with_spatial_config(0.1, 128)  // 2,097,152 cells - rarely needed
```

### When Spatial Hashing Helps

- **Many particles, small interaction radius** - Huge win
- **Few particles** - Overhead may not be worth it
- **Large interaction radius** - Less benefit (checking many neighbors anyway)

### Max Neighbors Limit

In dense clusters, particles may have hundreds of neighbors. Cap the iteration:

```rust
.with_max_neighbors(48)  // Stop after processing 48 neighbors
```

This trades some accuracy for a significant performance boost (2x or more in pathological cases). Values of 32-64 work well for most simulations.

## Rule Complexity

### Simple Rules (Fast)

```rust
Rule::Gravity(9.8)      // Single operation
Rule::Drag(1.0)         // Single multiply
Rule::BounceWalls       // Few conditionals
```

### Neighbor Rules (Slower)

```rust
Rule::Separate { ... }  // Loops over neighbors
Rule::Cohere { ... }    // Accumulates, then applies
Rule::Collide { ... }   // Distance checks per neighbor
```

### Typed Rules

Add conditional checks per neighbor:

```rust
Rule::Typed {
    self_type: 0,
    other_type: Some(1),
    rule: Box::new(Rule::Separate { ... }),
}
```

Each `Typed` wrapper adds 1-2 comparisons per neighbor.

## Reducing Work

### Combine Similar Rules

Instead of:
```rust
.with_rule(Rule::Typed { self_type: 0, other_type: Some(0), rule: ... })
.with_rule(Rule::Typed { self_type: 0, other_type: Some(1), rule: ... })
.with_rule(Rule::Typed { self_type: 0, other_type: Some(2), rule: ... })
```

Consider if `other_type: None` works:
```rust
.with_rule(Rule::Typed { self_type: 0, other_type: None, rule: ... })
```

### Limit Interaction Radius

Smaller radius = fewer neighbors checked:

```rust
// More neighbors to check:
Rule::Separate { radius: 0.2, strength: 1.0 }

// Fewer neighbors:
Rule::Separate { radius: 0.05, strength: 4.0 }  // Compensate with strength
```

### Reduce Particle Count for Complex Interactions

If you have many typed rules:

```rust
// 5 types × 5 types = 25 potential interaction pairs
// Maybe 10,000 particles is enough instead of 50,000
```

## Custom Rule Performance

### Avoid Expensive Operations

```wgsl
// Expensive:
let dist = length(some_vector);  // Square root

// Cheaper (when comparing distances):
let dist_sq = dot(some_vector, some_vector);
if dist_sq < radius * radius { ... }
```

### Minimize Conditionals

```wgsl
// Many branches:
if p.particle_type == 0u { ... }
else if p.particle_type == 1u { ... }
else if p.particle_type == 2u { ... }

// Consider: can you restructure to avoid this?
```

## Profiling

### Frame Time

Watch for dropped frames. Target: 16.6ms for 60 FPS.

### Identify Bottlenecks

1. Remove neighbor rules - does it speed up significantly?
2. Reduce particle count - linear slowdown or worse?
3. Remove `Typed` wrappers - any difference?

### GPU vs CPU

RDPE is GPU-bound. CPU does:
- Window event handling
- Uniform updates
- Command submission

These are typically not bottlenecks.

## Hardware Considerations

### Discrete GPU

Best performance. RDPE uses `wgpu` which supports:
- Vulkan (Linux, Windows)
- Metal (macOS)
- DX12 (Windows)

### Integrated GPU

Works but with lower particle limits. Intel UHD, AMD APUs, Apple Silicon all supported.

### Power Settings

Laptops may throttle GPU. Ensure:
- Plugged in (or high-performance mode)
- Not thermal throttling
