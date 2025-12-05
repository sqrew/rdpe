# Visual Configuration

RDPE provides extensive control over how particles are rendered. The `with_visuals` method configures the rendering pipeline.

## Basic Usage

```rust
Simulation::<MyParticle>::new()
    .with_visuals(|v| {
        v.background(Vec3::new(0.0, 0.0, 0.02));  // Dark blue
        v.blend_mode(BlendMode::Additive);
        v.trails(8);
        v.connections(0.1);
    })
    .run();
```

## Options

### Particle Shapes

Control the visual shape of each particle:

```rust
v.shape(ParticleShape::Circle);      // Soft circle with smooth falloff (default)
v.shape(ParticleShape::CircleHard);  // Hard-edged circle
v.shape(ParticleShape::Square);      // Square/rectangle
v.shape(ParticleShape::Ring);        // Ring/donut shape
v.shape(ParticleShape::Star);        // 5-pointed star
v.shape(ParticleShape::Triangle);    // Equilateral triangle
v.shape(ParticleShape::Hexagon);     // Regular hexagon
v.shape(ParticleShape::Diamond);     // Diamond/rhombus
v.shape(ParticleShape::Point);       // Single pixel (fastest)
```

| Shape | Best For |
|-------|----------|
| `Circle` | General purpose, soft edges |
| `CircleHard` | Sharp particles, dots |
| `Square` | Pixels, grid-based simulations |
| `Ring` | Bubbles, force fields |
| `Star` | Magic effects, sparkles |
| `Triangle` | Arrows, directional particles |
| `Hexagon` | Cells, tiles, molecules |
| `Diamond` | Crystals, gems |
| `Point` | Maximum performance, retro aesthetic |

### Background Color

Set the scene backdrop:

```rust
v.background(Vec3::new(0.0, 0.0, 0.0));  // Black
v.background(Vec3::new(0.02, 0.02, 0.04));  // Dark blue
v.background(Vec3::new(1.0, 1.0, 1.0));  // White
```

### Blend Modes

Control how overlapping particles combine:

```rust
v.blend_mode(BlendMode::Additive);  // Bright areas add up (glows, fire)
v.blend_mode(BlendMode::Alpha);     // Standard transparency (default)
```

**Additive** is ideal for:
- Glowing particles
- Fire, sparks, energy effects
- Light trails
- Anything where overlap should brighten

**Alpha** is ideal for:
- Solid particles
- Smoke, dust
- Anything where overlap should occlude

### Particle Trails

Leave a fading trail behind each particle:

```rust
v.trails(8);  // 8 frames of history
```

The number is how many previous positions to render. More = longer trails, but more GPU memory.

Trails work best with:
- Additive blending (trails glow)
- Fast-moving particles
- Dark backgrounds

### Connections

Draw lines between nearby particles:

```rust
v.connections(0.1);  // Max distance for connection
```

Creates a web/network effect. Particles within the specified distance get connected by lines.

Great for:
- Neural network visualizations
- Constellation effects
- Organic webs
- Network graphs

### Post-Processing

Apply screen-space effects to the final image:

```rust
v.post_process(r#"
    // Your WGSL code here
    let color = textureSample(scene, scene_sampler, in.uv);
    return color;
"#);
```

See [Post-Processing](./advanced/post-processing.md) for details.

## Complete Example

```rust
Simulation::<MyParticle>::new()
    .with_particle_count(5000)
    .with_visuals(|v| {
        // Dark background for contrast
        v.background(Vec3::new(0.01, 0.01, 0.02));

        // Additive blending for glow effect
        v.blend_mode(BlendMode::Additive);

        // Motion trails
        v.trails(6);

        // Connect nearby particles
        v.connections(0.08);

        // Add vignette post-processing
        v.post_process(r#"
            let color = textureSample(scene, scene_sampler, in.uv);
            let dist = length(in.uv - vec2(0.5));
            let vignette = 1.0 - smoothstep(0.3, 0.9, dist);
            return vec4(color.rgb * vignette, 1.0);
        "#);
    })
    .run();
```

## Related

- [Fragment Shaders](./advanced/fragment-shaders.md) - Customize particle appearance
- [Post-Processing](./advanced/post-processing.md) - Screen-space effects
