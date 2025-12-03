# Input Handling

RDPE provides a simple input system for keyboard and mouse interaction. Input state is available in your update callback via `UpdateContext`.

## Basic Usage

```rust
Simulation::<MyParticle>::new()
    .with_uniform::<f32>("burst", 0.0)
    .with_uniform::<[f32; 2]>("attractor", [0.0, 0.0])
    .with_update(|ctx| {
        // React to space bar press
        if ctx.input.key_pressed(KeyCode::Space) {
            ctx.set("burst", 1.0);
        }

        // Track mouse position while held
        if ctx.input.mouse_held(MouseButton::Left) {
            let pos = ctx.input.mouse_ndc();
            ctx.set("attractor", [pos.x, pos.y]);
        }
    })
    .with_rule(Rule::Custom(r#"
        // Use input in shader
        if uniforms.burst > 0.5 {
            p.velocity *= 2.0;
        }

        let target = vec3<f32>(uniforms.attractor[0], uniforms.attractor[1], 0.0);
        p.velocity += normalize(target - p.position) * 0.1;
    "#.into()))
    .run();
```

## Keyboard Input

### Key States

Three types of key queries are available:

| Method | Returns `true` when |
|--------|---------------------|
| `key_pressed(key)` | Key was just pressed this frame |
| `key_held(key)` | Key is currently down |
| `key_released(key)` | Key was just released this frame |

```rust
.with_update(|ctx| {
    // Toggle on key press (not hold)
    if ctx.input.key_pressed(KeyCode::T) {
        // Toggle something once per press
    }

    // Continuous action while held
    if ctx.input.key_held(KeyCode::W) {
        // Move forward every frame
    }

    // Cleanup on release
    if ctx.input.key_released(KeyCode::Shift) {
        // Stop sprint mode
    }
})
```

### Available Keys

```rust
use rdpe::prelude::KeyCode;

// Letters
KeyCode::A, KeyCode::B, ... KeyCode::Z

// Numbers
KeyCode::Key0, KeyCode::Key1, ... KeyCode::Key9

// Function keys
KeyCode::F1, KeyCode::F2, ... KeyCode::F12

// Arrows
KeyCode::Up, KeyCode::Down, KeyCode::Left, KeyCode::Right

// Common keys
KeyCode::Space, KeyCode::Enter, KeyCode::Escape
KeyCode::Tab, KeyCode::Backspace, KeyCode::Delete
KeyCode::Shift, KeyCode::Control, KeyCode::Alt
```

## Mouse Input

### Button States

Mouse buttons work the same as keys:

| Method | Returns `true` when |
|--------|---------------------|
| `mouse_pressed(button)` | Button was just clicked |
| `mouse_held(button)` | Button is currently down |
| `mouse_released(button)` | Button was just released |

```rust
use rdpe::prelude::MouseButton;

.with_update(|ctx| {
    if ctx.input.mouse_pressed(MouseButton::Left) {
        // Click action
    }

    if ctx.input.mouse_held(MouseButton::Right) {
        // Drag action
    }
})
```

Available buttons: `MouseButton::Left`, `MouseButton::Right`, `MouseButton::Middle`

### Mouse Position

Several position formats are available:

| Method | Returns |
|--------|---------|
| `mouse_position()` | Screen pixels `Vec2` |
| `mouse_ndc()` | Normalized coordinates (-1 to 1) `Vec2` |
| `mouse_delta()` | Movement since last frame in pixels `Vec2` |
| `scroll_delta()` | Scroll wheel movement `f32` |

```rust
.with_update(|ctx| {
    // NDC is most useful for particle interactions
    // Center of screen = (0, 0)
    // X: -1 (left) to +1 (right)
    // Y: -1 (bottom) to +1 (top)
    let pos = ctx.input.mouse_ndc();

    // Pass to shader as attractor point
    ctx.set("mouse_x", pos.x);
    ctx.set("mouse_y", pos.y);

    // Check for scroll zoom
    let scroll = ctx.input.scroll_delta();
    if scroll != 0.0 {
        // Zoom in/out
    }
})
```

## Common Patterns

### Mouse Attractor

Particles attracted to mouse position:

```rust
Simulation::<Particle>::new()
    .with_uniform::<[f32; 2]>("mouse", [0.0, 0.0])
    .with_uniform::<f32>("attract_strength", 0.0)
    .with_update(|ctx| {
        let pos = ctx.input.mouse_ndc();
        ctx.set("mouse", [pos.x, pos.y]);

        // Only attract while clicking
        let strength = if ctx.input.mouse_held(MouseButton::Left) { 2.0 } else { 0.0 };
        ctx.set("attract_strength", strength);
    })
    .with_rule(Rule::Custom(r#"
        let target = vec3<f32>(uniforms.mouse[0], uniforms.mouse[1], 0.0);
        let dir = target - p.position;
        p.velocity += normalize(dir) * uniforms.attract_strength * uniforms.delta_time;
    "#.into()))
    .run();
```

### WASD Movement

Move a point of interest with keyboard:

```rust
Simulation::<Particle>::new()
    .with_uniform::<[f32; 2]>("focus", [0.0, 0.0])
    .with_update(|ctx| {
        let mut focus = [0.0_f32, 0.0_f32];
        let speed = 2.0 * ctx.time.delta_time();

        if ctx.input.key_held(KeyCode::W) { focus[1] += speed; }
        if ctx.input.key_held(KeyCode::S) { focus[1] -= speed; }
        if ctx.input.key_held(KeyCode::A) { focus[0] -= speed; }
        if ctx.input.key_held(KeyCode::D) { focus[0] += speed; }

        // Accumulate movement
        // In practice, you'd store this in shared state
        ctx.set("focus", focus);
    })
    .run();
```

### Toggle Effects

Toggle particle behavior with key presses:

```rust
use std::sync::{Arc, Mutex};

let gravity_on = Arc::new(Mutex::new(true));
let gravity_clone = gravity_on.clone();

Simulation::<Particle>::new()
    .with_uniform::<f32>("gravity", 9.8)
    .with_update(move |ctx| {
        let mut on = gravity_clone.lock().unwrap();

        // Toggle with G key
        if ctx.input.key_pressed(KeyCode::G) {
            *on = !*on;
        }

        ctx.set("gravity", if *on { 9.8 } else { 0.0 });
    })
    .run();
```

## Notes

- Input is processed once per frame before the update callback runs
- `key_pressed` and `mouse_pressed` only return `true` for one frame
- Mouse NDC coordinates assume a standard coordinate system (Y-up)
- The scroll delta is positive for scrolling up/forward
