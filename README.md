# RDPE - Rapid Dev Particle Engine

GPU-accelerated particle simulations with a declarative API. Describe behaviors with composable rules, RDPE generates the compute shaders.

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
        .with_particle_count(10_000)
        .with_bounds(1.0)
        .with_spawner(|_, _| Boid {
            position: random_in_sphere(0.5),
            velocity: random_direction() * 0.5,
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

Add `--features egui` for interactive controls:

```rust
use rdpe::prelude::*;
use std::sync::{Arc, Mutex};

fn main() {
    let separation = Arc::new(Mutex::new(2.0_f32));
    let cohesion = Arc::new(Mutex::new(0.5_f32));

    Simulation::<Boid>::new()
        .with_particle_count(10_000)
        .with_bounds(1.0)
        .with_spawner(|_, _| /* ... */)
        // Uniforms can be updated at runtime
        .with_uniform("separation", 2.0)
        .with_uniform("cohesion", 0.5)
        // egui UI
        .with_ui({
            let sep = separation.clone();
            let coh = cohesion.clone();
            move |ctx| {
                egui::Window::new("Flocking").show(ctx, |ui| {
                    ui.add(egui::Slider::new(&mut *sep.lock().unwrap(), 0.0..=5.0).text("Separation"));
                    ui.add(egui::Slider::new(&mut *coh.lock().unwrap(), 0.0..=2.0).text("Cohesion"));
                });
            }
        })
        // Sync UI state to shader uniforms
        .with_update({
            let sep = separation.clone();
            let coh = cohesion.clone();
            move |ctx| {
                ctx.set("separation", *sep.lock().unwrap());
                ctx.set("cohesion", *coh.lock().unwrap());
            }
        })
        // Rules using dynamic uniforms
        .with_rule(Rule::Custom("p.velocity += separation_force * uniforms.separation;".into()))
        .with_rule(Rule::Custom("p.velocity += cohesion_force * uniforms.cohesion;".into()))
        .run();
}
```

## Why RDPE?

Most particle libraries are either too low-level (raw compute shaders, manual memory layouts) or too constrained (fixed behavior sets). RDPE hits a sweet spot:

- **Describe intent, not implementation** - Say "separate, cohere, align" not "implement Reynolds' boids in WGSL"
- **Rapid prototyping** - Go from idea to running simulation in minutes
- **Escape hatches** - Custom WGSL rules when built-ins don't cover it
- **GPU performance** - 10k+ particles at 60fps on integrated graphics

Perfect for creative coding, generative art, simulations, and quick experimentation.

## Features

### Simulation
- **30+ built-in rules** - Physics, flocking, forces, type interactions, lifecycle
- **Custom WGSL rules** - Arbitrary compute logic per particle
- **Spatial hashing** - O(n) neighbor queries for local interactions
- **3D spatial fields** - Pheromone trails, density fields, flow fields
- **Runtime emitters** - Continuous particle spawning (fountains, explosions)
- **Typed particles** - Different behaviors per particle type (predator/prey, infection)

### Visual
- **Custom fragment shaders** - Control particle appearance
- **Post-processing** - Screen-space effects (bloom, CRT, chromatic aberration)
- **Particle trails** - Motion blur / light trails
- **Connections** - Lines between nearby particles
- **Blend modes** - Additive (glows) or alpha (solid)
- **Textures** - Sample custom textures in shaders

### Interactivity
- **Input handling** - Keyboard and mouse state
- **egui integration** - Runtime UI controls (optional feature)
- **Custom uniforms** - Pass values from Rust to shaders each frame
- **Update callbacks** - Run Rust code every frame

## Quick Start

```bash
cargo run --example boids
cargo run --example predator_prey
cargo run --example slime_mold --features egui
cargo run --example neon_assault
```

## Examples

### Core
| Example | Description |
|---------|-------------|
| `boids` | Classic flocking (separation, cohesion, alignment) |
| `predator_prey` | Chase/evade between particle types |
| `infection` | SIR epidemic with type conversion |
| `slime_mold` | Physarum-inspired emergent patterns |
| `multi_field` | Competing pheromone fields |

### Rules (`examples/rules/`)
| Example | Description |
|---------|-------------|
| `bounce_walls` | Boundary reflection |
| `vortex` | Circular flow |
| `curl` | Divergence-free turbulence |
| `nbody` | Gravitational attraction |
| `fluid` | SPH-style dynamics |
| `magnetism` | Charge attraction/repulsion |

### Visual
| Example | Description |
|---------|-------------|
| `custom_shader` | Custom fragment shader |
| `post_process` | Screen-space effects |
| `texture_example` | Texture sampling |

### Experimental (`examples/experimental/`)
| Example | Description |
|---------|-------------|
| `ethereal_web` | All features combined |
| `neon_assault` | 80s arcade aesthetic |
| `cosmic_jellyfish` | Organic pulsing creature |
| `black_hole` | Gravitational lensing |
| `murmuration` | Starling flock patterns |

## API Overview

### Particles

```rust
#[derive(Particle, Clone)]
struct MyParticle {
    position: Vec3,    // Required
    velocity: Vec3,    // Required
    #[color]
    color: Vec3,       // Optional - particle tint
    particle_type: u32, // Optional - for typed interactions

    // Custom fields
    energy: f32,
    phase: f32,
}
```

The derive macro auto-injects `age`, `alive`, and `scale` fields for lifecycle management.

### Built-in Rules

**Physics**
| Rule | Description |
|------|-------------|
| `Gravity(f32)` | Downward acceleration |
| `Drag(f32)` | Velocity damping |
| `BounceWalls` / `WrapWalls` | Boundary handling |
| `SpeedLimit { min, max }` | Clamp velocity |

**Forces**
| Rule | Description |
|------|-------------|
| `AttractTo { point, strength }` | Pull toward point |
| `RepelFrom { point, strength, radius }` | Push from point |
| `PointGravity { point, strength, softening }` | Inverse-square attraction |
| `Vortex { center, axis, strength }` | Swirling flow |
| `Turbulence { scale, strength }` | Chaotic noise motion |
| `Curl { scale, strength }` | Divergence-free flow |

**Flocking** (require spatial hashing)
| Rule | Description |
|------|-------------|
| `Separate { radius, strength }` | Avoid crowding |
| `Cohere { radius, strength }` | Steer toward neighbors |
| `Align { radius, strength }` | Match neighbor velocity |
| `Collide { radius, response }` | Particle collision |

**Typed Interactions**
| Rule | Description |
|------|-------------|
| `Chase { self_type, target_type, radius, strength }` | Pursue target type |
| `Evade { self_type, threat_type, radius, strength }` | Flee from threat |
| `Convert { from, trigger, to, radius, probability }` | Type conversion on contact |

**Lifecycle**
| Rule | Description |
|------|-------------|
| `Age` | Increment particle age |
| `Lifetime(f32)` | Kill after duration |
| `FadeOut(f32)` | Dim color over lifetime |
| `ShrinkOut(f32)` | Shrink over lifetime |

### Emitters

Continuously spawn particles (requires `Age` + `Lifetime` rules):

```rust
.with_emitter(Emitter::Cone {
    position: Vec3::new(0.0, -0.5, 0.0),
    direction: Vec3::Y,
    speed: 2.0,
    spread: 0.3,
    rate: 1000.0,  // particles per second
})
```

Types: `Point`, `Cone`, `Sphere`, `Box`, `Burst`

### Spatial Fields

3D grids for indirect particle communication:

```rust
.with_field("pheromone", FieldConfig::new(64)
    .with_decay(0.98)
    .with_blur(0.1))
.with_rule(Rule::Custom(r#"
    field_write(0u, p.position, 0.1);  // deposit
    let val = field_read(0u, p.position);  // sample
    let grad = field_gradient(0u, p.position, 0.05);  // direction
"#.into()))
```

### Custom Rules

Write WGSL directly:

```rust
.with_rule(Rule::Custom(r#"
    // Available: p (particle), uniforms.time, uniforms.delta_time
    p.velocity.y += sin(uniforms.time + p.position.x * 5.0) * 0.1;
    p.color = mix(p.color, vec3(1.0, 0.5, 0.0), 0.01);
"#.into()))
```

### Input

```rust
.with_update(|ctx| {
    if ctx.input.key_pressed(KeyCode::Space) {
        ctx.set("burst", 1.0);
    }
    if ctx.input.mouse_held(MouseButton::Left) {
        let pos = ctx.input.mouse_ndc();
        ctx.set("attractor", [pos.x, pos.y]);
    }
})
```

### egui Integration

```rust
// Requires --features egui
.with_uniform("speed", 1.0)
.with_ui(|ctx| {
    egui::Window::new("Controls").show(ctx, |ui| {
        // UI code
    });
})
.with_update(|ctx| {
    ctx.set("speed", new_value);
})
```

## Performance

- All particle logic runs on GPU compute shaders
- Spatial hashing: O(n) neighbor queries
- Tested: 10k+ particles at 60fps on Intel HD 530
- Fields add overhead; start with 32³ or 64³ resolution

## Documentation

Full documentation at [book/](book/) or build locally:

```bash
cd book && mdbook serve
```

## License

MIT
