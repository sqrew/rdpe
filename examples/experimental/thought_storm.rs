//! # Thought Storm
//!
//! Abstract visualization of thoughts forming, growing, merging, and
//! exploding into new ideas. Particles cluster into "thought bubbles"
//! that grow in intensity until they burst, seeding new thoughts.
//!
//! Features:
//! - Clustering behavior creates thought bubbles
//! - Energy accumulation based on local density
//! - Explosive dispersal when energy peaks
//! - Color represents thought "mood" (energy state)
//! - Emergent patterns of creation and destruction
//!
//! Run with: `cargo run --example thought_storm`

use rand::Rng;
use rdpe::prelude::*;

#[derive(Particle, Clone)]
struct Neuron {
    position: Vec3,
    velocity: Vec3,
    energy: f32,    // Accumulated thought energy
    phase: f32,     // Individual oscillation phase
    #[color]
    color: Vec3,
}

fn main() {
    let mut rng = rand::thread_rng();

    let particles: Vec<Neuron> = (0..18_000)
        .map(|_| {
            Neuron {
                position: Vec3::new(
                    rng.gen_range(-0.8..0.8),
                    rng.gen_range(-0.8..0.8),
                    rng.gen_range(-0.8..0.8),
                ),
                velocity: Vec3::new(
                    rng.gen_range(-0.1..0.1),
                    rng.gen_range(-0.1..0.1),
                    rng.gen_range(-0.1..0.1),
                ),
                energy: rng.gen_range(0.0..0.3),
                phase: rng.gen_range(0.0..1.0),
                color: Vec3::new(0.5, 0.5, 0.5),
            }
        })
        .collect();

    Simulation::<Neuron>::new()
        .with_particle_count(18_000)
        .with_particle_size(0.012)
        .with_bounds(1.5)
        .with_spawner(move |i, _| particles[i as usize].clone())
        .with_spatial_config(0.12, 32)

        // === Thought Energy Functions ===
        .with_function(r#"
            fn thought_color(energy: f32, phase: f32) -> vec3<f32> {
                // Low energy: cool blue-purple (dormant thought)
                // Medium: warm orange-yellow (forming idea)
                // High: bright white-cyan (eureka!)

                let pulse = sin(phase * 6.28) * 0.1 + 0.9;

                if energy < 0.3 {
                    let t = energy / 0.3;
                    return mix(
                        vec3<f32>(0.2, 0.1, 0.3),  // Deep purple
                        vec3<f32>(0.3, 0.3, 0.6),  // Soft blue
                        t
                    ) * pulse;
                } else if energy < 0.7 {
                    let t = (energy - 0.3) / 0.4;
                    return mix(
                        vec3<f32>(0.4, 0.3, 0.6),  // Blue-purple
                        vec3<f32>(1.0, 0.7, 0.3),  // Warm orange
                        t
                    ) * pulse;
                } else {
                    let t = (energy - 0.7) / 0.3;
                    return mix(
                        vec3<f32>(1.0, 0.8, 0.4),  // Golden
                        vec3<f32>(1.0, 1.0, 1.0),  // Pure white
                        t
                    ) * (pulse + energy * 0.5);
                }
            }
        "#)

        // Explosion force pattern
        .with_function(r#"
            fn explosion_pattern(pos: vec3<f32>, time: f32) -> vec3<f32> {
                let noise_pos = pos * 3.0 + time;
                return vec3<f32>(
                    noise3(noise_pos),
                    noise3(noise_pos + vec3<f32>(100.0, 0.0, 0.0)),
                    noise3(noise_pos + vec3<f32>(0.0, 100.0, 0.0))
                );
            }
        "#)

        // === Phase and Energy Evolution ===
        .with_rule(Rule::Custom(r#"
            // Advance personal phase
            p.phase += 0.5 * uniforms.delta_time;
            if p.phase > 1.0 {
                p.phase -= 1.0;
            }

            // Very slow decay
            p.energy *= 1.0 - 0.02 * uniforms.delta_time;

            // Constant energy gain - will always eventually explode
            p.energy += 0.25 * uniforms.delta_time;

            // Periodic energy waves sweep through
            let wave = sin(uniforms.time * 1.5 + length(p.position) * 4.0);
            if wave > 0.8 {
                p.energy += 0.5 * uniforms.delta_time;
            }

            // Random energy spikes based on noise
            let spike = noise3(p.position * 5.0 + uniforms.time * 2.0);
            if spike > 0.6 {
                p.energy += (spike - 0.6) * 2.0 * uniforms.delta_time;
            }

            // Cap energy
            p.energy = min(p.energy, 1.2);
        "#.into()))

        // === Clustering Behavior ===
        // Attracted to areas of similar energy
        .with_rule(Rule::Custom(r#"
            // Flow toward energy concentrations
            let gradient = vec3<f32>(
                noise3(p.position * 3.0 + vec3<f32>(0.1, 0.0, 0.0) + uniforms.time * 0.1) -
                noise3(p.position * 3.0 - vec3<f32>(0.1, 0.0, 0.0) + uniforms.time * 0.1),
                noise3(p.position * 3.0 + vec3<f32>(0.0, 0.1, 0.0) + uniforms.time * 0.1) -
                noise3(p.position * 3.0 - vec3<f32>(0.0, 0.1, 0.0) + uniforms.time * 0.1),
                noise3(p.position * 3.0 + vec3<f32>(0.0, 0.0, 0.1) + uniforms.time * 0.1) -
                noise3(p.position * 3.0 - vec3<f32>(0.0, 0.0, 0.1) + uniforms.time * 0.1)
            );

            // Low energy particles cluster, high energy repel
            let cluster_strength = 1.0 - p.energy;
            p.velocity += gradient * cluster_strength * 2.0 * uniforms.delta_time;
        "#.into()))

        // === Explosion Behavior ===
        .with_rule(Rule::Custom(r#"
            // When energy exceeds threshold, EXPLODE
            if p.energy > 0.75 {
                // Burst outward dramatically!
                let burst_dir = explosion_pattern(p.position, uniforms.time);
                let burst_strength = (p.energy - 0.75) * 20.0;
                p.velocity += burst_dir * burst_strength;

                // Also burst away from center
                let dist = length(p.position);
                if dist > 0.01 {
                    p.velocity += normalize(p.position) * burst_strength * 0.5;
                }

                // Reset energy (thought released!)
                p.energy = 0.05;
            }

            // High energy particles jitter excitedly
            if p.energy > 0.4 {
                let jitter = explosion_pattern(p.position, uniforms.time * 3.0) * (p.energy - 0.4) * 2.0;
                p.velocity += jitter * uniforms.delta_time;
            }
        "#.into()))

        // === Containment and Movement ===
        .with_rule(Rule::Custom(r#"
            // Spherical soft boundary
            let dist = length(p.position);
            if dist > 0.7 {
                let push = (dist - 0.7) * 5.0;
                p.velocity -= normalize(p.position) * push * uniforms.delta_time;
            }

            // Gentle upward bias to prevent sinking
            p.velocity.y += 0.3 * uniforms.delta_time;

            // Curl noise for interesting movement patterns
            let curl_pos = p.position * 2.0 + uniforms.time * 0.2;
            let drift = vec3<f32>(
                noise3(curl_pos + vec3<f32>(0.0, 100.0, 0.0)) - noise3(curl_pos - vec3<f32>(0.0, 100.0, 0.0)),
                noise3(curl_pos + vec3<f32>(100.0, 0.0, 0.0)) - noise3(curl_pos - vec3<f32>(100.0, 0.0, 0.0)),
                noise3(curl_pos + vec3<f32>(0.0, 0.0, 100.0)) - noise3(curl_pos - vec3<f32>(0.0, 0.0, 100.0))
            );
            p.velocity += drift * 0.8 * uniforms.delta_time;
        "#.into()))

        // === Visual Representation ===
        .with_rule(Rule::Custom(r#"
            p.color = thought_color(p.energy, p.phase);
            p.scale = 0.5 + p.energy * 1.5;
        "#.into()))

        .with_rule(Rule::Drag(2.5))
        .with_rule(Rule::SpeedLimit { min: 0.0, max: 2.0 })
        .with_rule(Rule::WrapWalls)
        .run();
}
