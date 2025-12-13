//! # Crystal Growth (Diffusion-Limited Aggregation)
//!
//! Particles randomly diffuse until they touch the crystal, then stick.
//! Creates beautiful dendritic fractal structures.
//!
//! ## What This Demonstrates
//!
//! - Brownian motion (random velocity jitter)
//! - OnCollision rule for state changes
//! - Particles freezing in place when crystallized
//! - Color based on crystallization time
//!
//! ## Physics
//!
//! Each frame:
//! 1. Free particles receive random velocity nudges (Brownian motion)
//! 2. When a free particle collides with a crystal particle, it crystallizes
//! 3. Crystallized particles stop moving and record their freeze time
//! 4. Color shifts from blue (early) to pink (late) based on when crystallized
//!
//! Run with: `cargo run --example crystal_growth`

use rand::Rng;
use rdpe::prelude::*;

#[derive(Particle, Clone)]
struct CrystalParticle {
    position: Vec3,
    velocity: Vec3,
    #[color]
    color: Vec3,
    /// 0 = free (diffusing), 1 = crystallized (frozen)
    is_crystal: u32,
    /// Time when this particle crystallized (for coloring)
    crystallize_time: f32,
}

fn main() {
    let mut rng = rand::thread_rng();

    let num_particles = 15000;
    let num_seeds = 5; // Multiple seed crystals

    // Pre-generate seed positions
    let seed_positions: Vec<Vec3> = (0..num_seeds)
        .map(|i| {
            let angle = (i as f32) * std::f32::consts::TAU / (num_seeds as f32);
            let radius = 0.3;
            Vec3::new(angle.cos() * radius, angle.sin() * radius, 0.0)
        })
        .collect();

    // Pre-generate particles
    let particles: Vec<_> = (0..num_particles)
        .map(|i| {
            let is_seed = i < num_seeds;

            let pos = if is_seed {
                // Seed crystals scattered in a ring
                seed_positions[i]
            } else {
                // Free particles uniformly distributed in a sphere
                let r = rng.gen_range(0.05_f32..0.9).cbrt(); // cbrt for uniform volume distribution
                let theta = rng.gen_range(0.0..std::f32::consts::TAU);
                let cos_phi = rng.gen_range(-1.0_f32..1.0);
                let sin_phi = (1.0 - cos_phi * cos_phi).sqrt();
                Vec3::new(
                    r * sin_phi * theta.cos(),
                    r * sin_phi * theta.sin(),
                    r * cos_phi,
                )
            };

            (pos, is_seed)
        })
        .collect();

    Simulation::<CrystalParticle>::new()
        .with_particle_count(num_particles as u32)
        .with_bounds(1.0)
        .with_spawner(move |ctx| {
            let (pos, is_seed) = particles[ctx.index as usize];
            CrystalParticle {
                position: pos,
                velocity: Vec3::ZERO,
                color: if is_seed {
                    Vec3::new(0.3, 0.5, 1.0) // Seed is blue
                } else {
                    Vec3::new(0.5, 0.5, 0.5) // Free particles are gray
                },
                is_crystal: if is_seed { 1 } else { 0 },
                crystallize_time: 0.0,
            }
        })
        // Spatial hashing for collision detection
        .with_spatial_config(0.08, 32)
        // Brownian motion for free particles + drift toward center
        .with_rule(Rule::Custom(
            r#"
            if p.is_crystal == 0u {
                // Pure random walk - classic DLA behavior
                let frame = u32(uniforms.time * 60.0);
                let seed = index + frame * 65537u;
                let random_dir = rand_sphere(seed);

                // Pure Brownian motion - particles wander until they hit a crystal
                p.velocity = random_dir * 0.5;

                // Soft boundary - push back if too far from origin
                let dist = length(p.position);
                if dist > 0.85 {
                    p.velocity -= p.position * 0.3;
                }
            } else {
                // Crystallized - frozen in place
                p.velocity = vec3<f32>(0.0, 0.0, 0.0);
            }
            "#
            .into(),
        ))
        // When free particle touches crystal, it crystallizes
        .with_rule(Rule::OnCollision {
            radius: 0.025,
            response: r#"
                // Only crystallize if we're free and hit a crystal
                if p.is_crystal == 0u && other.is_crystal == 1u {
                    p.is_crystal = 1u;
                    p.crystallize_time = uniforms.time;
                    p.velocity = vec3<f32>(0.0, 0.0, 0.0);
                }
            "#
            .into(),
        })
        // Color based on state and crystallization time
        .with_rule(Rule::Custom(
            r#"
            if p.is_crystal == 1u {
                // Crystallized: color by time (blue -> cyan -> white -> pink)
                let t = fract(p.crystallize_time * 0.1);
                if t < 0.33 {
                    let blend = t * 3.0;
                    p.color = mix(vec3<f32>(0.2, 0.4, 1.0), vec3<f32>(0.2, 0.9, 1.0), blend);
                } else if t < 0.66 {
                    let blend = (t - 0.33) * 3.0;
                    p.color = mix(vec3<f32>(0.2, 0.9, 1.0), vec3<f32>(1.0, 1.0, 1.0), blend);
                } else {
                    let blend = (t - 0.66) * 3.0;
                    p.color = mix(vec3<f32>(1.0, 1.0, 1.0), vec3<f32>(1.0, 0.5, 0.8), blend);
                }
            } else {
                // Free particle - dim red-gray
                p.color = vec3<f32>(0.3, 0.9, 0.3);
            }
            "#
            .into(),
        ))
        // No drag/speed limit since we set velocity directly each frame
        // Visuals
        .with_visuals(|v| {
            v.background(Vec3::new(0.0, 0.0, 0.0));
            v.connections(0.05);
            v.shape(ParticleShape::Star);
        })
        .with_vertex_effect(VertexEffect::Pulse {
            frequency: 3.0,
            amplitude: 0.3,
        })
        .with_rule_inspector()
        .with_particle_inspector()
        .run().expect("Simulation failed");
}
