# Particles

Particles are the core data structure in RDPE. Each particle has properties that the GPU updates every frame based on the rules you define.

## Defining a Particle

Use the `#[derive(Particle)]` macro:

```rust
use rdpe::prelude::*;

#[derive(Particle, Clone)]
struct MyParticle {
    position: Vec3,  // Required: where the particle is
    velocity: Vec3,  // Required: how it's moving
}
```

The `position` and `velocity` fields are required by convention - rules expect them.

## Supported Field Types

| Rust Type | WGSL Type | Size | Alignment |
|-----------|-----------|------|-----------|
| `Vec3` | `vec3<f32>` | 12 bytes | 16 bytes |
| `Vec2` | `vec2<f32>` | 8 bytes | 8 bytes |
| `Vec4` | `vec4<f32>` | 16 bytes | 16 bytes |
| `f32` | `f32` | 4 bytes | 4 bytes |
| `u32` | `u32` | 4 bytes | 4 bytes |
| `i32` | `i32` | 4 bytes | 4 bytes |

The macro automatically adds padding for GPU alignment.

## Optional Fields

### Color

Mark a `Vec3` field with `#[color]` to control particle color:

```rust
#[derive(Particle, Clone)]
struct ColoredParticle {
    position: Vec3,
    velocity: Vec3,
    #[color]
    color: Vec3,  // RGB, 0.0-1.0
}
```

Without a color field, particles are colored based on their position.

### Particle Type

Add `particle_type: u32` for type-based interactions:

```rust
#[derive(Particle, Clone)]
struct TypedParticle {
    position: Vec3,
    velocity: Vec3,
    particle_type: u32,  // 0, 1, 2, etc.
}
```

If you don't add this field, it's auto-added with a default value of 0.

## Auto-Injected Lifecycle Fields

The `#[derive(Particle)]` macro automatically adds these lifecycle fields to every particle:

| Field | Type | Default | Purpose |
|-------|------|---------|---------|
| `particle_type` | `u32` | 0 | Type identifier for typed interactions |
| `age` | `f32` | 0.0 | Time since spawn (updated by `Rule::Age`) |
| `alive` | `u32` | 1 | 1 = alive, 0 = dead (set by `Rule::Lifetime`) |
| `scale` | `f32` | 1.0 | Per-particle size multiplier (used by `Rule::ShrinkOut`) |

These are always available in your WGSL code via `p.age`, `p.alive`, `p.scale`, even if you don't define them in your struct.

```rust
// These fields exist automatically:
.with_rule(Rule::Age)                    // Increments p.age each frame
.with_rule(Rule::Lifetime(5.0))          // Sets p.alive = 0 when p.age > 5.0
.with_rule(Rule::ShrinkOut(5.0))         // Scales p.scale from 1.0 to 0.0
```

## Spawning Particles

The spawner function is called once per particle at initialization:

```rust
.with_spawner(|index, total_count| {
    MyParticle {
        position: Vec3::new(
            rand::random::<f32>() - 0.5,
            rand::random::<f32>() - 0.5,
            rand::random::<f32>() - 0.5,
        ),
        velocity: Vec3::ZERO,
    }
})
```

Parameters:
- `index` - Particle index (0 to count-1)
- `total_count` - Total number of particles

Since the spawner must be `Send + Sync`, pre-generate random values:

```rust
let mut rng = rand::thread_rng();
let positions: Vec<Vec3> = (0..count)
    .map(|_| Vec3::new(rng.gen_range(-1.0..1.0), ...))
    .collect();

.with_spawner(move |i, _| MyParticle {
    position: positions[i as usize],
    velocity: Vec3::ZERO,
})
```

## Custom Fields

You can add any supported fields for custom logic:

```rust
#[derive(Particle, Clone)]
struct GameParticle {
    position: Vec3,
    velocity: Vec3,
    #[color]
    color: Vec3,
    particle_type: u32,
    health: f32,      // Custom field
    energy: f32,      // Custom field
    team_id: u32,     // Custom field
}
```

Access these in `Rule::Custom` WGSL code:

```rust
.with_rule(Rule::Custom(r#"
    // Drain energy over time
    p.energy -= uniforms.delta_time * 0.1;

    // Use auto-injected age for time-based effects
    let fade = 1.0 - (p.age / 5.0);
    p.color *= fade;

    // Mark as dead when health depleted
    if p.health <= 0.0 {
        p.alive = 0u;
    }
"#.to_string()))
```
