# Rules

Rules define how particles behave. They're applied every frame in the order you add them.

## Physics Rules

### Gravity

Applies constant downward acceleration:

```rust
Rule::Gravity(9.8)  // Strength in units/second²
```

### Drag

Slows particles over time (air resistance):

```rust
Rule::Drag(2.0)  // 0.0 = no drag, higher = more friction
```

### Acceleration

Constant acceleration in any direction:

```rust
Rule::Acceleration(Vec3::new(0.0, -9.8, 0.0))
```

### BounceWalls

Particles bounce off the bounding box:

```rust
Rule::BounceWalls
```

The bounds are set with `.with_bounds(size)` - creates a cube from `-size` to `+size`.

### WrapWalls

Particles wrap around to the opposite side (toroidal topology):

```rust
Rule::WrapWalls
```

Creates an infinite-feeling space where particles exiting one edge reappear on the other. Great for simulations where you don't want edge effects or want the arena to feel larger than it is.

## Force Rules

### AttractTo

Pull particles toward a point:

```rust
Rule::AttractTo {
    point: Vec3::ZERO,
    strength: 5.0,
}
```

### RepelFrom

Push particles away from a point:

```rust
Rule::RepelFrom {
    point: Vec3::new(0.0, 0.0, 0.0),
    strength: 10.0,
    radius: 0.5,  // Only affects particles within this distance
}
```

## Movement Rules

### Wander

Random wandering force for organic, natural movement:

```rust
Rule::Wander {
    strength: 0.5,    // How strong the random force is
    frequency: 100.0, // How fast direction changes (higher = jittery)
}
```

Each particle gets its own random direction based on a hash of its index and time.

### SpeedLimit

Clamp velocity to min/max bounds:

```rust
Rule::SpeedLimit {
    min: 0.1,  // Minimum speed (use 0.0 for no minimum)
    max: 2.0,  // Maximum speed
}
```

Useful for keeping simulations stable and preventing runaway velocities.

## Neighbor Rules

These rules require spatial hashing (automatically enabled when used).

### Separate

Particles avoid crowding neighbors:

```rust
Rule::Separate {
    radius: 0.1,      // Detection distance
    strength: 2.0,    // Push force
}
```

### Cohere

Particles steer toward the center of nearby neighbors:

```rust
Rule::Cohere {
    radius: 0.3,      // Detection distance
    strength: 1.0,    // Pull force
}
```

### Align

Particles match velocity with neighbors:

```rust
Rule::Align {
    radius: 0.2,      // Detection distance
    strength: 1.5,    // Alignment force
}
```

### Collide

Particle-particle collision response:

```rust
Rule::Collide {
    radius: 0.05,     // Collision distance
    response: 0.5,    // Bounce strength
}
```

## Type Rules

### Typed

Wraps any neighbor rule with type filters:

```rust
Rule::Typed {
    self_type: 0,           // This rule applies to type 0 particles
    other_type: Some(1),    // Only interact with type 1 neighbors
    rule: Box::new(Rule::Separate { radius: 0.1, strength: 5.0 }),
}
```

Use `other_type: None` to interact with all types.

### Convert

Changes particle type on contact:

```rust
Rule::Convert {
    from_type: 0,       // Healthy
    trigger_type: 1,    // Infected
    to_type: 1,         // Becomes infected
    radius: 0.08,       // Contact distance
    probability: 0.1,   // 10% chance per neighbor per frame
}
```

### Chase

Steer toward the nearest particle of a target type:

```rust
Rule::Chase {
    self_type: 1,       // Predators (type 1)
    target_type: 0,     // Chase prey (type 0)
    radius: 0.3,        // How far can see targets
    strength: 2.0,      // Steering force
}
```

Finds the closest visible target and steers toward it. Great for predator-prey dynamics.

### Evade

Steer away from the nearest particle of a threat type:

```rust
Rule::Evade {
    self_type: 0,       // Prey (type 0)
    threat_type: 1,     // Flee from predators (type 1)
    radius: 0.2,        // How far can see threats
    strength: 3.0,      // Steering force (often higher than chase)
}
```

Finds the closest visible threat and steers away. Combine with Chase for predator-prey simulations.

## Custom Rules

For anything not built-in, write raw WGSL:

```rust
Rule::Custom(r#"
    // Access particle as 'p'
    p.velocity.y += sin(uniforms.time) * 0.1;

    // Available variables:
    // - p: current particle (read/write)
    // - index: particle index (u32)
    // - uniforms.time: elapsed time (f32)
    // - uniforms.delta_time: frame time (f32)
"#.to_string())
```

## Rule Order

Rules execute in the order added. A typical order:

```rust
.with_rule(Rule::Gravity(9.8))           // 1. Apply forces
.with_rule(Rule::Wander { ... })         // 2. Random movement
.with_rule(Rule::Separate { ... })       // 3. Neighbor interactions
.with_rule(Rule::Cohere { ... })
.with_rule(Rule::SpeedLimit { ... })     // 4. Clamp velocity
.with_rule(Rule::Drag(1.0))              // 5. Apply drag
.with_rule(Rule::BounceWalls)            // 6. Boundary conditions
```

Velocity integration (`position += velocity * dt`) happens automatically after all rules.

## Spatial Configuration

For neighbor rules, configure the spatial hash:

```rust
.with_spatial_config(cell_size, grid_resolution)
```

- `cell_size` - Should be >= your largest interaction radius
- `grid_resolution` - Must be power of 2 (16, 32, 64, etc.)

Example: For a simulation with bounds of 1.0 and max interaction radius of 0.1:

```rust
.with_bounds(1.0)
.with_spatial_config(0.1, 32)  // 32³ cells covering -1.6 to +1.6
```
