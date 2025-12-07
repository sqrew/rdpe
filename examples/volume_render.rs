//! # Volume Rendering Example
//!
//! Demonstrates visualizing a 3D field as volumetric fog using ray marching.
//! Particles deposit into a field, and the field itself is rendered as glowing
//! volumetric clouds, not just the particles.
//!
//! ## What This Demonstrates
//!
//! - `.with_volume_render(VolumeConfig)` - enable field volume rendering
//! - `VolumeConfig` - configure ray marching parameters
//! - Field data visualized directly as volumetric fog
//! - Particles visible within the glowing volume
//!
//! ## Volume Config Options
//!
//! ```rust
//! VolumeConfig::new()
//!     .with_field(0)           // which field index to render
//!     .with_steps(64)          // ray march steps (quality vs performance)
//!     .with_density_scale(5.0) // how opaque the volume appears
//!     .with_palette(Palette::Inferno)  // color mapping
//!     .with_threshold(0.01)    // minimum density to render
//!     .with_additive(true)     // glow effect (vs solid fog)
//! ```
//!
//! ## Try This
//!
//! - Change palette: `Palette::Viridis`, `Palette::Plasma`, `Palette::Fire`
//! - Increase steps (128) for smoother volume, decrease (32) for performance
//! - Set `additive(false)` for solid fog instead of glow
//! - Adjust `density_scale` to make volume more/less visible
//!
//! Run with: `cargo run --example volume_render`

use rand::Rng;
use rdpe::prelude::*;

#[derive(Particle, Clone)]
struct Agent {
    position: Vec3,
    velocity: Vec3,
    #[color]
    color: Vec3,
}

fn main() {
    let mut rng = rand::thread_rng();

    // Pre-generate random positions
    let positions: Vec<Vec3> = (0..5_000)
        .map(|_| {
            Vec3::new(
                rng.gen_range(-0.5..0.5),
                rng.gen_range(-0.5..0.5),
                rng.gen_range(-0.5..0.5),
            )
        })
        .collect();

    Simulation::<Agent>::new()
        .with_particle_count(5_000)
        .with_bounds(1.0)
        .with_particle_size(0.008)
        .with_spawner(move |ctx| Agent {
            position: positions[ctx.index as usize],
            velocity: Vec3::ZERO,
            color: Vec3::new(1.0, 1.0, 1.0), // White particles
        })
        // Create a 3D field for particles to deposit into
        .with_field(
            "density",
            FieldConfig::new(64)
                .with_extent(1.0)
                .with_decay(0.95)      // Moderate fade
                .with_blur(0.2)        // Good diffusion
                .with_blur_iterations(2),
        )
        // Enable volume rendering to visualize the field
        .with_volume_render(
            VolumeConfig::new()
                .with_field(0)              // Render field index 0
                .with_steps(64)             // Ray march quality
                .with_density_scale(8.0)    // Make it visible
                .with_palette(Palette::Inferno)
                .with_threshold(0.005)      // Low threshold to catch faint trails
                .with_additive(true),       // Glow effect
        )
        // Particles swirl and deposit into the field
        .with_rule(Rule::Vortex {
            center: Vec3::ZERO,
            axis: Vec3::Y,
            strength: 0.5,
        })
        .with_rule(Rule::Wander { strength: 0.3, frequency: 100.0 })
        .with_rule(Rule::AttractTo {
            point: Vec3::ZERO,
            strength: 0.2,
        })
        .with_rule(Rule::SpeedLimit { min: 0.0, max: 1.0 })
        // Deposit into the field and bounce off walls
        .with_rule(Rule::Custom(
            r#"
            // Deposit particle presence into the field
            field_write(0u, p.position, 0.3);
            "#
            .into(),
        ))
        .with_rule(Rule::BounceWalls)
        .with_visuals(|v| {
            v.background(Vec3::new(0.02, 0.02, 0.05)); // Dark background
        })
        .run();
}
