//! # Predator & Prey
//!
//! A predator-prey ecosystem demonstrating typed particle interactions.
//!
//! - **Prey (green)**: Flock together using boids rules, flee from predators
//! - **Predators (red)**: Chase the nearest prey
//!
//! ## What This Demonstrates
//!
//! - `#[derive(ParticleType)]` - type-safe particle categories
//! - `Rule::Typed` - apply rules only to specific type combinations
//! - `Rule::Chase` - pursue nearest target of a type
//! - `Rule::Evade` - flee from nearest threat of a type
//!
//! ## Particle Types
//!
//! The `ParticleType` derive creates an enum that converts to `u32`:
//!
//! ```ignore
//! #[derive(ParticleType)]
//! enum Species { Prey, Predator }
//!
//! // Usage
//! particle_type: Species::Prey.into()  // -> 0u32
//! ```
//!
//! ## Typed Rules
//!
//! `Rule::Typed` wraps another rule to filter by particle type:
//!
//! ```ignore
//! Rule::Typed {
//!     self_type: Species::Prey.into(),      // Rule applies to prey
//!     other_type: Some(Species::Prey.into()), // When near other prey
//!     rule: Box::new(Rule::Cohere { ... }),
//! }
//! ```
//!
//! ## Chase & Evade
//!
//! These rules find the *nearest* target/threat within radius:
//! - `Chase`: Accelerate toward nearest target
//! - `Evade`: Accelerate away from nearest threat
//!
//! ## Try This
//!
//! - Increase predator count for more chaos
//! - Add `Rule::Convert` to let predators "eat" prey
//! - Reduce prey evade radius to make them easier to catch
//!
//! Run with: `cargo run --example predator_prey`

use rand::Rng;
use rdpe::prelude::*;

#[derive(ParticleType, Clone, Copy, PartialEq)]
enum Species {
    Prey,     // 0
    Predator, // 1
}

#[derive(Particle, Clone)]
struct Creature {
    position: Vec3,
    velocity: Vec3,
    #[color]
    color: Vec3,
    particle_type: u32,
}

fn main() {
    let mut rng = rand::thread_rng();

    let num_prey = 4000;
    let num_predators = 50;
    let total = num_prey + num_predators;

    let particles: Vec<Creature> = (0..total)
        .map(|i| {
            let is_predator = i >= num_prey;
            let species = if is_predator {
                Species::Predator
            } else {
                Species::Prey
            };

            // Predators start in center, prey spread out
            let spread = if is_predator { 0.3 } else { 0.8 };
            let pos = Vec3::new(
                rng.gen_range(-spread..spread),
                rng.gen_range(-spread..spread),
                rng.gen_range(-spread..spread),
            );

            let vel = Vec3::new(
                rng.gen_range(-0.1..0.1),
                rng.gen_range(-0.1..0.1),
                rng.gen_range(-0.1..0.1),
            );

            let color = if is_predator {
                Vec3::new(1.0, 0.2, 0.1) // Red
            } else {
                Vec3::new(0.2, 0.9, 0.3) // Green
            };

            Creature {
                position: pos,
                velocity: vel,
                color,
                particle_type: species.into(),
            }
        })
        .collect();

    Simulation::<Creature>::new()
        .with_particle_count(total as u32)
        .with_bounds(1.0)
        .with_spatial_config(0.3, 32)
        .with_spawner(move |i, _| particles[i as usize].clone())
        // === Prey Behavior ===
        // Prey flock with other prey (classic boids)
        .with_rule(Rule::Typed {
            self_type: Species::Prey.into(),
            other_type: Some(Species::Prey.into()),
            rule: Box::new(Rule::Separate {
                radius: 0.04,
                strength: 3.0,
            }),
        })
        .with_rule(Rule::Typed {
            self_type: Species::Prey.into(),
            other_type: Some(Species::Prey.into()),
            rule: Box::new(Rule::Cohere {
                radius: 0.15,
                strength: 1.0,
            }),
        })
        .with_rule(Rule::Typed {
            self_type: Species::Prey.into(),
            other_type: Some(Species::Prey.into()),
            rule: Box::new(Rule::Align {
                radius: 0.1,
                strength: 2.0,
            }),
        })
        // Prey evades nearest predator
        .with_rule(Rule::Evade {
            self_type: Species::Prey.into(),
            threat_type: Species::Predator.into(),
            radius: 0.25,
            strength: 6.0,
        })
        // === Predator Behavior ===
        // Predators chase nearest prey
        .with_rule(Rule::Chase {
            self_type: Species::Predator.into(),
            target_type: Species::Prey.into(),
            radius: 0.4,
            strength: 4.0,
        })
        // Predators avoid each other slightly
        .with_rule(Rule::Typed {
            self_type: Species::Predator.into(),
            other_type: Some(Species::Predator.into()),
            rule: Box::new(Rule::Separate {
                radius: 0.1,
                strength: 1.0,
            }),
        })
        // === Physics for All ===
        .with_rule(Rule::SpeedLimit { min: 0.0, max: 2.0 })
        .with_rule(Rule::Drag(1.5))
        .with_rule(Rule::BounceWalls)
        .run();
}
