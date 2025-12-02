# Post-Processing

Post-processing applies screen-space effects to the rendered scene - things like bloom, vignette, chromatic aberration, and CRT scanlines.

## Basic Usage

```rust
.with_visuals(|v| {
    v.post_process(r#"
        let color = textureSample(scene, scene_sampler, in.uv);
        // Modify color here
        return color;
    "#);
})
```

## Available Variables

| Variable | Type | Description |
|----------|------|-------------|
| `in.uv` | `vec2<f32>` | Screen coordinates (0 to 1, top-left is origin) |
| `scene` | `texture_2d<f32>` | The rendered particle scene |
| `scene_sampler` | `sampler` | Sampler for the scene texture |
| `uniforms.time` | `f32` | Seconds since simulation start |
| `uniforms.delta_time` | `f32` | Seconds since last frame |
| `uniforms.*` | varies | Any custom uniforms defined via `.with_uniform()` |

## How It Works

After all particles are rendered to an offscreen texture, your post-process shader runs once per screen pixel. You sample the scene texture and output a modified color.

```rust
// The identity post-process (does nothing)
v.post_process(r#"
    return textureSample(scene, scene_sampler, in.uv);
"#);
```

## Common Effects

### Vignette

Darken the edges of the screen:

```rust
v.post_process(r#"
    let color = textureSample(scene, scene_sampler, in.uv);
    let center = vec2<f32>(0.5, 0.5);
    let dist = length(in.uv - center);
    let vignette = 1.0 - smoothstep(0.3, 0.9, dist);
    return vec4<f32>(color.rgb * vignette, 1.0);
"#);
```

### Chromatic Aberration

Separate RGB channels for a lens distortion effect:

```rust
v.post_process(r#"
    let aberration = 0.005;
    let r = textureSample(scene, scene_sampler, in.uv + vec2<f32>(aberration, 0.0)).r;
    let g = textureSample(scene, scene_sampler, in.uv).g;
    let b = textureSample(scene, scene_sampler, in.uv - vec2<f32>(aberration, 0.0)).b;
    return vec4<f32>(r, g, b, 1.0);
"#);
```

### Radial Chromatic Aberration

Aberration that increases toward edges:

```rust
v.post_process(r#"
    let center = vec2<f32>(0.5, 0.5);
    let uv_centered = in.uv - center;
    let dist = length(uv_centered);
    let dir = normalize(uv_centered);

    let aberration = 0.003 + dist * 0.01;

    let r = textureSample(scene, scene_sampler, in.uv + dir * aberration).r;
    let g = textureSample(scene, scene_sampler, in.uv).g;
    let b = textureSample(scene, scene_sampler, in.uv - dir * aberration).b;

    return vec4<f32>(r, g, b, 1.0);
"#);
```

### Film Grain

Add noise for a film-like quality:

```rust
v.post_process(r#"
    let color = textureSample(scene, scene_sampler, in.uv);

    // Hash-based noise
    let grain = fract(sin(dot(in.uv * 1000.0, vec2<f32>(12.9898, 78.233)) + uniforms.time) * 43758.5453);
    let noise = (grain - 0.5) * 0.03;

    return vec4<f32>(color.rgb + noise, 1.0);
"#);
```

### CRT Scanlines

Classic CRT monitor effect:

```rust
v.post_process(r#"
    let color = textureSample(scene, scene_sampler, in.uv);

    let scanline_freq = 400.0;
    let scanline = sin(in.uv.y * scanline_freq) * 0.5 + 0.5;
    let scanline_intensity = 0.15;

    let result = color.rgb * (1.0 - scanline_intensity * (1.0 - scanline));
    return vec4<f32>(result, 1.0);
"#);
```

### Barrel Distortion

CRT-style curved screen:

```rust
v.post_process(r#"
    let center = vec2<f32>(0.5, 0.5);
    let uv_centered = in.uv - center;
    let dist_sq = dot(uv_centered, uv_centered);
    let barrel = 0.1;
    let distorted_uv = center + uv_centered * (1.0 + barrel * dist_sq);

    let color = textureSample(scene, scene_sampler, distorted_uv);
    return color;
"#);
```

### Bloom (Simple)

Boost bright areas:

```rust
v.post_process(r#"
    let color = textureSample(scene, scene_sampler, in.uv);
    let luminance = dot(color.rgb, vec3<f32>(0.299, 0.587, 0.114));
    let bloom = smoothstep(0.4, 1.0, luminance) * 0.4;
    return vec4<f32>(color.rgb + color.rgb * bloom, 1.0);
"#);
```

### Color Grading

Adjust color balance:

```rust
v.post_process(r#"
    let color = textureSample(scene, scene_sampler, in.uv);

    // Warm tint (boost red, reduce blue)
    var graded = pow(color.rgb, vec3<f32>(0.95, 1.0, 1.05));

    // Contrast boost
    graded = (graded - 0.5) * 1.1 + 0.5;

    // Saturation boost
    let gray = dot(graded, vec3<f32>(0.3, 0.3, 0.3));
    graded = mix(vec3<f32>(gray), graded, 1.3);

    return vec4<f32>(clamp(graded, vec3<f32>(0.0), vec3<f32>(1.0)), 1.0);
"#);
```

### Screen Flicker

Subtle brightness variation:

```rust
v.post_process(r#"
    let color = textureSample(scene, scene_sampler, in.uv);
    let flicker = sin(uniforms.time * 60.0) * 0.02 + 1.0;
    return vec4<f32>(color.rgb * flicker, 1.0);
"#);
```

## Combining Effects

Chain multiple effects together:

```rust
v.post_process(r#"
    let center = vec2<f32>(0.5, 0.5);
    var uv = in.uv;

    // 1. Barrel distortion
    let uv_centered = uv - center;
    let dist_sq = dot(uv_centered, uv_centered);
    uv = center + uv_centered * (1.0 + 0.1 * dist_sq);

    // 2. Chromatic aberration
    let aberr = 0.004;
    let r = textureSample(scene, scene_sampler, uv + vec2<f32>(aberr, 0.0)).r;
    let g = textureSample(scene, scene_sampler, uv).g;
    let b = textureSample(scene, scene_sampler, uv - vec2<f32>(aberr, 0.0)).b;
    var color = vec3<f32>(r, g, b);

    // 3. Scanlines
    let scanline = sin(in.uv.y * 400.0) * 0.5 + 0.5;
    color *= 1.0 - 0.1 * (1.0 - scanline);

    // 4. Vignette
    let vignette_dist = length(in.uv - center);
    let vignette = 1.0 - smoothstep(0.4, 1.0, vignette_dist);
    color *= vignette;

    // 5. Flicker
    let flicker = sin(uniforms.time * 60.0) * 0.01 + 1.0;
    color *= flicker;

    return vec4<f32>(color, 1.0);
"#);
```

## Performance Tips

- Post-processing runs once per screen pixel
- Texture samples are relatively expensive
- Multiple samples (for blur) can add up quickly
- Keep blur kernel sizes small (4-8 samples)

## Related

- [Visual Configuration](../visuals.md) - Blend modes, trails, connections
- [Fragment Shaders](./fragment-shaders.md) - Per-particle appearance
