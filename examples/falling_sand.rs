//! # Falling Sand Simulation
//!
//! Sand grains fall and pile up using physics-based stacking.
//!
//! ## What This Demonstrates
//!
//! - Gravity-driven falling
//! - Separation forces for pile formation
//! - Grid snapping for discrete look
//!
//! Run with: `cargo run --example falling_sand --release`

use rand::Rng;
use rdpe::prelude::*;

#[derive(Particle, Clone)]
struct SandGrain {
    position: Vec3,
    velocity: Vec3,
    #[color]
    color: Vec3,
}

fn main() {
    let mut rng = rand::thread_rng();

    let num_grains = 3000;
    let grid_size = 80;
    let cell_size = 2.0 / grid_size as f32;

    // Spawn all grains stacked vertically above the scene
    // They'll fall one after another like an hourglass
    let grains: Vec<_> = (0..num_grains)
        .map(|i| {
            let color_var = rng.gen_range(0.0..0.15);
            // Stack them in a tall column above the scene
            // Small random x offset to prevent perfect alignment
            let x_offset = rng.gen_range(-0.1..0.1);
            (
                Vec3::new(
                    x_offset,
                    1.0 + (i as f32) * cell_size * 0.5, // Stack upward
                    0.0,
                ),
                Vec3::new(0.9 + color_var * 0.5, 0.75 + color_var, 0.4 + color_var),
            )
        })
        .collect();

    Simulation::<SandGrain>::new()
        .with_particle_count(num_grains as u32)
        .with_bounds(2.0) // Larger bounds to fit tall stack
        .with_spawner(move |i, _| {
            let (pos, color) = grains[i as usize];
            SandGrain {
                position: pos,
                velocity: Vec3::ZERO,
                color,
            }
        })
        // Spatial hashing for separation
        .with_spatial_config(cell_size * 1.5, 32)
        .with_uniform::<f32>("cell_size", cell_size)
        // Gravity - strong pull down
        .with_rule(Rule::Gravity(3.0))
        // Separation with upward bias for stacking
        .with_rule(Rule::Separate {
            radius: cell_size * 0.95,
            strength: 15.0,
        })
        // Heavy damping for sand-like settling
        .with_rule(Rule::Drag(8.0))
        // Floor, walls, grid snap, and 2D constraint
        .with_rule(Rule::Custom(
            r#"
            let cell_size = uniforms.cell_size;

            // Hard floor
            if p.position.y < -0.95 {
                p.position.y = -0.95;
                if p.velocity.y < 0.0 {
                    p.velocity.y = 0.0;
                }
            }

            // Walls
            if p.position.x < -0.95 {
                p.position.x = -0.95;
                p.velocity.x = abs(p.velocity.x) * 0.3;
            }
            if p.position.x > 0.95 {
                p.position.x = 0.95;
                p.velocity.x = -abs(p.velocity.x) * 0.3;
            }

            // Keep 2D
            p.position.z = 0.0;
            p.velocity.z = 0.0;

            // Soft grid snap
            let gx = floor((p.position.x + 1.0) / cell_size);
            let gy = floor((p.position.y + 1.0) / cell_size);
            p.position.x = mix(p.position.x, -1.0 + (gx + 0.5) * cell_size, 0.15);
            p.position.y = mix(p.position.y, -1.0 + (gy + 0.5) * cell_size, 0.15);
            "#
            .into(),
        ))
        .with_visuals(|v| {
            v.background(Vec3::new(0.12, 0.14, 0.2));
            v.shape(ParticleShape::Square);
        })
        .run();
}
