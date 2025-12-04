//! # Falling Sand
//!
//! Simple granular physics - sand grains fall and pile up.
//!
//! Run with: `cargo run --example sand`

use rand::Rng;
use rdpe::prelude::*;

const GRAIN_COUNT: u32 = 20_000;
const GRAIN_SIZE: f32 = 0.015;

#[derive(Particle, Clone)]
struct SandGrain {
    position: Vec3,
    velocity: Vec3,
    #[color]
    color: Vec3,
}

fn main() {
    let mut rng = rand::thread_rng();

    // Pre-generate random positions and colors
    let grains: Vec<_> = (0..GRAIN_COUNT)
        .map(|_| {
            // Start scattered above
            let x = rng.gen_range(-0.6..0.6);
            let y = rng.gen_range(0.0..0.9);
            let z = rng.gen_range(-0.6..0.6);

            // Sandy colors - tans, browns, ochres
            let base = rng.gen_range(0.6..0.9);
            let color = Vec3::new(
                base,
                base * rng.gen_range(0.7..0.9),
                base * rng.gen_range(0.4..0.6),
            );

            (x, y, z, color)
        })
        .collect();

    Simulation::<SandGrain>::new()
        .with_particle_count(GRAIN_COUNT)
        .with_bounds(1.5)
        .with_spawner(move |i, _| {
            let (x, y, z, color) = grains[i as usize];
            SandGrain {
                position: Vec3::new(x, y, z),
                velocity: Vec3::ZERO,
                color,
            }
        })
        .with_spatial_config(0.08, 32)
        .with_uniform("grain_size", GRAIN_SIZE)
        .with_uniform("stiffness", 50.0f32)
        .with_uniform("friction", 0.85f32)
        // Gravity
        .with_rule(Rule::Gravity(3.0))
        // Grain-grain collision
        .with_rule(Rule::NeighborCustom(r#"
            if neighbor_dist < uniforms.grain_size * 2.0 && neighbor_dist > 0.0001 {
                // Repulsion - push apart when overlapping
                let overlap = uniforms.grain_size * 2.0 - neighbor_dist;
                let repel_force = overlap * uniforms.stiffness;
                p.velocity -= neighbor_dir * repel_force * uniforms.delta_time;

                // Friction - dampen when in contact
                p.velocity *= uniforms.friction;
            }
        "#.into()))
        // Floor and walls
        .with_rule(Rule::Custom(r#"
            let floor = -0.9;
            let wall = 0.7;
            let bounce = 0.1;

            // Floor
            if p.position.y < floor + uniforms.grain_size {
                p.position.y = floor + uniforms.grain_size;
                p.velocity.y *= -bounce;
                p.velocity.x *= uniforms.friction;
                p.velocity.z *= uniforms.friction;
            }

            // Walls
            if p.position.x > wall {
                p.position.x = wall;
                p.velocity.x *= -bounce;
            }
            if p.position.x < -wall {
                p.position.x = -wall;
                p.velocity.x *= -bounce;
            }
            if p.position.z > wall {
                p.position.z = wall;
                p.velocity.z *= -bounce;
            }
            if p.position.z < -wall {
                p.position.z = -wall;
                p.velocity.z *= -bounce;
            }
        "#.into()))
        // General damping
        .with_rule(Rule::Drag(0.5))
        .with_visuals(|v| {
            v.background(Vec3::new(0.1, 0.1, 0.12));
        })
        .with_particle_size(0.008)
        .run();
}
