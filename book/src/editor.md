# RDPE Editor

RDPE includes a visual editor for designing particle simulations without writing code.

## Running the Editor

```bash
cargo run --release --package rdpe-editor
```

## Features

- **Live Preview**: Real-time GPU-accelerated simulation viewport
- **100+ Rules**: Access all RDPE rules through dropdown menus
- **21 Presets**: Pre-built simulations to start from or learn
- **35 Post-Processing Effects**: Bloom, CRT, glitch, thermal, and more
- **18 Mouse Powers**: Interactive forces like attract, vortex, paint, black hole
- **3D Fields**: Configure spatial fields with volume rendering and vector glyphs
- **Multi-Type Systems**: Define particle types with interaction matrices
- **Custom WGSL**: Write custom shader code with live validation
- **Live Statistics**: Real-time metrics on particle behavior
- **Code Export**: Generate standalone Rust code from your configuration

## Interface

The editor has a central viewport showing the live simulation, with a configuration sidebar on the right containing 11 tabs:

| Tab | Purpose |
|-----|---------|
| **Spawn** | Particle count, bounds, spawn shape, initial velocity, type weights |
| **Rules** | Add and configure simulation rules (100+ available) |
| **Types** | Interaction matrix for multi-type particle systems |
| **Emitters** | Configure multiple particle emitters with different behaviors |
| **Particle** | Define custom particle fields (scalars, vectors) |
| **Fields** | 3D spatial fields, volume rendering, vector field glyphs |
| **Visuals** | Blend modes, shapes, palettes, trails, connections, wireframes |
| **Post FX** | Post-processing effect stack (35 effects) |
| **Mouse** | Mouse interaction powers (18 modes) |
| **Custom** | Custom uniforms, shaders, and code export |
| **Stats** | Live simulation statistics and metrics |

### Top Bar

The menu bar provides:

- **File**: New, Open, Save, Save As, Quit
- **Presets**: 21 pre-built simulations
- **Options**: Editor appearance settings

The right side of the menu bar has playback controls:

- **Reset**: Regenerate all particles
- **Pause/Play**: Stop or resume simulation
- **Step**: Advance one frame (when paused)
- **Speed**: Time scale slider (0.1x to 3.0x)
- **Orbit**: Toggle camera auto-rotation

## Panels in Detail

### Spawn

Configure how particles are created:

- **Particle Count**: Total number of particles
- **Bounds**: Simulation space size
- **Spawn Shape**: Sphere, cube, shell, disk, line, point, ring, torus, cylinder
- **Speed Range**: Initial velocity magnitude
- **Velocity Mode**: Outward, random, directional, tangent
- **Type Weights**: Distribution across particle types
- **Spatial Hashing**: Cell size and resolution (auto-enabled when needed)

### Rules

Add rules from categories:

- **Physics**: Gravity, Drag, Acceleration, BounceWalls, WrapWalls, SpeedLimit
- **Forces**: AttractTo, RepelFrom, Vortex, Turbulence, Orbit, Spring, Pulse
- **Neighbors**: Separate, Cohere, Align, Flock, Collide, NBodyGravity, LennardJones
- **Lifecycle**: Age, Lifetime, FadeOut, ScaleByAge, Respawn
- **Behavior**: Wander, Seek, Flee, Arrive, Follow
- **Visual**: ColorBySpeed, ColorByAge, TrailFade
- **Custom**: Write WGSL code directly

Rules are reorderable via drag-and-drop.

### Types (Interactions)

For multi-type particle systems:

- Define interaction strengths between types
- Randomize or invert matrices
- Enable/disable typed rules
- Visual matrix editor with presets

### Emitters

Add continuous particle sources:

- **Shape**: Point, sphere, disk, line, ring
- **Rate**: Particles per second
- **Velocity**: Direction and spread
- **Lifecycle**: Particle lifetime from this emitter

### Particle Fields

Define custom per-particle data:

- **Scalars**: Single float values (energy, temperature, charge)
- **Vectors**: Vec3 values (custom forces, targets)

Fields are accessible in custom shaders as `p.field_name`.

### Fields (3D)

Configure spatial fields that particles can read/write:

- **Resolution**: 3D grid size (8 to 64)
- **Initial Value**: Starting field values
- **Decay**: Field dissipation rate

Also includes:

- **Volume Rendering**: Visualize fields as 3D volumes
- **Vector Glyphs**: Display field directions as arrows

