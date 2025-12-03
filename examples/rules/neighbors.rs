//! # Custom Neighbor Rules Demo
//!
//! Demonstrates `Rule::NeighborCustom` - write your own particle-particle
//! interactions with full access to neighbor data.
//!
//! This example implements a simple "slime mold" behavior:
//! - Particles are attracted to neighbors at medium range
//! - Particles repel neighbors at close range
//! - Color blends between nearby particles
//!
//! ## What This Demonstrates
//!
//! - `Rule::NeighborCustom` - raw WGSL code with neighbor access
//! - Available variables: `other` (neighbor particle), `neighbor_dir`, `neighbor_dist`
//! - Distance-based attraction/repulsion zones
//! - Color blending between particles (`mix()` function)
//! - Combining custom rules with center attraction
//!
//! ## The Mechanics
//!
//! **WGSL Variables Available**:
//! - `other` - the neighbor particle struct (access `other.position`, `other.color`, etc.)
//! - `neighbor_dir` - normalized direction from this particle to neighbor
//! - `neighbor_dist` - distance between particles
//!
//! **Zone Logic**: This example creates two behavior zones:
//! - Close range (< 0.05): Strong repulsion (personal space)
//! - Medium range (0.05-0.12): Gentle attraction (social zone)
//!
//! The result is particles that cluster into groups at a comfortable
//! spacing, similar to slime mold aggregation.
//!
//! ## Try This
//!
//! - Adjust zone boundaries (0.05, 0.12) for different clustering
//! - Change attraction/repulsion strengths (8.0, 2.0)
//! - Remove color blending to see distinct color regions
//! - Add `other.velocity` alignment for flocking behavior
//! - Try repulsion at medium range for dispersed patterns
//!
//! Run with: `cargo run --example neighbors`

use rand::Rng;
use rdpe::prelude::*;

#[derive(Particle, Clone)]
struct Mold {
    position: Vec3,
    velocity: Vec3,
    #[color]
    color: Vec3,
}

fn main() {
    let mut rng = rand::thread_rng();

    let particles: Vec<Mold> = (0..15_000)
        .map(|_| {
            // Random hue for each particle
            let hue = rng.gen_range(0.0..1.0);
            let color = hsv_to_rgb(hue, 0.8, 1.0);

            Mold {
                position: Vec3::new(
                    rng.gen_range(-0.8..0.8),
                    rng.gen_range(-0.8..0.8),
                    rng.gen_range(-0.8..0.8),
                ),
                velocity: Vec3::ZERO,
                color,
            }
        })
        .collect();

    Simulation::<Mold>::new()
        .with_particle_count(15_000)
        .with_particle_size(0.012)
        .with_bounds(1.0)
        .with_spawner(move |i, _| particles[i as usize].clone())
        // Spatial hashing is REQUIRED for NeighborCustom
        .with_spatial_config(0.15, 32)

        // Custom neighbor interaction - this is the magic!
        .with_rule(Rule::NeighborCustom(r#"
            // Attraction zone: 0.05 - 0.12
            // Repulsion zone: 0 - 0.05

            if neighbor_dist < 0.05 && neighbor_dist > 0.001 {
                // Close range: strong repulsion (personal space)
                let repel = (0.05 - neighbor_dist) / 0.05;
                p.velocity += neighbor_dir * repel * 8.0 * uniforms.delta_time;
            } else if neighbor_dist < 0.12 {
                // Medium range: gentle attraction (social)
                let attract = (neighbor_dist - 0.05) / 0.07;
                p.velocity -= neighbor_dir * attract * 2.0 * uniforms.delta_time;
            }

            // Blend colors with very close neighbors
            if neighbor_dist < 0.06 {
                let blend = 0.3 * (1.0 - neighbor_dist / 0.06);
                p.color = mix(p.color, other.color, blend * uniforms.delta_time);
            }
        "#.into()))

        // Gentle center attraction to keep things visible
        .with_rule(Rule::Custom(r#"
            let to_center = -p.position;
            let dist = length(to_center);
            if dist > 0.5 {
                p.velocity += normalize(to_center) * (dist - 0.5) * 0.5 * uniforms.delta_time;
            }
        "#.into()))

        .with_rule(Rule::Drag(1.5))
        .with_rule(Rule::SpeedLimit { min: 0.0, max: 1.5 })
        .with_rule(Rule::BounceWalls)
        .run();
}

// Helper to convert HSV to RGB
fn hsv_to_rgb(h: f32, s: f32, v: f32) -> Vec3 {
    let c = v * s;
    let h_prime = h * 6.0;
    let x = c * (1.0 - ((h_prime % 2.0) - 1.0).abs());
    let m = v - c;

    let (r, g, b) = if h_prime < 1.0 {
        (c, x, 0.0)
    } else if h_prime < 2.0 {
        (x, c, 0.0)
    } else if h_prime < 3.0 {
        (0.0, c, x)
    } else if h_prime < 4.0 {
        (0.0, x, c)
    } else if h_prime < 5.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };

    Vec3::new(r + m, g + m, b + m)
}
