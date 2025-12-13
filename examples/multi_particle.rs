//! # Multi-Particle Types (Inline Syntax)
//!
//! Demonstrates using `#[derive(MultiParticle)]` with inline struct definitions
//! to create both standalone particle types AND a unified simulation.
//!
//! - **Boids (rainbow)**: Flock together, have a `flock_id` field
//! - **Predators (also rainbow)**: Hunt boids, have `hunger` and `target_id` fields
//!
//! ## What This Demonstrates
//!
//! - `#[derive(MultiParticle)]` with struct-like enum variants
//! - Auto-generated standalone structs (`Boid`, `Predator`) for single-type sims
//! - Unified enum implements `ParticleTrait` for mixed simulations
//! - Generated Rust constants (`Creature::BOID`, `Creature::PREDATOR`) for typed rules
//! - Generated WGSL constants (`BOID`, `PREDATOR`) and helpers (`is_boid()`) for shaders
//!
//! ## The Power of Inline Definitions
//!
//! From a single enum definition, you get:
//! - `Simulation::<Creature>::new()` - Mixed boid/predator simulation
//! - `Simulation::<Boid>::new()` - Boid-only simulation (uses generated struct)
//! - `Simulation::<Predator>::new()` - Predator-only simulation (uses generated struct)
//!
//! ## Clean Syntax
//!
//! Create particles directly with struct-like enum syntax:
//! ```ignore
//! Creature::Boid { position: pos, velocity: vel, flock_id: 0 }
//! Creature::Predator { position: pos, velocity: vel, hunger: 1.0, target_id: 0 }
//! ```
//!
//! Run with: `cargo run --example multi_particle`

use rand::Rng;
use rdpe::prelude::*;

/// Define multiple particle types in one place!
/// The macro generates:
/// - `struct Boid { ... }` with full `ParticleTrait` implementation (for standalone use)
/// - `struct Predator { ... }` with full `ParticleTrait` implementation (for standalone use)
/// - Unified `ParticleTrait` implementation on the enum itself for mixed simulations
#[derive(MultiParticle, Clone)]
enum Creature {
    Boid {
        position: Vec3,
        velocity: Vec3,
        flock_id: u32,
    },
    Predator {
        position: Vec3,
        velocity: Vec3,
        hunger: f32,
        target_id: u32,
    },
}

fn main() {
    let mut rng = rand::thread_rng();

    let num_boids = 3000;
    let num_predators = 30;
    let total = num_boids + num_predators;

    // Pre-generate particles using struct-like enum syntax
    let particles: Vec<Creature> = (0..total)
        .map(|i| {
            let is_predator = i >= num_boids;

            let spread = if is_predator { 0.4 } else { 0.9 };
            let pos = Vec3::new(
                rng.gen_range(-spread..spread),
                rng.gen_range(-spread..spread),
                rng.gen_range(-spread..spread),
            );

            let vel = Vec3::new(
                rng.gen_range(-0.05..0.05),
                rng.gen_range(-0.05..0.05),
                rng.gen_range(-0.05..0.05),
            );

            if is_predator {
                // Direct struct-like variant syntax
                Creature::Predator {
                    position: pos,
                    velocity: vel,
                    hunger: 1.0,
                    target_id: 0,
                }
            } else {
                // Direct struct-like variant syntax
                Creature::Boid {
                    position: pos,
                    velocity: vel,
                    flock_id: rng.gen_range(0..3),
                }
            }
        })
        .collect();

    Simulation::<Creature>::new()
        .with_particle_count(total as u32)
        .with_bounds(1.0)
        .with_spatial_config(0.25, 32)
        .with_spawner(move |ctx| particles[ctx.index as usize].clone())
        // Boid flocking behavior (using generated type constants)
        .with_rule(Rule::Typed {
            self_type: Creature::BOID,
            other_type: Some(Creature::BOID),
            rule: Box::new(Rule::Separate {
                radius: 0.05,
                strength: 2.5,
            }),
        })
        .with_rule(Rule::Typed {
            self_type: Creature::BOID,
            other_type: Some(Creature::BOID),
            rule: Box::new(Rule::Cohere {
                radius: 0.15,
                strength: 1.2,
            }),
        })
        .with_rule(Rule::Typed {
            self_type: Creature::BOID,
            other_type: Some(Creature::BOID),
            rule: Box::new(Rule::Align {
                radius: 0.12,
                strength: 1.8,
            }),
        })
        // Boids evade predators
        .with_rule(Rule::Evade {
            self_type: Creature::BOID,
            threat_type: Creature::PREDATOR,
            radius: 0.3,
            strength: 5.0,
        })
        // Predators chase boids
        .with_rule(Rule::Chase {
            self_type: Creature::PREDATOR,
            target_type: Creature::BOID,
            radius: 0.5,
            strength: 3.5,
        })
        // Predators avoid each other
        .with_rule(Rule::Typed {
            self_type: Creature::PREDATOR,
            other_type: Some(Creature::PREDATOR),
            rule: Box::new(Rule::Separate {
                radius: 0.15,
                strength: 1.5,
            }),
        })
        // Custom rule using variant-specific fields and generated helpers
        .with_rule(Rule::Custom(
            r#"
            // Use the generated is_predator helper
            if is_predator(p) {
                // Predators get hungrier over time
                p.hunger = max(0.0, p.hunger - uniforms.delta_time * 0.02);

                // Hungry predators move faster
                let speed_boost = 1.0 + (1.0 - p.hunger) * 0.5;
                p.velocity *= speed_boost;
            }

            // Could also access boid's flock_id if needed:
            // if is_boid(p) { let flock = p.flock_id; }
            "#
            .into(),
        ))
        // Physics
        .with_rule(Rule::SpeedLimit { min: 0.0, max: 2.5 })
        .with_rule(Rule::Drag(2.0))
        .with_rule(Rule::BounceWalls)
        // Visuals - use Rainbow palette mapped by index
        // Boids (first 90%) get blue-ish, predators (last 10%) get red-ish
        .with_visuals(|v| {
            v.background(Vec3::new(0.02, 0.02, 0.04));
            v.blend_mode(BlendMode::Additive);
            v.palette(Palette::Rainbow, ColorMapping::Index);
        })
        .run().expect("Simulation failed");
}
