# 3D Spatial Fields

Fields are persistent 3D grids that particles can read from and write to. Unlike particle-to-particle interactions, fields provide a shared spatial medium for indirect communication and environmental effects.

## Use Cases

- **Pheromone trails**: Particles deposit chemicals, others follow gradients (slime mold)
- **Density fields**: Accumulate particle presence for fluid-like behavior
- **Temperature/heat**: Particles emit or absorb heat from the environment
- **Flow fields**: Pre-computed or dynamic velocity guidance

## Basic Usage

Add a field with `.with_field()` and access it in custom rules:

```rust
Simulation::<Agent>::new()
    .with_field(
        "pheromone",
        FieldConfig::new(64)      // 64³ grid resolution
            .with_extent(1.0)     // World space: -1 to +1
            .with_decay(0.98)     // Fade each frame
            .with_blur(0.1)       // Diffusion strength
    )
    .with_rule(Rule::Custom(r#"
        // Deposit pheromone at current position
        field_write(0u, p.position, 0.1);

        // Read pheromone concentration
        let concentration = field_read(0u, p.position);

        // Color based on local concentration
        p.color = vec3<f32>(0.0, concentration, 0.5);
    "#.into()))
    .run();
```

## Field Configuration

```rust
FieldConfig::new(resolution)
    .with_extent(extent)           // World bounds (-extent to +extent)
    .with_decay(decay)             // Per-frame multiplier (0.0-1.0)
    .with_blur(blur)               // Diffusion strength (0.0-1.0)
    .with_blur_iterations(n)       // Blur passes per frame (1-3 typical)
```

| Parameter | Range | Effect |
|-----------|-------|--------|
| `resolution` | 8-256 | Grid cells per axis (memory = resolution³ × 4 bytes) |
| `extent` | > 0 | World space coverage (should match simulation bounds) |
| `decay` | 0.0-1.0 | 0.99 = slow fade, 0.5 = fast fade, 1.0 = permanent |
| `blur` | 0.0-1.0 | 0.0 = no spread, 0.5 = heavy diffusion |

### Memory Usage

| Resolution | Total Cells | Memory |
|------------|-------------|--------|
| 32³ | 32,768 | ~128 KB |
| 64³ | 262,144 | ~1 MB |
| 128³ | 2,097,152 | ~8 MB |

## Shader Functions

Fields are accessed by index (0, 1, 2...) in the order they were registered.

### `field_write(field_idx, position, value)`

Atomically adds a value to the field at the given world position. Multiple particles writing to the same cell accumulate their values.

```wgsl
// Deposit pheromone
field_write(0u, p.position, 0.1);

// Deposit more in high-velocity areas
field_write(0u, p.position, length(p.velocity) * 0.05);
```

### `field_read(field_idx, position)`

Samples the field at a world position using trilinear interpolation for smooth values between grid cells.

```wgsl
// Read at current position
let here = field_read(0u, p.position);

// Read ahead (for steering)
let ahead = field_read(0u, p.position + normalize(p.velocity) * 0.1);
```

### `field_gradient(field_idx, position, epsilon)`

Computes the gradient (direction of steepest increase) at a position. Useful for steering toward higher concentrations.

```wgsl
// Get gradient direction
let grad = field_gradient(0u, p.position, 0.05);

// Steer toward higher values
p.velocity += normalize(grad) * 0.5 * uniforms.delta_time;
```

## Example: Slime Mold

Classic Physarum simulation where agents deposit pheromones and follow gradients:

