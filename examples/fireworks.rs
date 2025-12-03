//! Fireworks example demonstrating sub-emitters.
//!
//! Rockets fly up and explode into sparks when they die.

use rdpe::prelude::*;

#[derive(Particle, Clone)]
struct Firework {
    position: Vec3,
    velocity: Vec3,
    #[color]
    color: Vec3,
    particle_type: u32,
}

// Particle types
const ROCKET: u32 = 0;
const SPARK: u32 = 1;

fn main() {
    Simulation::<Firework>::new()
        .with_particle_count(5000)
        .with_bounds(2.0)
        // Start all particles dead - emitter will spawn rockets
        .with_spawner(|_i, _count| Firework {
            position: Vec3::ZERO,
            velocity: Vec3::ZERO,
            color: Vec3::new(1.0, 0.8, 0.2), // Golden
            particle_type: ROCKET,
        })
        // Lifecycle: all start dead, rockets live 1.5-2.5 seconds
        .with_lifecycle(|l| l
            .start_dead()
            .lifetime_range(1.5..2.5)
            .fade_out()
        )
        // Emitter: spawn rockets from bottom going up
        .with_emitter(Emitter::Cone {
            position: Vec3::new(0.0, -1.8, 0.0),
            direction: Vec3::new(0.0, 1.0, 0.0),
            speed: 2.5,
            spread: 0.3,
            rate: 5.0,
        })
        // Sub-emitter: rockets spawn sparks when they die
        .with_sub_emitter(
            SubEmitter::new(ROCKET, SPARK)
                .count(30)  // 30 sparks per rocket
                .speed(0.5..2.0)  // Random speed
                .spread(std::f32::consts::TAU)  // Full sphere (explosion)
                .inherit_velocity(0.2)  // Inherit 20% of rocket velocity
                .child_lifetime(1.0)  // Sparks live 1 second
                .child_color(Vec3::new(1.0, 0.0, 0.0))  // RED sparks to distinguish
        )
        // Physics
        .with_rule(Rule::Gravity(2.0))  // Pull down
        .with_rule(Rule::Drag(0.5))  // Air resistance
        .with_rule(Rule::BounceWalls)
        // Visual enhancements
        .with_visuals(|v| {
            v.blend_mode(BlendMode::Additive);
            v.background(Vec3::new(0.02, 0.02, 0.05));
        })
        .with_particle_size(0.02)
        .run();
}
