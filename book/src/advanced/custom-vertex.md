# Custom Vertex Shaders

Vertex shaders control how particles are positioned and oriented in 3D space. Use them for rotation, wobble, size pulsing, custom billboarding, and screen-space effects.

## Basic Usage

```rust
Simulation::<MyParticle>::new()
    .with_vertex_shader(r#"
        // Your vertex transformation code here
        let world_pos = vec4<f32>(particle_pos, 1.0);
        var clip_pos = uniforms.view_proj * world_pos;
        clip_pos.x += quad_pos.x * particle_size * clip_pos.w;
        clip_pos.y += quad_pos.y * particle_size * clip_pos.w;

        out.clip_position = clip_pos;
        out.color = particle_color;
        out.uv = quad_pos;
        return out;
    "#)
    .run();
```

## Available Variables

### Inputs

| Variable | Type | Description |
|----------|------|-------------|
| `vertex_index` | `u32` | Which vertex of the quad (0-5) |
| `instance_index` | `u32` | Which particle (use for per-particle variation) |
| `particle_pos` | `vec3<f32>` | World position of the particle |
| `particle_color` | `vec3<f32>` | Color (from `#[color]` field or position-based) |
| `scale` | `f32` | Per-particle scale factor |
| `quad_pos` | `vec2<f32>` | Quad vertex offset (-1 to 1) |
| `particle_size` | `f32` | Base particle size × scale |
| `base_size` | `f32` | Raw base particle size |
| `uniforms.time` | `f32` | Seconds since simulation start |
| `uniforms.view_proj` | `mat4x4<f32>` | View-projection matrix |
| `uniforms.*` | varies | Any custom uniforms |

### Required Outputs

You **must** set these in your shader:

| Variable | Type | Description |
|----------|------|-------------|
| `out.clip_position` | `vec4<f32>` | Final clip-space position |
| `out.color` | `vec3<f32>` | Color passed to fragment shader |
| `out.uv` | `vec2<f32>` | UV coordinates for fragment shader |

## How It Works

Each particle is rendered as a billboard quad (2 triangles, 6 vertices). Your vertex shader runs for each vertex of each particle. The `quad_pos` variable tells you which corner of the quad you're processing:

```
(-1,-1) -------- (1,-1)
   |               |
   |    (0,0)      |
   |               |
(-1,1) --------- (1,1)
```

The standard approach:
1. Transform `particle_pos` to clip space using `uniforms.view_proj`
2. Offset the clip position by `quad_pos * particle_size`
3. Set `out.clip_position`, `out.color`, and `out.uv`

## Common Patterns

### Rotating Particles

```rust
.with_vertex_shader(r#"
    // Per-particle rotation speed based on index
    let speed = 2.0 + f32(instance_index % 10u) * 0.3;
    let angle = uniforms.time * speed;
    let cos_a = cos(angle);
    let sin_a = sin(angle);

    // Rotate the quad
    let rotated = vec2<f32>(
        quad_pos.x * cos_a - quad_pos.y * sin_a,
        quad_pos.x * sin_a + quad_pos.y * cos_a
    );

    let world_pos = vec4<f32>(particle_pos, 1.0);
    var clip_pos = uniforms.view_proj * world_pos;
    clip_pos.x += rotated.x * particle_size * clip_pos.w;
    clip_pos.y += rotated.y * particle_size * clip_pos.w;

    out.clip_position = clip_pos;
    out.color = particle_color;
    out.uv = rotated;
    return out;
"#)
```

Rotation looks best with `ParticleShape::Square` so you can see it spin.

### Wobbling Particles

```rust
.with_vertex_shader(r#"
    let freq = 3.0;
    let amp = 0.02;

    // Per-particle phase offset
    let phase = f32(instance_index) * 0.5;

    let wobble = vec3<f32>(
        sin(uniforms.time * freq + phase) * amp,
        cos(uniforms.time * freq * 1.3 + phase * 0.7) * amp,
        sin(uniforms.time * freq * 0.7 + phase * 0.3) * amp
    );

    let world_pos = vec4<f32>(particle_pos + wobble, 1.0);
    var clip_pos = uniforms.view_proj * world_pos;
    clip_pos.x += quad_pos.x * particle_size * clip_pos.w;
    clip_pos.y += quad_pos.y * particle_size * clip_pos.w;

    out.clip_position = clip_pos;
    out.color = particle_color;
    out.uv = quad_pos;
    return out;
"#)
```

### Pulsing Size

