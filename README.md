# RDPE

**R**ealtime **D**ata **P**resentation **E**ngine — GPU-accelerated particle simulations with a declarative API.

Describe behaviors with composable rules. RDPE generates the compute shaders.

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
        .with_spawner(|_, _| Boid {
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

## Why RDPE?

Most particle systems are either too low-level (raw compute shaders) or too rigid (fixed behaviors). RDPE sits in between:

- **Declarative** — Say "separate, cohere, align" not "implement boids in WGSL"
- **Composable** — Stack rules, mix built-ins with custom WGSL
- **Fast** — 50k+ particles with neighbor interactions, 500k+ without
- **Flexible** — Custom particle fields, typed interactions, spatial fields

Good for creative coding, generative art, simulations, data visualization, and experimentation.

## Quick Start

```bash
cargo run --example boids
cargo run --example aquarium
cargo run --example slime_mold --features egui
cargo run --example neon_assault
```

## Features

### 40+ Built-in Rules

**Physics** — `Gravity`, `Drag`, `BounceWalls`, `WrapWalls`, `SpeedLimit`

**Forces** — `AttractTo`, `RepelFrom`, `PointGravity`, `Vortex`, `Turbulence`, `Curl`

**Flocking** — `Separate`, `Cohere`, `Align`, `Collide`, `NBodyGravity`

**Multi-Species** — `Chase`, `Evade`, `Convert` (for predator/prey, infection, etc.)

**Lifecycle** — `Age`, `Lifetime`, `FadeOut`, `ShrinkOut`

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

O(n) neighbor queries via Morton-encoded spatial hashing:

```rust
.with_spatial_config(0.1, 32)  // cell_size, grid_resolution
.with_max_neighbors(48)        // cap for performance in dense clusters
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
- **Palettes** — Built-in color schemes (Viridis, Plasma, etc.)

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

| Category | Examples |
|----------|----------|
| **Core** | `boids`, `aquarium`, `predator_prey`, `infection`, `slime_mold` |
| **Forces** | `vortex`, `curl`, `nbody`, `fluid`, `magnetism` |
| **Visual** | `custom_shader`, `post_process`, `trails`, `connections` |
| **Showcase** | `neon_assault`, `cosmic_jellyfish`, `ethereal_web`, `murmuration` |

```bash
cargo run --example <name>
cargo run --example <name> --features egui  # for UI controls
```

## Performance

All logic runs on GPU compute shaders.

| Scenario | Particles | FPS |
|----------|-----------|-----|
| No neighbors (gravity, drag, etc.) | 500,000 | 60+ |
| Full boids (separate, cohere, align) | 50,000 | 20+ |
| Spatial fields | 100,000 | 30+ |

Tested on mid-range hardware. Your mileage may vary.

**Tuning tips:**
- Increase `cell_size` if interaction radii are large
- Use `with_max_neighbors(N)` to cap neighbor processing in dense clusters
- Start fields at 32³ or 64³ resolution

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

    // Custom fields accessible in WGSL
    energy: f32,
    phase: f32,
}
```

The derive macro auto-injects `age`, `alive`, and `scale` fields.

### Simulation Builder

```rust
Simulation::<P>::new()
    .with_particle_count(n)
    .with_bounds(size)
    .with_particle_size(radius)
    .with_spawner(|index, total| -> P { ... })
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

## License

MIT
