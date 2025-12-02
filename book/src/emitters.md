# Emitters

Emitters continuously spawn new particles into the simulation, replacing dead particles. They enable effects like fountains, explosions, rain, and any other continuous particle generation.

## How Emitters Work

When you add an emitter, RDPE:
1. Finds dead particles (those with `alive == 0`)
2. Respawns them based on the emitter's rate and position
3. Sets initial velocity according to the emitter type

Emitters work best with lifecycle rules:
- `Rule::Age` - increments particle age each frame
- `Rule::Lifetime(seconds)` - kills particles after a duration

## Emitter Types

### Point

Emits particles from a single point in all directions.

```rust
.with_emitter(Emitter::Point {
    position: Vec3::ZERO,
    rate: 500.0,    // particles per second
    speed: 1.0,     // initial speed (0 = random)
})
```

### Burst

One-time explosion of particles. Fires once at simulation start.

```rust
.with_emitter(Emitter::Burst {
    position: Vec3::new(0.0, 0.5, 0.0),
    count: 1000,    // total particles to spawn
    speed: 3.0,     // outward speed
})
```

### Cone

Directional emission in a cone shape. Great for fountains, jets, and thrusters.

```rust
.with_emitter(Emitter::Cone {
    position: Vec3::new(0.0, -0.5, 0.0),
    direction: Vec3::Y,     // points up
    speed: 2.5,
    spread: 0.3,            // cone half-angle in radians
    rate: 800.0,
})
```

The `spread` parameter controls the cone width:
- `0.0` = laser beam (no spread)
- `0.3` = tight cone (~17 degrees)
- `PI/4` = 45-degree cone
- `PI/2` = hemisphere

### Sphere

Spawns particles on a sphere surface, moving outward (or inward).

```rust
.with_emitter(Emitter::Sphere {
    center: Vec3::ZERO,
    radius: 0.5,
    speed: 1.0,     // positive = outward, negative = inward
    rate: 1000.0,
})
```

### Box

Spawns particles at random positions within a box volume.

```rust
.with_emitter(Emitter::Box {
    min: Vec3::new(-1.0, 1.0, -1.0),
    max: Vec3::new(1.0, 1.2, 1.0),
    velocity: Vec3::new(0.0, -2.0, 0.0),  // falling rain
    rate: 2000.0,
})
```

## Complete Example

Here's a fountain that continuously emits particles:

```rust
use rdpe::prelude::*;

#[derive(Particle, Clone)]
struct Drop {
    position: Vec3,
    velocity: Vec3,
    #[color]
    color: Vec3,
}

fn main() {
    Simulation::<Drop>::new()
        .with_particle_count(10_000)
        .with_bounds(2.0)
        // Start all particles dead - emitter will spawn them
        .with_spawner(|_, _| Drop {
            position: Vec3::ZERO,
            velocity: Vec3::ZERO,
            color: Vec3::new(0.3, 0.6, 1.0),
        })
        // Cone emitter shooting upward
        .with_emitter(Emitter::Cone {
            position: Vec3::new(0.0, -0.8, 0.0),
            direction: Vec3::Y,
            speed: 3.0,
            spread: 0.2,
            rate: 1000.0,
        })
        // Lifecycle management
        .with_rule(Rule::Age)
        .with_rule(Rule::Lifetime(2.0))
        // Physics
        .with_rule(Rule::Gravity(4.0))
        .with_rule(Rule::Drag(0.3))
        .with_rule(Rule::BounceWalls)
        .run();
}
```

## Multiple Emitters

You can add multiple emitters to create complex effects:

```rust
// Twin fountains
.with_emitter(Emitter::Cone {
    position: Vec3::new(-0.5, -0.8, 0.0),
    direction: Vec3::Y,
    speed: 3.0,
    spread: 0.15,
    rate: 500.0,
})
.with_emitter(Emitter::Cone {
    position: Vec3::new(0.5, -0.8, 0.0),
    direction: Vec3::Y,
    speed: 3.0,
    spread: 0.15,
    rate: 500.0,
})
```

## Tips

- **Rate tuning**: Match your rate to particle count and lifetime. If `rate * lifetime > particle_count`, you'll run out of dead particles to respawn.
- **Dead start**: When using emitters, initialize particles as dead in your spawner (they'll be spawned by the emitter).
- **Burst timing**: Burst emitters fire at `time < 0.1`, so they work immediately on startup.
