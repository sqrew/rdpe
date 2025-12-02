# Textures

RDPE supports custom textures that can be sampled in fragment shaders and post-processing effects. This enables color lookup tables, noise-based effects, sprites, and more.

## Quick Start

```rust
use rdpe::prelude::*;

Simulation::<MyParticle>::new()
    .with_texture("noise", TextureConfig::noise(256, 42))
    .with_fragment_shader(r#"
        let n = textureSample(tex_noise, tex_noise_sampler, in.uv * 0.5 + 0.5);
        return vec4<f32>(in.color * n.r, 1.0);
    "#)
    .run();
```

## Adding Textures

Use `.with_texture(name, config)` to add textures to your simulation:

```rust
.with_texture("gradient", TextureConfig::gradient(256, start, end))
.with_texture("pattern", TextureConfig::from_file("assets/pattern.png"))
```

Each texture you add becomes available in shaders as:
- `tex_name` - the texture itself
- `tex_name_sampler` - the sampler for that texture

## Creating Textures

### From Image Files

Load PNG or JPEG images:

```rust
TextureConfig::from_file("assets/noise.png")
TextureConfig::from_file("assets/sprite.jpg")
```

### Procedural Noise

Generate hash-based noise textures:

```rust
TextureConfig::noise(256, 42)  // 256x256, seed 42
TextureConfig::noise(512, 0)   // 512x512, seed 0
```

### Color Gradients

Create horizontal gradient textures (great for color lookup tables):

```rust
TextureConfig::gradient(
    256,                        // width
    [0, 0, 0, 255],            // start color (RGBA)
    [255, 200, 50, 255],       // end color (RGBA)
)
```

### Solid Colors

Single-pixel solid color textures:

```rust
TextureConfig::solid(255, 0, 0, 255)  // Red
TextureConfig::solid(0, 255, 0, 128)  // Semi-transparent green
```

### Checkerboard Patterns

```rust
TextureConfig::checkerboard(
    64,                         // size (64x64)
    8,                          // cell size
    [255, 255, 255, 255],      // color 1
    [0, 0, 0, 255],            // color 2
)
```

### Raw RGBA Data

Create textures from raw pixel data:

```rust
let data = vec![
    255, 0, 0, 255,    // Red pixel
    0, 255, 0, 255,    // Green pixel
    0, 0, 255, 255,    // Blue pixel
    255, 255, 0, 255,  // Yellow pixel
];
TextureConfig::from_rgba(data, 2, 2)  // 2x2 texture
```

## Texture Configuration

### Filter Mode

Control how textures are sampled between pixels:

```rust
TextureConfig::from_file("sprite.png")
    .with_filter(FilterMode::Nearest)  // Sharp pixels (pixel art)

TextureConfig::noise(256, 0)
    .with_filter(FilterMode::Linear)   // Smooth interpolation (default)
```

### Address Mode

Control what happens when UV coordinates go outside 0-1:

```rust
TextureConfig::from_file("tile.png")
    .with_address_mode(AddressMode::Repeat)       // Tile the texture
    .with_address_mode(AddressMode::ClampToEdge)  // Use edge pixels (default)
    .with_address_mode(AddressMode::MirrorRepeat) // Mirror at boundaries
```

## Sampling in Shaders

### In Fragment Shaders

```rust
.with_fragment_shader(r#"
    // Sample at particle UV (normalized quad coordinates)
    let color = textureSample(tex_sprite, tex_sprite_sampler, in.uv * 0.5 + 0.5);

    // Sample using custom coordinates
    let noise = textureSample(tex_noise, tex_noise_sampler, in.world_pos.xy);

    return vec4<f32>(color.rgb * noise.r, color.a);
"#)
```

### In Post-Processing

```rust
.with_visuals(|v| {
    v.post_process(r#"
        let scene_color = textureSample(scene, scene_sampler, in.uv);
        let noise = textureSample(tex_noise, tex_noise_sampler, in.uv * 10.0);

        // Film grain effect
        let grain = (noise.r - 0.5) * 0.1;
        return vec4<f32>(scene_color.rgb + grain, 1.0);
    "#);
})
```

## Common Use Cases

### Color Lookup Tables (LUTs)

Use gradients to map values to colors:

```rust
// Fire gradient: black -> red -> orange -> yellow -> white
let fire_lut = TextureConfig::gradient(256, [0, 0, 0, 255], [255, 255, 200, 255]);

.with_texture("fire_lut", fire_lut)
.with_fragment_shader(r#"
    // Use particle temperature/intensity to look up color
    let intensity = length(in.velocity) / max_speed;
    let color = textureSample(tex_fire_lut, tex_fire_lut_sampler, vec2<f32>(intensity, 0.5));
    return color;
"#)
```

### Noise-Based Effects

Add visual variation:

```rust
.with_texture("noise", TextureConfig::noise(256, 42))
.with_fragment_shader(r#"
    let n = textureSample(tex_noise, tex_noise_sampler, in.uv * 0.5 + 0.5).r;

    // Vary particle brightness
    let brightness = 0.5 + n * 0.5;

    // Vary particle edges
    let dist = length(in.uv);
    let edge = smoothstep(0.5 * n, 0.0, dist);

    return vec4<f32>(in.color * brightness, edge);
"#)
```

### Sprite Textures

Use image textures for particle appearance:

```rust
.with_texture("sprite",
    TextureConfig::from_file("assets/particle.png")
        .with_filter(FilterMode::Linear))
.with_fragment_shader(r#"
    let sprite = textureSample(tex_sprite, tex_sprite_sampler, in.uv * 0.5 + 0.5);
    return vec4<f32>(sprite.rgb * in.color, sprite.a);
"#)
```

## Complete Example

```rust
use rdpe::prelude::*;

#[derive(Particle, Clone)]
struct GlowParticle {
    position: Vec3,
    velocity: Vec3,
    #[color]
    color: Vec3,
}

fn main() {
    // Create textures
    let noise = TextureConfig::noise(256, 42);
    let gradient = TextureConfig::gradient(
        256,
        [50, 50, 200, 255],   // Blue
        [255, 100, 50, 255],  // Orange
    );

    Simulation::<GlowParticle>::new()
        .with_particle_count(10_000)
        .with_texture("noise", noise)
        .with_texture("gradient", gradient)
        .with_fragment_shader(r#"
            // Sample noise for variation
            let n = textureSample(tex_noise, tex_noise_sampler, in.uv * 0.5 + 0.5).r;

            // Use noise to look up gradient color
            let color = textureSample(tex_gradient, tex_gradient_sampler, vec2<f32>(n, 0.5));

            // Radial glow
            let dist = length(in.uv);
            let glow = 1.0 - smoothstep(0.0, 0.5, dist);

            return vec4<f32>(color.rgb * glow, glow);
        "#)
        .with_visuals(|v| {
            v.blend_mode(BlendMode::Additive);
            v.background(Vec3::ZERO);
        })
        .run();
}
```

## Related

- [Fragment Shaders](./fragment-shaders.md) - Customize particle appearance
- [Post-Processing](./post-processing.md) - Screen-space effects
- [Custom Uniforms](./custom-uniforms.md) - Pass dynamic values to shaders
