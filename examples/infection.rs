//! # Infection Spread (SIR Model)
//!
//! A disease spread simulation using the classic SIR epidemiological model:
//!
//! - **Susceptible/Healthy (green)**: Can become infected
//! - **Infected (red)**: Spreads disease, eventually recovers
//! - **Recovered (blue)**: Immune to further infection
//!
//! ## What This Demonstrates
//!
//! - `Rule::Convert` - particles change type based on neighbors
//! - `Rule::Custom` - update color based on current type
//! - `Rule::Wander` - random movement for mixing
//! - Emergent epidemic dynamics from simple rules
//!
//! ## The Convert Rule
//!
//! `Rule::Convert` changes a particle's type when near a trigger type:
//!
//! ```ignore
//! Rule::Convert {
//!     from_type: Health::Healthy.into(),    // Only affects healthy
//!     trigger_type: Health::Infected.into(), // When near infected
//!     to_type: Health::Infected.into(),      // Become infected
//!     radius: 0.08,                          // Infection range
//!     probability: 0.15,                     // 15% chance per frame
//! }
//! ```
//!
//! ## Recovery Mechanic
//!
//! Recovery uses a trick: infected particles "convert" themselves
//! with a tiny radius (essentially self-triggered) and low probability:
//!
//! ```ignore
//! Rule::Convert {
//!     from_type: Health::Infected.into(),
//!     trigger_type: Health::Infected.into(), // Self-trigger
//!     to_type: Health::Recovered.into(),
//!     radius: 0.01,        // Very small
//!     probability: 0.002,  // ~0.2% per frame
//! }
//! ```
//!
//! ## Try This
//!
//! - Increase `initial_infected` for faster spread
//! - Reduce separation strength to see faster epidemics
//! - Add more recovery probability for quicker immunity
//!
//! Run with: `cargo run --example infection`

use rand::Rng;
use rdpe::prelude::*;

#[derive(ParticleType, Clone, Copy, PartialEq)]
enum Health {
    Healthy,   // 0 - Susceptible
    Infected,  // 1 - Contagious
    Recovered, // 2 - Immune
}

#[derive(Particle, Clone)]
struct Person {
    position: Vec3,
    velocity: Vec3,
    #[color]
    color: Vec3,
    particle_type: u32,
}

fn main() {
    let mut rng = rand::thread_rng();

    let total = 3000;
    let initial_infected = 5;

    let particles: Vec<Person> = (0..total)
        .map(|i| {
            let is_infected = i < initial_infected;
            let health = if is_infected {
                Health::Infected
            } else {
                Health::Healthy
            };

            let pos = Vec3::new(
                rng.gen_range(-0.9..0.9),
                rng.gen_range(-0.9..0.9),
                rng.gen_range(-0.9..0.9),
            );

            let vel = Vec3::new(
                rng.gen_range(-0.3..0.3),
                rng.gen_range(-0.3..0.3),
                rng.gen_range(-0.3..0.3),
            );

            let color = if is_infected {
                Vec3::new(1.0, 0.1, 0.1) // Red
            } else {
                Vec3::new(0.1, 0.9, 0.2) // Green
            };

            Person {
                position: pos,
                velocity: vel,
                color,
                particle_type: health.into(),
            }
        })
        .collect();

    Simulation::<Person>::new()
        .with_particle_count(total as u32)
        .with_bounds(1.0)
        .with_spatial_config(0.1, 32)
        .with_spawner(move |i, _| particles[i as usize].clone())
        // === Disease Mechanics ===
        // Healthy -> Infected (when near infected)
        .with_rule(Rule::Convert {
            from_type: Health::Healthy.into(),
            trigger_type: Health::Infected.into(),
            to_type: Health::Infected.into(),
            radius: 0.08,
            probability: 0.15,
        })
        // Infected -> Recovered (self-triggered over time)
        .with_rule(Rule::Convert {
            from_type: Health::Infected.into(),
            trigger_type: Health::Infected.into(),
            to_type: Health::Recovered.into(),
            radius: 0.01, // Essentially self
            probability: 0.002,
        })
        // === Social Dynamics ===
        // Everyone maintains some distance
        .with_rule(Rule::Separate {
            radius: 0.06,
            strength: 1.0,
        })
        // Update color to match current health status
        .with_rule(Rule::Custom(
            r#"
    if p.particle_type == 0u {
        p.color = vec3<f32>(0.1, 0.9, 0.2); // Healthy: green
    } else if p.particle_type == 1u {
        p.color = vec3<f32>(1.0, 0.1, 0.1); // Infected: red
    } else {
        p.color = vec3<f32>(0.2, 0.4, 1.0); // Recovered: blue
    }
"#
            .to_string(),
        ))
        // Random wandering keeps population mixing
        .with_rule(Rule::Wander {
            strength: 0.5,
            frequency: 100.0,
        })
        // === Physics ===
        .with_rule(Rule::SpeedLimit { min: 0.0, max: 1.0 })
        .with_rule(Rule::Drag(1.0))
        .with_rule(Rule::BounceWalls)
        .run();
}
