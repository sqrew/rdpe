//! # Vertex Effects Example
//!
//! Demonstrates composable vertex effects for particle rendering.
//!
//! ## What This Demonstrates
//!
//! - `VertexEffect::Rotate` - Spinning particles
//! - `VertexEffect::Wobble` - Floating/swaying motion
//! - `VertexEffect::Pulse` - Size oscillation
//! - `VertexEffect::Wave` - Coordinated wave pattern
//! - Combining multiple effects
//!
//! ## Available Effects
//!
//! - `Rotate { speed }` - Spin around facing axis
//! - `Wobble { frequency, amplitude }` - Position oscillation
//! - `Pulse { frequency, amplitude }` - Size oscillation
//! - `Wave { direction, frequency, speed, amplitude }` - Coordinated wave
//! - `Jitter { amplitude }` - Random shake
//! - `ScaleByDistance { center, min_scale, max_scale, max_distance }`
//! - `FadeByDistance { near, far }`
//!
//! Run with: `cargo run --example vertex_effects`

use rdpe::prelude::*;

#[derive(rdpe::Particle, Clone)]
struct Spark {
    position: Vec3,
    velocity: Vec3,
    #[color]
    color: Vec3,
}

fn main() {
    Simulation::<Spark>::new()
        .with_particle_count(5_000)
        .with_bounds(1.5)
        .with_particle_size(0.02)
        .with_spawner(|ctx| {
            let pos = ctx.random_in_sphere(0.7);
            let speed = ctx.random_range(0.1, 0.3);
            Spark {
                position: pos,
                velocity: ctx.tangent_velocity(pos, speed),
                color: ctx.rainbow(0.8, 1.0),
            }
        })
        .with_visuals(|v| {
            v.blend_mode(BlendMode::Additive);
            v.shape(ParticleShape::Square); // Rotation visible with squares
        })
        // Stack multiple vertex effects!
        .with_vertex_effect(VertexEffect::Rotate { speed: 3.0 })
        .with_vertex_effect(VertexEffect::Wobble {
            frequency: 2.5,
            amplitude: 0.03,
        })
        .with_vertex_effect(VertexEffect::Pulse {
            frequency: 4.0,
            amplitude: 0.25,
        })
        .with_vertex_effect(VertexEffect::Wave {
            direction: Vec3::Y,
            frequency: 5.0,
            speed: 2.0,
            amplitude: 0.02,
        })
        // Simulation rules
        .with_rule(Rule::AttractTo {
            point: Vec3::ZERO,
            strength: 0.2,
        })
        .with_rule(Rule::Custom(
            r#"
            let r = length(p.position.xz);
            let swirl = 0.3 / (r + 0.15);
            p.velocity += vec3<f32>(-p.position.z, 0.0, p.position.x) * swirl * uniforms.delta_time;
            "#
            .into(),
        ))
        .with_rule(Rule::Drag(0.4))
        .with_rule(Rule::SpeedLimit { min: 0.0, max: 0.8 })
        .with_rule(Rule::BounceWalls)
        .run().expect("Simulation failed");
}
