# Custom Rules

When built-in rules aren't enough, `Rule::Custom` lets you write raw WGSL shader code.

## Basic Usage

```rust
.with_rule(Rule::Custom(r#"
    // Your WGSL code here
    p.velocity.y += sin(uniforms.time) * 0.1;
"#.to_string()))
```

## Available Variables

### Particle Data

```wgsl
p.position      // vec3<f32> - current position (read/write)
p.velocity      // vec3<f32> - current velocity (read/write)
p.color         // vec3<f32> - particle color (if defined)
p.particle_type // u32 - particle type
// Plus any custom fields you defined
```

### Context

```wgsl
index              // u32 - this particle's index
uniforms.time      // f32 - total elapsed time in seconds
uniforms.delta_time // f32 - time since last frame
```

### In Neighbor Loop (for neighbor rules only)

```wgsl
other_idx      // u32 - neighbor's index
other          // Particle - neighbor's data
neighbor_pos   // vec3<f32> - neighbor's position
neighbor_vel   // vec3<f32> - neighbor's velocity
neighbor_dist  // f32 - distance to neighbor
neighbor_dir   // vec3<f32> - normalized direction to neighbor
```

## Examples

### Oscillating Force

```rust
Rule::Custom(r#"
    let freq = 2.0;
    let amp = 0.5;
    p.velocity.y += sin(uniforms.time * freq) * amp * uniforms.delta_time;
"#.to_string())
```

### Color Based on Speed

```rust
Rule::Custom(r#"
    let speed = length(p.velocity);
    let normalized_speed = clamp(speed / 2.0, 0.0, 1.0);
    p.color = mix(
        vec3<f32>(0.0, 0.0, 1.0),  // Blue (slow)
        vec3<f32>(1.0, 0.0, 0.0),  // Red (fast)
        normalized_speed
    );
"#.to_string())
```

### Age-Based Behavior

```rust
#[derive(Particle, Clone)]
struct AgingParticle {
    position: Vec3,
    velocity: Vec3,
    age: f32,  // Custom field
}

// In simulation:
.with_rule(Rule::Custom(r#"
    p.age += uniforms.delta_time;

    // Slow down with age
    let age_factor = 1.0 / (1.0 + p.age * 0.1);
    p.velocity *= age_factor;

    // Change color with age
    p.color = mix(
        vec3<f32>(0.2, 1.0, 0.2),  // Young: green
        vec3<f32>(0.6, 0.3, 0.1),  // Old: brown
        clamp(p.age / 10.0, 0.0, 1.0)
    );
"#.to_string()))
```

### Vortex Force

```rust
Rule::Custom(r#"
    // Circular force around Y axis
    let to_center = -p.position;
    let tangent = vec3<f32>(-to_center.z, 0.0, to_center.x);
    let dist = length(to_center.xz);

    if dist > 0.01 {
        let vortex_strength = 1.0 / (dist + 0.1);
        p.velocity += normalize(tangent) * vortex_strength * uniforms.delta_time;
    }
"#.to_string())
```

### Pulsing Size (via Custom Field)

```rust
#[derive(Particle, Clone)]
struct PulsingParticle {
    position: Vec3,
    velocity: Vec3,
    phase: f32,  // Each particle has different phase
}

.with_rule(Rule::Custom(r#"
    // Update a "size" factor based on time and phase
    let pulse = sin(uniforms.time * 3.0 + p.phase) * 0.5 + 0.5;
    // Could use this in a custom renderer...
"#.to_string()))
```

### Random Noise Movement

```rust
Rule::Custom(r#"
    // Hash-based pseudo-random
    let seed = index ^ u32(uniforms.time * 60.0);
    let hash = (seed * 1103515245u + 12345u);

    let rx = f32((hash >> 0u) & 0xFFu) / 128.0 - 1.0;
    let ry = f32((hash >> 8u) & 0xFFu) / 128.0 - 1.0;
    let rz = f32((hash >> 16u) & 0xFFu) / 128.0 - 1.0;

    p.velocity += vec3<f32>(rx, ry, rz) * 0.1 * uniforms.delta_time;
"#.to_string())
```

## WGSL Tips

### Type Suffixes

```wgsl
let x = 1.0;      // f32
let y = 1u;       // u32
let z = 1i;       // i32
```

### Vector Construction

```wgsl
let v = vec3<f32>(1.0, 2.0, 3.0);
let v2 = vec3<f32>(0.0);  // All zeros
```

### Useful Functions

```wgsl
length(v)         // Vector magnitude
normalize(v)      // Unit vector
dot(a, b)         // Dot product
cross(a, b)       // Cross product
clamp(x, lo, hi)  // Clamp to range
mix(a, b, t)      // Linear interpolation
sin(x), cos(x)    // Trig functions
abs(x)            // Absolute value
min(a, b), max(a, b)
```

## Debugging

Custom rules can silently fail. Tips:

1. **Start simple** - Add one line at a time
2. **Check types** - WGSL is strictly typed
3. **Use color** - Set `p.color` to visualize values
4. **Check compilation** - Shader errors print on startup

```rust
// Debug: visualize a value as color
Rule::Custom(r#"
    let debug_value = length(p.velocity);
    p.color = vec3<f32>(debug_value, 0.0, 0.0);
"#.to_string())
```
