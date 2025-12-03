//! # Oscillate Demo
//!
//! Demonstrates `Rule::Oscillate` - sine-wave modulation for pulsing and wave effects.
//! Creates radial ripple waves emanating outward from the center.
//!
//! ## What This Demonstrates
//!
//! - `Rule::Oscillate` - sine-based position modulation
//! - `spatial_scale` parameter for wave wavelength
//! - Layering multiple oscillations for complexity
//! - Grid-based particle spawning for uniform coverage
//! - Height-based color mapping with `Rule::Custom`
//! - `Rule::Spring` to anchor particles in place
//!
//! ## The Physics
//!
//! **Wave Equation**: Position offset = amplitude * sin(time * frequency + distance * spatial_scale).
//! The `spatial_scale` creates radial ripples - higher values = tighter, more
//! frequent waves; lower values = broad, slow-moving swells.
//!
//! **Superposition**: Two oscillations with different frequencies and scales
//! combine to create more organic, less mechanical motion - like real water
//! with wind chop over larger swells.
//!
//! **Spring Anchor**: Particles would drift from accumulated velocity, so
//! the spring gently pulls them back to their grid positions.
//!
//! ## Try This
//!
//! - Increase `spatial_scale` to 20+ for tight ripples
//! - Add a third oscillation on a different axis (Vec3::X)
//! - Set `amplitude` higher for dramatic waves
//! - Remove `Spring` to see particles drift over time
//! - Try `axis: Vec3::new(1.0, 1.0, 0.0)` for diagonal waves
//!
//! Run with: `cargo run --example oscillate`

use rdpe::prelude::*;

#[derive(Particle, Clone)]
struct Wave {
    position: Vec3,
    velocity: Vec3,
    #[color]
    color: Vec3,
}

fn main() {
    // Create a grid of particles
    let grid_size = 60;
    let spacing = 1.8 / grid_size as f32;

    let particles: Vec<Wave> = (0..grid_size * grid_size)
        .map(|i| {
            let x = (i % grid_size) as f32;
            let z = (i / grid_size) as f32;

            // Center the grid
            let pos = Vec3::new(
                (x - grid_size as f32 / 2.0) * spacing,
                0.0,
                (z - grid_size as f32 / 2.0) * spacing,
            );

            Wave {
                position: pos,
                velocity: Vec3::ZERO,
                color: Vec3::new(0.3, 0.5, 0.9),
            }
        })
        .collect();

    let count = particles.len() as u32;

    Simulation::<Wave>::new()
        .with_particle_count(count)
        .with_particle_size(0.018)
        .with_bounds(1.5)
        .with_spawner(move |i, _| particles[i as usize].clone())

        // Primary radial wave - ripples spread outward from center
        .with_rule(Rule::Oscillate {
            axis: Vec3::Y,
            amplitude: 3.0,
            frequency: 1.0,
            spatial_scale: 8.0, // Higher = tighter ripples
        })

        // Secondary faster ripple for complexity
        .with_rule(Rule::Oscillate {
            axis: Vec3::Y,
            amplitude: 1.0,
            frequency: 1.5,
            spatial_scale: 12.0,
        })

        // Color based on height (Y position)
        .with_rule(Rule::Custom(r#"
            let height = p.position.y;
            let h = clamp((height + 0.3) / 0.6, 0.0, 1.0);
            p.color = mix(
                vec3<f32>(0.05, 0.15, 0.4),  // Low = deep blue
                vec3<f32>(0.7, 0.85, 1.0),   // High = light foam
                h
            );
        "#.into()))

        // Light spring to keep particles from drifting
        .with_rule(Rule::Spring {
            anchor: Vec3::ZERO,
            stiffness: 0.3,
            damping: 1.5,
        })

        .with_rule(Rule::Drag(2.0))
        .run();
}
