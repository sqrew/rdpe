# Egui Integration

RDPE supports [egui](https://github.com/emilk/egui) for adding interactive UI controls to your simulations. This enables real-time parameter tuning, debug displays, and rich user interfaces.

## Enabling Egui

Add the `egui` feature to your `Cargo.toml`:

```toml
[dependencies]
rdpe = { version = "0.1", features = ["egui"] }
```

Or run examples with:

```bash
cargo run --example egui_interactive --features egui
```

## Basic Usage

Use `.with_ui()` to add an egui callback:

```rust
Simulation::<Particle>::new()
    // ... particle setup ...
    .with_ui(|ctx| {
        egui::Window::new("Controls")
            .show(ctx, |ui| {
                ui.label("Hello from egui!");
            });
    })
    .run();
```

The callback receives an `&egui::Context` and runs every frame. You can create windows, panels, sliders, buttons, and any other egui widgets.

## Connecting UI to Simulation

The real power comes from connecting UI controls to simulation parameters. This requires:

1. **Custom uniforms** - GPU-side parameters the shader reads
2. **Update callback** - Syncs Rust values to uniforms each frame
3. **Shared state** - Connects UI to the update callback

### The Pattern: Arc<Mutex<T>>

Since both callbacks need access to the same state and must be `Send`, use `Arc<Mutex<T>>`:

```rust
use std::sync::{Arc, Mutex};

// Define your parameters
struct SimState {
    gravity: f32,
    speed: f32,
}

impl Default for SimState {
    fn default() -> Self {
        Self { gravity: 0.5, speed: 1.0 }
    }
}

fn main() {
    // Create shared state
    let state = Arc::new(Mutex::new(SimState::default()));
    let ui_state = state.clone();      // Clone for UI callback
    let update_state = state.clone();  // Clone for update callback

    Simulation::<Particle>::new()
        // Declare uniforms (must match defaults!)
        .with_uniform::<f32>("gravity", 0.5)
        .with_uniform::<f32>("speed", 1.0)

        // UI callback - modifies shared state
        .with_ui(move |ctx| {
            let mut s = ui_state.lock().unwrap();
            egui::Window::new("Controls").show(ctx, |ui| {
                ui.add(egui::Slider::new(&mut s.gravity, 0.0..=2.0).text("Gravity"));
                ui.add(egui::Slider::new(&mut s.speed, 0.1..=3.0).text("Speed"));
            });
        })

        // Update callback - syncs state to GPU uniforms
        .with_update(move |ctx| {
            let s = update_state.lock().unwrap();
            ctx.set("gravity", s.gravity);
            ctx.set("speed", s.speed);
        })

        // Shader reads uniforms
        .with_rule(Rule::Custom(r#"
            p.velocity.y -= uniforms.gravity * uniforms.delta_time;
            p.position += p.velocity * uniforms.delta_time * uniforms.speed;
        "#.into()))

        .run();
}
```

### Flow Summary

```
┌─────────────┐     ┌───────────────────┐     ┌─────────────┐
│   Egui UI   │────▶│  Arc<Mutex<State>>│────▶│  Uniforms   │
│  (sliders)  │     │   (shared state)  │     │   (GPU)     │
└─────────────┘     └───────────────────┘     └─────────────┘
       │                     │                      │
       │    .with_ui()       │    .with_update()    │    Rule::Custom
       └─────────────────────┴──────────────────────┴─────────────────▶ Shader
```

## Complete Example

Here's a full interactive simulation:

```rust
use rand::Rng;
use rdpe::prelude::*;
use std::sync::{Arc, Mutex};

#[derive(Particle, Clone)]
struct Ball {
    position: Vec3,
    velocity: Vec3,
    #[color]
    color: Vec3,
}

struct SimState {
    gravity: f32,
    drag: f32,
    bounce: f32,
}

impl Default for SimState {
    fn default() -> Self {
        Self { gravity: 1.0, drag: 0.5, bounce: 0.8 }
    }
}

fn main() {
    let mut rng = rand::thread_rng();

    let state = Arc::new(Mutex::new(SimState::default()));
    let ui_state = state.clone();
    let update_state = state.clone();

    let particles: Vec<_> = (0..5000)
        .map(|_| {
            let pos = Vec3::new(
                rng.gen_range(-0.8..0.8),
                rng.gen_range(0.0..0.8),
                rng.gen_range(-0.8..0.8),
            );
            let vel = Vec3::ZERO;
            let hue = rng.gen_range(0.0..1.0);
            let color = Vec3::new(hue, 0.8, 1.0); // HSV-ish
            (pos, vel, color)
        })
        .collect();

    Simulation::<Ball>::new()
        .with_particle_count(5000)
        .with_bounds(1.0)
        .with_spawner(move |i, _| {
            let (pos, vel, color) = particles[i as usize];
            Ball { position: pos, velocity: vel, color }
        })
        .with_uniform::<f32>("gravity", 1.0)
        .with_uniform::<f32>("drag", 0.5)
        .with_uniform::<f32>("bounce", 0.8)

        .with_ui(move |ctx| {
            let mut s = ui_state.lock().unwrap();
            egui::Window::new("Physics Controls")
                .default_pos([10.0, 10.0])
                .show(ctx, |ui| {
                    ui.heading("Parameters");
                    ui.add(egui::Slider::new(&mut s.gravity, 0.0..=5.0).text("Gravity"));
                    ui.add(egui::Slider::new(&mut s.drag, 0.0..=2.0).text("Drag"));
                    ui.add(egui::Slider::new(&mut s.bounce, 0.0..=1.0).text("Bounce"));

                    ui.separator();
                    if ui.button("Reset").clicked() {
                        *s = SimState::default();
                    }
                });
        })

        .with_update(move |ctx| {
            let s = update_state.lock().unwrap();
            ctx.set("gravity", s.gravity);
            ctx.set("drag", s.drag);
            ctx.set("bounce", s.bounce);
        })

        .with_rule(Rule::Custom(r#"
            let dt = uniforms.delta_time;

            // Gravity
            p.velocity.y -= uniforms.gravity * dt;

            // Drag
            p.velocity *= 1.0 - uniforms.drag * dt;

            // Integrate
            p.position += p.velocity * dt;

            // Floor bounce
            if p.position.y < -0.95 {
                p.position.y = -0.95;
                p.velocity.y = abs(p.velocity.y) * uniforms.bounce;
            }
        "#.into()))

        .with_rule(Rule::BounceWalls)
        .run();
}
```

## Tips

### Mutex Performance

Don't worry about `Mutex` overhead - both callbacks run on the main thread, so there's no contention. Lock/unlock is ~20ns, negligible at 60fps.

### Initial Values Must Match

Always ensure `.with_uniform()` values match your `Default` implementation:

```rust
// These MUST match!
.with_uniform::<f32>("gravity", 0.5)  // Uniform default
// ...
impl Default for SimState {
    fn default() -> Self {
        Self { gravity: 0.5, ... }    // State default
    }
}
```

### Type Annotations

Use explicit type annotations for uniform values:

```rust
.with_uniform::<f32>("value", 1.0)    // Good
.with_uniform("value", 1.0f32)        // Also good
.with_uniform("value", 1.0)           // May cause type inference issues
```

### Egui Widgets

Common egui widgets for simulations:

```rust
// Slider with range
ui.add(egui::Slider::new(&mut value, 0.0..=10.0).text("Label"));

// Checkbox
ui.checkbox(&mut enabled, "Enable feature");

// Button
if ui.button("Reset").clicked() {
    // handle click
}

// Color picker
ui.color_edit_button_rgb(&mut color);

// Collapsing section
ui.collapsing("Advanced", |ui| {
    // nested widgets
});
```

### Window Positioning

```rust
egui::Window::new("Title")
    .default_pos([10.0, 10.0])    // Initial position
    .resizable(false)             // Fixed size
    .collapsible(true)            // Can collapse
    .show(ctx, |ui| { ... });
```

## Examples

Run the interactive examples:

```bash
# Basic UI demo (controls don't affect simulation)
cargo run --example egui_controls --features egui

# Full interactive controls
cargo run --example egui_interactive --features egui

# Creative examples with egui
cargo run --example plasma_storm --features egui
cargo run --example fluid_galaxy --features egui
cargo run --example murmuration --features egui
```
