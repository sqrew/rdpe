# Introduction

**RDPE** (Realtime Data Presentation Engine) is a GPU-accelerated particle simulation library for Rust. It lets you create complex, interactive particle systems with minimal code by combining simple, composable rules.

## What Can You Build?

With RDPE, you can simulate:

- **Flocking behaviors** - Birds, fish, or any swarming entities
- **Predator-prey ecosystems** - Multiple species with different interactions
- **Disease spread** - SIR models and infection dynamics
- **Physics simulations** - Bouncing particles, gravity, collisions
- **Chemical reactions** - Particles that transform on contact
- **Crowd dynamics** - Social forces and emergent behavior

## Quick Example

```rust
use rdpe::prelude::*;

#[derive(Particle, Clone)]
struct MyParticle {
    position: Vec3,
    velocity: Vec3,
}

fn main() {
    Simulation::<MyParticle>::new()
        .with_particle_count(10_000)
        .with_bounds(1.0)
        .with_spawner(|i, count| MyParticle {
            position: random_position(),
            velocity: random_velocity(),
        })
        .with_rule(Rule::Gravity(9.8))
        .with_rule(Rule::BounceWalls)
        .run();
}
```

## Design Philosophy

RDPE is built around three core ideas:

1. **Declarative Rules** - Describe *what* should happen, not *how*. Rules like `Gravity`, `Separate`, and `Cohere` express intent clearly.

2. **Composability** - Rules combine freely. Wrap any rule with `Typed` for type-specific interactions. Use `Custom` for anything not built-in.

3. **GPU-First** - Everything runs on the GPU. The derive macro handles memory layout. Spatial hashing accelerates neighbor queries. You write Rust; RDPE generates WGSL shaders.

## How It Works

1. You define a particle struct with `#[derive(Particle)]`
2. You configure a simulation with rules
3. RDPE generates GPU shaders from your rules
4. The simulation runs entirely on the GPU
5. A window displays the particles in real-time

The next chapters explain each component in detail.
