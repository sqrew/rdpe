# RDPE - Reaction Diffusion Particle Engine

GPU-accelerated particle simulations with a simple, declarative API.

```rust
use rdpe::prelude::*;

#[derive(Particle, Clone)]
struct Ball {
    position: Vec3,
    velocity: Vec3,
}

fn main() {
    Simulation::<Ball>::new()
        .with_particle_count(10_000)
        .with_spawner(|_, _| Ball {
            position: Vec3::new(0.0, 0.8, 0.0),
            velocity: Vec3::ZERO,
        })
        .with_rule(Rule::Gravity(9.8))
        .with_rule(Rule::BounceWalls)
        .run();
}
```

## Features

- **Declarative** - Define particle behavior with composable rules
- **GPU-powered** - Compute shaders handle the heavy lifting
- **Type-safe** - Derive macros generate GPU code from your Rust structs
- **Batteries included** - Spatial hashing, flocking, typed interactions out of the box

## Quick Start

```bash
cargo run --example getting_started
```

## Examples

| Example           | Description                                 |
|-------------------|---------------------------------------------|
| `getting_started` | Minimal setup - particles in space          |
| `bouncing`        | Gravity and wall collisions                 |
| `boids`           | Flocking behavior (separate, cohere, align) |
| `predator_prey`   | Two types with chase/evade dynamics         |
| `infection`       | SIR epidemic model with type conversion     |

```bash
cargo run --example boids
```

## Rules

| Category   | Rules                                |
|------------|--------------------------------------|
| Physics    | `Gravity`, `Drag`, `Acceleration`    |
| Boundaries | `BounceWalls`, `WrapWalls`           |
| Flocking   | `Separate`, `Cohere`, `Align`        |
| Forces     | `AttractTo`, `RepelFrom`, `Wander`   |
| Typed      | `Chase`, `Evade`, `Convert`, `Typed` |
| Other      | `SpeedLimit`, `Collide`, `Custom`    |

## Documentation

```bash
cargo doc --open
```

## License

MIT
