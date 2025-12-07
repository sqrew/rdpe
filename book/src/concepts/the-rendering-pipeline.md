# The Rendering Pipeline

Understanding RDPE's rendering pipeline helps you know where to customize and what each stage controls.

## Pipeline Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                         COMPUTE PHASE                           │
├─────────────────────────────────────────────────────────────────┤
│  Rules execute in order, updating particle state on GPU         │
│  (position, velocity, color, custom fields)                     │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                         RENDER PHASE                            │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────────────┐ │
│  │   Vertex    │ → │  Fragment   │ → │   Post-Processing   │ │
│  │   Shader    │    │   Shader    │    │      Shader         │ │
│  └─────────────┘    └─────────────┘    └─────────────────────┘ │
│                                                                 │
│  Position &         Pixel color &       Screen-space           │
│  orientation        transparency        effects                 │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
                        ┌───────────┐
                        │  Screen   │
                        └───────────┘
```

## Stage 1: Compute (Rules)

**When:** Every frame, before rendering
**What:** Updates particle state (position, velocity, custom fields)
**Customize with:** `Rule::Custom()`, `.with_rule()`

```rust
.with_rule(Rule::Gravity(9.8))
.with_rule(Rule::Custom(r#"
    p.velocity += some_force * uniforms.delta_time;
"#.into()))
```

Rules run sequentially in the order you add them. Each rule reads the current particle state and writes updates. This is where physics, flocking, interactions, and behavior logic live.

**See:** [Rules](../basics/rules.md), [Custom Rules](../advanced/custom-rules.md)

## Stage 2: Vertex Shader

**When:** For each vertex of each particle (6 per quad)
**What:** Positions particles in 3D space, handles billboarding
**Customize with:** `.with_vertex_shader()`, `.with_vertex_effect()`

```rust
// Pre-built effects
.with_vertex_effect(VertexEffect::Rotate { speed: 2.0 })
.with_vertex_effect(VertexEffect::Wobble { frequency: 3.0, amplitude: 0.05 })

// Or full custom control
.with_vertex_shader(r#"
    // Transform particle to screen
"#)
```

The vertex shader transforms world positions to screen positions. By default, particles are screen-facing billboards. You can add rotation, wobble, size pulsing, or custom billboarding here.

**Inputs:** `particle_pos`, `particle_color`, `scale`, `quad_pos`, `uniforms.time`
**Outputs:** `clip_position`, `color`, `uv`

**See:** [Vertex Shaders](../advanced/custom-vertex.md), [Visual Configuration](../basics/visuals.md#vertex-effects)

## Stage 3: Fragment Shader

**When:** For each pixel of each particle
**What:** Determines the color and transparency of each pixel
**Customize with:** `.with_fragment_shader()`, `v.shape()`

```rust
// Pre-built shapes
.with_visuals(|v| v.shape(ParticleShape::Star))

// Or custom appearance
.with_fragment_shader(r#"
    let dist = length(in.uv);
    let glow = 1.0 / (dist * dist * 8.0 + 0.3);
    return vec4<f32>(in.color * glow, glow * 0.5);
"#)
```

The fragment shader runs for every pixel covered by a particle quad. It decides what color that pixel should be (or discards it entirely). Shapes, glows, rings, and per-particle visual effects happen here.

**Inputs:** `in.uv` (position within quad), `in.color`, `uniforms.time`
**Outputs:** `vec4<f32>` (RGBA color)

**See:** [Fragment Shaders](../advanced/fragment-shaders.md), [Visual Configuration](../basics/visuals.md#particle-shapes)

## Stage 4: Blending

**When:** As fragments are written to the framebuffer
**What:** Combines overlapping particle colors
**Customize with:** `v.blend_mode()`

```rust
.with_visuals(|v| {
    v.blend_mode(BlendMode::Additive);  // Overlap = brighter
    // or
    v.blend_mode(BlendMode::Alpha);     // Standard transparency
})
```

| Mode       | Effect                                 | Best For                   |
|------------|----------------------------------------|----------------------------|
| `Additive` | Colors add together, overlap brightens | Glows, fire, energy, light |
| `Alpha`    | Standard transparency compositing      | Smoke, dust, solid objects |

## Stage 5: Post-Processing

**When:** After all particles are rendered
**What:** Screen-space effects on the final image
**Customize with:** `v.post_process()`

```rust
.with_visuals(|v| {
    v.post_process(r#"
        let color = textureSample(scene, scene_sampler, in.uv);
        let dist = length(in.uv - vec2(0.5));
        let vignette = 1.0 - smoothstep(0.3, 0.9, dist);
        return vec4(color.rgb * vignette, 1.0);
    "#);
})
```

Post-processing operates on the entire rendered image. Use it for vignettes, color grading, bloom, distortion, or any effect that needs to see the whole scene.

**Inputs:** `scene` texture, `in.uv` (screen coordinates 0-1)
**Outputs:** Final pixel color

**See:** [Post-Processing](../advanced/post-processing.md)

## Special Render Modes

### Trails

```rust
v.trails(8);  // Store 8 frames of history
```

Trails render previous particle positions as fading copies. The trail buffer is updated each frame in the compute phase and rendered as additional instances.

### Connections

```rust
v.connections(0.1);  // Connect particles within distance 0.1
```

After particle rendering, a separate pass draws lines between nearby particles. Uses spatial hashing for efficient neighbor queries.

### Wireframes

```rust
v.wireframe(WireframeMesh::cube(), 0.002);
```

Instead of billboard quads, particles are rendered as 3D wireframe meshes. Each particle becomes a complete wireframe shape that rotates and scales.

## Customization Summary

| What to Change                   | Where to Customize                        |
|----------------------------------|-------------------------------------------|
| Particle behavior, physics       | `Rule::Custom()`                          |
| Position, rotation, billboarding | `.with_vertex_shader()` or `VertexEffect` |
| Shape, glow, per-pixel look      | `.with_fragment_shader()` or `v.shape()`  |
| How overlap combines             | `v.blend_mode()`                          |
| Full-screen effects              | `v.post_process()`                        |

## Data Flow

```
Particle Buffer (GPU)
    │
    ├── position: Vec3
    ├── velocity: Vec3
    ├── color: Vec3 (optional)
    ├── scale: f32
    ├── alive: u32
    └── custom fields...
         │
         ▼
    ┌─────────┐
    │  Rules  │ ← Read & write particle data
    └─────────┘
         │
         ▼
    ┌─────────┐
    │ Vertex  │ ← Read particle data, output clip positions
    └─────────┘
         │
         ▼
    ┌─────────┐
    │Fragment │ ← Read interpolated color/UV, output pixel color
    └─────────┘
         │
         ▼
    ┌─────────┐
    │ Blend   │ ← Combine with existing framebuffer
    └─────────┘
         │
         ▼
    ┌─────────┐
    │  Post   │ ← Read framebuffer, apply screen effects
    └─────────┘
         │
         ▼
      Screen
```

## Performance Implications

| Stage           | Runs Per            | Cost Scales With                |
|-----------------|---------------------|---------------------------------|
| Compute (Rules) | Particle            | Particle count, rule complexity |
| Vertex          | Vertex (6/particle) | Particle count                  |
| Fragment        | Pixel               | Particle count × particle size  |
| Post-Process    | Screen pixel        | Screen resolution               |

For maximum performance:
- Keep rules simple, especially neighbor-based ones
- Use smaller particle sizes when possible
- Avoid complex fragment shaders with many particles
- Post-processing cost is fixed regardless of particle count
