//! # Color Palettes Example
//!
//! Demonstrates automatic color palette and mapping system for particles
//! without needing a `#[color]` field.
//!
//! ## What This Demonstrates
//!
//! - `v.palette(Palette, ColorMapping)` - automatic coloring
//! - `Palette::Fire` - one of many built-in palettes
//! - `ColorMapping::Distance { max_dist }` - color by distance from center
//! - No `#[color]` field needed - colors computed automatically
//!
//! ## Available Palettes
//!
//! - `Palette::Fire` - Black → red → orange → yellow → white
//! - `Palette::Ice` - Dark blue → cyan → white
//! - `Palette::Rainbow` - Full spectrum hue cycle
//! - `Palette::Plasma` - Purple → pink → orange
//! - `Palette::Viridis` - Scientific visualization (colorblind-friendly)
//! - `Palette::Grayscale` - Black to white
//!
//! ## Color Mappings
//!
//! - `ColorMapping::Distance { max_dist }` - by distance from origin
//! - `ColorMapping::Speed { min, max }` - by velocity magnitude
//! - `ColorMapping::Age { max_age }` - by particle age
//! - `ColorMapping::PositionY { min, max }` - by vertical position
//! - `ColorMapping::Index` - by particle index (creates bands)
//! - `ColorMapping::Random` - random per particle
//!
//! ## Try This
//!
//! - Try `Palette::Rainbow` with `ColorMapping::Speed { min: 0.0, max: 1.0 }`
//! - Use `ColorMapping::Index` to color by spawn order
//! - Combine with `BlendMode::Alpha` for solid colors
//! - Try `Palette::Neon` or `Palette::Ocean` for different aesthetics
//!
//! Run with: `cargo run --example palettes`

use rdpe::prelude::*;

#[derive(rdpe::Particle, Clone)]
struct Particle {
    position: Vec3,
    velocity: Vec3,
}

fn main() {
    let count = 20_000;

    Simulation::<Particle>::new()
        .with_particle_count(count)
        .with_bounds(1.0)
        .with_particle_size(0.015)
        .with_spawner(|i, total| {
            // Spawn in a grid pattern
            let side = (total as f32).cbrt() as u32;
            let x = (i % side) as f32 / side as f32 * 2.0 - 1.0;
            let y = ((i / side) % side) as f32 / side as f32 * 2.0 - 1.0;
            let z = (i / (side * side)) as f32 / side as f32 * 2.0 - 1.0;

            Particle {
                position: Vec3::new(x * 0.8, y * 0.8, z * 0.8),
                velocity: Vec3::ZERO,
            }
        })
        // Use Fire palette, color by distance from center
        .with_visuals(|v| {
            v.palette(Palette::Fire, ColorMapping::Distance { max_dist: 1.5 });
            v.blend_mode(BlendMode::Additive);
        })
        // Gentle attraction to center
        .with_rule(Rule::AttractTo {
            point: Vec3::ZERO,
            strength: 0.5,
        })
        // Smooth swirl effect (constant angular velocity, doesn't explode at center)
        .with_rule(Rule::Custom(
            r#"
            let swirl_strength = 1.5;
            p.velocity += vec3<f32>(-p.position.z, 0.0, p.position.x) * swirl_strength * uniforms.delta_time;
            "#
            .into(),
        ))
        // High drag for smooth motion
        .with_rule(Rule::Drag(3.0))
        // Speed limit
        .with_rule(Rule::SpeedLimit { min: 0.0, max: 0.5 })
        // Soft bounce
        .with_rule(Rule::BounceWalls)
        .run();
}
