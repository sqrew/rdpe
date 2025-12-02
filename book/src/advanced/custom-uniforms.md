# Custom Uniforms

Custom uniforms let you pass dynamic values from Rust to your shader code every frame. This enables interactive simulations that respond to time, mouse input, or any other runtime data.

## Basic Usage

Define uniforms with `.with_uniform()` and access them in `Rule::Custom`:

```rust
Simulation::<Particle>::new()
    .with_uniform("target", Vec3::ZERO)
    .with_uniform("strength", 1.0f32)
    .with_rule(Rule::Custom(r#"
        let dir = uniforms.target - p.position;
        p.velocity += normalize(dir) * uniforms.strength * uniforms.delta_time;
    "#.into()))
    .run();
```

In your shader code, access uniforms via the `uniforms` struct:
- `uniforms.time` - simulation time in seconds (built-in)
- `uniforms.delta_time` - time since last frame (built-in)
- `uniforms.your_name` - your custom uniforms

## Supported Types

| Rust Type | WGSL Type | Example |
|-----------|-----------|---------|
| `f32` | `f32` | `1.0f32` |
| `i32` | `i32` | `-5i32` |
| `u32` | `u32` | `10u32` |
| `Vec2` | `vec2<f32>` | `Vec2::new(1.0, 2.0)` |
| `Vec3` | `vec3<f32>` | `Vec3::new(1.0, 2.0, 3.0)` |
| `Vec4` | `vec4<f32>` | `Vec4::new(1.0, 2.0, 3.0, 4.0)` |

## Updating Uniforms at Runtime

Use `.with_update()` to modify uniforms every frame:

```rust
Simulation::<Particle>::new()
    .with_uniform("attractor", Vec3::ZERO)
    .with_uniform("active", 0.0f32)
    .with_update(|ctx| {
        // Time-based animation
        let t = ctx.time();
        ctx.set("attractor", Vec3::new(t.cos(), 0.0, t.sin()));

        // Mouse interaction
        if ctx.mouse_pressed() {
            ctx.set("active", 1.0f32);
        } else {
            ctx.set("active", 0.0f32);
        }
    })
    .run();
```

### UpdateContext API

The `ctx` parameter provides:

| Method | Returns | Description |
|--------|---------|-------------|
| `ctx.time()` | `f32` | Simulation time in seconds |
| `ctx.delta_time()` | `f32` | Time since last frame |
| `ctx.mouse_ndc()` | `Option<Vec2>` | Mouse in normalized device coords (-1 to 1) |
| `ctx.mouse_pressed()` | `bool` | Is left mouse button down? |
| `ctx.set(name, value)` | - | Update a uniform value |
| `ctx.get(name)` | `Option<&UniformValue>` | Read current uniform value |

## Example: Mouse Attractor

Particles are attracted to the mouse when clicked:

```rust
Simulation::<Mote>::new()
    .with_particle_count(15_000)
    .with_spawner(|_, _| /* ... */)
    .with_uniform("attractor", Vec3::ZERO)
    .with_uniform("strength", 0.0f32)
    .with_update(|ctx| {
        if ctx.mouse_pressed() {
            if let Some(mouse) = ctx.mouse_ndc() {
                // Map NDC to world space (approximate)
                ctx.set("attractor", Vec3::new(
                    mouse.x * 2.0,
                    mouse.y * 2.0,
                    0.0
                ));
                ctx.set("strength", 5.0f32);
            }
        } else {
            ctx.set("strength", 0.0f32);
        }
    })
    .with_rule(Rule::Custom(r#"
        if uniforms.strength > 0.0 {
            let to_attractor = uniforms.attractor - p.position;
            let dist = length(to_attractor);
            if dist > 0.01 {
                let dir = to_attractor / dist;
                let force = uniforms.strength / (dist * dist + 0.5);
                p.velocity += dir * force * uniforms.delta_time;
            }
        }
    "#.into()))
    .with_rule(Rule::Drag(1.5))
    .run();
```

## Example: Pulsing Attractor

Automatic attraction/repulsion cycle:

```rust
.with_uniform("strength", 1.0f32)
.with_update(|ctx| {
    let cycle = ctx.time() % 4.0;
    let strength = if cycle < 3.0 {
        3.0   // Attract for 3 seconds
    } else {
        -5.0  // Repel for 1 second
    };
    ctx.set("strength", strength);
})
```

## Tips

- **Initialize all uniforms**: Always set initial values with `.with_uniform()` before using `.with_update()`.
- **Type suffixes**: Use `1.0f32` not `1.0` to ensure correct type inference.
- **NDC coordinates**: Mouse NDC ranges from -1 to 1 on both axes, with Y up.
