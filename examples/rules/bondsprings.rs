//! # Cloth Simulation
//!
//! Demonstrates the `Rule::BondSprings` helper for spring-based physics.
//! Compare to the 100+ line manual implementation - now just one rule!
//!
//! Run with: `cargo run --example cloth`

use rdpe::prelude::*;

const WIDTH: u32 = 30;
const HEIGHT: u32 = 25;
const SPACING: f32 = 0.05;
const NO_BOND: u32 = u32::MAX;

#[derive(Particle, Clone)]
struct ClothPoint {
    position: Vec3,
    velocity: Vec3,
    #[color]
    color: Vec3,
    // Bond indices to neighbors
    bond_left: u32,
    bond_right: u32,
    bond_up: u32,
    bond_down: u32,
    pinned: f32,
}

fn main() {
    let total = WIDTH * HEIGHT;

    Simulation::<ClothPoint>::new()
        .with_particle_count(total)
        .with_bounds(2.0)
        .with_spawner(move |i, _count| {
            let x = i % WIDTH;
            let y = i / WIDTH;

            let px = (x as f32 - WIDTH as f32 / 2.0) * SPACING;
            let py = 0.6 - (y as f32) * SPACING;
            let pz = 0.0;

            // Set up bonds to neighbors
            let bond_left = if x > 0 { i - 1 } else { NO_BOND };
            let bond_right = if x < WIDTH - 1 { i + 1 } else { NO_BOND };
            let bond_up = if y > 0 { i - WIDTH } else { NO_BOND };
            let bond_down = if y < HEIGHT - 1 { i + WIDTH } else { NO_BOND };

            // Pin 3 points on top row
            let pinned = if y == 0 && (x == 0 || x == WIDTH - 1 || x == WIDTH / 2) {
                1.0
            } else {
                0.0
            };

            let u = x as f32 / WIDTH as f32;
            let v = y as f32 / HEIGHT as f32;
            let color = Vec3::new(0.8 - v * 0.3, 0.3 + u * 0.4, 0.9);

            ClothPoint {
                position: Vec3::new(px, py, pz),
                velocity: Vec3::ZERO,
                color,
                bond_left,
                bond_right,
                bond_up,
                bond_down,
                pinned,
            }
        })
        // Skip pinned particles
        .with_rule(Rule::Custom("if p.pinned > 0.5 { return; }".into()))
        // THE MAGIC - all spring physics in one line!
        .with_rule(Rule::BondSprings {
            bonds: vec!["bond_left", "bond_right", "bond_up", "bond_down"],
            stiffness: 800.0,
            damping: 15.0,
            rest_length: SPACING,
            max_stretch: Some(1.3),
        })
        // Gravity
        .with_rule(Rule::Gravity(2.0))
        // Wind
        .with_rule(Rule::Custom(r#"
            let wind = sin(uniforms.time * 1.5 + p.position.x * 5.0) * 0.8;
            p.velocity.z += wind * uniforms.delta_time;
        "#.into()))
        // Damping
        .with_rule(Rule::Custom("p.velocity *= 0.98;".into()))
        .with_visuals(|v| {
            v.background(Vec3::new(0.02, 0.02, 0.05));
            v.connections(SPACING * 1.2);
        })
        .with_particle_size(0.01)
        .run();
}
