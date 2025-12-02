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
| `predator_prey` | Chase and evade behaviors between particle types |
| `infection` | SIR epidemic model with type conversion |
| `inbox` | Particle-to-particle communication system |
| `slime_mold` | Physarum-inspired emergent patterns (requires `--features egui`) |

## Single-Rule Examples

Located in `examples/rules/`, each demonstrates one specific rule:

| Example | Rule | Description |
|---------|------|-------------|
| `bounce_walls` | `BounceWalls` | Particles reflecting off boundaries |
| `vortex` | `Vortex` | Circular swirling motion |
| `turbulence` | `Turbulence` | Chaotic flow field |
| `orbit` | `Orbit` | Particles orbiting Y axis |
| `curl` | `Curl` | Turbulent curl noise motion |
| `neighbors` | Neighbor iteration | Demonstrates spatial queries |
| `nbody` | N-body gravity | Gravitational attraction |
| `fluid` | SPH-style | Fluid dynamics simulation |
| `magnetism` | Magnetic poles | Attraction/repulsion fields |
| `point_gravity` | Point attraction | Gravity toward a point |
| `spring` | Spring force | Elastic forces toward origin |
| `oscillate` | Oscillation | Harmonic motion |

## Visual Examples

Demonstrating rendering customization:

| Example | Description |
|---------|-------------|
| `custom_shader` | Custom fragment shader for particle appearance |
| `post_process` | Screen-space post-processing (vignette, aberration, grain) |

## Interactive Examples

These require `--features egui`:

| Example | Description |
|---------|-------------|
| `egui_controls` | Basic egui integration |
| `egui_interactive` | Full interactive parameter control |
| `slime_mold` | Physarum simulation with controls |
| `plasma_storm` | Interactive plasma effects |
| `fluid_galaxy` | Galaxy formation with controls |
| `murmuration` | Starling flock patterns |
| `neon_assault_interactive` | 80s arcade with runtime controls |

## Experimental / Creative

Located in `examples/experimental/`, these push creative boundaries:

| Example | Description |
|---------|-------------|
| `ethereal_web` | Dreamlike visualization combining all features |
| `neon_assault` | 80s arcade aesthetic with CRT post-processing |
| `neon_assault_interactive` | Neon assault with egui controls |
| `cosmic_jellyfish` | Organic pulsing creature |
| `firefly_grove` | Synchronized blinking lights |
| `black_hole` | Gravitational lensing effect |
| `thought_storm` | Neural activity visualization |
| `plasma_storm` | Interactive plasma (egui) |
| `fluid_galaxy` | Galaxy formation (egui) |
| `murmuration` | Starling flock patterns (egui) |

## Learning Path

### 1. Start with Basics
- `boids` - Core particle simulation
- `bounce_walls` - Simple rule
- `orbit` - Basic motion

### 2. Explore Interactions
- `predator_prey` - Typed particles
- `infection` - Type conversion
- `neighbors` - Spatial queries

### 3. Customize Visuals
- `custom_shader` - Fragment shaders
- `post_process` - Screen effects

### 4. Add Interactivity
- `egui_controls` - Basic UI
- `slime_mold` - Full interactive example

### 5. Go Creative
- `ethereal_web` - All features combined
- `neon_assault` - Aesthetic deep dive

## Running All Examples

To quickly try everything:

```bash
# Core
cargo run --example boids
cargo run --example predator_prey
cargo run --example infection

# Rules
cargo run --example orbit
cargo run --example curl
cargo run --example nbody

# Visual
cargo run --example custom_shader
cargo run --example post_process

# Creative
cargo run --example ethereal_web
cargo run --example neon_assault

# Interactive (requires egui feature)
cargo run --example slime_mold --features egui
cargo run --example neon_assault_interactive --features egui
```

Each example file contains `//!` doc comments explaining what it demonstrates and suggestions for experimentation.
