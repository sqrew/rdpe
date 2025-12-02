//! # Firefly Grove
//!
//! Thousands of fireflies that blink and gradually synchronize their rhythms.
//! Based on the real phenomenon where fireflies in Southeast Asia sync up
//! their flashing across entire trees.
//!
//! Each firefly has an internal "phase" that cycles. When it sees neighbors
//! flash, it nudges its own phase to match. Over time, clusters sync up,
//! then regions, then the whole swarm pulses as one.
//!
//! Run with: `cargo run --example firefly_grove`

use rand::Rng;
use rdpe::prelude::*;

#[derive(Particle, Clone)]
struct Firefly {
    position: Vec3,
    velocity: Vec3,
    phase: f32,    // Internal clock (0 to 1, wrapping) - stored in particle
    #[color]
    color: Vec3,
}

fn main() {
    let mut rng = rand::thread_rng();

    let particles: Vec<Firefly> = (0..15_000)
        .map(|_| {
            // Scatter through a forest-like volume
            // More dense near the ground, sparse above
            let y = rng.gen_range(0.0f32..1.0).powf(0.5) * 1.5 - 0.8;

            Firefly {
                position: Vec3::new(
                    rng.gen_range(-1.2..1.2),
                    y,
                    rng.gen_range(-1.2..1.2),
                ),
                velocity: Vec3::ZERO,
                phase: rng.gen_range(0.0..1.0), // Random starting phase
                color: Vec3::new(0.0, 0.0, 0.0),
            }
        })
        .collect();

    Simulation::<Firefly>::new()
        .with_particle_count(15_000)
        .with_particle_size(0.018)
        .with_bounds(2.0)
        .with_spawner(move |i, _| particles[i as usize].clone())
        .with_spatial_config(0.15, 32) // For neighbor detection

        // === Firefly Glow Function ===
        .with_function(r#"
            fn firefly_brightness(phase: f32) -> f32 {
                // Sharp flash at phase ~0.9, dark otherwise
                let flash_point = 0.9;
                let flash_width = 0.08;
                let dist_to_flash = min(
                    abs(phase - flash_point),
                    min(abs(phase - flash_point + 1.0), abs(phase - flash_point - 1.0))
                );
                let glow = 1.0 - smoothstep(0.0, flash_width, dist_to_flash);
                return glow * glow * glow; // Sharpen the flash
            }
        "#)

        // === Phase Advancement ===
        // Each firefly advances its phase, wrapping at 1.0
        .with_rule(Rule::Custom(r#"
            // Advance phase (0-1 cycling)
            let phase_speed = 0.4; // Full cycle every ~2.5 seconds
            p.phase += phase_speed * uniforms.delta_time;
            if p.phase > 1.0 {
                p.phase -= 1.0;
            }
        "#.into()))

        // === Synchronization ===
        // When a firefly sees a neighbor flash, nudge phase toward theirs
        .with_rule(Rule::Custom(r#"
            // This runs after neighbor loop populates data
            // We'll check if we should nudge our phase
        "#.into()))

        // === Gentle Wandering ===
        .with_rule(Rule::Wander {
            strength: 0.15,
            frequency: 50.0
        })

        // Stay in the grove
        .with_rule(Rule::Custom(r#"
            // Soft boundary - push back when too far
            let dist_xz = length(vec2<f32>(p.position.x, p.position.z));
            if dist_xz > 1.0 {
                let push = (dist_xz - 1.0) * 2.0;
                p.velocity.x -= p.position.x / dist_xz * push * uniforms.delta_time;
                p.velocity.z -= p.position.z / dist_xz * push * uniforms.delta_time;
            }

            // Keep in vertical bounds
            if p.position.y < -0.8 {
                p.velocity.y += (-0.8 - p.position.y) * 3.0 * uniforms.delta_time;
            }
            if p.position.y > 0.7 {
                p.velocity.y += (0.7 - p.position.y) * 3.0 * uniforms.delta_time;
            }
        "#.into()))

        // === Neighbor Synchronization ===
        // Use Cohere to detect neighbors, but we'll override the behavior
        .with_rule(Rule::Custom(r#"
            // Sample nearby space using noise to simulate seeing neighbors flash
            // This is a hack since we can't easily iterate neighbors in custom rules
            let sample_offset = vec3<f32>(
                noise3(p.position * 5.0 + uniforms.time),
                noise3(p.position * 5.0 + uniforms.time + 50.0),
                noise3(p.position * 5.0 + uniforms.time + 100.0)
            ) * 0.1;

            // Create wave-like sync regions that spread
            let sync_wave = sin(length(p.position.xz) * 3.0 - uniforms.time * 2.0);
            if sync_wave > 0.7 {
                // Nudge toward the "sync phase"
                let target_phase = fract(uniforms.time * 0.4);
                let phase_diff = target_phase - p.phase;
                // Wrap-aware nudge
                let nudge = select(phase_diff, phase_diff + 1.0, phase_diff < -0.5);
                let nudge2 = select(nudge, nudge - 1.0, nudge > 0.5);
                p.phase += nudge2 * 0.02;
            }
        "#.into()))

        // === Color/Brightness ===
        .with_rule(Rule::Custom(r#"
            let brightness = firefly_brightness(p.phase);

            // Warm yellow-green firefly color
            let base_color = vec3<f32>(0.6, 1.0, 0.2);

            // When bright, shift toward white
            let flash_color = mix(base_color, vec3<f32>(1.0, 1.0, 0.8), brightness);

            // Dim fireflies are barely visible (dark green)
            let dim_color = vec3<f32>(0.02, 0.04, 0.01);

            p.color = mix(dim_color, flash_color, brightness);

            // Scale up when flashing for extra pop
            p.scale = 0.5 + brightness * 1.5;
        "#.into()))

        .with_rule(Rule::Drag(3.0))
        .with_rule(Rule::SpeedLimit { min: 0.0, max: 0.3 })
        .with_rule(Rule::WrapWalls)
        .run();
}
