//! # Particle Connections Example
//!
//! Network/web visualization with lines drawn between nearby particles.
//! Creates organic, web-like structures as particles move.
//!
//! ## What This Demonstrates
//!
//! - `.with_visuals(|v| v.connections(radius))` - enable connection lines
//! - Spatial hashing required for neighbor detection
//! - `Rule::Separate` + `Rule::AttractTo` for organic spacing
//! - `Rule::Wander` for random exploration
//!
//! ## How Connections Work
//!
//! The renderer uses spatial hashing to find particles within `radius`
//! of each other, then draws lines between them. Line opacity fades
//! with distance - closer particles have brighter connections.
//!
//! **Requirements**:
//! - Must call `.with_spatial_config()` with cell size >= connection radius
//! - Fewer particles (100-1000) show clearer structure
//! - Many particles create dense webs
//!
//! ## Use Cases
//!
//! - Neural networks / brain visualization
//! - Social network graphs
//! - Constellation patterns
//! - Organic tissue simulation
//! - Plexus / geometric patterns
//!
//! ## Try This
//!
//! - Increase connection radius to 0.25 for denser web
//! - Add more particles (2000) for complex network
//! - Color particles by `particle_type` for multi-network visualization
//! - Remove `Wander` for more stable structures
//! - Add `Rule::Align` for flowing, directional networks
//!
//! Run with: `cargo run --example connections`

use rand::Rng;
use rdpe::prelude::*;

#[derive(rdpe::Particle, Clone)]
struct Node {
    position: Vec3,
    velocity: Vec3,
    #[color]
    color: Vec3,
}

fn main() {
    let mut rng = rand::thread_rng();
    let count = 500; // Fewer particles for visible connections

    // Pre-generate nodes
    let particles: Vec<Node> = (0..count)
        .map(|_| {
            // Spawn randomly in a smaller area
            let x = rng.gen_range(-0.5..0.5);
            let y = rng.gen_range(-0.5..0.5);
            let z = rng.gen_range(-0.5..0.5);

            Node {
                position: Vec3::new(x, y, z),
                velocity: Vec3::ZERO,
                color: Vec3::new(0.3, 0.6, 1.0), // Blue nodes
            }
        })
        .collect();

    Simulation::<Node>::new()
        .with_particle_count(count)
        .with_bounds(1.0)
        .with_particle_size(0.02)
        .with_spatial_config(0.15, 32) // Spatial config needed for connections
        .with_spawner(move |i, _| particles[i as usize].clone())
        // Enable connections - draw lines between particles within 0.15 units
        .with_visuals(|v| {
            v.connections(0.15);
            v.blend_mode(BlendMode::Additive);
        })
        // Gentle separation to spread out
        .with_rule(Rule::Separate {
            radius: 0.1,
            strength: 0.2,
        })
        // Soft cohesion to keep together
        .with_rule(Rule::AttractTo {
            point: Vec3::ZERO,
            strength: 0.1,
        })
        // Random wandering
        .with_rule(Rule::Wander {
            strength: 0.5,
            frequency: 2.0,
        })
        // Drag to keep stable
        .with_rule(Rule::Drag(2.0))
        // Speed limit
        .with_rule(Rule::SpeedLimit { min: 0.0, max: 0.3 })
        // Bounce off walls
        .with_rule(Rule::BounceWalls)
        .run();
}
