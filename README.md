# RDPE - Reaction Diffusion Particle Engine

GPU-accelerated particle simulations with a declarative API. Define particles, write rules, customize shaders - RDPE handles the GPU complexity.

```rust
use rdpe::prelude::*;

#[derive(Particle, Clone)]
struct Firefly {
    position: Vec3,
    velocity: Vec3,
    #[color]
    color: Vec3,
    phase: f32,
}

fn main() {
    Simulation::<Firefly>::new()
        .with_particle_count(5000)
        .with_spawner(|i, _| Firefly {
            position: random_sphere(0.8),
            velocity: Vec3::ZERO,
            color: Vec3::new(0.2, 1.0, 0.3),
            phase: i as f32 * 0.1,
        })
        .with_rule(Rule::Orbit { speed: 0.3 })
        .with_rule(Rule::Custom(r#"
            p.color.g = 0.5 + 0.5 * sin(uniforms.time * 2.0 + p.phase);
        "#.into()))
        .with_visuals(|v| {
            v.blend_mode(BlendMode::Additive);
            v.trails(4);
        })
        .run();
}
```

## Features

### Core
- **Declarative simulation** - Composable rules define particle behavior
- **GPU compute shaders** - All particle logic runs massively parallel on the GPU
- **Type-safe** - Derive macro generates GPU code from your Rust structs
- **Custom fields** - Add any `f32` fields to particles for state, phase, type, etc.

### Visual
- **Custom fragment shaders** - WGSL snippets control how particles look
- **Post-processing** - Screen-space effects (bloom, CRT, chromatic aberration, etc.)
- **Particle trails** - Motion blur / light trails with configurable length
- **Connections** - Lines between nearby particles (neural networks, webs)
- **Blend modes** - Additive (glows) or alpha (solid)
- **Background color** - Set the scene backdrop

### Interactivity
- **egui integration** - Runtime sliders, buttons, and controls (optional feature)
- **Custom uniforms** - Define variables, update them per-frame from Rust
- **Update callbacks** - Run Rust code every frame to sync state

### Simulation
- **Built-in rules** - Gravity, flocking, attraction, boundaries, and more
- **Custom WGSL rules** - Write arbitrary compute logic per particle
- **Neighbor iteration** - Efficient spatial queries for local interactions
- **Spatial hashing** - O(n) neighbor lookups instead of O(n²)

## Quick Start

```bash
# Simple example
cargo run --example boids

# With egui controls
cargo run --example slime_mold --features egui

# Creative/experimental
cargo run --example neon_assault
cargo run --example ethereal_web
```

## Examples

### Core Examples
| Example | Description |
|---------|-------------|
| `boids` | Classic flocking (separation, cohesion, alignment) |
| `predator_prey` | Two particle types with chase/evade |
| `infection` | SIR epidemic - particles convert types on contact |

### Single-Rule Demos (`examples/rules/`)
| Example | Description |
|---------|-------------|
| `bounce_walls` | Particles bouncing off boundaries |
| `vortex` | Circular flow field |
| `curl` | Turbulent curl noise motion |
| `orbit` | Particles orbiting a center point |
| `neighbors` | Demonstrates neighbor iteration |
| `nbody` | Gravitational attraction between all particles |
| `fluid` | SPH-style fluid dynamics |
| `magnetism` | Magnetic pole attraction/repulsion |
| `spring` | Spring forces toward origin |

### Visual Examples
| Example | Description |
|---------|-------------|
| `custom_shader` | Custom fragment shader (glow effect) |
| `post_process` | Post-processing (vignette, chromatic aberration, grain) |
| `inbox` | Particle communication via inbox system |

### Experimental (`examples/experimental/`)
| Example | Description |
|---------|-------------|
| `ethereal_web` | Dreamlike visualization with all features combined |
| `neon_assault` | 80s arcade aesthetic with CRT post-processing |
| `neon_assault_interactive` | Neon assault with egui runtime controls |
| `cosmic_jellyfish` | Organic pulsing creature |
| `firefly_grove` | Synchronized blinking lights |
| `black_hole` | Gravitational lensing effect |
| `thought_storm` | Neural activity visualization |
| `plasma_storm` | Interactive plasma with egui |
| `fluid_galaxy` | Galaxy formation with egui |
| `murmuration` | Starling flock patterns |

## API Reference

### Simulation Builder

```rust
Simulation::<MyParticle>::new()
    // Basics
    .with_particle_count(10_000)
    .with_bounds(2.0)
    .with_particle_size(0.02)
    .with_spawner(|index, total| { /* return particle */ })

    // Rules (physics/behavior)
    .with_rule(Rule::Gravity(9.8))
    .with_rule(Rule::Custom("/* WGSL code */".into()))

    // Visuals
    .with_fragment_shader("/* WGSL snippet */")
    .with_visuals(|v| {
        v.blend_mode(BlendMode::Additive);
        v.background(Vec3::new(0.0, 0.0, 0.02));
        v.trails(8);
        v.connections(0.1);  // max distance
        v.post_process("/* WGSL snippet */");
    })

    // Interactivity (requires `egui` feature)
    .with_uniform("my_value", 0.5)
    .with_ui(|ctx| { /* egui code */ })
    .with_update(|ctx| { ctx.set("my_value", new_value); })

    .run();
```

