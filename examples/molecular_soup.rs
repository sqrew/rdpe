//! Molecular Soup - Lennard-Jones potential simulation
//!
//! Watch particles self-organize into clusters and crystal-like structures
//! through realistic molecular forces. Particles repel at close range and
//! attract at medium range, finding natural equilibrium distances.
//!
//! Run with: cargo run --example molecular_soup --release

use rdpe::prelude::*;

#[derive(Particle, Clone)]
struct Molecule {
    position: Vec3,
    velocity: Vec3,
}

fn main() {
    // Lennard-Jones parameters
    let sigma = 0.04; // Effective particle diameter
    let epsilon = 2.0; // Well depth (attraction strength)
    let cutoff = sigma * 2.5; // Standard cutoff

    Simulation::<Molecule>::new()
        .with_particle_count(3000)
        .with_bounds(1.0)
        .with_spawner(|i, total| {
            // Start with random positions and small random velocities
            let angle1 = (i as f32 / total as f32) * std::f32::consts::TAU * 47.0;
            let angle2 = (i as f32 / total as f32) * std::f32::consts::TAU * 31.0;
            let r = 0.7 * ((i as f32 * 0.618033) % 1.0).sqrt();

            let x = r * angle1.cos() * angle2.cos();
            let y = r * angle1.sin() * 0.5;
            let z = r * angle1.cos() * angle2.sin();

            // Small random initial velocity
            let vx = ((i * 7) % 100) as f32 / 100.0 - 0.5;
            let vy = ((i * 13) % 100) as f32 / 100.0 - 0.5;
            let vz = ((i * 19) % 100) as f32 / 100.0 - 0.5;

            Molecule {
                position: Vec3::new(x, y, z),
                velocity: Vec3::new(vx, vy, vz) * 0.3,
            }
        })
        .with_spatial_config(cutoff, 32)
        // Lennard-Jones molecular forces
        .with_rule(Rule::LennardJones {
            epsilon,
            sigma,
            cutoff,
        })
        // Gentle drag to let system settle
        .with_rule(Rule::Drag(0.5))
        // Keep velocities reasonable
        .with_rule(Rule::SpeedLimit { min: 0.0, max: 2.0 })
        // Bounce off walls
        .with_rule(Rule::BounceWalls)
        .run();
}
