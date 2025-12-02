# Shader Utilities

RDPE includes built-in utility functions that are automatically available in all compute shaders. Use them in `Rule::Custom` or your custom functions.

## Random & Hash Functions

Pseudo-random number generation based on integer hashing.

### `hash(n: u32) -> u32`
Hash a u32 to a pseudo-random u32.

### `hash2(p: vec2<u32>) -> u32`
Hash a 2D coordinate.

### `hash3(p: vec3<u32>) -> u32`
Hash a 3D coordinate.

### `rand(seed: u32) -> f32`
Returns a random float in the range [0, 1).

```wgsl
let r = rand(index * 12345u);  // Different value per particle
```

### `rand_range(seed: u32, min_val: f32, max_val: f32) -> f32`
Returns a random float in the specified range.

```wgsl
let speed = rand_range(index, 0.5, 2.0);
```

### `rand_vec3(seed: u32) -> vec3<f32>`
Returns a random vector with components in [-1, 1]. Not normalized.

### `rand_sphere(seed: u32) -> vec3<f32>`
Returns a random point on a unit sphere (normalized).

```wgsl
let direction = rand_sphere(index * 7u);
p.velocity = direction * 2.0;
```

## Noise Functions

Gradient noise for smooth, natural-looking randomness.

### `noise2(p: vec2<f32>) -> f32`
2D simplex noise. Returns values in [-1, 1].

### `noise3(p: vec3<f32>) -> f32`
3D simplex noise. Returns values in [-1, 1].

```wgsl
// Noise-based force field
let force = vec3(
    noise3(p.position * 2.0 + uniforms.time),
    noise3(p.position * 2.0 + uniforms.time + vec3(100.0, 0.0, 0.0)),
    noise3(p.position * 2.0 + uniforms.time + vec3(0.0, 100.0, 0.0))
);
p.velocity += force * uniforms.delta_time;
```

### `fbm2(p: vec2<f32>, octaves: i32) -> f32`
2D fractal Brownian motion. Layered noise for more detail.

### `fbm3(p: vec3<f32>, octaves: i32) -> f32`
3D fractal Brownian motion.

```wgsl
// More detailed noise with 4 octaves
let turbulence = fbm3(p.position * 1.5, 4);
```

## Color Functions

Convert between color spaces.

### `hsv_to_rgb(h: f32, s: f32, v: f32) -> vec3<f32>`
Convert HSV to RGB.
- `h`: Hue [0, 1] (wraps)
- `s`: Saturation [0, 1]
- `v`: Value/brightness [0, 1]

```wgsl
// Rainbow based on particle position
let hue = (p.position.x + 1.0) * 0.5;  // Map -1..1 to 0..1
p.color = hsv_to_rgb(hue, 0.8, 1.0);
```

### `rgb_to_hsv(rgb: vec3<f32>) -> vec3<f32>`
Convert RGB to HSV. Returns `vec3(h, s, v)`.

```wgsl
let hsv = rgb_to_hsv(p.color);
let new_hue = hsv.x + 0.1;  // Shift hue
p.color = hsv_to_rgb(new_hue, hsv.y, hsv.z);
```

## Complete Example

```rust
use rdpe::prelude::*;

#[derive(Particle, Clone)]
struct Mote {
    position: Vec3,
    velocity: Vec3,
    #[color]
    color: Vec3,
}

fn main() {
    Simulation::<Mote>::new()
        .with_particle_count(25_000)
        .with_spawner(|i, _| Mote {
            position: Vec3::new(
                rand::random::<f32>() * 2.0 - 1.0,
                rand::random::<f32>() * 2.0 - 1.0,
                rand::random::<f32>() * 2.0 - 1.0,
            ),
            velocity: Vec3::ZERO,
            color: Vec3::ONE,
        })
        .with_rule(Rule::Custom(r#"
            // 3D noise force field
            let scale = 2.0;
            let t = uniforms.time * 0.3;

            let force = vec3<f32>(
                noise3(p.position * scale + vec3<f32>(t, 0.0, 0.0)),
                noise3(p.position * scale + vec3<f32>(0.0, t, 100.0)),
                noise3(p.position * scale + vec3<f32>(0.0, 100.0, t))
            );

            p.velocity += force * uniforms.delta_time * 2.0;

            // Color based on FBM noise
            let color_noise = fbm3(p.position * 1.5 + uniforms.time * 0.2, 3);
            let hue = (color_noise + 1.0) * 0.25 + 0.5;
            p.color = hsv_to_rgb(hue, 0.8, 1.0);
        "#.into()))
        .with_rule(Rule::Drag(1.0))
        .with_rule(Rule::WrapWalls)
        .run();
}
```

## Performance Notes

- **Hash functions** are very fast - use liberally
- **Noise functions** are moderately expensive - a few calls per particle is fine
- **FBM** multiplies the cost by the number of octaves
- For heavy noise use, consider lowering particle count or octaves
