# RDPE Editor

RDPE includes a visual editor for designing particle simulations without writing code.

## Running the Editor

```bash
cargo run --package rdpe-editor --bin rdpe-editor
```

## Features

- **Live Preview**: Real-time GPU-accelerated simulation viewport
- **100+ Rules**: Access all RDPE rules through dropdown menus
- **Visual Configuration**: Particle shapes, colors, blend modes, trails
- **Custom WGSL**: Write custom shader code with live validation
- **3D Fields**: Configure spatial fields with volume rendering
- **Code Export**: Generate standalone Rust code from your configuration
- **Presets**: 18 pre-built simulations to start from

## Interface

The editor has a central viewport showing the live simulation, with configuration panels on the right:

- **Spawn**: Particle count, bounds, speed, spawn shape and velocity
- **Rules**: Add and configure simulation rules
- **Particle**: Define custom particle fields
- **Fields**: Configure 3D spatial fields
- **Visuals**: Blend modes, shapes, palettes, trails, connections
- **Custom**: Custom uniforms, shaders, and code export

## Workflow

1. Start with a preset or the default configuration
2. Adjust spawn settings (particle count, shape, velocity)
3. Add rules to define particle behavior
4. Configure visuals (colors, shapes, effects)
5. Export to Rust code if you want to embed in your own project

## Controls

- **Mouse drag**: Orbit camera
- **Scroll**: Zoom
- **Click particle**: Select for inspection
- **Space**: Pause/resume (in some contexts)

## Saving and Loading

Configurations are saved as JSON files. Use File > Save/Load to manage your simulations.

## Code Export

The Custom tab includes an Export Code button that generates standalone Rust code using the rdpe library. This lets you iterate visually in the editor, then export to code for further customization or embedding in your own project.

## Runner

You can also run saved configurations without the editor UI:

```bash
cargo run --package rdpe-editor --bin rdpe-runner -- config.json
```

This opens a fullscreen simulation window, useful for presentations or testing.
