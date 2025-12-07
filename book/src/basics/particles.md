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

The spawner function receives a [`SpawnContext`] with helper methods for common patterns:

```rust
.with_spawner(|ctx| MyParticle {
    position: ctx.random_in_sphere(0.8),
    velocity: ctx.random_direction() * 0.2,
})
```

### SpawnContext Methods

**Info:**
- `ctx.index` - Particle index (0 to count-1)
- `ctx.count` - Total particle count
- `ctx.bounds` - Simulation bounds
- `ctx.progress()` - Normalized progress (0.0 to 1.0)

**Positions:**
- `ctx.random_in_sphere(radius)` - Random point inside a sphere
- `ctx.random_on_sphere(radius)` - Random point on sphere surface
- `ctx.random_in_cube(half_size)` - Random point in cube
- `ctx.random_in_bounds()` - Random within simulation bounds
- `ctx.random_in_cylinder(radius, half_height)` - Random in cylinder
- `ctx.random_in_disk(radius)` - Random in XZ disk

**Directions/Velocities:**
- `ctx.random_direction()` - Random unit vector
- `ctx.tangent_velocity(pos, speed)` - Velocity perpendicular to position (orbits)
- `ctx.outward_velocity(pos, speed)` - Velocity pointing away from origin

**Colors:**
- `ctx.random_color()` - Random RGB
- `ctx.random_hue(saturation, value)` - Random hue, fixed saturation/value
- `ctx.rainbow(saturation, value)` - Color based on spawn progress
- `ctx.hsv(hue, saturation, value)` - Specific HSV color

**Structured Layouts:**
- `ctx.grid_position(cols, rows, layers)` - 3D grid position
- `ctx.grid_position_2d(cols, rows)` - 2D grid in XZ plane
- `ctx.line_position(start, end)` - Point along a line
- `ctx.circle_position(radius)` - Point on a circle
- `ctx.helix_position(radius, height, turns)` - Point on a helix

**Random Values:**
- `ctx.random()` - f32 0.0 to 1.0
- `ctx.random_range(min, max)` - f32 in range

### Examples

**Swirling particles:**

```rust
.with_spawner(|ctx| {
    let pos = ctx.random_in_sphere(0.6);
    let speed = ctx.random_range(0.2, 0.5);
    Spark {
        position: pos,
        velocity: ctx.tangent_velocity(pos, speed),
        color: ctx.rainbow(0.9, 1.0),
    }
})
```

**Grid layout:**

```rust
.with_spawner(|ctx| Ball {
    position: ctx.grid_position(10, 10, 10),
    velocity: Vec3::ZERO,
})
```

**Type-based spawn:**

```rust
.with_spawner(|ctx| {
    let is_predator = ctx.index < 50;
    Creature {
        position: ctx.random_in_bounds(),
        velocity: ctx.random_direction() * 0.1,
        particle_type: if is_predator { 1 } else { 0 },
    }
})
```

### Pre-generated Data

For complex initialization, pre-generate and capture:

```rust
let particles: Vec<MyParticle> = generate_complex_data();

.with_spawner(move |ctx| particles[ctx.index as usize].clone())
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
