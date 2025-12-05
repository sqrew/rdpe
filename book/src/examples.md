# Examples

RDPE includes many examples demonstrating different features. Each example contains detailed comments explaining the concepts it demonstrates.

## Running Examples

```bash
cargo run --example <name>

# With egui feature (for interactive examples)
cargo run --example <name> --features egui
```

## Core Examples

These showcase fundamental RDPE capabilities:

| Example | Description |
|---------|-------------|
| `boids` | Classic flocking algorithm (Separate + Cohere + Align) |
| `aquarium` | Fish and sharks with Chase/Evade behaviors |
| `predator_prey` | Chase and evade behaviors between particle types |
| `infection` | SIR epidemic model with type conversion |
| `connections` | Drawing lines between nearby particles |
| `inbox` | Particle-to-particle communication system |
| `slime_mold` | Physarum-inspired emergent patterns (requires `--features egui`) |
| `slime_mold_field` | Slime mold using 3D spatial fields (requires `--features egui`) |
| `multi_field` | Multiple competing pheromone fields |

## Effect Examples

| Example | Description |
|---------|-------------|
| `explosion` | Burst emitter with particle effects |
| `fountain` | Cone emitter shooting upward |
| `rain` | Box emitter simulating rainfall |
| `shockwave` | Expanding shockwave and pulse effects |
| `trails` | Particle motion trails |
| `fireflies` | Synchronized blinking behavior |
| `falling_leaves` | Organic falling motion |

## Physics & Motion

| Example | Description |
|---------|-------------|
| `swirl` | Vortex-like circular motion |
| `noisy` | Noise-based movement |
| `wave_field` | Wave-like oscillating motion |
| `attractor` | Point attraction |
| `orbiting_attractor` | Orbiting gravity source |
| `gravity_visualizer` | Gravity field visualization |
| `density_fluids` | SPH-style fluid simulation |
| `particle_life` | Particle Life cellular automaton |

## Visual Examples

| Example | Description |
|---------|-------------|
| `shapes` | All available particle shapes (Circle, Star, Hexagon, etc.) |
| `custom_shader` | Custom fragment shader for particle appearance |
| `post_process` | Screen-space post-processing effects |
| `texture_example` | Custom texture sampling in shaders |
| `palettes` | Built-in color palettes and mappings |
| `glow` | Glowing particle effects |
| `sphere_shell` | Spherical particle distribution |

## Interactive Examples

These require `--features egui`:

| Example | Description |
|---------|-------------|
| `egui_controls` | Basic egui integration |
| `egui_interactive` | Full interactive parameter control |
| `slime_mold` | Physarum simulation with controls |

## Other Examples

| Example | Description |
|---------|-------------|
| `getting_started` | Minimal example to get started |
| `input_demo` | Keyboard and mouse input handling |
| `lifecycle_demo` | Particle aging, death, and respawning |
| `spatial_grid_demo` | Spatial hashing visualization |
| `agent_demo` | Particles as autonomous agents |
| `signal_swarm` | Swarm signaling behavior |
| `neural_network` | Neural network-style visualization |
| `chemistry` | Chemical reaction simulation |
| `cells` | Cell-like behavior |
| `rocket` | Rocket with exhaust particles |
| `volume_render` | 3D volume rendering |

## Learning Path

### 1. Start with Basics
- `getting_started` - Minimal setup
- `boids` - Core particle simulation

### 2. Explore Interactions
- `predator_prey` - Typed particles
- `infection` - Type conversion
- `aquarium` - Chase and evade

### 3. Add Effects
- `explosion` - Emitters
- `trails` - Motion trails
- `connections` - Visual connections

### 4. Customize Visuals
- `custom_shader` - Fragment shaders
- `post_process` - Screen effects
- `palettes` - Color schemes

### 5. Add Interactivity
- `egui_controls` - Basic UI
- `slime_mold` - Full interactive example

## Running Examples

```bash
# Core
cargo run --example boids
cargo run --example aquarium
cargo run --example predator_prey
cargo run --example infection

# Effects
cargo run --example explosion
cargo run --example fountain
cargo run --example shockwave
cargo run --example trails

# Visual
cargo run --example shapes
cargo run --example custom_shader
cargo run --example post_process
cargo run --example palettes

# Interactive (requires egui feature)
cargo run --example slime_mold --features egui
cargo run --example egui_interactive --features egui
```

Each example file contains `//!` doc comments explaining what it demonstrates and suggestions for experimentation.
