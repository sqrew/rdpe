//! Particle trails demonstration.
//!
//! Shows particles leaving trails behind them as they move.

use rand::Rng;
use rdpe::prelude::*;

#[derive(rdpe::Particle, Clone)]
struct Comet {
    position: Vec3,
    velocity: Vec3,
    #[color]
    color: Vec3,
}

fn main() {
    let mut rng = rand::thread_rng();
    let count = 5_000;

    // Pre-generate comets
    let particles: Vec<Comet> = (0..count)
        .map(|_| {
            // Spawn randomly
            let x = rng.gen_range(-0.8..0.8);
            let y = rng.gen_range(-0.8..0.8);
            let z = rng.gen_range(-0.8..0.8);

            // Random initial velocity
            let vx = rng.gen_range(-0.3..0.3);
            let vy = rng.gen_range(-0.3..0.3);
            let vz = rng.gen_range(-0.3..0.3);

            // Random color
            let r = rng.gen_range(0.5..1.0);
            let g = rng.gen_range(0.5..1.0);
            let b = rng.gen_range(0.5..1.0);

            Comet {
                position: Vec3::new(x, y, z),
                velocity: Vec3::new(vx, vy, vz),
                color: Vec3::new(r, g, b),
            }
        })
        .collect();

    Simulation::<Comet>::new()
        .with_particle_count(count)
        .with_bounds(1.0)
        .with_particle_size(0.02)
        .with_spawner(move |i, _| particles[i as usize].clone())
        // Enable trails - 20 frames of history
        .with_visuals(|v| {
            v.trails(20);
            v.blend_mode(BlendMode::Additive); // Glowy trails!
        })
        // Central attraction
        .with_rule(Rule::AttractTo {
            point: Vec3::ZERO,
            strength: 0.5,
        })
        // Some drag
        .with_rule(Rule::Drag(0.3))
        // Bounce off walls
        .with_rule(Rule::BounceWalls)
        .run();
}