### Particle Struct

```rust
#[derive(Particle, Clone)]
struct MyParticle {
    position: Vec3,    // Required
    velocity: Vec3,    // Required
    #[color]
    color: Vec3,       // Optional - particle tint

    // Custom fields (all f32)
    phase: f32,
    energy: f32,
    particle_type: f32,
}
```

### Built-in Rules

| Rule | Description |
|------|-------------|
| `Gravity(f32)` | Constant downward acceleration |
| `Drag(f32)` | Velocity damping (0.0-1.0) |
| `BounceWalls` | Reflect off simulation bounds |
| `WrapWalls` | Teleport to opposite side |
| `Separate { radius, strength }` | Push away from nearby particles |
| `Cohere { radius, strength }` | Pull toward nearby particles |
| `Align { radius, strength }` | Match velocity of nearby particles |
| `AttractTo(Vec3)` | Pull toward a point |
| `RepelFrom(Vec3)` | Push away from a point |
| `Orbit { speed }` | Circular motion around Y axis |
| `Vortex { strength }` | Swirling flow field |
| `Curl { scale, strength }` | Turbulent noise-based motion |
| `SpeedLimit(f32)` | Cap maximum velocity |
| `Custom(String)` | Arbitrary WGSL compute code |

### Custom Shaders

**Fragment Shader** - Controls particle appearance:
```rust
.with_fragment_shader(r#"
    let dist = length(in.uv);  // -1 to 1, center is 0
    let glow = 1.0 / (dist * dist * 8.0 + 0.3);
    let alpha = clamp(glow * 0.5, 0.0, 1.0);
    return vec4<f32>(in.color * glow, alpha);
"#)
```

Available in fragment shader:
- `in.uv` - Vec2, particle-local coordinates (-1 to 1)
- `in.color` - Vec3, particle color
- `uniforms.time` - f32, seconds since start

**Post-Process Shader** - Screen-space effects:
```rust
v.post_process(r#"
    // Chromatic aberration
    let r = textureSample(scene, scene_sampler, in.uv + vec2(0.003, 0.0)).r;
    let g = textureSample(scene, scene_sampler, in.uv).g;
    let b = textureSample(scene, scene_sampler, in.uv - vec2(0.003, 0.0)).b;
    return vec4<f32>(r, g, b, 1.0);
"#);
```

Available in post-process:
- `in.uv` - Vec2, screen coordinates (0 to 1)
- `scene` / `scene_sampler` - The rendered particle scene
- `uniforms.time` - f32, seconds since start

**Custom Rules** - Compute shader logic:
```rust
.with_rule(Rule::Custom(r#"
    // Access particle fields
    let pos = p.position;

    // Modify velocity
    p.velocity += some_force * 0.1;

    // Available: uniforms.time, uniforms.delta_time, uniforms.bounds
    // Custom uniforms: uniforms.my_value
"#.into()))
```

### Neighbor Iteration

For local interactions (flocking, SPH, etc.):
```rust
.with_rule(Rule::Custom(r#"
    var force = vec3<f32>(0.0);

    for (var i = 0u; i < neighbor_count; i++) {
        let other = get_neighbor(i);
        let diff = p.position - other.position;
        let dist = length(diff);
        if dist > 0.0 && dist < 0.2 {
            force += normalize(diff) / dist;
        }
    }

    p.velocity += force * 0.01;
"#.into()))
```

## egui Integration

Enable with `--features egui`:

```rust
use std::sync::{Arc, Mutex};

struct State { speed: f32 }

let state = Arc::new(Mutex::new(State { speed: 1.0 }));
let ui_state = state.clone();
let update_state = state.clone();

Simulation::new()
    .with_uniform("speed", 1.0)
    .with_ui(move |ctx| {
        let mut s = ui_state.lock().unwrap();
        egui::Window::new("Controls").show(ctx, |ui| {
            ui.add(egui::Slider::new(&mut s.speed, 0.0..=2.0));
        });
    })
    .with_update(move |ctx| {
        let s = update_state.lock().unwrap();
        ctx.set("speed", s.speed);
    })
    .with_rule(Rule::Custom(r#"
        p.velocity *= uniforms.speed;
    "#.into()))
    .run();
```

## Performance Notes

- All particle physics runs on GPU compute shaders
- Spatial hashing enables O(n) neighbor queries
- Tested smooth at 10k+ particles on integrated graphics (Intel HD 530)

## License

MIT
