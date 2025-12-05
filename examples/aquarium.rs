//! # Aquarium - Fish Schooling Simulation
//!
//! An underwater ecosystem with schooling fish and a lurking predator.
//! Fish exhibit natural behaviors: darting, gliding, depth preferences,
//! and mass panic when the shark gets too close.
//!
//! ## What This Demonstrates
//!
//! - Fish-specific behaviors beyond basic boids (darting, gliding)
//! - Multi-species ecosystem with predator/prey dynamics
//! - Depth layering (fish prefer certain depths)
//! - Panic propagation through the school via inbox system
//! - Underwater visual effects (caustics, color grading)
//! - `velocity_stretch` for elongated fish shapes
//!
//! ## Fish Behaviors
//!
//! Unlike simple boids, fish:
//! - **Dart**: Random bursts of speed (not constant velocity)
//! - **Glide**: Coast between bursts with low drag
//! - **Layer**: Prefer specific depths, avoid surface/bottom
//! - **Panic**: When shark is near, panic spreads through the school
//!
//! ## The Shark
//!
//! A single predator that cruises slowly. When it gets close to fish,
//! they scatter in panic.
//!
//! ## Try This
//!
//! - Increase shark count for more chaos
//! - Remove the shark to see peaceful schooling
//! - Change depth preferences to create layered schools
//!
//! Run with: `cargo run --example aquarium`

use rand::Rng;
use rdpe::prelude::*;

#[derive(ParticleType, Clone, Copy, PartialEq)]
enum Species {
    SmallFish, // 0 - schooling fish
    Shark,     // 1 - predator
}

#[derive(Particle, Clone)]
struct Fish {
    position: Vec3,
    velocity: Vec3,
    #[color]
    color: Vec3,
    particle_type: u32,
    // Custom fields for fish behavior
    energy: f32,         // Builds up, triggers darting
    panic: f32,          // Fear level (0-1)
    preferred_depth: f32, // Y position fish wants to be at
}

