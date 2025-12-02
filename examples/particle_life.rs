//! # Particle Life
//!
//! Complex emergent behavior from simple attraction/repulsion rules between
//! different particle types. This creates self-organizing patterns, clustering,
//! and lifelike dynamics.
//!
//! ## What This Demonstrates
//!
//! - `with_interactions()` - define attraction/repulsion between types
//! - Emergent behavior from local rules
//! - Interaction matrix for complex ecosystems
//!
//! ## The Interaction Matrix
//!
//! Each pair of types (A, B) has:
//! - `strength`: positive = attract, negative = repel
//! - `radius`: how far the interaction reaches
//!
//! ```ignore
//! .with_interactions(|m| {
//!     m.attract(Red, Green, 1.0, 0.3);   // Red attracted to Green
//!     m.repel(Green, Red, 0.5, 0.2);     // Green repelled by Red
//!     m.set_symmetric(Blue, Blue, 0.7, 0.25); // Blues cluster
//! })
//! ```
//!
//! ## Emergent Patterns
//!
//! Different matrix configurations create different behaviors:
//! - **Clustering**: Same-type attraction
//! - **Chasing**: A attracts B, B repels A
//! - **Orbiting**: Attract at distance, repel when close
//! - **Rock-Paper-Scissors**: Cyclic chase relationships
//!
//! ## Try This
//!
//! - Randomize the interaction strengths
//! - Add more types (5, 6, 7...)
//! - Make all repulsions symmetric
//! - Create predator-prey chains
//!
//! Run with: `cargo run --example particle_life`

use rand::Rng;
use rdpe::prelude::*;

// Four species with different behaviors
#[derive(Clone, Copy, PartialEq)]
#[repr(u32)]
enum Species {
    Red = 0,
    Green = 1,
    Blue = 2,
    Yellow = 3,
}

impl From<Species> for u32 {
    fn from(s: Species) -> u32 {
        s as u32
    }
}

#[derive(Particle, Clone)]
struct Cell {
    position: Vec3,
    velocity: Vec3,
    particle_type: u32,
    #[color]
    color: Vec3,
}

fn main() {
    use Species::*;

    let mut rng = rand::thread_rng();
    let species = [Red, Green, Blue, Yellow];
    let colors = [
        Vec3::new(1.0, 0.3, 0.3), // Red
        Vec3::new(0.3, 1.0, 0.3), // Green
        Vec3::new(0.3, 0.3, 1.0), // Blue
        Vec3::new(1.0, 1.0, 0.3), // Yellow
    ];

    // Equal numbers of each type
    let particles: Vec<Cell> = (0..4_000)
        .map(|i| {
            let species_idx = (i % 4) as usize;
            Cell {
                position: Vec3::new(
                    rng.gen_range(-1.0..1.0),
                    rng.gen_range(-1.0..1.0),
                    rng.gen_range(-1.0..1.0),
                ),
                velocity: Vec3::ZERO,
                particle_type: species[species_idx].into(),
                color: colors[species_idx],
            }
        })
        .collect();

    Simulation::<Cell>::new()
        .with_particle_count(4_000)
        .with_bounds(1.5)
        .with_spawner(move |i, _| particles[i as usize].clone())
        // === The Interaction Matrix ===
        // This defines all type-to-type relationships
        .with_interactions(|m| {
            // Red dynamics
            m.attract(Red, Red, 0.5, 0.3); // Red clusters with itself
            m.attract(Red, Green, 1.0, 0.4); // Red chases Green
            m.repel(Red, Blue, 0.3, 0.2); // Red avoids Blue

            // Green dynamics
            m.repel(Green, Red, 0.8, 0.3); // Green runs from Red
            m.attract(Green, Green, 0.3, 0.25); // Loose clustering
            m.attract(Green, Blue, 0.6, 0.35); // Green follows Blue

            // Blue dynamics
            m.attract(Blue, Red, 0.4, 0.3); // Blue curious about Red
            m.repel(Blue, Green, 0.5, 0.25); // Blue avoids Green
            m.attract(Blue, Blue, 0.2, 0.2); // Weak self-attraction

            // Yellow dynamics - chaotic attractor
            m.attract(Yellow, Red, 0.3, 0.4); // Attracted to everyone
            m.attract(Yellow, Green, 0.3, 0.4);
            m.attract(Yellow, Blue, 0.3, 0.4);
            m.repel(Yellow, Yellow, 0.8, 0.3); // But repels itself!
        })
        // Physics
        .with_rule(Rule::Drag(2.0))
        .with_rule(Rule::SpeedLimit { min: 0.0, max: 1.5 })
        .with_rule(Rule::WrapWalls)
        .run();
}
