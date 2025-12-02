# Custom Functions

Custom functions let you define reusable WGSL code that can be called from your rules. This keeps complex logic organized and avoids code duplication.

## Defining Functions

Use `.with_function()` to add WGSL functions:

```rust
Simulation::<Particle>::new()
    .with_function(r#"
        fn swirl(pos: vec3<f32>, strength: f32) -> vec3<f32> {
            let d = length(pos.xz);
            return vec3(-pos.z, 0.0, pos.x) * strength / (d + 0.1);
        }
    "#)
    .with_rule(Rule::Custom(r#"
        p.velocity += swirl(p.position, 2.0) * uniforms.delta_time;
    "#.into()))
    .run();
```

## Function Scope

Your custom functions have access to:
- All WGSL built-in functions (`sin`, `cos`, `length`, `normalize`, etc.)
- Built-in utility functions (see [Shader Utilities](./shader-utilities.md))
- Other custom functions defined before this one
- The `Particle` struct type

They do **not** have direct access to:
- The `uniforms` struct (pass values as parameters instead)
- The particle arrays (operate on values passed to the function)

## Multiple Functions

Add multiple functions for complex effects:

```rust
.with_function(r#"
    fn wave_height(x: f32, z: f32, t: f32) -> f32 {
        return sin(x * 3.0 + t) * cos(z * 2.0 + t * 0.7) * 0.2;
    }
"#)
.with_function(r#"
    fn wave_force(pos: vec3<f32>, t: f32) -> vec3<f32> {
        let h = wave_height(pos.x, pos.z, t);
        let target_y = h;
        return vec3(0.0, (target_y - pos.y) * 2.0, 0.0);
    }
"#)
.with_rule(Rule::Custom(r#"
    p.velocity += wave_force(p.position, uniforms.time) * uniforms.delta_time;
"#.into()))
```

## Example: Orbital Mechanics

```rust
.with_function(r#"
    fn gravity_force(pos: vec3<f32>, center: vec3<f32>, mass: f32) -> vec3<f32> {
        let diff = center - pos;
        let dist_sq = dot(diff, diff);
        if dist_sq < 0.01 {
            return vec3(0.0);
        }
        let dist = sqrt(dist_sq);
        return normalize(diff) * mass / dist_sq;
    }
"#)
.with_function(r#"
    fn orbital_velocity(pos: vec3<f32>, center: vec3<f32>, mass: f32) -> vec3<f32> {
        let r = length(pos - center);
        let speed = sqrt(mass / r);
        let radial = normalize(pos - center);
        // Perpendicular to radial, in XZ plane
        return vec3(-radial.z, 0.0, radial.x) * speed;
    }
"#)
```

## Example: Turbulence

Combine custom functions with built-in noise:

```rust
.with_function(r#"
    fn turbulence(pos: vec3<f32>, time: f32, strength: f32) -> vec3<f32> {
        let scale = 2.0;
        let t = time * 0.5;
        return vec3(
            noise3(pos * scale + vec3(t, 0.0, 0.0)),
            noise3(pos * scale + vec3(0.0, t, 100.0)),
            noise3(pos * scale + vec3(100.0, 0.0, t))
        ) * strength;
    }
"#)
.with_rule(Rule::Custom(r#"
    p.velocity += turbulence(p.position, uniforms.time, 1.5) * uniforms.delta_time;
"#.into()))
```

## Tips

- **Parameter passing**: Pass uniforms as function parameters rather than accessing them directly.
- **Return types**: Always specify return types for WGSL functions.
- **Order matters**: Functions can only call functions defined before them.
- **Keep it simple**: Complex logic is fine, but avoid excessive branching for GPU performance.
