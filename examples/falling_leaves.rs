//! # Falling Leaves
//!
//! Demonstrates `Rule::Wind` and `Rule::RespawnBelow` for endless falling leaves.
//! Leaves drift in turbulent wind and respawn at the top when they hit the ground.
//!
//! Run with: `cargo run --example falling_leaves`

use rand::Rng;
use rdpe::prelude::*;

#[derive(Particle, Clone)]
struct Leaf {
    position: Vec3,
    velocity: Vec3,
    #[color]
    color: Vec3,
    phase: f32,  // For individual flutter timing
}

fn main() {
    let mut rng = rand::thread_rng();

    // Pre-generate random data
    let leaves: Vec<_> = (0..3000)
        .map(|_| {
            let x = rng.gen_range(-1.0..1.0);
            let y = rng.gen_range(-1.0..1.0);
            let z = rng.gen_range(-1.0..1.0);
            let phase = rng.gen_range(0.0..std::f32::consts::TAU);

            // Autumn colors: reds, oranges, yellows, browns
            let hue = rng.gen_range(0.0..0.15);  // Red to yellow range
            let sat = rng.gen_range(0.6..1.0);
            let val = rng.gen_range(0.5..0.9);

            // HSV to RGB (simplified)
            let h = hue * 6.0;
            let c = val * sat;
            let x_c = c * (1.0 - (h % 2.0 - 1.0_f32).abs());
            let (r, g, b) = if h < 1.0 {
                (c, x_c, 0.0)
            } else if h < 2.0 {
                (x_c, c, 0.0)
            } else {
                (c, x_c, 0.0)
            };
            let m = val - c;

            (x, y, z, phase, Vec3::new(r + m, g + m, b + m))
        })
        .collect();

    Simulation::<Leaf>::new()
        .with_particle_count(3000)
        .with_bounds(1.5)
        .with_spawner(move |i, _| {
            let (x, y, z, phase, color) = leaves[i as usize];

            Leaf {
                position: Vec3::new(x, y, z),
                velocity: Vec3::ZERO,
                color,
                phase,
            }
        })
        // Gentle gravity
        .with_rule(Rule::Gravity(1.5))
        // Wind with high turbulence (gusty autumn wind)
        .with_rule(Rule::Wind {
            direction: Vec3::new(0.7, 0.0, 0.3),
            strength: 1.0,
            turbulence: 0.8,
        })
        // Individual leaf flutter
        .with_rule(Rule::Custom(r#"
            // Each leaf flutters based on its phase
            let flutter = sin(uniforms.time * 5.0 + p.phase) * 0.5;
            p.velocity.x += flutter * uniforms.delta_time;
            p.velocity.z += cos(uniforms.time * 4.0 + p.phase * 1.3) * 0.3 * uniforms.delta_time;
        "#.into()))
        // Respawn at top when hitting ground
        .with_rule(Rule::RespawnBelow {
            threshold_y: -1.2,
            spawn_y: 1.2,
            reset_velocity: true,
        })
        // Air resistance
        .with_rule(Rule::Drag(1.0))
        .with_visuals(|v| {
            v.background(Vec3::new(0.4, 0.5, 0.6));  // Overcast sky
        })
        .with_particle_size(0.02)
        .run();
}
