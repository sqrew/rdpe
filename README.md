# RDPE

[![Crates.io](https://img.shields.io/crates/v/rdpe.svg)](https://crates.io/crates/rdpe)
[![Documentation](https://docs.rs/rdpe/badge.svg)](https://docs.rs/rdpe)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

**GPU-accelerated particle simulations with a declarative API.**

Describe behaviors with composable rules. RDPE generates optimized compute shaders.

<!-- TODO: Replace with actual gif -->
<!-- ![RDPE Demo](assets/demo.gif) -->

```rust
use rdpe::prelude::*;

#[derive(Particle, Clone)]
struct Boid {
    position: Vec3,
    velocity: Vec3,
    #[color]
    color: Vec3,
}

fn main() {
    Simulation::<Boid>::new()
        .with_particle_count(50_000)
        .with_bounds(1.0)
        .with_spatial_config(0.1, 32)
        .with_max_neighbors(48)
        .with_spawner(|ctx| Boid {
            position: random_in_sphere(0.8),
            velocity: random_direction() * 0.3,
            color: Vec3::new(0.2, 0.8, 1.0),
        })
        .with_rule(Rule::Separate { radius: 0.05, strength: 2.0 })
        .with_rule(Rule::Cohere { radius: 0.15, strength: 0.5 })
        .with_rule(Rule::Align { radius: 0.1, strength: 1.0 })
        .with_rule(Rule::SpeedLimit { min: 0.3, max: 1.5 })
        .with_rule(Rule::BounceWalls)
        .run();
}
```

Good for creative coding, generative art, simulations, visualizations, and experimentation.

## Why RDPE?

Most particle systems are either too low-level (raw compute shaders) or too rigid (fixed behaviors). RDPE sits in between:

- **Declarative** — Say "separate, cohere, align" not "implement boids in WGSL"
- **Composable** — Stack rules, mix built-ins with custom WGSL
- **Fast** — GPU-resident simulation with O(N) neighbor queries via radix-sorted spatial hashing
- **Flexible** — Custom particle fields, typed interactions, spatial fields, runtime uniforms

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
rdpe = "0.1"
```

With GUI support:

```toml
[dependencies]
rdpe = { version = "0.1", features = ["egui"] }
```

### Requirements

- Rust 1.70+
- GPU with Vulkan, Metal, or DX12 support
- Windows, macOS, or Linux

## Quick Start

```bash
cargo run --example boids
cargo run --example aquarium
cargo run --example slime_mold --features egui
cargo run --example neon_assault
```

### Controls

| Input | Action |
|-------|--------|
| Left-drag | Orbit camera |
| Right-drag | Pan camera |
| Scroll | Zoom |

## Visual Editor

RDPE includes a visual editor for designing simulations without writing code:

```bash
cargo run --package rdpe-editor
```

<!-- TODO: Replace with editor screenshot -->
<!-- ![RDPE Editor](assets/editor.png) -->

- **Live Preview** — Real-time GPU-accelerated viewport
- **100+ Rules** — All rules accessible through dropdown menus
- **Visual Configuration** — Particle shapes, colors, blend modes, trails
- **Custom WGSL** — Write custom shader code with live validation
- **3D Fields** — Configure spatial fields with volume rendering
- **Code Export** — Generate standalone Rust code from your configuration
- **18 Presets** — Pre-built simulations to start from

## Features

### 100+ Built-in Rules

**Physics** — `Gravity`, `Drag`, `Acceleration`, `BounceWalls`, `WrapWalls`, `SpeedLimit`, `Mass`

**Point Forces** — `AttractTo`, `RepelFrom`, `PointGravity`, `Spring`, `Radial`, `Shockwave`, `Pulse`, `Arrive`, `Seek`, `Flee`

**Field Effects** — `Vortex`, `Turbulence`, `Orbit`, `Curl`, `Wind`, `Current`, `DensityBuoyancy`, `Diffuse`, `Gradient`

**Wave/Noise** — `Oscillate`, `PositionNoise`, `Wander`, `Lerp`, `Noise`, `Remap`, `Smooth`, `Quantize`, `Modulo`

**Flocking** — `Separate`, `Cohere`, `Align`, `Collide`, `Avoid`, `Flock`, `Sync`

**Fluid** — `NBodyGravity`, `Viscosity`, `Pressure`, `SurfaceTension`, `Buoyancy`, `Friction`, `LennardJones`, `DLA`

**Multi-Species** — `Typed`, `Chase`, `Evade`, `Convert`, `Magnetism`, `Absorb`, `Consume`, `Signal`

**Lifecycle** — `Age`, `Lifetime`, `FadeOut`, `ShrinkOut`, `ColorOverLife`, `ColorBySpeed`, `ColorByAge`, `ScaleBySpeed`, `Die`, `Grow`, `Decay`, `Split`

**Logic/Control** — `Custom`, `NeighborCustom`, `Maybe`, `Trigger`, `Periodic`, `State`, `Agent`, `Threshold`, `Gate`, `Select`, `Blend`, `And`, `Or`, `Not`, `Xor`, `Hysteresis`, `Latch`, `Tween`

**Spring Systems** — `BondSprings`, `ChainSprings`, `RadialSprings`, `OnCollision`

**Events** — `OnSpawn`, `OnDeath`, `OnCondition`, `OnInterval`

**Dynamic Rules** — `CustomDynamic`, `NeighborCustomDynamic`, `OnCollisionDynamic` (runtime-configurable)

### Custom Rules

Drop into WGSL when built-ins aren't enough:

```rust
.with_rule(Rule::Custom(r#"
    // Available: p (particle), uniforms.time, index
    p.velocity.y += sin(uniforms.time + p.position.x * 5.0) * 0.1;
    p.color = hsv_to_rgb(p.age * 0.1, 0.8, 1.0);
"#.into()))
```

### Spatial Hashing

O(N) neighbor queries via Morton-encoded radix sort:

```rust
.with_spatial_config(0.1, 32)  // cell_size, grid_resolution
.with_max_neighbors(48)        // cap for performance in dense clusters
```

**How it works:**
1. Each particle's position is encoded into a 30-bit Morton code (10 bits per axis)
2. GPU radix sort orders particles by Morton code — preserving spatial locality
3. Cell lookup tables enable O(1) access to neighbors in 27 adjacent cells
4. `max_neighbors` bounds the inner loop to prevent quadratic behavior in dense regions

### Typed Particles

Different behaviors per particle type:

```rust
#[derive(ParticleType)]
enum Species { Fish, Shark }

.with_rule(Rule::Chase {
    self_type: Species::Shark.into(),
    target_type: Species::Fish.into(),
    radius: 0.3,
    strength: 0.5,
})
.with_rule(Rule::Evade {
    self_type: Species::Fish.into(),
    threat_type: Species::Shark.into(),
    radius: 0.2,
    strength: 2.0,
})
```

### Spatial Fields

3D grids for pheromones, density, flow:

```rust
.with_field("pheromone", FieldConfig::new(64)
    .with_decay(0.98)
    .with_blur(0.1))
.with_rule(Rule::Custom(r#"
    field_write(0u, p.position, 0.1);           // deposit
    let grad = field_gradient(0u, p.position);  // follow gradient
    p.velocity += grad * 0.5;
"#.into()))
```

**Field processing pipeline:**
1. Particles atomically write to field (parallel-safe via i32 atomics)
2. Merge pass converts atomic counts to floats
3. Blur pass diffuses values (configurable iterations)
4. Decay pass fades values toward zero

### Sub-Emitters

Particles that spawn particles:

```rust
.with_sub_emitter(SubEmitter::on_death()
    .with_count(8)
    .with_spread(0.5)
    .with_inherit_velocity(0.3))
```

### Particle Messaging

Direct particle-to-particle communication:

```rust
.with_inbox()
.with_rule(Rule::NeighborCustom(r#"
    inbox_send(other_idx, 0u, p.energy * 0.1);  // send to neighbor
"#.into()))
.with_rule(Rule::Custom(r#"
    let received = inbox_receive_at(index, 0u);  // receive accumulated
    p.energy += received;
"#.into()))
```

### Visual Effects

- **Post-processing** — Bloom, chromatic aberration, CRT, custom shaders
- **Velocity stretch** — Elongate particles in motion direction
- **Trails** — Motion blur / light trails
- **Connections** — Lines between nearby particles
- **Wireframe meshes** — Render particles as 3D wireframe shapes
- **Volume rendering** — 3D field visualization with raymarching
- **Palettes** — 12 built-in color schemes (Viridis, Plasma, Fire, Neon, etc.)
- **Blend modes** — Alpha, Additive, Multiply
- **Shapes** — Circle, Square, Triangle

### Runtime Controls

```rust
// egui integration (--features egui)
.with_uniform("strength", 1.0)
.with_ui(|ctx| {
    egui::Window::new("Controls").show(ctx, |ui| {
        // sliders, buttons, etc.
    });
})
.with_update(|ctx| {
    ctx.set("strength", new_value);

    // Input handling
    if ctx.input.key_pressed(KeyCode::Space) { /* ... */ }
    if ctx.input.mouse_held(MouseButton::Left) { /* ... */ }
})
```

## Examples

45+ examples included:

| Category | Examples |
|----------|----------|
| **Core** | `boids`, `aquarium`, `infection`, `molecular_soup`, `chemistry` |
| **Simulation** | `slime_mold_field`, `erosion`, `crystal_growth`, `wave_field`, `neural_network` |
| **Forces** | `galaxy`, `gravity_visualizer`, `shockwave`, `glow` |
| **Visual** | `custom_shader`, `custom_vertex`, `post_process`, `wireframe`, `volume_render`, `texture_example` |
| **Advanced** | `multi_particle`, `multi_field`, `inbox`, `agent_demo`, `custom_dynamic` |
| **Experimental** | 20+ creative examples in `examples/experimental/` |

```bash
cargo run --example boids
cargo run --example slime_mold_field --features egui
cargo run --example galaxy
```

## Architecture

RDPE is designed around a GPU-first philosophy: all simulation state lives on the GPU with no CPU-GPU sync during frame updates.

### Core Components

| Component | Purpose |
|-----------|---------|
| **Simulation** | Builder pattern orchestrator; generates WGSL from rules |
| **Rules** | 100+ composable behaviors compiled into compute shader |
| **Spatial Hashing** | Radix sort + Morton codes for O(N) neighbor discovery |
| **Fields** | 3D grids with atomic writes, blur, decay |
| **Derive Macros** | Auto-generate GPU structs with proper WGSL alignment |
| **Visual Editor** | GUI for designing simulations without code |

### Memory Layout

Particles use 16-byte aligned structs for optimal GPU access. The derive macro handles padding automatically:

```rust
#[derive(Particle)]  // Generates aligned GPU struct
struct MyParticle {
    position: Vec3,  // 12 bytes + 4 padding
    velocity: Vec3,  // 12 bytes + 4 padding
    // ...
}
```

### Shader Generation

Rules compile into a monolithic compute shader:
1. Built-in utilities (noise, random, color, lifecycle)
2. Particle and field struct definitions
3. Uniforms and buffer bindings
4. Rule logic in execution order
5. Velocity integration

This single-kernel approach minimizes synchronization overhead.

## Performance

All simulation runs on GPU compute shaders with no CPU-GPU synchronization during updates.

| Scenario | Particles | Notes |
|----------|-----------|-------|
| No neighbors (gravity, drag, etc.) | 500k+ | Compute-bound only |
| Full boids (separate, cohere, align) | 50k+ | Neighbor-bound |
| With spatial fields | 100k+ | Field resolution dependent |

### Tuning Guide

**Spatial Hashing** (most important):
- `cell_size` should be >= your largest interaction radius
- Too small: excessive sorting overhead
- Too large: too many neighbors per cell (quadratic behavior)
- `grid_resolution` must be power-of-2 (16, 32, 64, 128, 256)

**Neighbor Limits**:
- `with_max_neighbors(N)` caps neighbor processing per particle
- Critical for dense clusters — prevents worst-case O(N²) behavior
- Typical values: 32-64 for flocking, 16-32 for collision-only

**Fields**:
- Resolution scales as N³ — 128³ costs 8x more than 64³
- Use lower resolution with more blur iterations for equivalent visuals
- `decay` near 1.0 (e.g., 0.98) for persistent fields; lower for quick fade

**Rule Complexity**:
- Rules execute in add order — put cheap rules first
- Neighbor rules are more expensive than non-neighbor rules
- Custom WGSL isn't optimized — keep it simple

### Memory Usage

```
Particles:     count × stride (typically 64-128 bytes each)
Spatial grid:  resolution³ × 8 bytes (cell start/end indices)
Morton codes:  count × 16 bytes (two buffers for ping-pong sort)
Fields:        resolution³ × 4 bytes per scalar field × 3 buffers
```

Example: 50k particles, 64³ grid, one 64³ field:
- Particles: ~6 MB
- Spatial: ~2 MB
- Morton: ~1.5 MB
- Field: ~3 MB
- **Total: ~12.5 MB GPU VRAM**

## API Reference

### Particle Definition

```rust
#[derive(Particle, Clone)]
struct MyParticle {
    position: Vec3,     // Required
    velocity: Vec3,     // Required
    #[color]
    color: Vec3,        // Optional - particle tint
    particle_type: u32, // Optional - for typed interactions

    // Custom fields accessible in WGSL
    energy: f32,
    phase: f32,
}
```

The derive macro auto-injects `age`, `alive`, and `scale` fields for lifecycle management.

### Simulation Builder

```rust
Simulation::<P>::new()
    .with_particle_count(n)
    .with_bounds(size)
    .with_particle_size(radius)
    .with_spawner(|ctx| -> P { ... })  // ctx.index, ctx.count, ctx.bounds
    .with_spatial_config(cell_size, grid_resolution)
    .with_max_neighbors(max)
    .with_field(name, config)
    .with_emitter(emitter)
    .with_sub_emitter(sub_emitter)
    .with_inbox()
    .with_rule(rule)
    .with_uniform(name, value)
    .with_visuals(|v| { ... })
    .with_ui(|ctx| { ... })          // requires egui feature
    .with_update(|ctx| { ... })
    .run();
```

### Built-in WGSL Functions

Available in custom rules:

```wgsl
// Random
hash(u32) -> u32
rand() -> f32                    // 0..1
rand_range(min, max) -> f32
rand_sphere() -> vec3<f32>       // uniform point in unit sphere

// Noise
noise2(vec2) -> f32              // simplex noise
noise3(vec3) -> f32
fbm2(vec2, octaves) -> f32       // fractal brownian motion
fbm3(vec3, octaves) -> f32

// Color
hsv_to_rgb(h, s, v) -> vec3<f32>
rgb_to_hsv(r, g, b) -> vec3<f32>

// Lifecycle
kill_particle()                   // mark dead
respawn_at(position, velocity)    // resurrect
is_alive() -> bool
is_dead() -> bool
```

## License

[MIT](LICENSE)
