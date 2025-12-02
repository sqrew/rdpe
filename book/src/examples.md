# Examples

RDPE includes many self-documented examples. Each example contains detailed comments explaining the concepts it demonstrates.

## Running Examples

```bash
cargo run --example <name>
```

## Multi-Feature Examples

These showcase combinations of rules and features:

| Example | Description |
|---------|-------------|
| `getting_started` | Minimal RDPE simulation - start here! |
| `fountain` | Cone emitter with gravity and lifecycle |
| `boids` | Classic flocking algorithm (Separate + Cohere + Align) |
| `predator_prey` | Chase and evade behaviors between types |
| `infection` | SIR model with Convert rule |
| `particle_life` | Emergent behavior from interaction matrix |
| `attractor` | Mouse-controlled attraction with custom uniforms |
| `orbiting_attractor` | Time-animated orbiting attractor |
| `noisy` | Flow field using built-in noise functions |
| `swirl` | Custom WGSL functions for vortex effect |
| `explosion` | Burst emitter + FadeOut + ShrinkOut + ColorOverLife |
| `rain` | Box emitter for environmental particles |
| `rocket` | Downward cone emitter for thrust |
| `sphere_shell` | Sphere emitter for 3D distributions |

## Single-Rule Examples

Located in `examples/rules/`, each demonstrates one specific rule:

| Example | Rule |
|---------|------|
| `bounce_walls` | `Rule::BounceWalls` |
| `vortex` | `Rule::Vortex` |
| `turbulence` | `Rule::Turbulence` |
| `orbit` | `Rule::Orbit` |
| `curl` | `Rule::Curl` |

## Learning Path

1. **Start with basics**: `getting_started`, `bounce_walls`, `fountain`
2. **Explore interactions**: `boids`, `predator_prey`, `infection`, `particle_life`
3. **Custom shaders**: `attractor`, `orbiting_attractor`, `noisy`, `swirl`
4. **Emitter types**: `explosion`, `rain`, `rocket`, `sphere_shell`
5. **Force rules**: `vortex`, `turbulence`, `orbit`, `curl`

Each example file contains `//!` doc comments explaining what it demonstrates and suggestions for experimentation.
