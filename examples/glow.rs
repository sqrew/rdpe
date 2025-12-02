//! Glowy particles with additive blending.
//!
//! Demonstrates the visual system with BlendMode::Additive.
//! Overlapping particles add up to create bright, glowing effects.

use rand::Rng;
use rdpe::prelude::*;

#[derive(rdpe::Particle, Clone)]
struct Spark {
    position: Vec3,
    velocity: Vec3,
    #[color]
    color: Vec3,
}

fn main() {
    let mut rng = rand::thread_rng();
    let count = 50_000;

    // Pre-generate particles
    let particles: Vec<Spark> = (0..count)
        .map(|_| {
            // Spawn in a sphere around origin
            let theta = rng.gen_range(0.0..std::f32::consts::TAU);
            let phi = rng.gen_range(0.0..std::f32::consts::PI);
            let r = rng.gen_range(0.3..0.8);

            let x = r * phi.sin() * theta.cos();
            let y = r * phi.sin() * theta.sin();
            let z = r * phi.cos();

            // Give initial tangent velocity for orbiting
            let speed = rng.gen_range(0.3..0.6);
            let vel = Vec3::new(-y, rng.gen_range(-0.1..0.1), x).normalize() * speed;

            // Color based on spawn position (creates nice gradients)
            let hue = theta / std::f32::consts::TAU;
            let color = hsv_to_rgb(hue, 0.8, 1.0);

            Spark {
                position: Vec3::new(x, y, z),
                velocity: vel,
                color,
            }
        })
        .collect();

    Simulation::<Spark>::new()
        .with_particle_count(count)
        .with_bounds(2.0)
        .with_particle_size(0.008) // Smaller particles work well with additive
        .with_spawner(move |i, _| particles[i as usize].clone())
        // Additive blending - overlapping particles glow brighter!
        .with_visuals(|v| {
            v.blend_mode(BlendMode::Additive);
        })
        // Pull toward center
        .with_rule(Rule::AttractTo {
            point: Vec3::ZERO,
            strength: 0.5,
        })
        // Some swirl
        .with_rule(Rule::Custom(
            r#"
            let r = length(p.position.xz);
            let swirl_strength = 0.3 / (r + 0.1);
            p.velocity += vec3<f32>(-p.position.z, 0.0, p.position.x) * swirl_strength * uniforms.delta_time;
            "#
            .into(),
        ))
        // Light drag to keep things stable
        .with_rule(Rule::Drag(0.3))
        // Speed limit
        .with_rule(Rule::SpeedLimit { min: 0.0, max: 1.5 })
        // Bounce off walls
        .with_rule(Rule::BounceWalls)
        .run();
}

// Simple HSV to RGB conversion
fn hsv_to_rgb(h: f32, s: f32, v: f32) -> Vec3 {
    let c = v * s;
    let x = c * (1.0 - ((h * 6.0) % 2.0 - 1.0).abs());
    let m = v - c;

    let (r, g, b) = match (h * 6.0) as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };

    Vec3::new(r + m, g + m, b + m)
}
