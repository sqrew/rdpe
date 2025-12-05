# Multi-Particle Types

The `MultiParticle` derive macro lets you define multiple particle types with different fields in a single enum, then use them together in heterogeneous simulations.

## The Problem

With regular `#[derive(Particle)]`, all particles in a simulation share the same struct. If you want boids with a `flock_id` and predators with `hunger`, you'd need to put both fields on every particle:

```rust
#[derive(Particle, Clone)]
struct Creature {
    position: Vec3,
    velocity: Vec3,
    particle_type: u32,
    flock_id: u32,   // Only boids use this
    hunger: f32,     // Only predators use this
}
```

This works, but it's awkward and wastes memory.

## The Solution: MultiParticle

`MultiParticle` lets you define each type with only its relevant fields:

```rust
#[derive(MultiParticle, Clone)]
enum Creature {
    Boid {
        position: Vec3,
        velocity: Vec3,
        flock_id: u32,
    },
    Predator {
        position: Vec3,
        velocity: Vec3,
        hunger: f32,
        target_id: u32,
    },
}
```

From this single definition, the macro generates:

1. **Standalone structs** - `Boid` and `Predator` as separate types, each implementing `ParticleTrait`
2. **Type constants** - `Creature::BOID` and `Creature::PREDATOR` for use in typed rules
3. **Unified GPU struct** - A combined struct with all fields for the GPU
4. **WGSL helpers** - Constants (`BOID`, `PREDATOR`) and functions (`is_boid()`, `is_predator()`)

## Creating Particles

Use clean struct-like enum syntax:

```rust
// Boid with its specific fields
Creature::Boid {
    position: pos,
    velocity: vel,
    flock_id: 0,
}

// Predator with its specific fields
Creature::Predator {
    position: pos,
    velocity: vel,
    hunger: 1.0,
    target_id: 0,
}
```

## Type Constants in Rules

The generated constants make typed rules self-documenting:

```rust
// Predators chase boids
.with_rule(Rule::Chase {
    self_type: Creature::PREDATOR,
    target_type: Creature::BOID,
    radius: 0.5,
    strength: 3.5,
})

// Boids evade predators
.with_rule(Rule::Evade {
    self_type: Creature::BOID,
    threat_type: Creature::PREDATOR,
    radius: 0.3,
    strength: 5.0,
})

// Boids flock with other boids
.with_rule(Rule::Typed {
    self_type: Creature::BOID,
    other_type: Some(Creature::BOID),
    rule: Box::new(Rule::Cohere { radius: 0.15, strength: 1.2 }),
})
```

## WGSL Helpers

In custom rules, use the generated helpers to access variant-specific fields:

```rust
.with_rule(Rule::Custom(r#"
    // Check type with helper function
    if is_predator(p) {
        // Access predator-specific field
        p.hunger = max(0.0, p.hunger - uniforms.delta_time * 0.1);

        // Hungry predators move faster
        let speed_boost = 1.0 + (1.0 - p.hunger) * 0.5;
        p.velocity *= speed_boost;
    }

    if is_boid(p) {
        // Access boid-specific field
        let flock = p.flock_id;
    }
"#.into()))
```

The generated WGSL includes:

```wgsl
// Type constants
const BOID: u32 = 0u;
const PREDATOR: u32 = 1u;

// Helper functions
fn is_boid(p: Particle) -> bool { return p.particle_type == 0u; }
fn is_predator(p: Particle) -> bool { return p.particle_type == 1u; }
```

## Standalone Simulations

The generated structs work independently too:

```rust
// Mixed simulation
Simulation::<Creature>::new()

// Boid-only simulation (uses generated Boid struct)
Simulation::<Boid>::new()

// Predator-only simulation (uses generated Predator struct)
Simulation::<Predator>::new()
```

## Requirements

- The enum must also derive `Clone`
- Each variant must have `position: Vec3` and `velocity: Vec3`
- Use struct-like syntax (named fields, not tuple variants)

## How It Works

On the GPU, all particles share a unified struct containing every field from every variant:

```wgsl
struct Particle {
    position: vec3<f32>,
    velocity: vec3<f32>,
    flock_id: u32,      // From Boid
    hunger: f32,        // From Predator
    target_id: u32,     // From Predator
    particle_type: u32, // Discriminant
    // ... lifecycle fields
}
```

When a `Creature::Boid` is converted to GPU format:
- `flock_id` is set from the boid's value
- `hunger` and `target_id` are zeroed
- `particle_type` is set to `0` (BOID)

This means accessing the "wrong" variant's fields in WGSL just reads zeros - it's safe but meaningless. Always check `particle_type` or use the helper functions first.

## Complete Example

```rust
use rdpe::prelude::*;

#[derive(MultiParticle, Clone)]
enum Creature {
    Boid {
        position: Vec3,
        velocity: Vec3,
        flock_id: u32,
    },
    Predator {
        position: Vec3,
        velocity: Vec3,
        hunger: f32,
    },
}

fn main() {
    Simulation::<Creature>::new()
        .with_particle_count(1000)
        .with_spawner(|i, count| {
            if i < count * 9 / 10 {
                Creature::Boid {
                    position: random_position(),
                    velocity: Vec3::ZERO,
                    flock_id: i % 3,
                }
            } else {
                Creature::Predator {
                    position: random_position(),
                    velocity: Vec3::ZERO,
                    hunger: 1.0,
                }
            }
        })
        // Boid flocking
        .with_rule(Rule::Typed {
            self_type: Creature::BOID,
            other_type: Some(Creature::BOID),
            rule: Box::new(Rule::Cohere { radius: 0.15, strength: 1.0 }),
        })
        // Predator hunting
        .with_rule(Rule::Chase {
            self_type: Creature::PREDATOR,
            target_type: Creature::BOID,
            radius: 0.5,
            strength: 3.0,
        })
        // Prey evasion
        .with_rule(Rule::Evade {
            self_type: Creature::BOID,
            threat_type: Creature::PREDATOR,
            radius: 0.3,
            strength: 5.0,
        })
        .run();
}
```

## When to Use MultiParticle

| Use Case                         | Approach                                    |
|----------------------------------|---------------------------------------------|
| Single particle type             | Regular `#[derive(Particle)]`               |
| Multiple types, same fields      | `ParticleType` enum + `particle_type` field |
| Multiple types, different fields | `#[derive(MultiParticle)]`                  |

`MultiParticle` shines when your types genuinely need different data - predators tracking hunger, boids tracking flock membership, infected particles tracking infection time, etc.