fn main() {
    let mut rng = rand::thread_rng();

    let num_fish = 1500;
    let num_sharks = 1;
    let total = num_fish + num_sharks;

    let particles: Vec<Fish> = (0..total)
        .map(|i| {
            let is_shark = i >= num_fish;

            if is_shark {
                // Shark: large, red, starts in center
                Fish {
                    position: Vec3::new(
                        rng.gen_range(-0.3..0.3),
                        rng.gen_range(-0.2..0.2),
                        rng.gen_range(-0.3..0.3),
                    ),
                    velocity: Vec3::new(
                        rng.gen_range(-0.1..0.1),
                        0.0,
                        rng.gen_range(-0.1..0.1),
                    ),
                    color: Vec3::new(0.9, 0.2, 0.15), // Red - easy to spot
                    particle_type: Species::Shark.into(),
                    energy: 0.0,
                    panic: 0.0,
                    preferred_depth: 0.0,
                }
            } else {
                // Small fish: colorful, spread out
                let depth = rng.gen_range(-0.6..0.6);

                // Color varies by depth - warmer colors up top, cooler below
                let depth_factor = (depth + 0.6) / 1.2; // 0-1
                let color = Vec3::new(
                    0.3 + depth_factor * 0.4,         // More orange/gold near surface
                    0.5 + rng.gen_range(0.0..0.3),    // Green varies
                    0.5 + (1.0 - depth_factor) * 0.4, // More blue at depth
                );

                Fish {
                    position: Vec3::new(
                        rng.gen_range(-0.8..0.8),
                        depth,
                        rng.gen_range(-0.8..0.8),
                    ),
                    velocity: Vec3::new(
                        rng.gen_range(-0.05..0.05),
                        rng.gen_range(-0.01..0.01),
                        rng.gen_range(-0.05..0.05),
                    ),
                    color,
                    particle_type: Species::SmallFish.into(),
                    energy: rng.gen_range(0.0..1.0),
                    panic: 0.0,
                    preferred_depth: depth + rng.gen_range(-0.1..0.1),
                }
            }
        })
        .collect();

    Simulation::<Fish>::new()
        .with_particle_count(total as u32)
        .with_bounds(1.0)
        .with_spatial_config(0.15, 32)
        .with_particle_size(0.018)
        .with_spawner(move |i, _| particles[i as usize].clone())

        // === Visual Setup ===
        .with_visuals(|v| {
            // Dark blue underwater background
            v.background(Vec3::new(0.01, 0.04, 0.1));

            // Stretch particles in velocity direction (fish shape)
            v.velocity_stretch(3.0);

            // Underwater caustics and color grading
            v.post_process(
                r#"
                // Sample the scene
                var color = textureSample(scene, scene_sampler, in.uv).rgb;

                // Caustics - animated light patterns
                let t = uniforms.time * 0.4;
                let caustic_uv = in.uv * 6.0;
                let c1 = sin(caustic_uv.x * 3.0 + t) * sin(caustic_uv.y * 2.0 + t * 0.7);
                let c2 = sin(caustic_uv.x * 2.0 - t * 0.8) * sin(caustic_uv.y * 3.0 + t * 0.5);
                let caustic = (c1 + c2) * 0.5 + 0.5;
                let caustic_intensity = caustic * 0.06;

                // Apply caustics more at top of screen (near surface)
                let depth_factor = in.uv.y;
                color += vec3<f32>(0.08, 0.12, 0.15) * caustic_intensity * depth_factor;

                // Depth fog - darker at bottom
                let fog = vec3<f32>(0.01, 0.04, 0.1);
                color = mix(color, fog, (1.0 - depth_factor) * 0.25);

                // Blue-green tint
                color = color * vec3<f32>(0.85, 0.95, 1.1);

                // Vignette
                let vignette = 1.0 - length(in.uv - 0.5) * 0.7;
                color *= vignette;

                return vec4<f32>(color, 1.0);
            "#,
            );
        })

        // === Fish Schooling (Small Fish Only) ===
        .with_rule(Rule::Typed {
            self_type: Species::SmallFish.into(),
            other_type: Some(Species::SmallFish.into()),
            rule: Box::new(Rule::Separate {
                radius: 0.03,
                strength: 1.5,
            }),
        })
        .with_rule(Rule::Typed {
            self_type: Species::SmallFish.into(),
            other_type: Some(Species::SmallFish.into()),
            rule: Box::new(Rule::Cohere {
                radius: 0.15,
                strength: 0.3,
            }),
        })
        .with_rule(Rule::Typed {
            self_type: Species::SmallFish.into(),
            other_type: Some(Species::SmallFish.into()),
            rule: Box::new(Rule::Align {
                radius: 0.1,
                strength: 1.0,
            }),
        })

        // === Fish Evade Shark ===
        .with_rule(Rule::Evade {
            self_type: Species::SmallFish.into(),
            threat_type: Species::Shark.into(),
            radius: 0.2,
            strength: 2.0,
        })

        // === Panic Propagation (disabled for calmer behavior) ===
        // Keeping the neighbor rule simple - just shark detection
        .with_rule(Rule::NeighborCustom(
            r#"
            // If I'm a fish near a shark, set panic (no spreading)
            if (p.particle_type == 0u && other.particle_type == 1u && neighbor_dist < 0.15) {
                p.panic = 0.5;
            }
        "#
            .into(),
        ))

        // === Shark Behavior ===
        // Shark cruises slowly, loosely chases fish
        .with_rule(Rule::Chase {
            self_type: Species::Shark.into(),
            target_type: Species::SmallFish.into(),
            radius: 0.6,
            strength: 0.3, // Slow pursuit
        })

        // === Fish-Specific Behaviors ===
        .with_rule(Rule::Custom(
            r#"
            // Only for fish
            if (p.particle_type == 0u) {
                // Panic decay
                p.panic = max(0.0, p.panic - 0.02);

                // Darting: random bursts of speed
                p.energy += 0.015;

                // Create a time-varying seed for randomness
                let time_seed = u32(uniforms.time * 60.0);
                let seed = index * 31337u + time_seed;

                // Normal dart (when not panicking)
                if (p.energy > 1.0 && length(p.velocity) > 0.01 && p.panic < 0.3) {
                    p.velocity += normalize(p.velocity) * 0.1;
                    p.energy = rand(seed) * 0.5;
                }

                // Panic dart (random direction, gentler)
                if (p.panic > 0.5) {
                    let panic_strength = p.panic * 0.08;
                    p.velocity += vec3<f32>(
                        (rand(seed + 1u) - 0.5) * panic_strength,
                        (rand(seed + 2u) - 0.5) * panic_strength * 0.2,
                        (rand(seed + 3u) - 0.5) * panic_strength
                    );
                }

                // Depth preference: gently return to preferred depth
                let depth_diff = p.preferred_depth - p.position.y;
                p.velocity.y += depth_diff * 0.015;

                // Avoid surface and bottom
                if (p.position.y > 0.85) { p.velocity.y -= 0.08; }
                if (p.position.y < -0.85) { p.velocity.y += 0.08; }
            }

            // Shark behavior
            if (p.particle_type == 1u) {
                // Make shark bigger
                p.scale = 4.0;

                // Shark turns away from walls gradually
                if (p.position.x > 0.7) { p.velocity.x -= 0.015; }
                if (p.position.x < -0.7) { p.velocity.x += 0.015; }
                if (p.position.y > 0.5) { p.velocity.y -= 0.01; }
                if (p.position.y < -0.5) { p.velocity.y += 0.01; }
                if (p.position.z > 0.7) { p.velocity.z -= 0.015; }
                if (p.position.z < -0.7) { p.velocity.z += 0.015; }

                // Shark prefers mid-depths
                p.velocity.y -= p.position.y * 0.01;
            }
        "#
            .into(),
        ))

        // === Physics ===
        // High drag for calmer motion
        .with_rule(Rule::Typed {
            self_type: Species::SmallFish.into(),
            other_type: None,
            rule: Box::new(Rule::Drag(2.5)),
        })

        // Shark is slower, heavier
        .with_rule(Rule::Typed {
            self_type: Species::Shark.into(),
            other_type: None,
            rule: Box::new(Rule::Drag(1.8)),
        })

        // Speed limits
        .with_rule(Rule::Typed {
            self_type: Species::SmallFish.into(),
            other_type: None,
            rule: Box::new(Rule::SpeedLimit { min: 0.02, max: 0.5 }),
        })
        .with_rule(Rule::Typed {
            self_type: Species::Shark.into(),
            other_type: None,
            rule: Box::new(Rule::SpeedLimit { min: 0.01, max: 0.35 }),
        })

        // Walls
        .with_rule(Rule::BounceWalls)
        .run();
}
