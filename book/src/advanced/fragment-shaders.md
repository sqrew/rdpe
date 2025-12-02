# Fragment Shaders

Fragment shaders control how each particle looks - its shape, glow, color effects, and more.

## Basic Usage

```rust
Simulation::<MyParticle>::new()
    .with_fragment_shader(r#"
        let dist = length(in.uv);
        let glow = 1.0 / (dist * dist * 8.0 + 0.3);
        return vec4<f32>(in.color * glow, glow * 0.5);
    "#)
    .run();
```

## Available Variables

In your fragment shader snippet, you have access to:

| Variable | Type | Description |
|----------|------|-------------|
| `in.uv` | `vec2<f32>` | Position within particle quad (-1 to 1, center is 0) |
| `in.color` | `vec3<f32>` | Particle's color (from `#[color]` field) |
| `uniforms.time` | `f32` | Seconds since simulation start |
| `uniforms.delta_time` | `f32` | Seconds since last frame |
| `uniforms.*` | varies | Any custom uniforms defined via `.with_uniform()` |

## How It Works

Your snippet is injected into a fragment shader that runs for every pixel of every particle. The `in.uv` coordinates tell you where you are within the particle's billboard quad:

```
(-1,-1) -------- (1,-1)
   |               |
   |    (0,0)      |
   |               |
(-1,1) --------- (1,1)
```

The center is `(0,0)`, so `length(in.uv)` gives distance from center.

## Common Patterns

### Soft Circle (Default Look)

```rust
.with_fragment_shader(r#"
    let dist = length(in.uv);
    if dist > 1.0 { discard; }
    let alpha = 1.0 - smoothstep(0.0, 1.0, dist);
    return vec4<f32>(in.color, alpha);
"#)
```

### Glowing Particle

```rust
.with_fragment_shader(r#"
    let dist = length(in.uv);
    let glow = 1.0 / (dist * dist * 8.0 + 0.3);
    let alpha = clamp(glow * 0.5, 0.0, 1.0);
    return vec4<f32>(in.color * glow, alpha);
"#)
```

### Ring

```rust
.with_fragment_shader(r#"
    let dist = length(in.uv);
    let ring = smoothstep(0.6, 0.7, dist) - smoothstep(0.8, 0.9, dist);
    return vec4<f32>(in.color, ring);
"#)
```

### Pulsing

```rust
.with_fragment_shader(r#"
    let dist = length(in.uv);
    let pulse = sin(uniforms.time * 4.0) * 0.3 + 0.7;
    let glow = 1.0 / (dist * dist * 8.0 + 0.2);
    return vec4<f32>(in.color * glow * pulse, glow * 0.5);
"#)
```

### Animated Interference

```rust
.with_fragment_shader(r#"
    let dist = length(in.uv);

    // Core
    let core = 1.0 - smoothstep(0.0, 0.3, dist);

    // Animated rings
    let rings = sin(dist * 20.0 - uniforms.time * 5.0) * 0.5 + 0.5;
    let ring_fade = exp(-dist * 3.0);

    let intensity = core + rings * ring_fade * 0.5;
    return vec4<f32>(in.color * intensity, intensity * 0.6);
"#)
```

### Color Shift Based on Position

```rust
.with_fragment_shader(r#"
    let dist = length(in.uv);
    let glow = 1.0 / (dist * dist * 6.0 + 0.3);

    // Shift hue based on angle
    let angle = atan2(in.uv.y, in.uv.x);
    let hue_shift = angle / 6.28318;

    // Simple hue rotation (approximate)
    let shifted = vec3<f32>(
        in.color.r * cos(hue_shift * 6.28) - in.color.g * sin(hue_shift * 6.28),
        in.color.r * sin(hue_shift * 6.28) + in.color.g * cos(hue_shift * 6.28),
        in.color.b
    );

    return vec4<f32>(shifted * glow, glow * 0.5);
"#)
```

### Sharp Core + Soft Halo

```rust
.with_fragment_shader(r#"
    let dist = length(in.uv);

    // Sharp inner core
    let core = 1.0 - smoothstep(0.0, 0.2, dist);

    // Soft outer glow
    let halo = 1.0 / (dist * dist * 4.0 + 0.5);

    let intensity = core * 2.0 + halo * 0.5;
    let alpha = clamp(intensity * 0.4, 0.0, 1.0);

    return vec4<f32>(in.color * intensity, alpha);
"#)
```

## Tips

### Coordinate System

- `in.uv` ranges from -1 to 1
- `length(in.uv)` = distance from center (0 at center, 1 at edge, >1 at corners)
- Use `in.uv * 0.5 + 0.5` to get 0-1 range for texture coordinates

### Performance

- Fragment shaders run per-pixel per-particle
- Keep math simple for thousands of particles
- Avoid loops if possible

### Blending

Fragment shader output interacts with blend mode:
- **Additive**: RGB values add together (bright + bright = brighter)
- **Alpha**: Standard alpha compositing

For additive blending, the alpha channel still matters for intensity.

### Debugging

Set solid colors to debug:

```rust
// Debug: show UV coordinates as colors
.with_fragment_shader(r#"
    return vec4<f32>(in.uv * 0.5 + 0.5, 0.0, 1.0);
"#)
```

## Related

- [Visual Configuration](../visuals.md) - Blend modes, trails, connections
- [Post-Processing](./post-processing.md) - Screen-space effects
