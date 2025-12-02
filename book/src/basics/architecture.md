# Architecture Overview

RDPE consists of several layers that work together to run particle simulations on the GPU.

## High-Level Flow

```
┌─────────────────┐     ┌──────────────────┐     ┌─────────────────┐
│  User's Rust    │────▶│  RDPE Compile    │────▶│   GPU Runtime   │
│  Particle +     │     │  Time (Derive    │     │   (wgpu +       │
│  Rules          │     │  Macro + Shader  │     │   Compute       │
│                 │     │  Generation)     │     │   Shaders)      │
└─────────────────┘     └──────────────────┘     └─────────────────┘
```

## Components

### 1. Particle Derive Macro (`rdpe-derive`)

The `#[derive(Particle)]` macro transforms your Rust struct into GPU-compatible form:

```rust
#[derive(Particle, Clone)]
struct Boid {
    position: Vec3,
    velocity: Vec3,
    #[color]
    tint: Vec3,
    particle_type: u32,
}
```

The macro generates:
- A `BoidGpu` struct with correct memory alignment (16-byte for GPU arrays)
- WGSL struct definition for shaders
- `to_gpu()` conversion method
- Tracking of color field offset for rendering

### 2. Simulation Builder (`simulation.rs`)

The builder pattern configures everything before running:

```rust
Simulation::<MyParticle>::new()
    .with_particle_count(10_000)
    .with_bounds(1.0)
    .with_spatial_config(0.1, 32)  // For neighbor rules
    .with_spawner(|i, count| { ... })
    .with_rule(Rule::Gravity(9.8))
    .with_rule(Rule::Separate { radius: 0.1, strength: 1.0 })
    .run();
```

At `.run()` time, the simulation:
1. Detects if any rules need neighbor queries
2. Generates appropriate WGSL compute shaders
3. Generates render shaders
4. Spawns particles using your spawner function
5. Initializes GPU state and runs the event loop

### 3. Shader Generation

Rules are translated to WGSL code:

| Rule | Generated Code Location |
|------|------------------------|
| `Gravity`, `Drag`, `BounceWalls` | Main compute shader body |
| `Separate`, `Cohere`, `Collide` | Inside neighbor iteration loop |
| `Typed { ... }` | Wraps inner rule with type checks |
| `Convert { ... }` | Inside neighbor loop with probability |
| `Custom(code)` | Inserted directly |

### 4. GPU State (`gpu/mod.rs`)

Manages all GPU resources:

- **Particle buffer** - Storage buffer with all particle data
- **Uniform buffer** - View/projection matrix, time, delta_time
- **Compute pipeline** - Runs the physics simulation
- **Render pipeline** - Draws particles as billboarded quads
- **Spatial hashing** (optional) - For neighbor queries

### 5. Spatial Hashing (`gpu/spatial_gpu.rs`)

When rules need neighbor queries, RDPE builds a spatial hash:

1. **Morton encoding** - Convert 3D position to 1D cell index
2. **Radix sort** - Sort particles by cell (8 passes, 4 bits each)
3. **Cell table** - Build start/end indices for each cell

This accelerates neighbor queries from O(n²) to O(n × average_neighbors).

## Render Loop

Each frame:

```
1. Update uniforms (time, camera)
2. [If spatial] Run spatial hashing passes
3. Run compute shader (apply all rules, integrate velocity)
4. Run render pass (draw particles as quads)
5. Present frame
```

## Memory Layout

The derive macro ensures GPU-compatible alignment:

```
Particle in Rust          GPU Memory (16-byte aligned)
┌──────────────┐          ┌──────────────────────────────┐
│ position: Vec3│    ──▶   │ position: vec3<f32> (12 bytes)│
│ velocity: Vec3│          │ _pad0: f32 (4 bytes)          │
└──────────────┘          │ velocity: vec3<f32> (12 bytes)│
                          │ particle_type: u32 (4 bytes)  │
                          └──────────────────────────────┘
```

The `particle_type` field is auto-added if not present.
