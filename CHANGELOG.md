# Changelog

All notable changes to RDPE will be documented in this file.

## [0.1.0] - 2025-12-29

Initial release.

### Added

- GPU-accelerated particle simulation via WGPU compute shaders
- `#[derive(Particle)]` macro for automatic GPU struct generation
- 100+ built-in rules (physics, flocking, forces, lifecycle, logic)
- Custom WGSL shader injection
- Spatial hashing for O(N) neighbor queries
- 3D spatial fields with volume rendering
- Visual editor (`rdpe-editor`) with 21 presets
- 35 post-processing effects
- 18 mouse interaction modes
- Multi-type particle systems with interaction matrices
- Trail rendering, particle connections, wireframe overlays
- JSON save/load and Rust code export
- 45+ example simulations
- Cross-platform: Windows, macOS, Linux, Raspberry Pi

[0.1.0]: https://github.com/sqrew/rdpe/releases/tag/v0.1.0
