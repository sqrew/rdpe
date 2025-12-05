//! # Particle Shapes
//!
//! Demonstrates all available particle shapes in RDPE.
//!
//! ## What This Demonstrates
//!
//! - Using `ParticleShape` to change particle appearance
//! - Different shapes: Circle, CircleHard, Square, Ring, Star, Triangle, Hexagon, Diamond, Point
//! - Using particle types to show different shapes simultaneously
//!
//! ## Controls
//!
//! - **Left-click + drag**: Rotate camera
//! - **Scroll wheel**: Zoom in/out
//! - **1-9 keys**: Switch between shapes
//!
//! Run with: `cargo run --example shapes`

use rand::Rng;
use rdpe::prelude::*;

#[derive(Particle, Clone)]
struct ShapeParticle {
    position: Vec3,
    velocity: Vec3,
    #[color]
    color: Vec3,
}

fn main() {
    let mut rng = rand::thread_rng();

    // Generate particles in a swirling pattern
    let particles: Vec<ShapeParticle> = (0..5000)
        .map(|i| {
            let t = i as f32 / 5000.0;
            let angle = t * 20.0 * std::f32::consts::PI;
            let radius = t * 0.8 + 0.1;

            // Rainbow colors based on position
            let hue = t * 360.0;
            let color = hsv_to_rgb(hue, 0.8, 1.0);

            ShapeParticle {
                position: Vec3::new(
                    angle.cos() * radius + rng.gen_range(-0.02..0.02),
                    rng.gen_range(-0.3..0.3),
                    angle.sin() * radius + rng.gen_range(-0.02..0.02),
                ),
                velocity: Vec3::new(-angle.sin() * 0.3, 0.0, angle.cos() * 0.3),
                color,
            }
        })
        .collect();

    Simulation::<ShapeParticle>::new()
        .with_particle_count(5000)
        .with_particle_size(0.015)
        .with_spawner(move |i, _| particles[i as usize].clone())
        .with_visuals(|v| {
            // Try different shapes by changing this line:
            // v.shape(ParticleShape::Circle); // Soft circle (default)
            // v.shape(ParticleShape::CircleHard); // Hard-edged circle
            // v.shape(ParticleShape::Square); // Square
            // v.shape(ParticleShape::Ring); // Ring/donut
            // v.shape(ParticleShape::Star); // 5-pointed star
            v.shape(ParticleShape::Triangle); // Triangle
            // v.shape(ParticleShape::Hexagon); // Hexagon
            // v.shape(ParticleShape::Diamond); // Diamond
            // v.shape(ParticleShape::Point);        // Single pixel

            v.blend_mode(BlendMode::Additive);
        })
        .with_rule(Rule::Drag(0.5))
        .with_rule(Rule::PointGravity {
            point: Vec3::ZERO,
            strength: 0.5,
            softening: 0.1,
        })
        .run();
}

// Simple HSV to RGB conversion
fn hsv_to_rgb(h: f32, s: f32, v: f32) -> Vec3 {
    let h = h % 360.0;
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;

    let (r, g, b) = match (h / 60.0) as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };

    Vec3::new(r + m, g + m, b + m)
}