### Visuals

Particle rendering options:

- **Blend Mode**: Alpha, Additive, Multiply, Screen, Overlay, Soft Light, Subtractive
- **Shape**: Circle, Square, Ring, Star, Triangle, Hexagon, Diamond, Point
- **Palette**: Viridis, Magma, Plasma, Rainbow, Fire, Ice, Neon, custom
- **Color Mapping**: Map palette to speed, age, position, distance, or index
- **Spawn Color**: Uniform, random hue, by position, by velocity, gradient
- **Trails**: Trail length (0-50)
- **Connections**: Draw lines between nearby particles
- **Velocity Stretch**: Elongate particles in direction of motion
- **Wireframe**: Overlay geometric shapes (platonic solids, prisms, spirals)

### Post FX

Stackable post-processing effects (order matters):

| Category | Effects |
|----------|---------|
| **Blur/Glow** | Bloom, Blur, RadialBlur, EdgeGlow |
| **Color** | Grayscale, Sepia, Posterize, Invert, Tint, Duotone, Thermal, HueShift |
| **Distortion** | ChromaticAberration, BarrelDistortion, Ripple, Glitch, Kaleidoscope |
| **Retro** | CRT, FilmGrain, Pixelate, VHS, Dithering, Halftone, ScanlineInterlace |
| **Artistic** | Emboss, SobelOutline, NightVision |
| **Adjustment** | ColorAdjust, Exposure, Sharpen, ChannelMixer, Vignette, Letterbox |
| **Custom** | Write your own WGSL effect |

### Mouse

Interactive mouse powers (click and drag in viewport):

| Power | Effect |
|-------|--------|
| Attract | Pull particles toward cursor |
| Repel | Push particles away |
| Vortex | Swirl particles around cursor |
| Explode | One-time outward burst |
| GravityWell | Realistic gravity falloff |
| Paint | Change particle colors |
| Turbulence | Add chaotic motion |
| Freeze | Stop particles in radius |
| Kill | Remove particles |
| Spawn | Create new particles |
| BlackHole | Strong attraction with event horizon |
| Orbit | Force particles into orbit |
| Scatter | Random velocity burst |
| Wind | Directional force |
| Pulse | Rhythmic push/pull |
| Repulsor | Soft repulsion field |
| SpiralIn | Inward spiral motion |
| RandomVelocity | Randomize directions |

Configure radius, strength, and (for Paint) color.

### Custom

Advanced customization:

- **Custom Uniforms**: Add float, vec2, vec3, vec4 values accessible in shaders
- **Custom Shaders**: Write WGSL code that runs per-particle
- **Export Code**: Generate standalone Rust code

### Stats

Live simulation metrics:

- **Particles**: Alive count and percentage
- **Spatial**: Center of mass, bounding box
- **Velocity**: Average speed, max speed, average direction
- **Per-Field**: Min/max/avg for each particle field

Stats update every ~30 frames to minimize performance impact.

## Controls

| Input | Action |
|-------|--------|
| Mouse drag | Orbit camera |
| Scroll wheel | Zoom in/out |
| Middle click drag | Pan camera |
| F11 | Toggle fullscreen |
| Click particle | Select for inspection |

## Workflow

1. **Start**: Load a preset or begin with defaults
2. **Spawn**: Set particle count and spawn shape
3. **Rules**: Add behaviors (gravity, flocking, etc.)
4. **Visuals**: Choose colors, shapes, effects
5. **Iterate**: Adjust parameters with live preview
6. **Export**: Save JSON config or export to Rust code

## Saving and Loading

Configurations are saved as JSON files via File > Save/Open. The format is human-readable and version-controlled.

## Code Export

The Custom tab includes an Export button that generates standalone Rust code using the rdpe library. This workflow lets you:

1. Design visually in the editor
2. Export to code
3. Customize further in Rust
4. Embed in your own project

## Runner

Run saved configurations without the editor UI:

```bash
cargo run --release --package rdpe-editor --bin rdpe-runner -- config.json
```

Opens a fullscreen simulation window - useful for presentations, screensavers, or testing.

## Performance Tips

- Start with fewer particles while designing, scale up later
- Neighbor rules (Separate, Cohere, Collide) are expensive - spatial hashing helps
- Post-processing effects stack - more effects = more GPU work
- Volume rendering is costly - use lower field resolutions when possible
- The Stats tab shows alive particle count - check if particles are dying unexpectedly
