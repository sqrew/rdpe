//! # Simple Benchmark (No Neighbors)
//!
//! Tests raw particle throughput without spatial hashing overhead.
//! Useful for measuring baseline GPU compute performance.
//!
//! Run with: `cargo run --example bench_simple --release`

use rand::Rng;
use rdpe::prelude::*;

#[derive(Particle, Clone)]
struct Dot {
    position: Vec3,
    velocity: Vec3,
    #[color]
    color: Vec3,
}

fn main() {
    let count: u32 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(500_000);

    println!("=== RDPE Simple Benchmark ===");
    println!("Particles: {}", count);
    println!("Rules: Gravity, Turbulence, Drag, ColorBySpeed, BounceWalls");
    println!("Spatial hashing: OFF");
    println!();
    println!("Watch the window title for FPS...");

    let mut rng = rand::thread_rng();

    let particles: Vec<Dot> = (0..count)
        .map(|_| Dot {
            position: Vec3::new(
                rng.gen_range(-0.9..0.9),
                rng.gen_range(-0.9..0.9),
                rng.gen_range(-0.9..0.9),
            ),
            velocity: Vec3::new(
                rng.gen_range(-0.5..0.5),
                rng.gen_range(-0.5..0.5),
                rng.gen_range(-0.5..0.5),
            ),
            color: Vec3::new(0.5, 0.5, 1.0),
        })
        .collect();

    Simulation::<Dot>::new()
        .with_particle_count(count)
        .with_particle_size(0.004)
        .with_bounds(1.0)
        .with_spawner(move |i, _| particles[i as usize].clone())
        // Stack several non-neighbor rules
        .with_rule(Rule::Gravity(2.0))
        .with_rule(Rule::Turbulence {
            scale: 3.0,
            strength: 2.0,
        })
        .with_rule(Rule::Vortex {
            center: Vec3::ZERO,
            axis: Vec3::Y,
            strength: 1.0,
        })
        .with_rule(Rule::Drag(0.5))
        .with_rule(Rule::ColorBySpeed {
            slow_color: Vec3::new(0.2, 0.2, 0.5),
            fast_color: Vec3::new(1.0, 0.5, 0.2),
            max_speed: 2.0,
        })
        .with_rule(Rule::SpeedLimit { min: 0.0, max: 3.0 })
        .with_rule(Rule::BounceWalls)
        .run();
}
