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
- **Portable** — Runs everywhere WGPU does: desktop, Raspberry Pi, even browsers via WebGPU

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
- Windows, macOS, Linux (including Raspberry Pi and ARM devices)

## Quick Start

```bash
cargo run --example boids
cargo run --example aquarium
cargo run --example slime_mold --features egui
cargo run --example neon_assault
```

### Controls

| Input      | Action       |
|------------|--------------|
| Left-drag  | Orbit camera |
| Right-drag | Pan camera   |
| Scroll     | Zoom         |

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
- **Code Export** — Generate standalone Rust code from your configuration
- **18 Presets** — Pre-built simulations to start from

## Features

### 100+ Built-in Rules

**Physics** — `Gravity`, `Drag`, `Acceleration`, `BounceWalls`, `WrapWalls`, `SpeedLimit`, `Mass`

**Point Forces** — `AttractTo`, `RepelFrom`, `PointGravity`, `Spring`, `Radial`, `Shockwave`, `Pulse`, `Arrive`, `Seek`, `Flee`

**Field Effects** — `Vortex`, `Turbulence`, `Orbit`, `Curl`, `Wind`, `Current`, `DensityBuoyancy`, `Diffuse`, `Gradient`

**Flocking** — `Separate`, `Cohere`, `Align`, `Collide`, `Avoid`, `Flock`, `Sync`

**Fluid** — `NBodyGravity`, `Viscosity`, `Pressure`, `SurfaceTension`, `Buoyancy`, `Friction`, `LennardJones`, `DLA`

**Multi-Species** — `Typed`, `Chase`, `Evade`, `Convert`, `Magnetism`, `Absorb`, `Consume`, `Signal`

**Lifecycle** — `Age`, `Lifetime`, `FadeOut`, `ShrinkOut`, `ColorOverLife`, `ColorBySpeed`, `Die`, `Grow`, `Decay`, `Split`

**Logic/Control** — `Custom`, `NeighborCustom`, `Maybe`, `Trigger`, `Periodic`, `State`, `Agent`, `Switch`, `Threshold`, `Gate`

**Events** — `OnSpawn`, `OnDeath`, `OnCondition`, `OnInterval`, `OnCollision`

### Custom Rules

Drop into WGSL when built-ins aren't enough:

```rust
.with_rule(Rule::Custom(r#"
    p.velocity.y += sin(uniforms.time + p.position.x * 5.0) * 0.1;
    p.color = hsv_to_rgb(p.age * 0.1, 0.8, 1.0);
"#.into()))
```

### Spatial Hashing

O(N) neighbor queries via Morton-encoded radix sort:

```rust
.with_spatial_config(0.1, 32)  // cell_size, grid_resolution
.with_max_neighbors(48)        // cap for dense clusters
```

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

### Sub-Emitters

Particles that spawn particles:

```rust
.with_sub_emitter(SubEmitter::on_death()
    .with_count(8)
    .with_spread(0.5)
    .with_inherit_velocity(0.3))
```

### Visual Effects

- **Post-processing** — Bloom, chromatic aberration, CRT, custom shaders
- **Trails** — Motion blur / light trails
- **Connections** — Lines between nearby particles
- **Wireframe meshes** — Render particles as 3D wireframe shapes
- **Volume rendering** — 3D field visualization with raymarching
- **Palettes** — 12 built-in color schemes

### Runtime Controls

```rust
.with_uniform("strength", 1.0)
.with_ui(|ctx| {
    egui::Window::new("Controls").show(ctx, |ui| {
        // sliders, buttons, etc.
    });
})
.with_update(|ctx| {
    ctx.set("strength", new_value);
    if ctx.input.key_pressed(KeyCode::Space) { /* ... */ }
})
```

## Examples

45+ examples included:

| Category         | Examples                                                        |
|------------------|-----------------------------------------------------------------|
| **Core**         | `boids`, `aquarium`, `infection`, `molecular_soup`, `chemistry` |
| **Simulation**   | `slime_mold_field`, `erosion`, `crystal_growth`, `wave_field`   |
| **Forces**       | `galaxy`, `gravity_visualizer`, `shockwave`, `glow`             |
| **Visual**       | `custom_shader`, `post_process`, `wireframe`, `volume_render`   |
| **Advanced**     | `multi_particle`, `multi_field`, `inbox`, `agent_demo`          |
| **Experimental** | 20+ creative examples in `examples/experimental/`               |

```bash
cargo run --example boids
cargo run --example slime_mold_field --features egui
cargo run --example galaxy
```

## Performance

All simulation runs on GPU compute shaders with no CPU-GPU sync during updates.

| Scenario            | Particles | Notes                      |
|---------------------|-----------|----------------------------|
| No neighbors        | 500k+     | Compute-bound only         |
| Full boids          | 50k+      | Neighbor-bound             |
| With spatial fields | 100k+     | Field resolution dependent |

### Tuning Tips

- **`cell_size`** — Should be >= your largest interaction radius
- **`grid_resolution`** — Power-of-2 (16, 32, 64, 128); larger grids cost more
- **`max_neighbors`** — Cap at 32-64 to prevent O(N²) in dense clusters
- **Field resolution** — Scales as N³; use lower res with more blur for same effect
- **Rule order** — Put cheap rules first; neighbor rules cost more

## Architecture

RDPE is GPU-first: all simulation state lives on the GPU with no CPU-GPU sync during frame updates. Rules compile into a single compute shader to minimize dispatch overhead.

| Component           | Purpose                                                 |
|---------------------|---------------------------------------------------------|
| **Simulation**      | Builder pattern orchestrator; generates WGSL from rules |
| **Rules**           | 100+ composable behaviors compiled into compute shader  |
| **Spatial Hashing** | Radix sort + Morton codes for O(N) neighbor discovery   |
| **Fields**          | 3D grids with atomic writes, blur, decay                |
| **Derive Macros**   | Auto-generate GPU structs with proper alignment         |

## API

### Particle Definition

```rust
#[derive(Particle, Clone)]
struct MyParticle {
    position: Vec3,     // Required
    velocity: Vec3,     // Required
    #[color]
    color: Vec3,        // Optional - particle tint
    particle_type: u32, // Optional - for typed interactions
    energy: f32,        // Custom fields accessible in WGSL
}
```

### Simulation Builder

```rust
Simulation::<P>::new()
    .with_particle_count(n)
    .with_bounds(size)
    .with_spawner(|ctx| -> P { ... })
    .with_spatial_config(cell_size, grid_resolution)
    .with_field(name, config)
    .with_rule(rule)
    .with_uniform(name, value)
    .with_visuals(|v| { ... })
    .with_ui(|ctx| { ... })      // requires egui feature
    .with_update(|ctx| { ... })
    .run();
```

See the [API documentation](https://docs.rs/rdpe) for full details.

## License

[MIT](LICENSE)
