# RDPE Editor

Visual editor for designing GPU-accelerated particle simulations. Design, preview, and export RDPE simulations without writing code.

## Features

- **Live Preview** — Embedded GPU simulation viewport with real-time updates
- **Auto-Rebuild** — Changes automatically rebuild the simulation after a short debounce
- **100+ Rules** — Full access to all RDPE rules through dropdown menus
- **Visual Configuration** — Adjust particle shapes, colors, blend modes, trails, and effects
- **Custom WGSL** — Write custom compute and render shader code with live validation
- **3D Fields** — Configure spatial fields for pheromones, density, and flow effects
- **Volume Rendering** — Visualize scalar fields with ray-marched volume rendering
- **Particle Inspector** — Click particles to inspect and edit their properties live
- **Code Export** — Generate standalone Rust code from your configuration
- **18 Presets** — Start from pre-built simulations like Boids, Galaxy, Slime Mold

## Installation

```bash
# Build and run the editor
cargo run --release --package rdpe-editor
```

## Usage

### Editor

Launch the visual editor:

```bash
cargo run --package rdpe-editor
```

The editor window contains:
- **Central Viewport** — Live simulation preview with orbit camera (drag to rotate, scroll to zoom)
- **Right Panel** — Tabbed configuration panels (Spawn, Rules, Particle, Fields, Visuals, Custom)
- **Menu Bar** — File operations, presets, pause/play, reset
- **Bottom Panel** — Particle inspector (when a particle is selected)

### Runner

Execute saved configurations without the editor UI:

```bash
cargo run --package rdpe-editor --bin rdpe-runner -- config.json
```

The runner opens a fullscreen simulation window with particle and rule inspectors.

## Configuration Panels

### Spawn
Configure how particles are spawned:
- **Particle Count** — Number of particles (1 to 500,000)
- **Bounds** — Simulation boundary size
- **Particle Size** — Render size of each particle
- **Shape** — Cube, Sphere, Shell, Ring, Point, Line, Plane
- **Velocity** — Zero, Random, Outward, Inward, Swirl, Directional
- **Color Mode** — Uniform, RandomHue, ByPosition, ByVelocity, Gradient
- **Spatial Hashing** — Cell size and resolution for neighbor queries

### Rules
Stack rules to define particle behavior. Categories include:
- **Forces** — Gravity, Drag, Acceleration
- **Boundaries** — BounceWalls, WrapWalls
- **Point Forces** — AttractTo, RepelFrom, PointGravity, Orbit, Spring, Radial, Vortex, Pulse
- **Noise** — Turbulence, Curl, Wind, PositionNoise
- **Steering** — Seek, Flee, Arrive, Avoid, Wander
- **Flocking** — Separate, Cohere, Align, Flock
- **Collisions** — Collide, NBodyGravity, LennardJones, Viscosity, Pressure
- **Types** — Chase, Evade, Convert, TypedNeighbor
- **State** — State, Agent (state machines)
- **Lifecycle** — Age, Lifetime, FadeOut, ShrinkOut, ColorOverLife
- **Custom** — Custom WGSL, NeighborCustom, OnCollision
- **Events** — OnSpawn, OnDeath, OnCondition, OnInterval
- **Fields** — Deposit, Sense, Consume, Signal, Absorb
- **Math/Logic** — Lerp, Threshold, Gate, Noise, Remap, And, Or, Not

### Particle
Define custom particle fields beyond the built-in `position`, `velocity`, `color`:
- Add fields of type `f32`, `u32`, `i32`, `Vec2`, `Vec3`, `Vec4`
- Fields are accessible in custom WGSL code

### Fields
Configure 3D spatial fields:
- **Resolution** — Grid size (16, 32, 64, 128)
- **Extent** — World-space coverage
- **Decay** — Per-frame fade factor
- **Blur** — Diffusion strength and iterations
- **Type** — Scalar or Vector

Enable volume rendering to visualize fields with configurable palettes.

### Visuals
- **Blend Mode** — Alpha, Additive, Multiply
- **Shape** — Circle, Square, Triangle, Star, Hexagon, etc.
- **Palette** — 12 color schemes (Viridis, Plasma, Fire, Neon, etc.)
- **Color Mapping** — Index, Speed, Age, Distance, Random
- **Trails** — Motion blur length
- **Connections** — Lines between nearby particles
- **Velocity Stretch** — Elongate particles in motion direction
- **Wireframe** — Debug mesh overlay

### Custom
- **Uniforms** — Add custom `f32`, `Vec2`, `Vec3`, `Vec4` values accessible in shaders
- **Vertex Shader** — Custom WGSL for vertex processing
- **Fragment Shader** — Custom WGSL for fragment coloring
- **Export** — Generate standalone Rust code

## Presets

| Preset             | Description                              |
|--------------------|------------------------------------------|
| Boids Flocking     | Classic separation, cohesion, alignment  |
| Explosion          | Particles exploding outward with gravity |
| Fluid Simulation   | SPH-like pressure and viscosity          |
| Predator Prey      | Chase and evade dynamics                 |
| Curl Noise Flow    | Smooth, divergence-free motion           |
| N-Body Gravity     | Mutual gravitational attraction          |
| Custom Shader Demo | Custom uniforms and shader code          |
| Volume Field Demo  | 3D fields with volume rendering          |
| Pheromone Trails   | Ant-like trail following                 |
| Shockwave          | Expanding shockwaves                     |
| Galaxy             | Spiral arm dynamics                      |
| Crystal Growth     | Diffusion-limited aggregation            |
| Slime Mold         | Physarum-inspired pheromone agents       |
| Aurora             | Northern lights effect                   |
| Aquarium           | Schooling fish with shark                |
| Magnetic Field     | Attraction and repulsion                 |
| Fountain           | Upward water with respawning             |
| Fireflies          | Pulsing, wandering lights                |
| Tornado            | Swirling vortex                          |

## Keyboard Shortcuts

| Key            | Action                  |
|----------------|-------------------------|
| Space          | Pause/Resume simulation |
| Click particle | Select for inspection   |
| Mouse drag     | Orbit camera            |
| Scroll         | Zoom camera             |

## Dependencies

- **rdpe** — Core particle simulation library
- **eframe/egui** — Immediate-mode GUI
- **wgpu** — GPU rendering
- **serde** — JSON serialization
- **naga** — WGSL shader validation
- **rfd** — Native file dialogs

## License

MIT
