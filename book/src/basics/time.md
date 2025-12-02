# Time

RDPE provides a `Time` module as the universal source of truth for all timing-related values in the simulation. It uses `std::time` internally with no external dependencies.

## Overview

The `Time` struct tracks:
- **Elapsed time** - total time since the simulation started
- **Delta time** - time since the last frame (for frame-rate independent movement)
- **Frame count** - total frames rendered
- **FPS** - calculated frames per second

## Basic Usage

```rust
use rdpe::time::Time;

let mut time = Time::new();

// In your game/simulation loop:
loop {
    let (elapsed, delta) = time.update();

    // Use elapsed for time-based effects
    let wave = (elapsed * 2.0).sin();

    // Use delta for frame-rate independent movement
    position += velocity * delta;
}
```

## Accessing Time Values

```rust
time.update();  // Call once per frame

// Get individual values
let elapsed = time.elapsed();     // Total seconds since start
let delta = time.delta();         // Seconds since last frame
let frame = time.frame();         // Frame count (u64)
let fps = time.fps();             // Calculated FPS
```

## Time Control

### Pausing

```rust
time.pause();           // Pause - delta becomes 0, elapsed stops
time.resume();          // Resume from where it left off
time.toggle_pause();    // Toggle pause state

if time.is_paused() {
    // Handle paused state
}
```

### Time Scale

Slow motion or fast-forward effects:

```rust
time.set_time_scale(0.5);  // Half speed (slow motion)
time.set_time_scale(1.0);  // Normal speed
time.set_time_scale(2.0);  // Double speed

let scale = time.time_scale();  // Get current scale
```

### Fixed Timestep

For deterministic physics simulations:

```rust
// Use fixed 60 FPS timestep regardless of actual frame rate
time.set_fixed_delta(Some(1.0 / 60.0));

// Return to real frame timing
time.set_fixed_delta(None);
```

### Reset

```rust
time.reset();  // Reset to initial state (elapsed = 0, frame = 0, etc.)
```

## Duration Access

For cases where you need `std::time::Duration` instead of `f32`:

```rust
let elapsed_duration = time.elapsed_duration();  // Duration
let delta_duration = time.delta_duration();      // Duration
let start = time.start_instant();                // Instant
```

## Integration with Simulation

The `Time` module is automatically used internally by `Simulation`. The values are passed to:
- Your update callback via `UpdateContext`
- GPU uniforms as `uniforms.time` and `uniforms.delta_time`

```rust
Simulation::<MyParticle>::new()
    .with_update(|ctx| {
        // ctx.time and ctx.delta_time come from the Time module
        println!("Time: {:.2}s, Delta: {:.4}s", ctx.time, ctx.delta_time);
    })
    .run();
```

In your WGSL rules, access time via uniforms:

```wgsl
// In custom rules or shaders
let t = uniforms.time;
let dt = uniforms.delta_time;

// Time-based oscillation
p.position.y += sin(t * 2.0) * 0.1 * dt;
```

## API Reference

| Method | Returns | Description |
|--------|---------|-------------|
| `new()` | `Time` | Create a new time tracker |
| `update()` | `(f32, f32)` | Update and return (elapsed, delta) |
| `elapsed()` | `f32` | Total elapsed seconds |
| `delta()` | `f32` | Seconds since last frame |
| `frame()` | `u64` | Total frame count |
| `fps()` | `f32` | Calculated FPS |
| `is_paused()` | `bool` | Whether time is paused |
| `time_scale()` | `f32` | Current time scale multiplier |
| `pause()` | `()` | Pause time progression |
| `resume()` | `()` | Resume time progression |
| `toggle_pause()` | `()` | Toggle pause state |
| `set_time_scale(f32)` | `()` | Set time scale (0.0+) |
| `set_fixed_delta(Option<f32>)` | `()` | Set fixed timestep |
| `reset()` | `()` | Reset to initial state |