```rust
#[derive(Particle, Clone)]
struct SlimeAgent {
    position: Vec3,
    velocity: Vec3,
    #[color]
    color: Vec3,
    heading: f32,  // Direction angle in XZ plane
}

Simulation::<SlimeAgent>::new()
    .with_particle_count(30_000)
    .with_field(
        "pheromone",
        FieldConfig::new(64)
            .with_extent(1.2)
            .with_decay(0.98)
            .with_blur(0.1)
    )
    .with_uniform::<f32>("sense_dist", 0.1)
    .with_uniform::<f32>("sense_angle", 0.4)
    .with_uniform::<f32>("turn_speed", 4.0)
    .with_rule(Rule::Custom(r#"
        let dt = uniforms.delta_time;

        // Deposit pheromone trail
        field_write(0u, p.position, 0.2);

        // Sense in three directions
        let fwd = vec3<f32>(cos(p.heading), 0.0, sin(p.heading));
        let left_ang = p.heading + uniforms.sense_angle;
        let right_ang = p.heading - uniforms.sense_angle;

        let sense_fwd = p.position + fwd * uniforms.sense_dist;
        let sense_left = p.position + vec3<f32>(cos(left_ang), 0.0, sin(left_ang)) * uniforms.sense_dist;
        let sense_right = p.position + vec3<f32>(cos(right_ang), 0.0, sin(right_ang)) * uniforms.sense_dist;

        let val_fwd = field_read(0u, sense_fwd);
        let val_left = field_read(0u, sense_left);
        let val_right = field_read(0u, sense_right);

        // Turn toward highest concentration
        if val_left > val_fwd && val_left > val_right {
            p.heading += uniforms.turn_speed * dt;
        } else if val_right > val_fwd {
            p.heading -= uniforms.turn_speed * dt;
        }

        // Move forward
        p.position += vec3<f32>(cos(p.heading), 0.0, sin(p.heading)) * 0.5 * dt;
        p.position.y = 0.0;

        // Color by local pheromone
        let pheromone = field_read(0u, p.position);
        p.color = vec3<f32>(0.1, 0.3 + pheromone * 0.5, 0.2);

        p.velocity = vec3<f32>(0.0);  // We handle movement directly
    "#.into()))
    .run();
```

## Example: Heat Diffusion

Particles emit heat that spreads through the field:

```rust
Simulation::<Particle>::new()
    .with_field(
        "temperature",
        FieldConfig::new(32)
            .with_decay(0.995)    // Slow cooling
            .with_blur(0.3)       // Fast heat spread
            .with_blur_iterations(2)
    )
    .with_rule(Rule::Custom(r#"
        // Hot particles emit heat
        if p.color.r > 0.5 {
            field_write(0u, p.position, 0.1);
        }

        // All particles absorb ambient temperature
        let temp = field_read(0u, p.position);
        p.color = vec3<f32>(temp, 0.2, 1.0 - temp);
    "#.into()))
    .run();
```

## Multiple Fields

Register multiple fields for complex simulations. Each field can have independent resolution, decay, blur, and extent settings. Fields are accessed by index in registration order.

```rust
Simulation::<Agent>::new()
    .with_field("food", FieldConfig::new(64).with_decay(0.99))       // Index 0
    .with_field("danger", FieldConfig::new(32).with_decay(0.9).with_blur(0.2))  // Index 1
    .with_rule(Rule::Custom(r#"
        let food = field_read(0u, p.position);      // Field 0
        let danger = field_read(1u, p.position);    // Field 1

        // Seek food, avoid danger
        let food_grad = field_gradient(0u, p.position, 0.05);
        let danger_grad = field_gradient(1u, p.position, 0.05);

        p.velocity += food_grad * 2.0 - danger_grad * 5.0;
    "#.into()))
    .run();
```

### Example: Competing Teams

Two particle teams, each depositing to their own field and avoiding the other:

```rust
Simulation::<Agent>::new()
    // Each team gets its own pheromone field
    .with_field("red_pheromone", FieldConfig::new(48).with_decay(0.97).with_blur(0.15))
    .with_field("blue_pheromone", FieldConfig::new(48).with_decay(0.97).with_blur(0.15))
    .with_rule(Rule::Custom(r#"
        // p.team is 0 or 1
        let my_field = p.team;
        let other_field = 1u - p.team;

        // Deposit to my team's field
        field_write(my_field, p.position, 0.15);

        // Sense ahead
        let ahead = p.position + normalize(p.velocity) * 0.1;

        // Follow my team, avoid other team
        let my_val = field_read(my_field, ahead);
        let other_val = field_read(other_field, ahead);
        let score = my_val - other_val * 1.5;

        // Steer based on combined score
        let my_grad = field_gradient(my_field, p.position, 0.05);
        let other_grad = field_gradient(other_field, p.position, 0.05);
        p.velocity += (my_grad - other_grad * 1.5) * uniforms.delta_time;
    "#.into()))
    .run();
```

See `examples/multi_field.rs` for a complete working example.

## Tips

- **Match extents**: Set field extent to match or exceed your simulation bounds
- **Resolution tradeoffs**: Higher resolution = more detail but more memory and slower blur
- **Decay vs blur**: High decay + low blur = sharp trails; low decay + high blur = ambient clouds
- **2D simulations**: For XZ-plane simulations, the Y axis still exists but particles sample from y=0
- **Performance**: Field processing adds GPU overhead; start with 32³ or 64³ resolution