```rust
.with_vertex_shader(r#"
    // Each particle pulses at a slightly different phase
    let phase = f32(instance_index) * 0.2;
    let pulse = 1.0 + sin(uniforms.time * 4.0 + phase) * 0.3;
    let size = particle_size * pulse;

    let world_pos = vec4<f32>(particle_pos, 1.0);
    var clip_pos = uniforms.view_proj * world_pos;
    clip_pos.x += quad_pos.x * size * clip_pos.w;
    clip_pos.y += quad_pos.y * size * clip_pos.w;

    out.clip_position = clip_pos;
    out.color = particle_color;
    out.uv = quad_pos;
    return out;
"#)
```

### Combined Effects

```rust
.with_vertex_shader(r#"
    // Rotation
    let angle = uniforms.time * 2.0 + f32(instance_index) * 0.1;
    let cos_a = cos(angle);
    let sin_a = sin(angle);
    let rotated = vec2<f32>(
        quad_pos.x * cos_a - quad_pos.y * sin_a,
        quad_pos.x * sin_a + quad_pos.y * cos_a
    );

    // Wobble
    let wobble = vec3<f32>(
        sin(uniforms.time * 3.0 + f32(instance_index) * 0.5) * 0.02,
        cos(uniforms.time * 3.9 + f32(instance_index) * 0.7) * 0.02,
        0.0
    );

    // Pulse
    let pulse = 1.0 + sin(uniforms.time * 4.0 + f32(instance_index) * 0.2) * 0.2;

    let world_pos = vec4<f32>(particle_pos + wobble, 1.0);
    var clip_pos = uniforms.view_proj * world_pos;
    clip_pos.x += rotated.x * particle_size * pulse * clip_pos.w;
    clip_pos.y += rotated.y * particle_size * pulse * clip_pos.w;

    out.clip_position = clip_pos;
    out.color = particle_color;
    out.uv = rotated;
    return out;
"#)
```

### Wave Pattern

```rust
.with_vertex_shader(r#"
    // Coordinated wave across all particles
    let wave_dir = vec3<f32>(0.0, 1.0, 0.0);
    let freq = 5.0;
    let speed = 2.0;
    let amp = 0.03;

    let wave_phase = dot(particle_pos, wave_dir) * freq - uniforms.time * speed;
    let offset = wave_dir * sin(wave_phase) * amp;

    let world_pos = vec4<f32>(particle_pos + offset, 1.0);
    var clip_pos = uniforms.view_proj * world_pos;
    clip_pos.x += quad_pos.x * particle_size * clip_pos.w;
    clip_pos.y += quad_pos.y * particle_size * clip_pos.w;

    out.clip_position = clip_pos;
    out.color = particle_color;
    out.uv = quad_pos;
    return out;
"#)
```

## Tips

### Per-Particle Variation

Use `instance_index` to make each particle behave differently:

```rust
// Different speeds
let speed = 1.0 + f32(instance_index % 5u) * 0.5;

// Random-ish phase offset
let phase = f32(instance_index) * 0.123;

// Staggered animation
let delay = f32(instance_index) * 0.01;
let t = max(uniforms.time - delay, 0.0);
```

### Clip Space Math

After `uniforms.view_proj * world_pos`, you're in clip space where:
- Position is divided by `w` for perspective
- Multiply offsets by `clip_pos.w` to maintain constant screen size

```rust
// Correct: constant screen size regardless of depth
clip_pos.x += quad_pos.x * size * clip_pos.w;

// Wrong: particles shrink with distance
clip_pos.x += quad_pos.x * size;
```

### Debugging

Output debug values as colors:

```rust
// Visualize instance_index
out.color = vec3<f32>(
    f32(instance_index % 256u) / 255.0,
    f32((instance_index / 256u) % 256u) / 255.0,
    0.0
);

// Visualize quad position
out.color = vec3<f32>(quad_pos * 0.5 + 0.5, 0.0);
```

### Performance

- Vertex shaders run 6× per particle (once per quad vertex)
- Keep math simple for large particle counts
- Avoid branches when possible

## Vertex Effects vs Custom Shaders

For common effects, consider using the pre-built [`VertexEffect`](../basics/visuals.md#vertex-effects) system instead:

```rust
// Pre-built effects (composable, type-safe)
.with_vertex_effect(VertexEffect::Rotate { speed: 2.0 })
.with_vertex_effect(VertexEffect::Wobble { frequency: 3.0, amplitude: 0.05 })
.with_vertex_effect(VertexEffect::Pulse { frequency: 4.0, amplitude: 0.3 })
```

Use custom vertex shaders when you need:
- Effects not covered by `VertexEffect`
- Complex conditional logic
- Access to multiple particle fields
- Custom billboarding behavior

**Note:** If both are specified, custom shaders take precedence and effects are ignored.

## Related

- [Visual Configuration](../basics/visuals.md) - Vertex effects, shapes, blend modes
- [Fragment Shaders](./fragment-shaders.md) - Customize particle appearance
- [Custom Uniforms](./custom-uniforms.md) - Pass data to shaders
