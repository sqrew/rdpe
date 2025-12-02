//! # Inbox Example: Energy Transfer
//!
//! Demonstrates particle-to-particle communication using the inbox system.
//! Particles transfer energy to their neighbors, creating a diffusion effect.
//!
//! Run with: `cargo run --example inbox`

use rand::Rng;
use rdpe::prelude::*;

#[derive(Particle, Clone)]
struct EnergyParticle {
    position: Vec3,
    velocity: Vec3,
    #[color]
    color: Vec3,
    /// Energy level (0.0 to 1.0)
    energy: f32,
}

fn main() {
    let mut rng = rand::thread_rng();

    // Create particles in a grid pattern
    // Some particles start with high energy, others with none
    let particles: Vec<_> = (0..5000)
        .map(|i| {
            let x = (i % 50) as f32 / 50.0 * 2.0 - 1.0;
            let z = (i / 50) as f32 / 100.0 * 2.0 - 1.0;
            let pos = Vec3::new(
                x + rng.gen_range(-0.01..0.01),
                0.0,
                z + rng.gen_range(-0.01..0.01),
            );

            // Seed a few "hot spots" with high energy
            let energy = if (i % 500) < 10 {
                1.0 // Hot particle
            } else {
                0.0 // Cold particle
            };

            (pos, energy)
        })
        .collect();

    Simulation::<EnergyParticle>::new()
        .with_particle_count(5000)
        .with_bounds(1.5)
        .with_spawner(move |i, _| {
            let (pos, energy) = particles[i as usize];
            let color = energy_to_color(energy);
            EnergyParticle {
                position: pos,
                velocity: Vec3::ZERO,
                color,
                energy,
            }
        })
        // Enable particle communication!
        .with_inbox()
        .with_spatial_config(0.15, 32)

        // Neighbor rule: transfer energy to neighbors
        .with_rule(Rule::NeighborCustom(r#"
            let transfer_radius = 0.1;

            if neighbor_dist < transfer_radius && neighbor_dist > 0.001 {
                // Transfer some energy to neighbor (through their inbox)
                // Transfer more to closer neighbors
                let weight = 1.0 - neighbor_dist / transfer_radius;
                let transfer_amount = p.energy * 0.02 * weight;

                // Send energy to neighbor's inbox channel 0
                inbox_send(other_idx, 0u, transfer_amount);
            }
        "#.into()))

        // Custom rule: receive energy and update visuals
        .with_rule(Rule::Custom(r#"
            // Receive accumulated energy from inbox channel 0
            let received = inbox_receive_at(index, 0u);

            // Also lose some energy (conservation by what we sent)
            // Count approximate neighbors for energy loss
            p.energy = p.energy * 0.96 + received;

            // Clamp energy
            p.energy = clamp(p.energy, 0.0, 1.0);

            // Update color based on energy (cold = blue, hot = red)
            if p.energy < 0.33 {
                // Blue to cyan
                let t = p.energy / 0.33;
                p.color = mix(vec3<f32>(0.1, 0.2, 0.8), vec3<f32>(0.2, 0.8, 0.9), t);
            } else if p.energy < 0.66 {
                // Cyan to yellow
                let t = (p.energy - 0.33) / 0.33;
                p.color = mix(vec3<f32>(0.2, 0.8, 0.9), vec3<f32>(1.0, 0.9, 0.2), t);
            } else {
                // Yellow to red
                let t = (p.energy - 0.66) / 0.34;
                p.color = mix(vec3<f32>(1.0, 0.9, 0.2), vec3<f32>(1.0, 0.3, 0.1), t);
            }

            // Add subtle floating motion
            p.velocity.y = sin(uniforms.time * 2.0 + p.position.x * 5.0) * 0.01;
            p.position.y = sin(uniforms.time + p.position.x * 3.0 + p.position.z * 2.0) * 0.05;
        "#.into()))

        .run();
}

fn energy_to_color(energy: f32) -> Vec3 {
    if energy < 0.33 {
        let t = energy / 0.33;
        Vec3::new(0.1, 0.2, 0.8).lerp(Vec3::new(0.2, 0.8, 0.9), t)
    } else if energy < 0.66 {
        let t = (energy - 0.33) / 0.33;
        Vec3::new(0.2, 0.8, 0.9).lerp(Vec3::new(1.0, 0.9, 0.2), t)
    } else {
        let t = (energy - 0.66) / 0.34;
        Vec3::new(1.0, 0.9, 0.2).lerp(Vec3::new(1.0, 0.3, 0.1), t)
    }
}
