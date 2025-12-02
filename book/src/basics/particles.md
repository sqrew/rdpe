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
    age: f32,         // Custom field
    team_id: u32,     // Custom field
}
```

Access these in `Rule::Custom` WGSL code:

```rust
.with_rule(Rule::Custom(r#"
    p.age += uniforms.delta_time;
    if p.health < 0.0 {
        p.particle_type = 2u; // Dead
    }
"#.to_string()))
```
